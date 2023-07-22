mod chip8;
mod display;
use winit::{
    event::{Event, WindowEvent}
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
    .with_resizable(false)
    .build(&event_loop)
    .unwrap();

    let mut chip8 = chip8::Chip8::new(&window);

    chip8.load_program(rom_data);

    let start_time = time::Instant::now();
    
    event_loop.run(move |event, _, control_flow| {
        control_flow.set_wait();

        chip8.cycle();

        match event {
            Event::WindowEvent {
                event,
                window_id,
            } if window_id == window.id() => {
                match  event {
                    WindowEvent::CloseRequested => {control_flow.set_exit();},
                    _ => {}
                }
            },
            Event::RedrawRequested(..) => {chip8.render();},
            _ => (),
        }
    });
}
