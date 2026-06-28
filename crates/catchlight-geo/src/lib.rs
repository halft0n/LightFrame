use reverse_geocoder::ReverseGeocoder;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub latitude: f64,
    pub longitude: f64,
    pub city: Option<String>,
    pub country: Option<String>,
    pub region: Option<String>,
    pub display_name: String,
}

static GEOCODER: OnceLock<ReverseGeocoder> = OnceLock::new();

fn geocoder() -> &'static ReverseGeocoder {
    GEOCODER.get_or_init(ReverseGeocoder::new)
}

fn build_display_name(
    city: &Option<String>,
    region: &Option<String>,
    country: &Option<String>,
) -> String {
    let mut parts = Vec::new();
    if let Some(c) = city
        && !c.is_empty()
    {
        parts.push(c.as_str());
    }
    if let Some(r) = region
        && !r.is_empty()
        && !parts.contains(&r.as_str())
    {
        parts.push(r.as_str());
    }
    if let Some(c) = country
        && !c.is_empty()
    {
        parts.push(c.as_str());
    }
    if parts.is_empty() {
        "Unknown".to_string()
    } else {
        parts.join(", ")
    }
}

pub fn reverse_geocode(lat: f64, lon: f64) -> Option<Location> {
    let result = geocoder().search((lat, lon));
    let record = result.record;

    let city = if record.name.is_empty() {
        None
    } else {
        Some(record.name.clone())
    };
    let region = if record.admin1.is_empty() {
        None
    } else {
        Some(record.admin1.clone())
    };
    let country = if record.cc.is_empty() {
        None
    } else {
        Some(record.cc.clone())
    };

    let display_name = build_display_name(&city, &region, &country);

    Some(Location {
        latitude: lat,
        longitude: lon,
        city,
        country,
        region,
        display_name,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reverse_geocode_beijing() {
        let loc = reverse_geocode(39.9042, 116.4074).expect("should resolve");
        assert!(loc.country.is_some());
        assert!(!loc.display_name.is_empty());
    }

    #[test]
    fn location_serde_roundtrip() {
        let loc = Location {
            latitude: 39.9042,
            longitude: 116.4074,
            city: Some("Beijing".into()),
            country: Some("CN".into()),
            region: Some("Beijing".into()),
            display_name: "Beijing, CN".into(),
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
            region: None,
            display_name: "Unknown".into(),
        };
        assert!(loc.city.is_none());
        assert_eq!(loc.display_name, "Unknown");
    }
}
