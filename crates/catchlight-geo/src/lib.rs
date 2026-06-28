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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reverse_geocode_returns_none_before_init() {
        assert!(reverse_geocode(39.9042, 116.4074).is_none());
        assert!(reverse_geocode(0.0, 0.0).is_none());
    }

    #[test]
    fn location_serde_roundtrip() {
        let loc = Location {
            latitude: 39.9042,
            longitude: 116.4074,
            city: Some("Beijing".into()),
            country: Some("China".into()),
            display_name: "Beijing, China".into(),
        };

        let json = serde_json::to_string(&loc).unwrap();
        let back: Location = serde_json::from_str(&json).unwrap();
        assert_eq!(back.city.as_deref(), Some("Beijing"));
        assert!((back.latitude - 39.9042).abs() < 0.001);
    }

    #[test]
    fn location_without_city() {
        let loc = Location {
            latitude: 0.0,
            longitude: 0.0,
            city: None,
            country: None,
            display_name: "Unknown".into(),
        };
        assert!(loc.city.is_none());
        assert_eq!(loc.display_name, "Unknown");
    }
}
