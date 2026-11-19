use crate::field;
use crate::model::Show;
use ab_glyph::{Font, FontArc, PxScale, ScaleFont, point};
use std::path::Path;
use tiny_skia::{Color, Paint, PathBuilder, Pixmap, Rect, Transform};

const FIELD_W: f32 = field::LENGTH_STEPS;
const FIELD_H: f32 = field::WIDTH_STEPS;
const CONSTRUCTION_DASH: f32 = 10.0;
const CONSTRUCTION_GAP: f32 = 6.0;

fn fill_rect(image: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, color: Color) {
    if let Some(rect) = Rect::from_xywh(x, y, w, h) {
        let mut paint = Paint::default();
        paint.set_color(color);
        image.fill_rect(rect, &paint, Transform::identity(), None);
    }
}

fn line(image: &mut Pixmap, a: (f32, f32), b: (f32, f32), width: f32, color: Color) {
    let mut path = PathBuilder::new();
    path.move_to(a.0, a.1);
    path.line_to(b.0, b.1);
    if let Some(path) = path.finish() {
        let mut paint = Paint::default();
        paint.set_color(color);
        image.stroke_path(
            &path,
            &paint,
            &tiny_skia::Stroke {
                width,
                ..Default::default()
            },
            Transform::identity(),
            None,
        );
    }
}

fn dashed_line(image: &mut Pixmap, a: (f32, f32), b: (f32, f32), width: f32, color: Color) {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    let length = (dx * dx + dy * dy).sqrt();
    if length <= f32::EPSILON {
        return;
    }
    let direction = (dx / length, dy / length);
    let mut distance = 0.0;
    while distance < length {
        let end = (distance + CONSTRUCTION_DASH).min(length);
        line(
            image,
            (a.0 + direction.0 * distance, a.1 + direction.1 * distance),
            (a.0 + direction.0 * end, a.1 + direction.1 * end),
            width,
            color,
        );
        distance += CONSTRUCTION_DASH + CONSTRUCTION_GAP;
    }
}

fn circle(image: &mut Pixmap, x: f32, y: f32, radius: f32, color: Color) {
    let mut path = PathBuilder::new();
    path.push_circle(x, y, radius);
    if let Some(path) = path.finish() {
        let mut paint = Paint::default();
        paint.set_color(color);
        image.fill_path(
            &path,
            &paint,
            tiny_skia::FillRule::Winding,
            Transform::identity(),
            None,
        );
    }
}

fn label(
    image: &mut Pixmap,
    font: &FontArc,
    text: &str,
    x: f32,
    y: f32,
    size: f32,
    color: [u8; 3],
) {
    let scaled = font.as_scaled(PxScale::from(size));
    let mut cursor = x;
    for character in text.chars() {
        let id = scaled.glyph_id(character);
        if let Some(glyph) = font.outline_glyph(id.with_scale_and_position(size, point(cursor, y)))
        {
            let bounds = glyph.px_bounds();
            glyph.draw(|gx, gy, coverage| {
                let px = bounds.min.x as i32 + gx as i32;
                let py = bounds.min.y as i32 + gy as i32;
                if px < 0 || py < 0 || px >= image.width() as i32 || py >= image.height() as i32 {
                    return;
                }
                let offset = ((py as u32 * image.width() + px as u32) * 4) as usize;
                let data = image.data_mut();
                for channel in 0..3 {
                    data[offset + channel] = ((data[offset + channel] as f32) * (1.0 - coverage)
                        + (color[channel] as f32) * coverage)
                        as u8;
                }
                data[offset + 3] = 255;
            });
        }
        cursor += scaled.h_advance(id);
    }
}

pub fn export(show: &Show, set_index: usize, path: &Path) -> Result<(), String> {
    let set = show.sets.get(set_index).ok_or("Set does not exist")?;
    let mut image = Pixmap::new(1600, 1050).ok_or("Could not allocate chart")?;
    image.fill(Color::from_rgba8(245, 247, 242, 255));
    let font = [
        "C:/Windows/Fonts/arial.ttf",
        "/System/Library/Fonts/Supplemental/Arial.ttf",
        "/Library/Fonts/Arial.ttf",
        "/usr/share/fonts/TTF/OpenSans-Regular.ttf",
        "/usr/share/fonts/liberation/LiberationSans-Regular.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    ]
    .iter()
    .find_map(|path| {
        std::fs::read(path)
            .ok()
            .and_then(|bytes| FontArc::try_from_vec(bytes).ok())
    });

    let scale = ((1600.0 - 128.0) / FIELD_W).min((1050.0 - 220.0) / FIELD_H);
    let x0 = (1600.0 - FIELD_W * scale) / 2.0;
    let y0 = 130.0;
    let x1 = x0 + FIELD_W * scale;
    let y1 = y0 + FIELD_H * scale;
    fill_rect(
        &mut image,
        x0,
        y0,
        x1 - x0,
        y1 - y0,
        Color::from_rgba8(236, 244, 235, 255),
    );
    for yard in 0..=20 {
        let x = x0 + yard as f32 * 8.0 * scale;
        line(
            &mut image,
            (x, y0),
            (x, y1),
            if yard % 2 == 0 { 2.0 } else { 1.0 },
            Color::from_rgba8(92, 135, 105, 255),
        );
        if yard > 0
            && yard < 20
            && let Some(font) = &font
        {
            let number = if yard <= 10 {
                yard * 5
            } else {
                (20 - yard) * 5
            };
            label(
                &mut image,
                font,
                &number.to_string(),
                x - 12.0,
                y0 + 55.0,
                22.0,
                [31, 63, 47],
            );
            label(
                &mut image,
                font,
                &number.to_string(),
                x - 12.0,
                y1 - 38.0,
                22.0,
                [31, 63, 47],
            );
        }
    }
    for hash in [
        field::yards_to_steps(field::FRONT_HASH_YARDS),
        field::yards_to_steps(field::BACK_HASH_YARDS),
    ] {
        let y = y0 + hash * scale;
        for step in 0..=40 {
            let x = x0 + step as f32 * 4.0 * scale;
            line(
                &mut image,
                (x, y - 5.0),
                (x, y + 5.0),
                1.0,
                Color::from_rgba8(100, 144, 114, 255),
            );
        }
    }
    for (a, b) in [
        ((x0, y0), (x1, y0)),
        ((x0, y1), (x1, y1)),
        ((x0, y0), (x0, y1)),
        ((x1, y0), (x1, y1)),
    ] {
        line(&mut image, a, b, 2.0, Color::from_rgba8(29, 66, 44, 255));
    }
    for axis in &set.symmetry_axes {
        dashed_line(
            &mut image,
            (x0 + axis.start.x * scale, y0 + axis.start.y * scale),
            (x0 + axis.end.x * scale, y0 + axis.end.y * scale),
            1.5,
            Color::from_rgba8(102, 102, 102, 255),
        );
    }
    for performer in &show.performers {
        if let Some(at) = set.positions.get(&performer.id) {
            let x = x0 + at.x * scale;
            let y = y0 + at.y * scale;
            circle(&mut image, x, y, 7.0, Color::from_rgba8(15, 45, 33, 255));
            if let Some(font) = &font {
                label(
                    &mut image,
                    font,
                    &performer.label,
                    x + 11.0,
                    y - 10.0,
                    20.0,
                    [17, 43, 35],
                );
            }
        }
    }
    if let Some(font) = &font {
        label(
            &mut image,
            font,
            &format!("{}  /  {}", show.name, set.name),
            x0,
            82.0,
            30.0,
            [21, 41, 34],
        );
        label(
            &mut image,
            font,
            &format!(
                "Count {}  •  {} performers",
                show.set_start(set_index) as u32,
                show.performers.len()
            ),
            x0,
            1010.0,
            20.0,
            [54, 75, 65],
        );
    }
    image.save_png(path).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exports_png() {
        let path = std::env::temp_dir().join(format!("marching-chart-{}.png", std::process::id()));
        export(&Show::default(), 0, &path).unwrap();
        assert!(
            std::fs::read(&path)
                .unwrap()
                .starts_with(b"\x89PNG\r\n\x1a\n")
        );
        std::fs::remove_file(path).unwrap();
    }
}
