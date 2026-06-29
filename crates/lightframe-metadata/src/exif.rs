use crate::PhotoMetadata;
use lightframe_core::Result;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

pub fn extract_exif(path: &Path) -> Result<PhotoMetadata> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    let exif_reader = exif::Reader::new();
    let exif = match exif_reader.read_from_container(&mut reader) {
        Ok(exif) => exif,
        Err(_) => return Ok(PhotoMetadata::default()),
    };

    let mut meta = PhotoMetadata::default();

    if let Some(field) = exif.get_field(exif::Tag::PixelXDimension, exif::In::PRIMARY)
        && let Some(v) = field.value.get_uint(0)
    {
        meta.width = Some(v);
    }

    if let Some(field) = exif.get_field(exif::Tag::PixelYDimension, exif::In::PRIMARY)
        && let Some(v) = field.value.get_uint(0)
    {
        meta.height = Some(v);
    }

    if let Some(field) = exif.get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY) {
        let val = field.display_value().to_string();
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&val, "%Y-%m-%d %H:%M:%S") {
            meta.date_taken = Some(dt);
        }
    }

    if let Some(field) = exif.get_field(exif::Tag::Make, exif::In::PRIMARY) {
        meta.camera_make = Some(
            field
                .display_value()
                .to_string()
                .trim_matches('"')
                .to_string(),
        );
    }

    if let Some(field) = exif.get_field(exif::Tag::Model, exif::In::PRIMARY) {
        meta.camera_model = Some(
            field
                .display_value()
                .to_string()
                .trim_matches('"')
                .to_string(),
        );
    }

    if let Some(field) = exif.get_field(exif::Tag::ISOSpeed, exif::In::PRIMARY)
        && let Some(v) = field.value.get_uint(0)
    {
        meta.iso = Some(v);
    }

    if let Some(field) = exif.get_field(exif::Tag::Orientation, exif::In::PRIMARY)
        && let Some(v) = field.value.get_uint(0)
    {
        meta.orientation = Some(v as u16);
    }

    match extract_gps(&exif) {
        Ok((lat, lon)) => {
            meta.latitude = Some(lat);
            meta.longitude = Some(lon);
        }
        Err(_) => {
            tracing::debug!(path = %path.display(), "no GPS data found");
        }
    }

    Ok(meta)
}

fn extract_gps(exif: &exif::Exif) -> std::result::Result<(f64, f64), ()> {
    let lat = exif
        .get_field(exif::Tag::GPSLatitude, exif::In::PRIMARY)
        .ok_or(())?;
    let lat_ref = exif
        .get_field(exif::Tag::GPSLatitudeRef, exif::In::PRIMARY)
        .ok_or(())?;
    let lon = exif
        .get_field(exif::Tag::GPSLongitude, exif::In::PRIMARY)
        .ok_or(())?;
    let lon_ref = exif
        .get_field(exif::Tag::GPSLongitudeRef, exif::In::PRIMARY)
        .ok_or(())?;

    let lat_val = parse_dms(&lat.value).ok_or(())?;
    let lon_val = parse_dms(&lon.value).ok_or(())?;

    let lat_sign = if lat_ref.display_value().to_string().contains('S') {
        -1.0
    } else {
        1.0
    };
    let lon_sign = if lon_ref.display_value().to_string().contains('W') {
        -1.0
    } else {
        1.0
    };

    Ok((lat_val * lat_sign, lon_val * lon_sign))
}

fn parse_dms(value: &exif::Value) -> Option<f64> {
    match value {
        exif::Value::Rational(v) if v.len() >= 3 => {
            let d = v[0].to_f64();
            let m = v[1].to_f64();
            let s = v[2].to_f64();
            Some(d + m / 60.0 + s / 3600.0)
        }
        _ => None,
    }
}
