//! Tela de abertura do Nider Text, no mesmo espírito do Nider Etapas: o hexágono com a logo,
//! "Carregando…", e quem fez (Willian Dias · nider.com.br). Some sozinha, ou com um clique.

use egui::{Align2, Color32, CornerRadius, Pos2, Rect, Sense, Stroke, pos2, vec2};

use crate::theme::semibold;

const HOLD: f64 = 1.6;
const FADE_IN: f64 = 0.22;
const FADE_OUT: f64 = 0.2;
const AUTHOR: &str = "Willian Dias";
const EMAIL: &str = "nider@nider.com.br";
const SITE: &str = "nider.com.br";

/// Progresso e fade da tela de abertura. O desktop liga; testes e o canal de controle deixam `None`.
pub struct Splash {
    start: Option<f64>,
    closing: bool,
    close_at: f64,
}

impl Splash {
    pub fn new() -> Self {
        Self { start: None, closing: false, close_at: 0.0 }
    }

    /// Desenha por cima da janela. `false`: já sumiu — o chamador pode largar o valor.
    pub fn show(&mut self, ctx: &egui::Context) -> bool {
        let now = ctx.input(|i| i.time);
        let start = *self.start.get_or_insert(now);
        let elapsed = (now - start).max(0.0);
        if !self.closing && elapsed >= HOLD {
            self.closing = true;
            self.close_at = now;
        }
        let skip = ctx.input(|i| {
            i.pointer.any_released()
                || i.key_pressed(egui::Key::Escape)
                || i.key_pressed(egui::Key::Enter)
                || i.key_pressed(egui::Key::Space)
        });
        if skip && !self.closing {
            self.closing = true;
            self.close_at = now;
        }
        let _ = ctx.input_mut(|i| {
            i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)
                || i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)
                || i.consume_key(egui::Modifiers::NONE, egui::Key::Space)
        });
        let opacity = if self.closing {
            (1.0 - ((now - self.close_at) / FADE_OUT) as f32).clamp(0.0, 1.0)
        } else {
            (elapsed / FADE_IN) as f32
        }
        .clamp(0.0, 1.0);
        if self.closing && opacity <= 0.01 {
            return false;
        }
        let progress = ease_in_out((elapsed / HOLD).clamp(0.0, 1.0) as f32);
        let accent = crate::theme::BRAND_CYAN;
        let screen = ctx.content_rect();
        egui::Area::new(egui::Id::new("nider_splash"))
            .order(egui::Order::Foreground)
            .fixed_pos(screen.min)
            .sense(Sense::click())
            .show(ctx, |ui| {
                ui.set_min_size(screen.size());
                let _ = ui.allocate_rect(screen, Sense::click());
                let painter = ui.painter();
                painter.rect_filled(screen, 0.0, with_alpha(Color32::from_rgb(0x05, 0x08, 0x0B), 0.92 * opacity));
                let card = Rect::from_center_size(screen.center(), vec2(412.0, 430.0));
                painter.rect(
                    card,
                    CornerRadius::same(28),
                    with_alpha(Color32::from_rgb(0x16, 0x1A, 0x21), opacity),
                    Stroke::new(1.0, with_alpha(Color32::from_rgb(0x3A, 0x41, 0x4B), opacity)),
                    egui::StrokeKind::Inside,
                );
                let hex_c = pos2(card.center().x, card.min.y + 44.0 + 75.0);
                paint_hex(painter, hex_c, 70.0, elapsed as f32, accent, opacity);
                let icon = Rect::from_center_size(hex_c, vec2(104.0, 104.0));
                crate::chrome::paint_brand_icon(ctx, painter, icon);

                let title = pos2(card.center().x, hex_c.y + 75.0 + 22.0);
                painter.text(title, Align2::CENTER_TOP, "Nider Text", semibold(30.0), with_alpha(Color32::from_rgb(0xF5, 0xF2, 0xEC), opacity));
                painter.text(
                    pos2(title.x, title.y + 38.0),
                    Align2::CENTER_TOP,
                    format!("by {SITE}"),
                    egui::FontId::proportional(14.0),
                    with_alpha(Color32::from_rgb(0xB8, 0xB4, 0xAE), opacity),
                );
                let bar = Rect::from_center_size(pos2(card.center().x, title.y + 38.0 + 24.0 + 8.0), vec2(220.0, 6.0));
                painter.rect_filled(bar, 3.0, with_alpha(Color32::from_rgb(0xFF, 0xFF, 0xFF), 0.12 * opacity));
                let fill = Rect::from_min_size(bar.min, vec2(bar.width() * progress, bar.height()));
                painter.rect_filled(fill, 3.0, with_alpha(accent, opacity));
                painter.text(
                    pos2(card.center().x, bar.max.y + 10.0),
                    Align2::CENTER_TOP,
                    "Carregando…",
                    egui::FontId::proportional(12.0),
                    with_alpha(Color32::from_rgb(0x8A, 0x93, 0x9C), opacity),
                );
                let version = env!("CARGO_PKG_VERSION");
                painter.text(
                    pos2(card.center().x, card.max.y - 30.0),
                    Align2::CENTER_BOTTOM,
                    format!("{AUTHOR}  ·  {EMAIL}  ·  v{version}"),
                    egui::FontId::proportional(11.0),
                    with_alpha(Color32::from_rgb(0x8A, 0x93, 0x9C), opacity),
                );
            });
        ctx.request_repaint();
        true
    }
}

fn ease_in_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 { 2.0 * t * t } else { 1.0 - ((-2.0 * t + 2.0).powi(2) / 2.0) }
}

fn with_alpha(c: Color32, a: f32) -> Color32 {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), ((a.clamp(0.0, 1.0) * 255.0) + 0.5) as u8)
}

fn hex_vertex(c: Pos2, r: f32, i: usize) -> Pos2 {
    let a = -std::f32::consts::FRAC_PI_2 + (i as f32) * std::f32::consts::TAU / 6.0;
    pos2(c.x + r * a.cos(), c.y + r * a.sin())
}

fn point_on_hex(c: Pos2, r: f32, t: f32) -> Pos2 {
    let t = t.rem_euclid(1.0);
    let scaled = t * 6.0;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let i = scaled.floor() as usize;
    let f = scaled - scaled.floor();
    let a = hex_vertex(c, r, i % 6);
    let b = hex_vertex(c, r, (i + 1) % 6);
    pos2(a.x + (b.x - a.x) * f, a.y + (b.y - a.y) * f)
}

fn paint_hex(painter: &egui::Painter, c: Pos2, r: f32, elapsed: f32, accent: Color32, opacity: f32) {
    let ring: Vec<Pos2> = (0..6).map(|i| hex_vertex(c, r, i)).collect();
    painter.add(egui::Shape::closed_line(ring, Stroke::new(3.0, with_alpha(Color32::WHITE, 0.15 * opacity))));
    // Traço que percorre a borda (~1,1 s por volta, como no Etapas).
    let head = (elapsed / 1.1).rem_euclid(1.0);
    let mut tracer = Vec::with_capacity(12);
    for k in 0..12 {
        tracer.push(point_on_hex(c, r, head - (k as f32) * (0.16 / 11.0)));
    }
    if tracer.len() >= 2 {
        painter.add(egui::Shape::line(tracer.clone(), Stroke::new(6.0, with_alpha(accent, 0.28 * opacity))));
        painter.add(egui::Shape::line(tracer, Stroke::new(3.0, with_alpha(accent, opacity))));
    }
}

#[cfg(test)]
mod tests {
    use super::ease_in_out;

    #[test]
    fn ease_starts_and_ends_at_the_ends() {
        assert_eq!(ease_in_out(0.0), 0.0);
        assert!((ease_in_out(1.0) - 1.0).abs() < 1e-5);
        assert!(ease_in_out(0.25) < 0.25);
        assert!(ease_in_out(0.75) > 0.75);
    }
}
