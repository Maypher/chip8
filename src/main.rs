mod chip8;
mod display;
mod keyboard;
use winit::{
    event::{Event, WindowEvent, KeyboardInput, VirtualKeyCode}
};

use std::time;

use rfd::AsyncFileDialog;

fn main() {
    let rom = pollster::block_on(AsyncFileDialog::new().set_directory("./").add_filter("chip8", &["ch8"]).pick_file());

    let rom_data = pollster::block_on(rom.unwrap().read());

    let event_loop = winit::event_loop::EventLoop::new();

    let window = winit::window::WindowBuilder::new()
    .with_title("Chip 8")
    .with_inner_size(winit::dpi::Size::Physical(winit::dpi::PhysicalSize::new(1080, 540)))
    .build(&event_loop)
    .unwrap();

    let mut chip8 = chip8::Chip8::new(&window);

    chip8.load_program(rom_data);

    let instructions_per_frame: u64 = 60;
    
    let sound_interval = std::time::Instant::now();

    event_loop.run(move |event, _, control_flow| {
        if sound_interval.elapsed() >= std::time::Duration::from_secs_f32(0.016) {
            chip8.tick_timers();
        }

        control_flow.set_wait_timeout(std::time::Duration::from_millis(1000 / instructions_per_frame));
        
        match event {
            Event::WindowEvent {
                event,
                ..
            } => {
                match event {
                    WindowEvent::CloseRequested => {control_flow.set_exit();},
                    WindowEvent::KeyboardInput { 
                        input: KeyboardInput {
                            virtual_keycode: Some(VirtualKeyCode::P),
                            ..
                        },
                        .. 
                    } => {
                        chip8.paused = !chip8.paused;
                    },
                    WindowEvent::KeyboardInput { 
                        input,
                        ..
                    } => {
                        if input.virtual_keycode.is_some() {
                            match input.state {
                                winit::event::ElementState::Pressed => {
                                    chip8.on_key_down(&input.virtual_keycode.unwrap());
                                },
                                winit::event::ElementState::Released => {
                                    chip8.on_key_up(&input.virtual_keycode.unwrap());
                                }
                            }
                        }
                    },
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        chip8.handle_resize(new_inner_size);
                    },
                    WindowEvent::Resized(new_inner_size) => {
                        chip8.handle_resize(&new_inner_size);
                    }
                    _ => {}
                }

            },
            Event::MainEventsCleared => {
                for _ in 0..instructions_per_frame {
                    chip8.cycle();
                }
            },
            _ => (),
        }
    });
}
