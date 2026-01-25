use jiff::{SignedDuration, SpanTotal, Timestamp, Unit, Zoned, civil::date};

pub struct Sun {
    pub sunrise: Zoned,
    pub sunset: Zoned,
}

fn julian_to_timestamp(julian: f64) -> Timestamp {
    Timestamp::UNIX_EPOCH
        + SignedDuration::from_secs_f64((julian - 2440587.5) * 86400.0)
}

pub fn calculate_sun(
    today: Zoned,
    latitude: f64,
    longitude: f64,
    elevation: f64,
) -> Sun {
    let tz = today.time_zone().clone();

    let midnight_jan_1_2020 = date(2000, 1, 1)
        .at(0, 0, 0, 0)
        .to_zoned(tz.clone())
        .unwrap();
    let days_since_jan_1_2020 = (today - midnight_jan_1_2020)
        .total(SpanTotal::from(Unit::Day).days_are_24_hours())
        .unwrap()
        .floor();

    let mean_solar_time = days_since_jan_1_2020 + 0.0009 - longitude / 360.0;

    let solar_mean_anomaly_degrees =
        (357.5291 + 0.98560028 * mean_solar_time) % 360.0;
    let solar_mean_anomaly_radians = solar_mean_anomaly_degrees.to_radians();

    let center_degrees = 1.9148 * solar_mean_anomaly_radians.sin()
        + 0.02 * (2.0 * solar_mean_anomaly_radians).sin()
        + 0.0003 * (3.0 * solar_mean_anomaly_radians).sin();

    let ecliptic_longitude_degrees =
        (solar_mean_anomaly_degrees + center_degrees + 180.0 + 102.9372)
            % 360.0;
    let ecliptic_longitude_radians = ecliptic_longitude_degrees.to_radians();

    let solar_transit =
        2451545.0 + mean_solar_time + 0.0053 * solar_mean_anomaly_radians.sin()
            - 0.0069 * (2.0 * ecliptic_longitude_radians).sin();

    let sin_declination =
        ecliptic_longitude_radians.sin() * f64::to_radians(23.4397).sin();
    let cos_declination = sin_declination.asin().cos();

    let some_cos = ((-0.833 - 2.076 * elevation.sqrt() / 60.0)
        .to_radians()
        .sin()
        - latitude.to_radians().sin() * sin_declination)
        / (latitude.to_radians().cos() * cos_declination);

    let w0_radians = some_cos.acos();
    let w0_degrees = w0_radians.to_degrees();

    let sunrise = solar_transit - w0_degrees / 360.0;
    let sunset = solar_transit + w0_degrees / 360.0;

    Sun {
        sunrise: julian_to_timestamp(sunrise).to_zoned(tz.clone()),
        sunset: julian_to_timestamp(sunset).to_zoned(tz),
    }
}
