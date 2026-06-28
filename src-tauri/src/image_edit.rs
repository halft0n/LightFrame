use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};
use serde::{Deserialize, Serialize};
use std::f32::consts::PI;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SelectiveColorChannel {
    #[serde(default)]
    pub hue: f32,
    #[serde(default)]
    pub saturation: f32,
    #[serde(default)]
    pub luminance: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SelectiveColorData {
    #[serde(default)]
    pub reds: Option<SelectiveColorChannel>,
    #[serde(default)]
    pub yellows: Option<SelectiveColorChannel>,
    #[serde(default)]
    pub greens: Option<SelectiveColorChannel>,
    #[serde(default)]
    pub cyans: Option<SelectiveColorChannel>,
    #[serde(default)]
    pub blues: Option<SelectiveColorChannel>,
    #[serde(default)]
    pub magentas: Option<SelectiveColorChannel>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CurvesData {
    #[serde(default = "default_curve_points")]
    pub rgb: Vec<[u16; 2]>,
    #[serde(default)]
    pub r: Option<Vec<[u16; 2]>>,
    #[serde(default)]
    pub g: Option<Vec<[u16; 2]>>,
    #[serde(default)]
    pub b: Option<Vec<[u16; 2]>>,
}

fn default_curve_points() -> Vec<[u16; 2]> {
    vec![[0, 0], [128, 128], [255, 255]]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LevelsData {
    #[serde(default)]
    pub input_black: u8,
    #[serde(default = "default_input_white")]
    pub input_white: u8,
    #[serde(default = "default_gamma")]
    pub gamma: f32,
    #[serde(default)]
    pub output_black: u8,
    #[serde(default = "default_output_white")]
    pub output_white: u8,
}

fn default_input_white() -> u8 {
    255
}

fn default_output_white() -> u8 {
    255
}

fn default_gamma() -> f32 {
    1.0
}

fn default_vignette_radius() -> f32 {
    50.0
}

impl Default for LevelsData {
    fn default() -> Self {
        Self {
            input_black: 0,
            input_white: 255,
            gamma: 1.0,
            output_black: 0,
            output_white: 255,
        }
    }
}

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
    pub perspective_v: f32,
    #[serde(default)]
    pub perspective_h: f32,

    #[serde(default)]
    pub curves: Option<CurvesData>,
    #[serde(default)]
    pub levels: Option<LevelsData>,
    #[serde(default)]
    pub selective_color: Option<SelectiveColorData>,

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
    #[serde(default = "default_vignette_radius")]
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
            perspective_v: 0.0,
            perspective_h: 0.0,
            curves: None,
            levels: None,
            selective_color: None,
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

    if params.perspective_v.abs() > 0.01 || params.perspective_h.abs() > 0.01 {
        img = apply_perspective(img, params.perspective_v, params.perspective_h);
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

fn apply_perspective(img: DynamicImage, perspective_v: f32, perspective_h: f32) -> DynamicImage {
    let src = img.to_rgba8();
    let (sw, sh) = src.dimensions();
    if sw == 0 || sh == 0 {
        return DynamicImage::ImageRgba8(src);
    }

    let v = perspective_v / 100.0;
    let h = perspective_h / 100.0;
    let ti = v * sw as f32 * 0.12;
    let li = h * sh as f32 * 0.12;

    let src_corners = [
        (0.0_f32, 0.0),
        (sw as f32, 0.0),
        (0.0, sh as f32),
        (sw as f32, sh as f32),
    ];
    let dst_corners = [
        (ti, li),
        (sw as f32 - ti, -li),
        (-ti, sh as f32 + li),
        (sw as f32 + ti, sh as f32 - li),
    ];

    let matrix = compute_perspective_matrix(&src_corners, &dst_corners);
    let inv = invert_homography(&matrix).unwrap_or(matrix);

    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;
    for &(x, y) in &dst_corners {
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    }

    let dw = (max_x - min_x).ceil().max(1.0) as u32;
    let dh = (max_y - min_y).ceil().max(1.0) as u32;
    let mut dst = RgbaImage::from_pixel(dw, dh, Rgba([0, 0, 0, 0]));

    for y in 0..dh {
        for x in 0..dw {
            let px = x as f32 + min_x;
            let py = y as f32 + min_y;
            let (sx, sy) = transform_point(&inv, px, py);
            if sx >= 0.0 && sy >= 0.0 && sx < sw as f32 - 1.0 && sy < sh as f32 - 1.0 {
                dst.put_pixel(x, y, bilinear_sample(&src, sx, sy));
            }
        }
    }

    DynamicImage::ImageRgba8(dst)
}

fn compute_perspective_matrix(src: &[(f32, f32); 4], dst: &[(f32, f32); 4]) -> [f32; 9] {
    let mut a = [[0.0f32; 8]; 8];
    let mut b = [0.0f32; 8];
    for i in 0..4 {
        let (sx, sy) = src[i];
        let (dx, dy) = dst[i];
        a[i * 2] = [sx, sy, 1.0, 0.0, 0.0, 0.0, -dx * sx, -dx * sy];
        b[i * 2] = dx;
        a[i * 2 + 1] = [0.0, 0.0, 0.0, sx, sy, 1.0, -dy * sx, -dy * sy];
        b[i * 2 + 1] = dy;
    }
    let h = solve_8x8(&a, &b);
    [h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7], 1.0]
}

fn solve_8x8(a: &[[f32; 8]; 8], b: &[f32; 8]) -> [f32; 8] {
    let mut m = [[0.0f32; 9]; 8];
    for i in 0..8 {
        for j in 0..8 {
            m[i][j] = a[i][j];
        }
        m[i][8] = b[i];
    }
    for col in 0..8 {
        let mut pivot = col;
        for row in col + 1..8 {
            if m[row][col].abs() > m[pivot][col].abs() {
                pivot = row;
            }
        }
        if m[pivot][col].abs() < 1e-8 {
            continue;
        }
        m.swap(col, pivot);
        let div = m[col][col];
        for j in col..9 {
            m[col][j] /= div;
        }
        for row in 0..8 {
            if row == col {
                continue;
            }
            let factor = m[row][col];
            for j in col..9 {
                m[row][j] -= factor * m[col][j];
            }
        }
    }
    [m[0][8], m[1][8], m[2][8], m[3][8], m[4][8], m[5][8], m[6][8], m[7][8]]
}

fn invert_homography(m: &[f32; 9]) -> Option<[f32; 9]> {
    let a = m[0];
    let b = m[1];
    let c = m[2];
    let d = m[3];
    let e = m[4];
    let f = m[5];
    let g = m[6];
    let h = m[7];
    let i = m[8];

    let det = a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g);
    if det.abs() < 1e-10 {
        return None;
    }
    let inv_det = 1.0 / det;
    Some([
        (e * i - f * h) * inv_det,
        (c * h - b * i) * inv_det,
        (b * f - c * e) * inv_det,
        (f * g - d * i) * inv_det,
        (a * i - c * g) * inv_det,
        (c * d - a * f) * inv_det,
        (d * h - e * g) * inv_det,
        (b * g - a * h) * inv_det,
        (a * e - b * d) * inv_det,
    ])
}

fn transform_point(m: &[f32; 9], x: f32, y: f32) -> (f32, f32) {
    let w = m[6] * x + m[7] * y + m[8];
    if w.abs() < 1e-8 {
        return (0.0, 0.0);
    }
    ((m[0] * x + m[1] * y + m[2]) / w, (m[3] * x + m[4] * y + m[5]) / w)
}

pub(crate) fn build_curve_lut(points: &[[u16; 2]]) -> [u8; 256] {
    let mut lut = [0u8; 256];
    if points.is_empty() {
        for i in 0..256 {
            lut[i] = i as u8;
        }
        return lut;
    }

    let mut sorted: Vec<[u16; 2]> = points.to_vec();
    sorted.sort_by_key(|p| p[0]);

    let n = sorted.len();
    if n == 1 {
        let v = sorted[0][1].min(255) as u8;
        lut.fill(v);
        return lut;
    }

    let xs: Vec<f32> = sorted.iter().map(|p| p[0] as f32).collect();
    let ys: Vec<f32> = sorted.iter().map(|p| p[1].min(255) as f32).collect();

    let mut ms = vec![0.0f32; n];
    for i in 0..n - 1 {
        let dx = xs[i + 1] - xs[i];
        ms[i] = if dx != 0.0 { (ys[i + 1] - ys[i]) / dx } else { 0.0 };
    }
    ms[n - 1] = ms[n - 2];

    let mut tangents = vec![0.0f32; n];
    tangents[0] = ms[0];
    tangents[n - 1] = ms[n - 2];
    for i in 1..n - 1 {
        tangents[i] = if ms[i - 1] * ms[i] <= 0.0 {
            0.0
        } else {
            (ms[i - 1] + ms[i]) / 2.0
        };
    }

    for x in 0..256 {
        let xf = x as f32;
        if xf <= xs[0] {
            lut[x] = ys[0].round().clamp(0.0, 255.0) as u8;
            continue;
        }
        if xf >= xs[n - 1] {
            lut[x] = ys[n - 1].round().clamp(0.0, 255.0) as u8;
            continue;
        }

        let mut seg = 0;
        while seg < n - 2 && xf > xs[seg + 1] {
            seg += 1;
        }

        let x0 = xs[seg];
        let x1 = xs[seg + 1];
        let y0 = ys[seg];
        let y1 = ys[seg + 1];
        let t = if x1 != x0 { (xf - x0) / (x1 - x0) } else { 0.0 };
        let h = x1 - x0;
        let m0 = tangents[seg];
        let m1 = tangents[seg + 1];
        let t2 = t * t;
        let t3 = t2 * t;
        let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
        let h10 = t3 - 2.0 * t2 + t;
        let h01 = -2.0 * t3 + 3.0 * t2;
        let h11 = t3 - t2;
        let y = h00 * y0 + h10 * h * m0 + h01 * y1 + h11 * h * m1;
        lut[x] = y.round().clamp(0.0, 255.0) as u8;
    }

    lut
}

fn is_identity_lut(lut: &[u8; 256]) -> bool {
    lut.iter().enumerate().all(|(i, &v)| v as usize == i)
}

pub(crate) fn apply_levels_value(v: f32, levels: &LevelsData) -> f32 {
    let in_black = levels.input_black as f32;
    let in_white = levels.input_white as f32;
    let out_black = levels.output_black as f32;
    let out_white = levels.output_white as f32;
    let gamma = levels.gamma.clamp(0.1, 9.9);

    let range = (in_white - in_black).max(1.0);
    let normalized = ((v * 255.0 - in_black) / range).clamp(0.0, 1.0);
    let gamma_adj = normalized.powf(1.0 / gamma);
    (gamma_adj * (out_white - out_black) + out_black) / 255.0
}

fn selective_color_hue_range(name: &str) -> (f32, f32) {
    match name {
        "reds" => (330.0 / 360.0, 30.0 / 360.0),
        "yellows" => (30.0 / 360.0, 90.0 / 360.0),
        "greens" => (90.0 / 360.0, 150.0 / 360.0),
        "cyans" => (150.0 / 360.0, 210.0 / 360.0),
        "blues" => (210.0 / 360.0, 270.0 / 360.0),
        "magentas" => (270.0 / 360.0, 330.0 / 360.0),
        _ => (0.0, 0.0),
    }
}

fn hue_in_range(h: f32, start: f32, end: f32) -> bool {
    if start <= end {
        h >= start && h <= end
    } else {
        h >= start || h <= end
    }
}

fn apply_selective_color(h: f32, s: f32, l: f32, channel: &SelectiveColorChannel) -> (f32, f32, f32) {
    let mut nh = (h + channel.hue / 360.0).fract();
    if nh < 0.0 {
        nh += 1.0;
    }
    let ns = (s + channel.saturation / 100.0).clamp(0.0, 1.0);
    let nl = (l + channel.luminance / 100.0).clamp(0.0, 1.0);
    (nh, ns, nl)
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

    let rgb_lut = params
        .curves
        .as_ref()
        .map(|c| build_curve_lut(&c.rgb))
        .filter(|lut| !is_identity_lut(lut));
    let r_lut = params
        .curves
        .as_ref()
        .and_then(|c| c.r.as_ref())
        .map(|p| build_curve_lut(p))
        .filter(|lut| !is_identity_lut(lut));
    let g_lut = params
        .curves
        .as_ref()
        .and_then(|c| c.g.as_ref())
        .map(|p| build_curve_lut(p))
        .filter(|lut| !is_identity_lut(lut));
    let b_lut = params
        .curves
        .as_ref()
        .and_then(|c| c.b.as_ref())
        .map(|p| build_curve_lut(p))
        .filter(|lut| !is_identity_lut(lut));

    let levels = params.levels.as_ref().filter(|l| {
        l.input_black != 0
            || l.input_white != 255
            || (l.gamma - 1.0).abs() > 0.01
            || l.output_black != 0
            || l.output_white != 255
    });

    let selective = params.selective_color.as_ref();

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

            if let Some(l) = levels {
                r = apply_levels_value(r, l);
                g = apply_levels_value(g, l);
                b = apply_levels_value(b, l);
            }

            if let Some(lut) = &rgb_lut {
                r = lut[(r * 255.0).round().clamp(0.0, 255.0) as usize] as f32 / 255.0;
                g = lut[(g * 255.0).round().clamp(0.0, 255.0) as usize] as f32 / 255.0;
                b = lut[(b * 255.0).round().clamp(0.0, 255.0) as usize] as f32 / 255.0;
            }
            if let Some(lut) = &r_lut {
                r = lut[(r * 255.0).round().clamp(0.0, 255.0) as usize] as f32 / 255.0;
            }
            if let Some(lut) = &g_lut {
                g = lut[(g * 255.0).round().clamp(0.0, 255.0) as usize] as f32 / 255.0;
            }
            if let Some(lut) = &b_lut {
                b = lut[(b * 255.0).round().clamp(0.0, 255.0) as usize] as f32 / 255.0;
            }

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

            let (mut h_val, mut s, mut l) = rgb_to_hsl(r, g, b);

            if let Some(sc) = selective {
                let channels: [(&str, Option<&SelectiveColorChannel>); 6] = [
                    ("reds", sc.reds.as_ref()),
                    ("yellows", sc.yellows.as_ref()),
                    ("greens", sc.greens.as_ref()),
                    ("cyans", sc.cyans.as_ref()),
                    ("blues", sc.blues.as_ref()),
                    ("magentas", sc.magentas.as_ref()),
                ];
                for (name, ch) in channels {
                    if let Some(channel) = ch {
                        if channel.hue == 0.0 && channel.saturation == 0.0 && channel.luminance == 0.0 {
                            continue;
                        }
                        let (start, end) = selective_color_hue_range(name);
                        if hue_in_range(h_val, start, end) {
                            (h_val, s, l) = apply_selective_color(h_val, s, l, channel);
                        }
                    }
                }
            }

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

pub(crate) fn rgb_to_hsl(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
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

pub(crate) fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use image::{GenericImageView, RgbaImage};
    use std::path::Path;

    const EPS: f32 = 0.01;

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < EPS
    }

    /// 100×100 四象限测试图：红 / 绿 / 蓝 / 黄
    fn create_test_image() -> DynamicImage {
        create_test_image_sized(100, 100)
    }

    fn create_test_image_sized(width: u32, height: u32) -> DynamicImage {
        let mut img = RgbaImage::new(width, height);
        for y in 0..height {
            for x in 0..width {
                let color = if x < width / 2 {
                    if y < height / 2 {
                        Rgba([255, 0, 0, 255])
                    } else {
                        Rgba([0, 0, 255, 255])
                    }
                } else if y < height / 2 {
                    Rgba([0, 255, 0, 255])
                } else {
                    Rgba([255, 255, 0, 255])
                };
                img.put_pixel(x, y, color);
            }
        }
        DynamicImage::ImageRgba8(img)
    }

    fn save_test_png(img: &DynamicImage, path: &Path) {
        img.save(path).expect("failed to save test png");
    }

    fn avg_luminance(img: &DynamicImage) -> f32 {
        let rgba = img.to_rgba8();
        let (w, h) = rgba.dimensions();
        let mut sum = 0.0f32;
        let count = (w * h) as f32;
        for y in 0..h {
            for x in 0..w {
                let p = rgba.get_pixel(x, y).0;
                sum += 0.2126 * p[0] as f32 + 0.7152 * p[1] as f32 + 0.0722 * p[2] as f32;
            }
        }
        sum / count
    }

    fn pixel_diff_sum(a: &DynamicImage, b: &DynamicImage) -> u32 {
        let ra = a.to_rgba8();
        let rb = b.to_rgba8();
        assert_eq!(ra.dimensions(), rb.dimensions());
        let mut diff = 0u32;
        for (pa, pb) in ra.pixels().zip(rb.pixels()) {
            for i in 0..3 {
                diff += (pa.0[i] as i32 - pb.0[i] as i32).unsigned_abs();
            }
        }
        diff
    }

    fn create_midtone_image() -> DynamicImage {
        let mut img = RgbaImage::new(100, 100);
        for y in 0..100 {
            for x in 0..100 {
                let v = (128 + (x + y) % 40) as u8;
                img.put_pixel(x, y, Rgba([v, (v / 2).max(32), (v / 3).max(16), 255]));
            }
        }
        DynamicImage::ImageRgba8(img)
    }

    /// HSL 往返会在每像素引入少量舍入误差
    fn hsl_roundtrip_baseline_diff() -> u32 {
        let original = create_test_image();
        let params = EditParams::default();
        pixel_diff_sum(&original, &apply_edits(original.clone(), &params))
    }

    fn is_near_grayscale(img: &DynamicImage) -> bool {
        let rgba = img.to_rgba8();
        rgba.pixels().all(|p| {
            let r = p[0] as i32;
            let g = p[1] as i32;
            let b = p[2] as i32;
            (r - g).unsigned_abs() <= 2 && (g - b).unsigned_abs() <= 2
        })
    }

    // ── EditParams parsing ──────────────────────────────────────────────

    #[test]
    fn test_parse_default_params() {
        let params = parse_edit_params("{}").expect("empty json should parse");
        assert_eq!(params.rotate, 0);
        assert!(!params.flip_h);
        assert!(!params.flip_v);
        assert!(approx_eq(params.brightness, 0.0));
        assert!(approx_eq(params.contrast, 0.0));
        assert!(approx_eq(params.saturation, 0.0));
        assert!(params.crop.is_none());
        assert!(params.curves.is_none());
        assert!(params.levels.is_none());
        assert!(params.selective_color.is_none());
        assert!(approx_eq(params.vignette_radius, 50.0));
    }

    #[test]
    fn test_parse_full_params() {
        let json = r#"{
            "rotate": 90,
            "flipH": true,
            "flipV": false,
            "brightness": 10.0,
            "contrast": 20.0,
            "saturation": -15.0,
            "exposure": 5.0,
            "sharpness": 30.0,
            "vignette": 25.0,
            "bwIntensity": 50.0
        }"#;
        let params = parse_edit_params(json).expect("full json should parse");
        assert_eq!(params.rotate, 90);
        assert!(params.flip_h);
        assert!(!params.flip_v);
        assert!(approx_eq(params.brightness, 10.0));
        assert!(approx_eq(params.contrast, 20.0));
        assert!(approx_eq(params.saturation, -15.0));
        assert!(approx_eq(params.exposure, 5.0));
        assert!(approx_eq(params.sharpness, 30.0));
        assert!(approx_eq(params.vignette, 25.0));
        assert!(approx_eq(params.bw_intensity, 50.0));
    }

    #[test]
    fn test_parse_with_curves() {
        let json = r#"{
            "curves": {
                "rgb": [[0, 0], [128, 140], [255, 255]],
                "r": [[0, 0], [255, 200]]
            }
        }"#;
        let params = parse_edit_params(json).expect("curves json should parse");
        let curves = params.curves.expect("curves should be present");
        assert_eq!(curves.rgb.len(), 3);
        assert_eq!(curves.rgb[1], [128, 140]);
        let r = curves.r.expect("r channel should be present");
        assert_eq!(r[1], [255, 200]);
    }

    #[test]
    fn test_parse_with_levels() {
        let json = r#"{
            "levels": {
                "inputBlack": 10,
                "inputWhite": 240,
                "gamma": 1.5,
                "outputBlack": 5,
                "outputWhite": 250
            }
        }"#;
        let params = parse_edit_params(json).expect("levels json should parse");
        let levels = params.levels.expect("levels should be present");
        assert_eq!(levels.input_black, 10);
        assert_eq!(levels.input_white, 240);
        assert!(approx_eq(levels.gamma, 1.5));
        assert_eq!(levels.output_black, 5);
        assert_eq!(levels.output_white, 250);
    }

    #[test]
    fn test_parse_with_selective_color() {
        let json = r#"{
            "selectiveColor": {
                "reds": { "hue": 10.0, "saturation": 20.0, "luminance": -5.0 },
                "blues": { "hue": -15.0, "saturation": 0.0, "luminance": 10.0 }
            }
        }"#;
        let params = parse_edit_params(json).expect("selective color json should parse");
        let sc = params.selective_color.expect("selective color should be present");
        let reds = sc.reds.expect("reds channel should be present");
        assert!(approx_eq(reds.hue, 10.0));
        assert!(approx_eq(reds.saturation, 20.0));
        assert!(approx_eq(reds.luminance, -5.0));
        let blues = sc.blues.expect("blues channel should be present");
        assert!(approx_eq(blues.hue, -15.0));
        assert!(approx_eq(blues.luminance, 10.0));
    }

    #[test]
    fn test_parse_with_perspective() {
        let json = r#"{"perspectiveV": 15.0, "perspectiveH": -10.0}"#;
        let params = parse_edit_params(json).expect("perspective json should parse");
        assert!(approx_eq(params.perspective_v, 15.0));
        assert!(approx_eq(params.perspective_h, -10.0));
    }

    #[test]
    fn test_parse_invalid_json() {
        let result = parse_edit_params("{not valid json");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid edit params"));
    }

    // ── Color conversion ──────────────────────────────────────────────────

    #[test]
    fn test_rgb_to_hsl_pure_red() {
        let (h, s, l) = rgb_to_hsl(1.0, 0.0, 0.0);
        assert!(approx_eq(h, 0.0));
        assert!(approx_eq(s, 1.0));
        assert!(approx_eq(l, 0.5));
    }

    #[test]
    fn test_rgb_to_hsl_pure_green() {
        let (h, s, l) = rgb_to_hsl(0.0, 1.0, 0.0);
        assert!(approx_eq(h, 1.0 / 3.0));
        assert!(approx_eq(s, 1.0));
        assert!(approx_eq(l, 0.5));
    }

    #[test]
    fn test_rgb_to_hsl_pure_blue() {
        let (h, s, l) = rgb_to_hsl(0.0, 0.0, 1.0);
        assert!(approx_eq(h, 2.0 / 3.0));
        assert!(approx_eq(s, 1.0));
        assert!(approx_eq(l, 0.5));
    }

    #[test]
    fn test_rgb_to_hsl_white() {
        let (h, s, l) = rgb_to_hsl(1.0, 1.0, 1.0);
        assert!(approx_eq(h, 0.0));
        assert!(approx_eq(s, 0.0));
        assert!(approx_eq(l, 1.0));
    }

    #[test]
    fn test_rgb_to_hsl_gray() {
        let (h, s, l) = rgb_to_hsl(0.5, 0.5, 0.5);
        assert!(approx_eq(h, 0.0));
        assert!(approx_eq(s, 0.0));
        assert!(approx_eq(l, 0.5));
    }

    #[test]
    fn test_hsl_roundtrip() {
        let samples = [
            (1.0, 0.0, 0.0),
            (0.0, 1.0, 0.0),
            (0.0, 0.0, 1.0),
            (0.5, 0.5, 0.5),
            (0.8, 0.3, 0.6),
            (0.2, 0.9, 0.1),
        ];
        for (r, g, b) in samples {
            let (h, s, l) = rgb_to_hsl(r, g, b);
            let (rr, gg, bb) = hsl_to_rgb(h, s, l);
            assert!(approx_eq(r, rr), "r mismatch for ({r},{g},{b})");
            assert!(approx_eq(g, gg), "g mismatch for ({r},{g},{b})");
            assert!(approx_eq(b, bb), "b mismatch for ({r},{g},{b})");
        }
    }

    // ── Image transformation ────────────────────────────────────────────

    #[test]
    fn test_apply_edits_identity() {
        let baseline = hsl_roundtrip_baseline_diff();
        let original = create_test_image();
        let params = EditParams::default();
        let edited = apply_edits(original.clone(), &params);
        assert_eq!(edited.dimensions(), original.dimensions());
        assert!(
            pixel_diff_sum(&original, &edited) <= baseline,
            "identity params should not exceed HSL roundtrip baseline"
        );
    }

    #[test]
    fn test_apply_edits_rotate_90() {
        let original = create_test_image_sized(120, 80);
        let params = EditParams {
            rotate: 90,
            ..Default::default()
        };
        let edited = apply_edits(original, &params);
        assert_eq!(edited.dimensions(), (80, 120));
    }

    #[test]
    fn test_apply_edits_rotate_180() {
        let original = create_test_image_sized(120, 80);
        let params = EditParams {
            rotate: 180,
            ..Default::default()
        };
        let edited = apply_edits(original, &params);
        assert_eq!(edited.dimensions(), (120, 80));
    }

    #[test]
    fn test_apply_edits_flip_h() {
        let original = create_test_image();
        let (w, h) = original.dimensions();
        let params = EditParams {
            flip_h: true,
            ..Default::default()
        };
        let edited = apply_edits(original.clone(), &params);
        assert_eq!(edited.dimensions(), (w, h));
        let orig_px = original.to_rgba8().get_pixel(0, 0).0;
        let flipped_px = edited.to_rgba8().get_pixel(w - 1, 0).0;
        assert_eq!(orig_px, flipped_px, "horizontal flip should mirror columns");
        let orig_top_right = original.to_rgba8().get_pixel(w - 1, 0).0;
        let flipped_top_left = edited.to_rgba8().get_pixel(0, 0).0;
        assert_eq!(orig_top_right, flipped_top_left);
    }

    #[test]
    fn test_apply_edits_flip_v() {
        let original = create_test_image();
        let (w, h) = original.dimensions();
        let params = EditParams {
            flip_v: true,
            ..Default::default()
        };
        let edited = apply_edits(original.clone(), &params);
        assert_eq!(edited.dimensions(), (w, h));
        let orig_px = original.to_rgba8().get_pixel(0, 0).0;
        let flipped_px = edited.to_rgba8().get_pixel(0, h - 1).0;
        assert_eq!(orig_px, flipped_px, "vertical flip should mirror rows");
    }

    #[test]
    fn test_apply_edits_crop() {
        let original = create_test_image();
        let params = EditParams {
            crop: Some(CropRect {
                x: 0.25,
                y: 0.25,
                width: 0.5,
                height: 0.5,
            }),
            ..Default::default()
        };
        let edited = apply_edits(original, &params);
        assert_eq!(edited.dimensions(), (50, 50));
    }

    #[test]
    fn test_apply_edits_brightness() {
        let original = create_midtone_image();
        let orig_lum = avg_luminance(&original);
        let params = EditParams {
            brightness: 50.0,
            ..Default::default()
        };
        let edited = apply_edits(original, &params);
        let edited_lum = avg_luminance(&edited);
        assert!(
            edited_lum > orig_lum,
            "brightness +50 should increase average luminance"
        );
    }

    #[test]
    fn test_apply_edits_contrast() {
        let original = create_midtone_image();
        let orig_rgba = original.to_rgba8();
        let params = EditParams {
            contrast: 50.0,
            ..Default::default()
        };
        let edited = apply_edits(original, &params);
        let edited_rgba = edited.to_rgba8();
        let orig_min = orig_rgba.pixels().map(|p| p[0]).min().unwrap();
        let orig_max = orig_rgba.pixels().map(|p| p[0]).max().unwrap();
        let edited_min = edited_rgba.pixels().map(|p| p[0]).min().unwrap();
        let edited_max = edited_rgba.pixels().map(|p| p[0]).max().unwrap();
        assert!(
            (edited_max - edited_min) > (orig_max - orig_min),
            "contrast +50 should widen R channel range"
        );
    }

    #[test]
    fn test_apply_edits_saturation() {
        let original = create_midtone_image();
        let orig_rgba = original.to_rgba8();
        let orig_px = orig_rgba.get_pixel(10, 10).0;
        let params = EditParams {
            saturation: 80.0,
            ..Default::default()
        };
        let edited = apply_edits(original, &params);
        let edited_px = edited.to_rgba8().get_pixel(10, 10).0;
        let orig_chroma = orig_px[0] as i32 - orig_px[1].min(orig_px[2]) as i32;
        let edited_chroma = edited_px[0] as i32 - edited_px[1].min(edited_px[2]) as i32;
        assert!(
            edited_chroma.abs() > orig_chroma.abs(),
            "saturation boost should increase color separation"
        );
    }

    #[test]
    fn test_apply_edits_bw() {
        let original = create_test_image();
        let params = EditParams {
            bw_intensity: 100.0,
            ..Default::default()
        };
        let edited = apply_edits(original, &params);
        assert!(is_near_grayscale(&edited), "B&W at 100% should yield grayscale");
    }

    #[test]
    fn test_apply_edits_vignette() {
        let original = create_test_image();
        let (w, h) = original.dimensions();
        let params = EditParams {
            vignette: 80.0,
            vignette_radius: 30.0,
            ..Default::default()
        };
        let edited = apply_edits(original, &params);
        let rgba = edited.to_rgba8();
        let center = rgba.get_pixel(w / 2, h / 2).0;
        let corner = rgba.get_pixel(0, 0).0;
        let center_lum =
            0.2126 * center[0] as f32 + 0.7152 * center[1] as f32 + 0.0722 * center[2] as f32;
        let corner_lum =
            0.2126 * corner[0] as f32 + 0.7152 * corner[1] as f32 + 0.0722 * corner[2] as f32;
        assert!(
            corner_lum < center_lum,
            "vignette should darken corners relative to center"
        );
    }

    #[test]
    fn test_apply_edits_sharpening() {
        let original = create_test_image();
        let params = EditParams {
            sharpness: 75.0,
            ..Default::default()
        };
        let edited = apply_edits(original, &params);
        assert_eq!(edited.dimensions(), (100, 100));
    }

    #[test]
    fn test_apply_edits_noise_reduction() {
        let original = create_test_image();
        let params = EditParams {
            noise_reduction: 60.0,
            ..Default::default()
        };
        let edited = apply_edits(original, &params);
        assert_eq!(edited.dimensions(), (100, 100));
    }

    // ── Curves LUT ──────────────────────────────────────────────────────

    #[test]
    fn test_curves_identity() {
        let points = vec![[0, 0], [128, 128], [255, 255]];
        let lut = build_curve_lut(&points);
        assert_eq!(lut[0], 0);
        assert_eq!(lut[128], 128);
        assert_eq!(lut[255], 255);
    }

    #[test]
    fn test_curves_custom() {
        let points = vec![[0, 0], [255, 128]];
        let lut = build_curve_lut(&points);
        assert_eq!(lut[0], 0);
        assert_eq!(lut[255], 128);
        assert!(lut[64] > 0 && lut[64] < 64, "midpoint should be lifted");
    }

    #[test]
    fn test_curves_per_channel() {
        let r_points = vec![[0, 0], [255, 0]];
        let g_points = vec![[0, 0], [255, 255]];
        let b_points = vec![[0, 0], [255, 255]];
        let r_lut = build_curve_lut(&r_points);
        let g_lut = build_curve_lut(&g_points);
        let b_lut = build_curve_lut(&b_points);
        assert_eq!(r_lut[128], 0, "R channel crushed to 0 at midpoint");
        assert_eq!(g_lut[128], 128, "G channel identity at midpoint");
        assert_eq!(b_lut[255], 255, "B channel full at white point");
    }

    // ── Levels ──────────────────────────────────────────────────────────

    #[test]
    fn test_levels_identity() {
        let levels = LevelsData::default();
        for v in [0.0, 0.25, 0.5, 0.75, 1.0] {
            let out = apply_levels_value(v, &levels);
            assert!(approx_eq(out, v), "identity levels should preserve {v}");
        }
    }

    #[test]
    fn test_levels_crush_blacks() {
        let levels = LevelsData {
            input_black: 50,
            input_white: 255,
            gamma: 1.0,
            output_black: 0,
            output_white: 255,
        };
        for v in [0.0, 10.0 / 255.0, 50.0 / 255.0] {
            let out = apply_levels_value(v, &levels);
            assert!(approx_eq(out, 0.0), "values at/below inputBlack should map to 0");
        }
        let out_high = apply_levels_value(200.0 / 255.0, &levels);
        assert!(out_high > 0.5, "values above inputBlack should remain bright");
    }

    #[test]
    fn test_levels_gamma() {
        let levels = LevelsData {
            input_black: 0,
            input_white: 255,
            gamma: 2.0,
            output_black: 0,
            output_white: 255,
        };
        let mid = apply_levels_value(0.5, &levels);
        assert!(
            mid > 0.5,
            "gamma > 1 (pow 1/gamma) should lighten midtones, got {mid}"
        );
    }

    // ── Selective Color ─────────────────────────────────────────────────

    #[test]
    fn test_selective_color_reds() {
        let original = create_test_image();
        let red_before = original.to_rgba8().get_pixel(10, 10).0;
        let blue_before = original.to_rgba8().get_pixel(10, 90).0;
        let params = EditParams {
            selective_color: Some(SelectiveColorData {
                reds: Some(SelectiveColorChannel {
                    hue: 30.0,
                    saturation: 50.0,
                    luminance: 20.0,
                }),
                ..Default::default()
            }),
            ..Default::default()
        };
        let edited = apply_edits(original, &params);
        let red_after = edited.to_rgba8().get_pixel(10, 10).0;
        let blue_after = edited.to_rgba8().get_pixel(10, 90).0;
        let red_diff: u32 = (0..3)
            .map(|i| (red_before[i] as i32 - red_after[i] as i32).unsigned_abs())
            .sum();
        let blue_diff: u32 = (0..3)
            .map(|i| (blue_before[i] as i32 - blue_after[i] as i32).unsigned_abs())
            .sum();
        assert!(red_diff > blue_diff, "reds adjustment should affect red pixels more than blue");
    }

    #[test]
    fn test_selective_color_no_effect() {
        let baseline = hsl_roundtrip_baseline_diff();
        let original = create_test_image();
        let params = EditParams {
            selective_color: Some(SelectiveColorData {
                reds: Some(SelectiveColorChannel {
                    hue: 0.0,
                    saturation: 0.0,
                    luminance: 0.0,
                }),
                ..Default::default()
            }),
            ..Default::default()
        };
        let edited = apply_edits(original.clone(), &params);
        assert!(
            pixel_diff_sum(&original, &edited) <= baseline,
            "zero selective color adjustments should only incur HSL roundtrip diff"
        );
    }

    // ── Perspective ─────────────────────────────────────────────────────

    #[test]
    fn test_perspective_identity() {
        let original = create_test_image();
        let (w, h) = original.dimensions();
        let params = EditParams {
            perspective_v: 0.0,
            perspective_h: 0.0,
            ..Default::default()
        };
        let edited = apply_edits(original, &params);
        assert_eq!(edited.dimensions(), (w, h));
    }

    #[test]
    fn test_perspective_vertical() {
        let original = create_test_image();
        let params = EditParams {
            perspective_v: 25.0,
            perspective_h: 0.0,
            ..Default::default()
        };
        let edited = apply_edits(original, &params);
        assert!(edited.width() > 0 && edited.height() > 0);
    }

    // ── Export ────────────────────────────────────────────────────────────

    #[test]
    fn test_export_creates_file() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("source.png");
        let dst = dir.path().join("output.jpg");
        save_test_png(&create_test_image(), &src);

        export_edited_image(&src, &dst, "{}", 85).expect("export should succeed");
        assert!(dst.exists(), "exported file should exist");

        let bytes = std::fs::read(&dst).expect("should read exported file");
        assert!(bytes.len() > 2, "exported file should not be empty");
        assert_eq!(bytes[0], 0xFF);
        assert_eq!(bytes[1], 0xD8, "output should be valid JPEG (SOI marker)");
    }

    #[test]
    fn test_export_with_quality() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("source.png");
        save_test_png(&create_test_image(), &src);

        let low = dir.path().join("low.jpg");
        let high = dir.path().join("high.jpg");
        export_edited_image(&src, &low, "{}", 10).expect("low quality export should succeed");
        export_edited_image(&src, &high, "{}", 95).expect("high quality export should succeed");

        let low_size = std::fs::metadata(&low).unwrap().len();
        let high_size = std::fs::metadata(&high).unwrap().len();
        assert!(
            high_size > low_size,
            "quality 95 ({high_size} bytes) should produce larger file than quality 10 ({low_size} bytes)"
        );
    }
}
