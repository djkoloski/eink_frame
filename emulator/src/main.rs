use core::num::NonZeroU32;
use std::{
    env::temp_dir,
    fs::File,
    io::{Read, Seek as _, SeekFrom},
    rc::Rc,
};

use anyhow::Result;
use softbuffer::{Context, Surface};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop, OwnedDisplayHandle},
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
}

impl App {
    fn new() -> Result<Self> {
        let mut path = temp_dir();
        path.push("inky.buf");

        let file = File::open(&path)?;
        println!("opened buffer at {}", path.display());

        Ok(Self { file, state: None })
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

                let mut buffer = Vec::new();
                self.file.seek(SeekFrom::Start(0)).unwrap();
                self.file.read_to_end(&mut buffer).unwrap();

                self.file.unlock().unwrap();

                let mut display = state.surface.buffer_mut().unwrap();
                for (index, byte) in buffer.iter().enumerate() {
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
                state.window.pre_present_notify();
                display.present().unwrap();

                state.window.request_redraw();
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
