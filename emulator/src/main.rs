use core::{num::NonZeroU32, slice};
use std::{
    env::temp_dir,
    fs::{File, OpenOptions},
    io::{Read, Seek as _, SeekFrom, Write},
    rc::Rc,
};

use anyhow::Result;
use softbuffer::{Context, Surface};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop, OwnedDisplayHandle},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowButtons, WindowId},
};

const RESOLUTION_X: usize = 800;
const RESOLUTION_Y: usize = 480;

struct State {
    window: Rc<Window>,
    #[expect(unused)]
    context: Context<OwnedDisplayHandle>,
    surface: Surface<OwnedDisplayHandle, Rc<Window>>,
}

struct App {
    file: File,
    state: Option<State>,
    events: u8,
}

impl App {
    fn new() -> Result<Self> {
        let mut path = temp_dir();
        path.push("inky.buf");

        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(&path)?;
        println!("opened buffer at {}", path.display());

        Ok(Self {
            file,
            state: None,
            events: 0,
        })
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Rc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("Inky Emulator")
                        .with_inner_size(LogicalSize::new(
                            RESOLUTION_X as f64,
                            RESOLUTION_Y as f64,
                        ))
                        .with_resizable(false)
                        .with_enabled_buttons(
                            WindowButtons::CLOSE | WindowButtons::MINIMIZE,
                        ),
                )
                .unwrap(),
        );

        let context = Context::new(event_loop.owned_display_handle()).unwrap();
        let mut surface = Surface::new(&context, window.clone()).unwrap();

        surface
            .resize(
                NonZeroU32::new(RESOLUTION_X as u32).unwrap(),
                NonZeroU32::new(RESOLUTION_Y as u32).unwrap(),
            )
            .unwrap();

        self.state = Some(State {
            window,
            context,
            surface,
        });
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                let state = self.state.as_mut().unwrap();

                self.file.lock().unwrap();

                if self.events != 0 {
                    self.file.rewind().unwrap();
                    let mut existing_events = 0;
                    self.file
                        .read_exact(slice::from_mut(&mut existing_events))
                        .unwrap();
                    self.file.rewind().unwrap();
                    self.file
                        .write_all(&[existing_events | self.events])
                        .unwrap();
                    self.events = 0;
                }

                let mut buffer = Vec::new();
                self.file.seek(SeekFrom::Start(1)).unwrap();
                self.file.read_to_end(&mut buffer).unwrap();

                self.file.unlock().unwrap();

                let mut display = state.surface.buffer_mut().unwrap();
                for (index, byte) in buffer.iter().skip(1).enumerate() {
                    display[index] = match byte {
                        0 => 0x000000,
                        1 => 0xffffff,
                        2 => 0xffff00,
                        3 => 0xff0000,
                        5 => 0x0000ff,
                        6 => 0x00ff00,
                        _ => panic!(),
                    };
                }

                // Render LED
                if buffer[0] != 0 {
                    let led_x = 100;
                    let led_y = 10;
                    for dy in 0..10 {
                        for dx in 0..10 {
                            let x = led_x + dx;
                            let y = led_y + dy;
                            display[x + y * RESOLUTION_X] = 0xffffff;
                        }
                    }
                }

                state.window.pre_present_notify();
                display.present().unwrap();

                state.window.request_redraw();
            }
            WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } => {
                if event.state == ElementState::Pressed && !event.repeat {
                    match event.physical_key {
                        PhysicalKey::Code(KeyCode::Digit1) => {
                            self.events |= 0b0001
                        }
                        PhysicalKey::Code(KeyCode::Digit2) => {
                            self.events |= 0b0010
                        }
                        PhysicalKey::Code(KeyCode::Digit3) => {
                            self.events |= 0b0100
                        }
                        PhysicalKey::Code(KeyCode::Digit4) => {
                            self.events |= 0b1000
                        }
                        _ => (),
                    }
                }
            }
            _ => (),
        }
    }
}

fn main() -> Result<()> {
    let event_loop = EventLoop::builder().build().unwrap();

    let mut app = App::new()?;
    event_loop.run_app(&mut app)?;

    Ok(())
}
