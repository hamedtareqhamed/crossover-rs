use crate::config::{Config, CrosshairStyle};
use crate::svg_assets;
use tiny_skia::{Color, FillRule, Paint, PathBuilder, Pixmap, Rect, Transform};

pub struct Renderer;

impl Renderer {
    pub fn render(config: &Config, buffer_size: u32) -> Option<Pixmap> {
        let mut pixmap = Pixmap::new(buffer_size, buffer_size)?;
        pixmap.fill(Color::TRANSPARENT);

        if !config.visible {
            return Some(pixmap);
        }

        let cx = (buffer_size as f32 / 2.0) + config.offset_x as f32;
        let cy = (buffer_size as f32 / 2.0) + config.offset_y as f32;

        let main_color = Color::from_rgba8(
            config.color[0],
            config.color[1],
            config.color[2],
            (config.color[3] as f32 * config.opacity) as u8,
        );

        let outline_color = Color::from_rgba8(
            config.outline_color[0],
            config.outline_color[1],
            config.outline_color[2],
            (config.outline_color[3] as f32 * config.opacity) as u8,
        );

        let size = config.size as f32;
        let thickness = config.thickness as f32;
        let gap = config.gap as f32;
        let outline_t = config.outline_thickness as f32;

        match &config.style {
            CrosshairStyle::Cross => {
                Self::draw_cross(&mut pixmap, cx, cy, size, thickness, gap, main_color, config.outline, outline_t, outline_color, false);
            }
            CrosshairStyle::TStyle => {
                Self::draw_cross(&mut pixmap, cx, cy, size, thickness, gap, main_color, config.outline, outline_t, outline_color, true);
            }
            CrosshairStyle::Dot => {
                Self::draw_dot(&mut pixmap, cx, cy, size / 2.0, main_color, config.outline, outline_t, outline_color);
            }
            CrosshairStyle::Circle => {
                Self::draw_circle(&mut pixmap, cx, cy, size / 2.0, thickness, main_color, config.outline, outline_t, outline_color);
            }
            CrosshairStyle::CircleDot => {
                Self::draw_circle(&mut pixmap, cx, cy, size / 2.0, thickness, main_color, config.outline, outline_t, outline_color);
                Self::draw_dot(&mut pixmap, cx, cy, (thickness * 1.2).max(2.0), main_color, config.outline, outline_t, outline_color);
            }
            CrosshairStyle::Box => {
                Self::draw_box(&mut pixmap, cx, cy, size, thickness, gap, main_color, config.outline, outline_t, outline_color);
            }
            CrosshairStyle::Chevron => {
                Self::draw_chevron(&mut pixmap, cx, cy, size, thickness, main_color, config.outline, outline_t, outline_color);
            }
            CrosshairStyle::Svg(name) => {
                if let Some(svg_str) = svg_assets::get_svg(name) {
                    Self::draw_svg(&mut pixmap, svg_str, cx, cy, size, main_color);
                } else {
                    // Fallback to cross if not found
                    Self::draw_cross(&mut pixmap, cx, cy, size, thickness, gap, main_color, config.outline, outline_t, outline_color, false);
                }
            }
        }

        // Draw independent center dot if enabled
        if config.dot && !matches!(config.style, CrosshairStyle::Dot | CrosshairStyle::CircleDot) {
            let dot_color = Color::from_rgba8(
                config.dot_color[0],
                config.dot_color[1],
                config.dot_color[2],
                (config.dot_color[3] as f32 * config.opacity) as u8,
            );
            Self::draw_dot(&mut pixmap, cx, cy, config.dot_size as f32, dot_color, config.outline, outline_t, outline_color);
        }

        Some(pixmap)
    }

    fn draw_svg(pixmap: &mut Pixmap, svg_str: &str, cx: f32, cy: f32, target_size: f32, tint: Color) {
        let opt = resvg::usvg::Options::default();
        if let Ok(tree) = resvg::usvg::Tree::from_str(svg_str, &opt) {
            let original_w = tree.size().width();
            let original_h = tree.size().height();
            if original_w > 0.0 && original_h > 0.0 {
                let scale = target_size / original_w.max(original_h);
                let rendered_w = (original_w * scale) as u32;
                let rendered_h = (original_h * scale) as u32;

                if let Some(mut temp_pixmap) = Pixmap::new(rendered_w.max(1), rendered_h.max(1)) {
                    temp_pixmap.fill(Color::TRANSPARENT);
                    let transform = Transform::from_scale(scale, scale);
                    resvg::render(&tree, transform, &mut temp_pixmap.as_mut());

                    // Tint non-transparent pixels with target color and blit to centered pixmap
                    let offset_x = (cx - rendered_w as f32 / 2.0).round() as i32;
                    let offset_y = (cy - rendered_h as f32 / 2.0).round() as i32;

                    let (tr, tg, tb, ta) = (
                        tint.red(),
                        tint.green(),
                        tint.blue(),
                        tint.alpha(),
                    );

                    for y in 0..temp_pixmap.height() {
                        for x in 0..temp_pixmap.width() {
                            let src_pixel = temp_pixmap.pixel(x, y).unwrap_or(tiny_skia::PremultipliedColorU8::TRANSPARENT);
                            let alpha = src_pixel.alpha() as f32 / 255.0;
                            if alpha > 0.01 {
                                let dst_x = offset_x + x as i32;
                                let dst_y = offset_y + y as i32;
                                if dst_x >= 0 && dst_x < pixmap.width() as i32 && dst_y >= 0 && dst_y < pixmap.height() as i32 {
                                    let final_color = Color::from_rgba(
                                        tr,
                                        tg,
                                        tb,
                                        ta * alpha,
                                    );
                                    if let Some(final_c) = final_color {
                                        let mut p = Paint::default();
                                        p.set_color(final_c);
                                        if let Some(r) = Rect::from_xywh(dst_x as f32, dst_y as f32, 1.0, 1.0) {
                                            pixmap.fill_rect(r, &p, Transform::identity(), None);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn draw_rect(pixmap: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, color: Color) {
        let mut paint = Paint::default();
        paint.set_color(color);
        paint.anti_alias = false;
        if let Some(rect) = Rect::from_xywh(x, y, w, h) {
            pixmap.fill_rect(rect, &paint, Transform::identity(), None);
        }
    }

    fn draw_cross(
        pixmap: &mut Pixmap,
        cx: f32,
        cy: f32,
        size: f32,
        thickness: f32,
        gap: f32,
        color: Color,
        outline: bool,
        outline_t: f32,
        outline_color: Color,
        t_style: bool,
    ) {
        let half_t = (thickness / 2.0).floor();
        let actual_t = thickness.max(1.0);

        let mut arms = vec![
            (cx - gap - size, cy - half_t, size, actual_t),
            (cx + gap, cy - half_t, size, actual_t),
            (cx - half_t, cy + gap, actual_t, size),
        ];

        if !t_style {
            arms.push((cx - half_t, cy - gap - size, actual_t, size));
        }

        if outline {
            for &(x, y, w, h) in &arms {
                Self::draw_rect(
                    pixmap,
                    x - outline_t,
                    y - outline_t,
                    w + outline_t * 2.0,
                    h + outline_t * 2.0,
                    outline_color,
                );
            }
        }

        for (x, y, w, h) in arms {
            Self::draw_rect(pixmap, x, y, w, h, color);
        }
    }

    fn draw_dot(
        pixmap: &mut Pixmap,
        cx: f32,
        cy: f32,
        radius: f32,
        color: Color,
        outline: bool,
        outline_t: f32,
        outline_color: Color,
    ) {
        let mut paint = Paint::default();
        paint.anti_alias = true;

        if outline {
            paint.set_color(outline_color);
            let mut pb = PathBuilder::new();
            pb.push_circle(cx, cy, radius + outline_t);
            if let Some(path) = pb.finish() {
                pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
            }
        }

        paint.set_color(color);
        let mut pb = PathBuilder::new();
        pb.push_circle(cx, cy, radius);
        if let Some(path) = pb.finish() {
            pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
        }
    }

    fn draw_circle(
        pixmap: &mut Pixmap,
        cx: f32,
        cy: f32,
        radius: f32,
        thickness: f32,
        color: Color,
        outline: bool,
        outline_t: f32,
        outline_color: Color,
    ) {
        let mut paint = Paint::default();
        paint.anti_alias = true;

        if outline {
            paint.set_color(outline_color);
            let mut pb = PathBuilder::new();
            pb.push_circle(cx, cy, radius + thickness / 2.0 + outline_t);
            pb.push_circle(cx, cy, (radius - thickness / 2.0 - outline_t).max(0.0));
            if let Some(path) = pb.finish() {
                pixmap.fill_path(&path, &paint, FillRule::EvenOdd, Transform::identity(), None);
            }
        }

        paint.set_color(color);
        let mut pb = PathBuilder::new();
        pb.push_circle(cx, cy, radius + thickness / 2.0);
        pb.push_circle(cx, cy, (radius - thickness / 2.0).max(0.0));
        if let Some(path) = pb.finish() {
            pixmap.fill_path(&path, &paint, FillRule::EvenOdd, Transform::identity(), None);
        }
    }

    fn draw_box(
        pixmap: &mut Pixmap,
        cx: f32,
        cy: f32,
        size: f32,
        thickness: f32,
        gap: f32,
        color: Color,
        outline: bool,
        outline_t: f32,
        outline_color: Color,
    ) {
        let half_s = size / 2.0 + gap;

        let outer_r = Rect::from_xywh(cx - half_s, cy - half_s, half_s * 2.0, half_s * 2.0);
        let inner_r = Rect::from_xywh(
            cx - half_s + thickness,
            cy - half_s + thickness,
            (half_s - thickness) * 2.0,
            (half_s - thickness) * 2.0,
        );

        let mut paint = Paint::default();
        paint.anti_alias = false;

        if outline {
            paint.set_color(outline_color);
            let mut opb = PathBuilder::new();
            if let Some(r1) = Rect::from_xywh(cx - half_s - outline_t, cy - half_s - outline_t, (half_s + outline_t) * 2.0, (half_s + outline_t) * 2.0) {
                opb.push_rect(r1);
            }
            if let Some(r2) = Rect::from_xywh(cx - half_s + thickness + outline_t, cy - half_s + thickness + outline_t, (half_s - thickness - outline_t) * 2.0, (half_s - thickness - outline_t) * 2.0) {
                opb.push_rect(r2);
            }
            if let Some(path) = opb.finish() {
                pixmap.fill_path(&path, &paint, FillRule::EvenOdd, Transform::identity(), None);
            }
        }

        let mut pb = PathBuilder::new();
        if let Some(r1) = outer_r {
            pb.push_rect(r1);
        }
        if let Some(r2) = inner_r {
            pb.push_rect(r2);
        }

        paint.set_color(color);
        if let Some(path) = pb.finish() {
            pixmap.fill_path(&path, &paint, FillRule::EvenOdd, Transform::identity(), None);
        }
    }

    fn draw_chevron(
        pixmap: &mut Pixmap,
        cx: f32,
        cy: f32,
        size: f32,
        thickness: f32,
        color: Color,
        outline: bool,
        outline_t: f32,
        outline_color: Color,
    ) {
        let half_w = size / 2.0;
        let height = size * 0.8;

        let mut pb = PathBuilder::new();
        pb.move_to(cx - half_w, cy + height / 2.0);
        pb.line_to(cx, cy - height / 2.0);
        pb.line_to(cx + half_w, cy + height / 2.0);
        pb.line_to(cx + half_w, cy + height / 2.0 - thickness);
        pb.line_to(cx, cy - height / 2.0 - thickness);
        pb.line_to(cx - half_w, cy + height / 2.0 - thickness);
        pb.close();

        let mut paint = Paint::default();
        paint.anti_alias = true;

        if outline {
            let mut opb = PathBuilder::new();
            opb.move_to(cx - half_w - outline_t, cy + height / 2.0 + outline_t);
            opb.line_to(cx, cy - height / 2.0 - thickness - outline_t);
            opb.line_to(cx + half_w + outline_t, cy + height / 2.0 + outline_t);
            opb.line_to(cx + half_w + outline_t, cy + height / 2.0 - thickness - outline_t);
            opb.line_to(cx, cy - height / 2.0 - thickness * 2.0 - outline_t);
            opb.line_to(cx - half_w - outline_t, cy + height / 2.0 - thickness - outline_t);
            opb.close();
            paint.set_color(outline_color);
            if let Some(path) = opb.finish() {
                pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
            }
        }

        paint.set_color(color);
        if let Some(path) = pb.finish() {
            pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
        }
    }
}
