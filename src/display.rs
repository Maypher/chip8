use wgpu::util::DeviceExt;
use pollster;
use bytemuck::{Pod, Zeroable};

const PIXEL_VERTICES: [f32; 12] = [
  // first triangle: top left -> bottom left -> top right
  0.0, 1.0,
  0.0, 0.0,
  1.0, 1.0,
  // second triangle: bottom left -> bottom right -> top right
  0.0, 0.0,
  1.0, 0.0,
  1.0, 1.0
];


#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
struct Instance {
    pos: [f32; 2],
    on: f32
}

impl Instance {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;

        wgpu::VertexBufferLayout { 
            array_stride: mem::size_of::<Instance>() as u64, 
            step_mode: wgpu::VertexStepMode::Instance, 
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 0,
                    shader_location: 1
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32,
                    offset: mem::size_of::<[f32; 2]>() as u64,
                    shader_location: 2
                }
            ]
        }
    }
}

pub struct Display {
    pixels: [[bool; Display::WIDTH]; Display::HEIGHT], // Each row (Display::HEIGHT) will have Display::WIDTH columns in it
    pub scale: u8,
    surface: wgpu::Surface,
    //surface_config: wgpu::SurfaceConfiguration,
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
}

impl Display {
    pub const WIDTH: usize = 64;
    pub const HEIGHT: usize = 32;

    pub fn new(window: &winit::window::Window) -> Self {
        let wgpu_instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: Default::default()
        });

        let surface = unsafe { wgpu_instance.create_surface(window) }.unwrap();

        let adapter = pollster::block_on(wgpu_instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface)
        })).unwrap();

        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("Chip 8 device and queue"),
            features: wgpu::Features::empty(),
            limits: wgpu::Limits::default()
        }, None)).unwrap();

        let surface_caps = surface.get_capabilities(&adapter);

        let window_size = window.inner_size();

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_caps.formats[0],
            width: window_size.width,
            height: window_size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![]
        };

        surface.configure(&device, &surface_config);

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Pixel vertex buffer"),
            contents: bytemuck::bytes_of(&PIXEL_VERTICES),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST
        });

        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Pixel instance buffer"),
            size: (std::mem::size_of::<Instance>() * Self::WIDTH * Self::HEIGHT) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Chip8 pipeline layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[]
        });

        let shader_src = wgpu::include_wgsl!("./shaders.wgsl");
        let shader_module = device.create_shader_module(shader_src);

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Chip 8 pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: "vs_main",
                buffers: &[
                    wgpu::VertexBufferLayout{
                    array_stride: 2 * 4,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2]
                }, Instance::desc()]
            },
            primitive: wgpu::PrimitiveState { 
                topology: wgpu::PrimitiveTopology::TriangleList, 
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw, 
                cull_mode: Some(wgpu::Face::Back), 
                unclipped_depth: false, 
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState { count: 1, mask: !0, alpha_to_coverage_enabled: false },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module, 
                entry_point: "fs_main", 
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Bgra8UnormSrgb,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL
                })] 
            }),
            multiview: None
        });

        Display { 
            pixels: [[false; Display::WIDTH]; Display::HEIGHT],
            scale: 1,
            surface,
           // surface_config,
            device,
            queue,
            pipeline,
            instance_buffer,
            vertex_buffer,
        }
    }

    pub fn clear_screen(&mut self) {
        self.pixels = [[false; Display::WIDTH]; Display::HEIGHT]
    }

    pub fn get_pixel(&self, x: usize, y: usize) -> bool {
        self.pixels[y][x]
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, on: bool) {
        self.pixels[y][x] = on;
    }

    pub fn draw(&mut self, starting_x: u8, starting_y: u8, memory: &[u8]) -> bool {
        let mut pixel_turned_off = false;

        for (byte_number, block) in memory.iter().enumerate() {
            let y = (starting_y as usize + byte_number) % Display::HEIGHT;

            for bit_number in 0..8 {
                let x = (starting_x as usize + bit_number) % Display::WIDTH;
                let current_pixel = self.pixels[y][x] as u8;

                let current_bit = (block >> (7 - bit_number)) & 1;
                let new_pixel = current_bit ^ current_pixel;

                self.pixels[y][x] = new_pixel != 0;

                pixel_turned_off = current_pixel == 1 && new_pixel == 0;
            }
        }
        pixel_turned_off
    }

    // pub fn resize(&mut self, new_size: &winit::dpi::PhysicalSize<u32>) {
    //     if new_size.width > 0 && new_size.height > 0 {
    //         self.surface_config.width = new_size.width;
    //         self.surface_config.height = new_size.height;

    //         self.surface.configure(&self.device, &self.surface_config);
    //     }
    // }

    fn gen_instances(&self) -> [Instance; Display::WIDTH * Display::HEIGHT] {
        let mut instances = [Instance {pos: [0.0, 0.0], on: 0.0}; Display::WIDTH * Display::HEIGHT];

        for y in 0..Display::HEIGHT {
            for x in 0..Display::WIDTH {
                
                instances[Display::WIDTH * y + x] = Instance {
                    pos: [x as f32, y as f32],
                    on: self.pixels[y][x] as u32 as f32
                };
            }
        }

        instances
    }

    pub fn render(&self) {
        let frame = self.surface.get_current_texture().unwrap();
        let frame_view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());

        self.queue.write_buffer(&self.instance_buffer, 0, bytemuck::bytes_of(&self.gen_instances()));

        let mut command_encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Chip 8 command enconder")
        });

        {
            let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Chip 8 Render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &frame_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: true
                    }
                })],
                depth_stencil_attachment: None
            });
            
            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            render_pass.draw(0..6, 0..(Display::WIDTH as u32 * Display::HEIGHT as u32));

        }

        self.queue.submit(Some(command_encoder.finish()));
        frame.present();
    }
}