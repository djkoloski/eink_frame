use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Points {
    pub properties: PointsProperties,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PointsProperties {
    pub grid_id: String,
    pub grid_x: i32,
    pub grid_y: i32,
    pub relative_location: RelativeLocation,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelativeLocation {
    pub properties: RelativeLocationProperties,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelativeLocationProperties {
    pub city: String,
    pub state: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Forecast {
    pub properties: ForecastProperties,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForecastProperties {
    pub elevation: ForecastUnit,
    pub periods: Vec<ForecastPeriod>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForecastPeriod {
    #[expect(unused)]
    pub number: u32,
    #[expect(unused)]
    pub name: String,
    pub start_time: String,
    #[expect(unused)]
    pub end_time: String,
    #[expect(unused)]
    pub is_daytime: bool,
    pub temperature: f32,
    pub probability_of_precipitation: ForecastUnit,
    #[expect(unused)]
    pub dewpoint: Option<ForecastUnit>,
    #[expect(unused)]
    pub relative_humidity: Option<ForecastUnit>,
    #[expect(unused)]
    pub wind_speed: String,
    #[expect(unused)]
    pub wind_direction: String,
    #[expect(unused)]
    pub short_forecast: String,
    #[expect(unused)]
    pub detailed_forecast: String,
    pub icon: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForecastUnit {
    pub value: f32,
}
