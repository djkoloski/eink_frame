use core::{
    sync::atomic::{AtomicUsize, Ordering},
    task::Poll,
};
use std::{env, pin::Pin, sync::Arc, task::Context};

use anyhow::{Result, bail};
use futures::task::AtomicWaker;
use inky::{Button, Inky};
use inky_graphics::Graphics;
use reqwest::Client;
use tokio::{fs, select};

use crate::{Config, screens::*};

pub trait Screen {
    fn new(config: &Config, client: &Client) -> Self;
    fn render(&mut self, inky: &mut Inky, graphics: &Graphics);
    async fn updated(&mut self);
}

pub enum Screens {
    Weather(Weather),
    Calendar(Calendar),
    Astronomy(Astronomy),
}

impl Screens {
    pub fn new(name: &str, config: &Config, client: &Client) -> Result<Self> {
        Ok(match name {
            "weather" => Self::Weather(Weather::new(config, client)),
            "calendar" => Self::Calendar(Calendar::new(config, client)),
            "astronomy" => Self::Astronomy(Astronomy::new(config, client)),
            _ => bail!("invalid screen '{name}'"),
        })
    }

    pub fn render(&mut self, inky: &mut Inky, graphics: &Graphics) {
        match self {
            Self::Weather(screen) => screen.render(inky, graphics),
            Self::Calendar(screen) => screen.render(inky, graphics),
            Self::Astronomy(screen) => screen.render(inky, graphics),
        }
    }

    pub async fn updated(&mut self) {
        match self {
            Self::Weather(screen) => screen.updated().await,
            Self::Calendar(screen) => screen.updated().await,
            Self::Astronomy(screen) => screen.updated().await,
        }
    }
}

const CONTROL_STATE_BITS: usize = 1;
const CONTROL_SWITCHING_BIT: usize = 1;

struct Control {
    state: AtomicUsize,
    waker: AtomicWaker,
}

impl Control {
    fn new(screen: usize) -> Self {
        Self {
            state: AtomicUsize::new(
                screen << CONTROL_STATE_BITS | CONTROL_SWITCHING_BIT,
            ),
            waker: AtomicWaker::new(),
        }
    }

    fn try_switch(&self, screen: usize) {
        let state = self.state.load(Ordering::Relaxed);
        let is_switching = state & CONTROL_SWITCHING_BIT != 0;
        let current = state >> CONTROL_STATE_BITS;
        if is_switching || current == screen {
            return;
        }

        let result = self.state.compare_exchange(
            state,
            screen << CONTROL_STATE_BITS | CONTROL_SWITCHING_BIT,
            Ordering::Relaxed,
            Ordering::Relaxed,
        );
        if result.is_ok() {
            self.waker.wake();
        }
    }

    fn wait_for_switch(&self) -> SwitchFuture<'_> {
        self.state
            .fetch_and(!CONTROL_SWITCHING_BIT, Ordering::Relaxed);
        SwitchFuture { control: self }
    }
}

struct SwitchFuture<'a> {
    control: &'a Control,
}

impl<'a> Future for SwitchFuture<'a> {
    type Output = usize;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.control.waker.register(cx.waker());
        let result = self.control.state.load(Ordering::Acquire);
        if result & CONTROL_SWITCHING_BIT != 0 {
            Poll::Ready(result >> CONTROL_STATE_BITS)
        } else {
            Poll::Pending
        }
    }
}

pub struct App {
    inky: Inky,
    graphics: Graphics,

    control: Arc<Control>,

    status: Status,
    screens: Vec<Screens>,
}

impl App {
    pub async fn start() -> Result<Self> {
        let config_path =
            env::args().nth(1).unwrap_or("config.json".to_string());
        let config = fs::read_to_string(&config_path).await?;
        let config = serde_json::from_str::<Config>(&config)?;

        let graphics = Graphics::new(&config.graphics_resources)?;
        let client = Client::new();

        let control = Arc::new(Control::new(0));

        let inky = Inky::new(Self::button_handler(control.clone())).await?;

        let status = Status::new(&config, &client);
        let mut screens = Vec::with_capacity(config.screens.len());
        for screen in config.screens.iter() {
            screens.push(Screens::new(screen, &config, &client)?);
        }

        Ok(Self {
            inky,
            graphics,

            control,

            status,
            screens,
        })
    }

    fn button_handler(control: Arc<Control>) -> impl FnMut(Button) {
        move |button| match button {
            Button::A => control.try_switch(0),
            Button::B => control.try_switch(1),
            Button::C => control.try_switch(2),
            Button::D => {
                // TODO: hard refresh
            }
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut active = 0;
        loop {
            let current = &mut self.screens[active];
            // current.initialized().await;

            self.inky.clear();

            current.render(&mut self.inky, &self.graphics);
            self.status.render(&mut self.inky, &self.graphics);

            self.inky.show().await?;

            select! {
                next = self.control.wait_for_switch() => {
                    active = next;
                }
                _ = current.updated() => {}
            }
        }
    }
}
