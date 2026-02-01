mod app;
mod chart;
mod config;
mod de;
mod screens;
mod sunrise;

use anyhow::Result;

use self::config::Config;
use crate::app::App;

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = App::start().await?;
    app.run().await
}
