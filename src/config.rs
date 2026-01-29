use jiff::Zoned;
use serde::Deserialize;

#[derive(Clone, Copy, Deserialize)]
pub struct Location {
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Clone, Deserialize)]
pub struct Birthday {
    pub name: String,
    pub date: Zoned,
}

#[derive(Deserialize)]
pub struct Config {
    pub graphics_resources: String,
    pub screens: Vec<String>,
    pub location: Location,
    pub birthdays: Vec<Birthday>,
    pub nasa_api_key: String,
    pub weather_update_interval_secs: f64,
    pub calendar_update_interval_secs: f64,
}
