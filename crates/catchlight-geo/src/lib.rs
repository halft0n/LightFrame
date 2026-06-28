use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub latitude: f64,
    pub longitude: f64,
    pub city: Option<String>,
    pub country: Option<String>,
    pub display_name: String,
}

pub fn reverse_geocode(_lat: f64, _lon: f64) -> Option<Location> {
    // TODO: integrate rrgeo with GeoNames cities1000.bin
    None
}
