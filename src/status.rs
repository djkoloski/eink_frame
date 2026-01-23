use anyhow::Result;
use inky::{Color, Inky};
use inky_graphics::{Alignment, Graphics};
use jiff::{Unit, Zoned};
use tokio::process::Command;

pub struct Data {
    hostname: String,
    wifi: Option<Wifi>,
}

struct Wifi {
    ssid: String,
    signal: u32,
}

pub async fn update() -> Result<Data> {
    let output = Command::new("nmcli")
        .args(["-g", "SSID,SIGNAL", "device", "wifi"])
        .output()
        .await?;
    let output = str::from_utf8(&output.stdout)?.trim();
    let wifi = output.split_once(':').map(|(ssid, signal)| Wifi {
        ssid: ssid.to_string(),
        signal: signal.parse().unwrap(),
    });

    Ok(Data {
        hostname: hostname::get()
            .map(|h| h.into_string().unwrap())
            .unwrap_or("???".to_string()),
        wifi,
    })
}

pub fn render(inky: &mut Inky, graphics: &Graphics, data: &Data) {
    let now = Zoned::now().round(Unit::Minute).unwrap();

    // Status bar
    graphics.draw_rect(
        inky,
        0,
        0,
        inky.resolution_x() as i32,
        30,
        Color::Black,
    );

    // Hostname
    graphics.draw_text(
        inky,
        10,
        20,
        &data.hostname,
        Alignment::Left,
        "helvR12",
        Color::White,
    );

    // Update time
    let last_updated = now.strftime("Last updated at %-I:%M %p").to_string();
    graphics.draw_text(
        inky,
        inky.resolution_x() as i32 / 2,
        20,
        &last_updated,
        Alignment::Center,
        "helvB12",
        Color::White,
    );

    // WiFi status
    let wifi_status;
    let bars;
    if let Some(wifi) = &data.wifi {
        wifi_status = format!("{}", wifi.ssid);
        bars = wifi.signal.div_ceil(25) as i32;
    } else {
        wifi_status = format!("WiFi disconnected");
        bars = 0;
    }
    graphics.draw_text(
        inky,
        inky.resolution_x() as i32 - 40,
        20,
        &wifi_status,
        Alignment::Right,
        "helvR12",
        Color::White,
    );
    for i in 0..4 {
        const BAR_WIDTH: i32 = 4;
        const BAR_SPACING: i32 = 2;
        const BAR_BORDER: i32 = 1;

        let x =
            inky.resolution_x() as i32 - 10 - 4 * BAR_WIDTH - 3 * BAR_SPACING
                + i * BAR_WIDTH
                + i * BAR_SPACING;
        let y = 22 - (i + 1) * BAR_WIDTH;
        let height = (i + 1) * BAR_WIDTH;
        if i < bars {
            graphics.draw_rect(inky, x, y, BAR_WIDTH, height, Color::White);
        } else {
            graphics.draw_box(
                inky,
                x,
                y,
                BAR_WIDTH,
                height,
                BAR_BORDER,
                Color::White,
            );
        }
    }
}
