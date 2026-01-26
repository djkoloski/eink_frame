mod chart;
mod screens;
mod sunrise;

use core::{
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};
use std::{
    env,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use anyhow::Result;
use futures::task::AtomicWaker;
use inky::{Button, Inky};
use inky_graphics::Graphics;
use reqwest::Client;
use serde::Deserialize;
use tokio::{fs, join, select, time::sleep};

use self::screens::{calendar, status, weather};

#[derive(Deserialize)]
struct Config {
    update_interval_secs: f64,
    graphics: inky_graphics::Config,
    weather: screens::weather::Config,
    calendar: screens::calendar::Config,
}

#[derive(Clone, Copy, Debug)]
enum Screen {
    Weather = 0,
    Calendar = 1,
}

impl Screen {
    fn from_int(int: usize) -> Option<Self> {
        Some(match int {
            0 => Screen::Weather,
            1 => Screen::Calendar,
            _ => return None,
        })
    }
}

struct Screens {
    status: status::Status,
    weather: weather::Weather,
    calendar: calendar::Calendar,
}

impl Screens {
    async fn update(&mut self, client: &Client) {
        join!(
            self.status.update(),
            self.weather.update(client),
            self.calendar.update(),
        );
    }

    fn render(&self, inky: &mut Inky, graphics: &Graphics, active: Screen) {
        match active {
            Screen::Weather => self.weather.render(inky, graphics),
            Screen::Calendar => self.calendar.render(inky, graphics),
        }

        self.status.render(inky, graphics);
    }

    fn force_refresh(&mut self) {
        self.weather.force_refresh();
    }
}

enum Refresh {
    Soft,
    Hard,
}

struct RefreshFuture<'a> {
    shared: &'a Shared,
}

impl Future for RefreshFuture<'_> {
    type Output = Refresh;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.shared.refresh_waker.register(cx.waker());
        match self.shared.refresh.swap(0, Ordering::Acquire) {
            0 => Poll::Pending,
            1 => Poll::Ready(Refresh::Soft),
            2 => Poll::Ready(Refresh::Hard),
            _ => unimplemented!(),
        }
    }
}

struct Shared {
    active: AtomicUsize,
    refresh: AtomicUsize,
    refresh_waker: AtomicWaker,
}

impl Shared {
    fn new(active: Screen) -> Arc<Self> {
        Arc::new(Self {
            active: AtomicUsize::new(active as usize),
            refresh: AtomicUsize::new(0),
            refresh_waker: AtomicWaker::new(),
        })
    }

    fn active(&self) -> Screen {
        Screen::from_int(self.active.load(Ordering::Acquire)).unwrap()
    }

    fn set_active(&self, active: Screen) {
        let next = active as usize;
        if self.active.swap(next, Ordering::Acquire) != next {
            self.soft_refresh();
        }
    }

    fn clear_refresh(&self, active: Screen) {
        self.active.store(active as usize, Ordering::Release);
        self.refresh.store(0, Ordering::Release);
    }

    fn soft_refresh(&self) {
        self.refresh.store(1, Ordering::Release);
        self.refresh_waker.wake();
    }

    fn hard_refresh(&self) {
        self.refresh.store(2, Ordering::Release);
        self.refresh_waker.wake();
    }

    fn on_refresh(&self) -> RefreshFuture<'_> {
        RefreshFuture { shared: self }
    }
}

struct App {
    update_interval_secs: f64,

    inky: Inky,
    graphics: Graphics,
    client: Client,

    shared: Arc<Shared>,
    screens: Screens,
}

impl App {
    async fn start() -> Result<Self> {
        let config_path =
            env::args().nth(1).unwrap_or("config.json".to_string());
        let config = fs::read_to_string(&config_path).await?;
        let config = serde_json::from_str::<Config>(&config)?;

        let shared = Shared::new(Screen::Weather);
        let button_shared = shared.clone();

        let inky = Inky::new(move |button| match button {
            Button::A => button_shared.set_active(Screen::Weather),
            Button::B => button_shared.set_active(Screen::Calendar),
            Button::D => button_shared.hard_refresh(),
            _ => (),
        })
        .await?;
        let graphics = Graphics::new(&config.graphics)?;
        let client = Client::new();

        Ok(Self {
            update_interval_secs: config.update_interval_secs,

            inky,
            graphics,
            client,

            shared,
            screens: Screens {
                status: status::Status::new().await?,
                weather: weather::Weather::new(config.weather),
                calendar: calendar::Calendar::new(config.calendar),
            },
        })
    }

    async fn run(&mut self) -> Result<()> {
        loop {
            self.inky.set_led(true).await;

            self.screens.update(&self.client).await;

            self.inky.clear();

            let active = self.shared.active();
            self.screens.render(&mut self.inky, &self.graphics, active);

            self.inky.show().await?;

            // Clear any changes that occurred during rendering
            self.shared.clear_refresh(active);
            self.inky.set_led(false).await;

            select! {
                refresh = self.shared.on_refresh() => {
                    if matches!(refresh, Refresh::Hard) {
                        self.screens.force_refresh();
                    }
                }
                _ = sleep(Duration::from_secs_f64(self.update_interval_secs))
                    => ()
            };
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = App::start().await?;
    app.run().await
}
