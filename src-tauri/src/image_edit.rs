use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};
use serde::{Deserialize, Serialize};
use std::f32::consts::PI;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CropRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditParams {
    #[serde(default)]
    pub crop: Option<CropRect>,
    #[serde(default)]
    pub rotate: i32,
    #[serde(default)]
    pub straighten: f32,
    #[serde(default)]
    pub flip_h: bool,
    #[serde(default)]
    pub flip_v: bool,
    #[serde(default)]
    pub aspect_ratio: Option<String>,

    #[serde(default)]
    pub brightness: f32,
    #[serde(default)]
    pub contrast: f32,
    #[serde(default)]
    pub exposure: f32,
    #[serde(default)]
    pub highlights: f32,
    #[serde(default)]
    pub shadows: f32,
    #[serde(default)]
    pub brilliance: f32,
    #[serde(default)]
    pub black_point: f32,

    #[serde(default)]
    pub saturation: f32,
    #[serde(default)]
    pub vibrance: f32,
    #[serde(default)]
    pub warmth: f32,
    #[serde(default)]
    pub tint: f32,

    #[serde(default)]
    pub sharpness: f32,
    #[serde(default)]
    pub definition: f32,
    #[serde(default)]
    pub noise_reduction: f32,

    #[serde(default)]
    pub vignette: f32,
    #[serde(default)]
    pub vignette_radius: f32,
    #[serde(default)]
    pub grain: f32,

    #[serde(default)]
    pub bw_intensity: f32,
    #[serde(default)]
    pub bw_tone: f32,
}

impl Default for EditParams {
    fn default() -> Self {
        Self {
            crop: None,
            rotate: 0,
            straighten: 0.0,
            flip_h: false,
            flip_v: false,
            aspect_ratio: None,
            brightness: 0.0,
            contrast: 0.0,
            exposure: 0.0,
            highlights: 0.0,
            shadows: 0.0,
            brilliance: 0.0,
            black_point: 0.0,
            saturation: 0.0,
            vibrance: 0.0,
            warmth: 0.0,
            tint: 0.0,
            sharpness: 0.0,
            definition: 0.0,
            noise_reduction: 0.0,
            vignette: 0.0,
            vignette_radius: 50.0,
            grain: 0.0,
            bw_intensity: 0.0,
            bw_tone: 0.0,
        }
    }
}

pub fn parse_edit_params(json: &str) -> Result<EditParams, String> {
    serde_json::from_str(json).map_err(|e| format!("invalid edit params: {e}"))
}

pub fn export_edited_image(
    src_path: &Path,
    output_path: &Path,
    params_json: &str,
    quality: u8,
) -> Result<(), String> {
    let params = parse_edit_params(params_json)?;
    let img = image::open(src_path).map_err(|e| format!("failed to open image: {e}"))?;
    let edited = apply_edits(img, &params);
    save_jpeg(&edited, output_path, quality)
}

pub fn apply_edits(mut img: DynamicImage, params: &EditParams) -> DynamicImage {
    if let Some(crop) = &params.crop {
        if crop.width > 0.0 && crop.height > 0.0 {
            let (w, h) = img.dimensions();
            let x = (crop.x * w as f32).clamp(0.0, w as f32 - 1.0) as u32;
            let y = (crop.y * h as f32).clamp(0.0, h as f32 - 1.0) as u32;
            let cw = (crop.width * w as f32).clamp(1.0, w as f32 - x as f32) as u32;
            let ch = (crop.height * h as f32).clamp(1.0, h as f32 - y as f32) as u32;
            img = img.crop_imm(x, y, cw, ch);
        }
    }

    let rotate_norm = params.rotate.rem_euclid(360);
    img = match rotate_norm {
        90 => img.rotate90(),
        180 => img.rotate180(),
        270 => img.rotate270(),
        _ => img,
    };

    if params.straighten.abs() > 0.01 {
        img = rotate_by_degrees(img, params.straighten);
    }

    if params.flip_h {
        img = img.fliph();
    }
    if params.flip_v {
        img = img.flipv();
    }

    let mut rgba = img.to_rgba8();
    apply_pixel_adjustments(&mut rgba, params);
    DynamicImage::ImageRgba8(rgba)
}

fn rotate_by_degrees(img: DynamicImage, degrees: f32) -> DynamicImage {
    let src = img.to_rgba8();
    let (sw, sh) = src.dimensions();
    let rad = degrees * PI / 180.0;
    let cos_a = rad.cos();
    let sin_a = rad.sin();

    let cx = sw as f32 / 2.0;
    let cy = sh as f32 / 2.0;

    let corners = [
        (-cx, -cy),
        (sw as f32 - cx, -cy),
        (-cx, sh as f32 - cy),
        (sw as f32 - cx, sh as f32 - cy),
    ];
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;
    for (x, y) in corners {
        let rx = x * cos_a - y * sin_a;
        let ry = x * sin_a + y * cos_a;
        min_x = min_x.min(rx);
        min_y = min_y.min(ry);
        max_x = max_x.max(rx);
        max_y = max_y.max(ry);
    }

    let dw = (max_x - min_x).ceil() as u32;
    let dh = (max_y - min_y).ceil() as u32;
    let mut dst = RgbaImage::from_pixel(dw, dh, Rgba([0, 0, 0, 0]));

    let dcx = dw as f32 / 2.0;
    let dcy = dh as f32 / 2.0;

    for y in 0..dh {
        for x in 0..dw {
            let dx = x as f32 - dcx;
            let dy = y as f32 - dcy;
            let sx = dx * cos_a + dy * sin_a + cx;
            let sy = -dx * sin_a + dy * cos_a + cy;
            if sx >= 0.0 && sy >= 0.0 && sx < sw as f32 - 1.0 && sy < sh as f32 - 1.0 {
                let pixel = bilinear_sample(&src, sx, sy);
                dst.put_pixel(x, y, pixel);
            }
        }
    }

    DynamicImage::ImageRgba8(dst)
}

fn bilinear_sample(img: &RgbaImage, x: f32, y: f32) -> Rgba<u8> {
    let x0 = x.floor() as u32;
    let y0 = y.floor() as u32;
    let x1 = (x0 + 1).min(img.width() - 1);
    let y1 = (y0 + 1).min(img.height() - 1);
    let fx = x - x0 as f32;
    let fy = y - y0 as f32;

    let p00 = img.get_pixel(x0, y0).0;
    let p10 = img.get_pixel(x1, y0).0;
    let p01 = img.get_pixel(x0, y1).0;
    let p11 = img.get_pixel(x1, y1).0;

    let mut out = [0u8; 4];
    for i in 0..4 {
        let v = p00[i] as f32 * (1.0 - fx) * (1.0 - fy)
            + p10[i] as f32 * fx * (1.0 - fy)
            + p01[i] as f32 * (1.0 - fx) * fy
            + p11[i] as f32 * fx * fy;
        out[i] = v.clamp(0.0, 255.0) as u8;
    }
    Rgba(out)
}

fn apply_pixel_adjustments(img: &mut RgbaImage, params: &EditParams) {
    let (w, h) = img.dimensions();
    let brightness = 1.0 + params.brightness / 100.0;
    let contrast = 1.0 + params.contrast / 100.0;
    let exposure = 2.0_f32.powf(params.exposure / 100.0 * 0.5);
    let saturation = 1.0 + params.saturation / 100.0;
    let vibrance = params.vibrance / 100.0;
    let warmth = params.warmth / 100.0;
    let tint = params.tint / 100.0;
    let brilliance = params.brilliance / 100.0;
    let black_point = params.black_point / 100.0;
    let highlights = params.highlights / 100.0;
    let shadows = params.shadows / 100.0;
    let definition = params.definition / 100.0;
    let bw_intensity = params.bw_intensity / 100.0;
    let bw_tone = params.bw_tone / 100.0;
    let vignette = params.vignette / 100.0;
    let vignette_radius = (params.vignette_radius / 100.0).clamp(0.1, 1.0);
    let grain = params.grain / 100.0;

    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    let max_dist = ((cx * cx + cy * cy).sqrt()).max(1.0);

    let mut seed = w.wrapping_mul(7919).wrapping_add(h.wrapping_mul(104729));

    for y in 0..h {
        for x in 0..w {
            let pixel = img.get_pixel(x, y);
            let mut r = pixel[0] as f32 / 255.0;
            let mut g = pixel[1] as f32 / 255.0;
            let mut b = pixel[2] as f32 / 255.0;
            let a = pixel[3];

            if black_point != 0.0 {
                let bp = black_point * 0.3;
                r = ((r - bp) / (1.0 - bp)).clamp(0.0, 1.0);
                g = ((g - bp) / (1.0 - bp)).clamp(0.0, 1.0);
                b = ((b - bp) / (1.0 - bp)).clamp(0.0, 1.0);
            }

            r *= exposure * brightness;
            g *= exposure * brightness;
            b *= exposure * brightness;

            let lum = 0.2126 * r + 0.7152 * g + 0.0722 * b;
            if shadows != 0.0 && lum < 0.5 {
                let factor = 1.0 + shadows * (0.5 - lum);
                r *= factor;
                g *= factor;
                b *= factor;
            }
            if highlights != 0.0 && lum > 0.5 {
                let factor = 1.0 + highlights * (lum - 0.5);
                r *= factor;
                g *= factor;
                b *= factor;
            }

            if brilliance != 0.0 {
                let boost = 1.0 + brilliance * (1.0 - (lum - 0.5).abs() * 2.0);
                r *= boost;
                g *= boost;
                b *= boost;
            }

            r = ((r - 0.5) * contrast + 0.5).clamp(0.0, 1.0);
            g = ((g - 0.5) * contrast + 0.5).clamp(0.0, 1.0);
            b = ((b - 0.5) * contrast + 0.5).clamp(0.0, 1.0);

            if definition != 0.0 {
                let mid = 0.5;
                r = ((r - mid) * (1.0 + definition * 0.5) + mid).clamp(0.0, 1.0);
                g = ((g - mid) * (1.0 + definition * 0.5) + mid).clamp(0.0, 1.0);
                b = ((b - mid) * (1.0 + definition * 0.5) + mid).clamp(0.0, 1.0);
            }

            let (h_val, mut s, l) = rgb_to_hsl(r, g, b);
            s *= saturation;
            if vibrance != 0.0 {
                s *= 1.0 + vibrance * (1.0 - s);
            }
            s = s.clamp(0.0, 1.0);
            (r, g, b) = hsl_to_rgb(h_val, s, l);

            r += warmth * 0.15;
            b -= warmth * 0.15;
            g += tint * 0.1;
            r -= tint * 0.05;

            if bw_intensity > 0.0 {
                let gray = 0.2126 * r + 0.7152 * g + 0.0722 * b;
                let toned = (gray + bw_tone * 0.15).clamp(0.0, 1.0);
                r = r * (1.0 - bw_intensity) + toned * bw_intensity;
                g = g * (1.0 - bw_intensity) + toned * bw_intensity;
                b = b * (1.0 - bw_intensity) + toned * bw_intensity;
            }

            if vignette > 0.0 {
                let dx = x as f32 - cx;
                let dy = y as f32 - cy;
                let dist = (dx * dx + dy * dy).sqrt() / max_dist;
                if dist > vignette_radius {
                    let falloff = ((dist - vignette_radius) / (1.0 - vignette_radius)).clamp(0.0, 1.0);
                    let factor = 1.0 - vignette * falloff;
                    r *= factor;
                    g *= factor;
                    b *= factor;
                }
            }

            if grain > 0.0 {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                let noise = (seed % 1000) as f32 / 1000.0 - 0.5;
                r += noise * grain * 0.15;
                g += noise * grain * 0.15;
                b += noise * grain * 0.15;
            }

            let _ = h_val;
            img.put_pixel(
                x,
                y,
                Rgba([
                    (r.clamp(0.0, 1.0) * 255.0) as u8,
                    (g.clamp(0.0, 1.0) * 255.0) as u8,
                    (b.clamp(0.0, 1.0) * 255.0) as u8,
                    a,
                ]),
            );
        }
    }

    if params.sharpness > 0.0 {
        apply_sharpen(img, params.sharpness / 100.0);
    }
    if params.noise_reduction > 0.0 {
        apply_box_blur(img, params.noise_reduction / 100.0 * 0.5);
    }
}

fn apply_sharpen(img: &mut RgbaImage, amount: f32) {
    let (w, h) = img.dimensions();
    if w < 3 || h < 3 {
        return;
    }
    let src = img.clone();
    let kernel = [
        [0.0, -amount, 0.0],
        [-amount, 1.0 + 4.0 * amount, -amount],
        [0.0, -amount, 0.0],
    ];
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let mut rgb = [0.0f32; 3];
            for ky in 0..3usize {
                for kx in 0..3usize {
                    let p = src.get_pixel(x + kx as u32 - 1, y + ky as u32 - 1);
                    let k = kernel[ky][kx];
                    rgb[0] += p[0] as f32 * k;
                    rgb[1] += p[1] as f32 * k;
                    rgb[2] += p[2] as f32 * k;
                }
            }
            let a = src.get_pixel(x, y)[3];
            img.put_pixel(
                x,
                y,
                Rgba([
                    rgb[0].clamp(0.0, 255.0) as u8,
                    rgb[1].clamp(0.0, 255.0) as u8,
                    rgb[2].clamp(0.0, 255.0) as u8,
                    a,
                ]),
            );
        }
    }
}

fn apply_box_blur(img: &mut RgbaImage, radius: f32) {
    if radius <= 0.0 {
        return;
    }
    let r = radius.clamp(0.5, 2.0) as i32;
    let (w, h) = img.dimensions();
    let src = img.clone();
    for y in 0..h {
        for x in 0..w {
            let mut rgb = [0.0f32; 3];
            let mut count = 0.0f32;
            for dy in -r..=r {
                for dx in -r..=r {
                    let sx = (x as i32 + dx).clamp(0, w as i32 - 1) as u32;
                    let sy = (y as i32 + dy).clamp(0, h as i32 - 1) as u32;
                    let p = src.get_pixel(sx, sy);
                    rgb[0] += p[0] as f32;
                    rgb[1] += p[1] as f32;
                    rgb[2] += p[2] as f32;
                    count += 1.0;
                }
            }
            let a = src.get_pixel(x, y)[3];
            img.put_pixel(
                x,
                y,
                Rgba([
                    (rgb[0] / count) as u8,
                    (rgb[1] / count) as u8,
                    (rgb[2] / count) as u8,
                    a,
                ]),
            );
        }
    }
}

fn rgb_to_hsl(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    if (max - min).abs() < f32::EPSILON {
        return (0.0, 0.0, l);
    }
    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };
    let h = if (max - r).abs() < f32::EPSILON {
        (g - b) / d + if g < b { 6.0 } else { 0.0 }
    } else if (max - g).abs() < f32::EPSILON {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    } / 6.0;
    (h, s, l)
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    if s.abs() < f32::EPSILON {
        return (l, l, l);
    }
    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    let hue_to_rgb = |t: f32| {
        let t = (t + 1.0).fract();
        if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 1.0 / 2.0 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        }
    };
    (
        hue_to_rgb(h + 1.0 / 3.0),
        hue_to_rgb(h),
        hue_to_rgb(h - 1.0 / 3.0),
    )
}

fn save_jpeg(img: &DynamicImage, output_path: &Path, quality: u8) -> Result<(), String> {
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("failed to create output dir: {e}"))?;
    }
    let rgb = img.to_rgb8();
    let mut buf = std::io::Cursor::new(Vec::new());
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, quality);
    encoder
        .encode(
            rgb.as_raw(),
            rgb.width(),
            rgb.height(),
            image::ExtendedColorType::Rgb8,
        )
        .map_err(|e| format!("failed to encode jpeg: {e}"))?;
    std::fs::write(output_path, buf.into_inner())
        .map_err(|e| format!("failed to write output: {e}"))
}
