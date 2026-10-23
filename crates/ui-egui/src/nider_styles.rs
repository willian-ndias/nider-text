//! Nider "Vidro" (model B): the Styles gallery as a side panel of glass cards on the right of the
//! page. Each card shows the style's name set in its weight and slant, and its font size; the one
//! at the caret is outlined in the app's colour. A click applies the style; right-click offers the
//! Styles pane's actions. The panel's foot shows the page and the word count.

use egui::{Align2, CornerRadius, Rect, Sense, Stroke, Ui, pos2, vec2};
use serde_json::{Value, json};

use crate::WordApp;
use crate::theme::{Tokens, regular, semibold};

/// The panel's width, and the narrowest window that still gets it (the page needs the room).
pub const WIDTH: f32 = 260.0;
const MIN_WINDOW: f32 = 1000.0;

pub fn show(app: &mut WordApp, ui: &mut Ui) {
    if !crate::widgets::PILLS || !app.has_document() || ui.ctx().content_rect().width() < MIN_WINDOW {
        return;
    }
    let t = Tokens::get(ui.ctx());
    let panel = egui::Panel::right("nider_styles")
        .exact_size(WIDTH)
        .resizable(false)
        .frame(
            egui::Frame::NONE
                .fill(t.glass)
                .stroke(Stroke::new(1.0, t.glass_edge))
                .corner_radius(CornerRadius::same(18))
                .inner_margin(egui::Margin { left: 14, right: 14, top: 14, bottom: 12 })
                .outer_margin(egui::Margin { left: 0, right: 16, top: 0, bottom: 4 }),
        )
        .show(ui, |ui| body(app, ui, &t));
    let r = panel.response.rect;
    let card = Rect::from_min_max(r.min, r.max - vec2(16.0, 4.0));
    crate::theme::glass_sheen(&ui.ctx().layer_painter(panel.response.layer_id), card, 18.0, &t);
}

fn body(app: &mut WordApp, ui: &mut Ui, t: &Tokens) {
    let state = app.session.run("format.state", &json!({})).unwrap_or_default();
    let current = state.get("style").and_then(Value::as_str).unwrap_or("Normal").to_string();
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(tl!("Styles").to_uppercase()).font(semibold(9.5)).color(t.group_label).extra_letter_spacing(1.4));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let (r, resp) = ui.allocate_exact_size(vec2(22.0, 20.0), Sense::click());
            if resp.hovered() {
                ui.painter().rect_filled(r, 5.0, t.hover);
            }
            crate::icons::paint(ui.painter(), Rect::from_center_size(r.center(), vec2(12.0, 12.0)), "launcher", t.text_dim, t.accent);
            if resp.on_hover_text(tl!("Styles")).clicked() {
                let _ = app.run("view.stylesPane", json!({"value": true}));
            }
        });
    });
    ui.add_space(6.0);
    let styles: Vec<(String, String, f32, bool, bool)> = app
        .session
        .doc
        .styles
        .gallery()
        .iter()
        .map(|s| {
            let c = app.session.doc.styles.resolve_char(Some(&s.id), &wordcraft_doc::CharProps::default());
            (s.id.clone(), crate::i18n::style_name(s), c.size, c.bold, c.italic)
        })
        .collect();
    let foot_h = 34.0;
    let list_h = (ui.available_height() - foot_h - 8.0).max(0.0);
    egui::ScrollArea::vertical().max_height(list_h).auto_shrink([false, false]).show(ui, |ui| {
        ui.spacing_mut().item_spacing = vec2(0.0, 8.0);
        for (id, name, size, bold, italic) in &styles {
            card(app, ui, t, id, name, *size, *bold, *italic, *id == current);
        }
    });
    // The foot: the page at the caret and the document's words, as in the status bar.
    let rest = ui.available_rect_before_wrap();
    let foot = Rect::from_min_max(pos2(rest.min.x, rest.max.y - foot_h), rest.max);
    let (fill, edge) = crate::widgets::pill_colors(t);
    ui.painter().rect(foot, 10.0, fill, Stroke::new(1.0, edge), egui::StrokeKind::Inside);
    let l = app.session.layout();
    let pages = l.pages.len().max(1);
    let page = l.caret_on(&app.session.sel.focus, app.session.page_hint).map(|c| c.page + 1).unwrap_or(1).min(pages);
    let words = app.cached_word_count();
    let mono = egui::FontId::monospace(11.0);
    ui.painter().text(
        pos2(foot.min.x + 12.0, foot.center().y),
        Align2::LEFT_CENTER,
        crate::i18n::fmt(tl!("Page {page} of {pages}"), &[("page", &page.to_string()), ("pages", &pages.to_string())]),
        mono.clone(),
        t.text_dim,
    );
    ui.painter().text(
        pos2(foot.max.x - 12.0, foot.center().y),
        Align2::RIGHT_CENTER,
        crate::i18n::fmt(tl!("{words} words"), &[("words", &words.to_string())]),
        mono,
        t.text_dim,
    );
}

#[allow(clippy::too_many_arguments)]
fn card(app: &mut WordApp, ui: &mut Ui, t: &Tokens, id: &str, name: &str, size: f32, bold: bool, italic: bool, active: bool) {
    let w = ui.available_width();
    let (r, resp) = ui.allocate_exact_size(vec2(w, 42.0), Sense::click());
    let (fill, edge) = crate::widgets::pill_colors(t);
    let fill = if resp.hovered() { t.hover } else { fill };
    let stroke = if active { Stroke::new(1.5, t.accent) } else { Stroke::new(1.0, edge) };
    ui.painter().rect(r, 10.0, fill, stroke, egui::StrokeKind::Inside);
    // The name in the style's weight, a little larger for the big styles (capped so cards stay one
    // height).
    let shown = if size.is_finite() { (11.0 + (size - 11.0) * 0.3).clamp(12.0, 17.0) } else { 13.0 };
    let font = if bold { semibold(shown) } else { regular(shown) };
    let mut job = egui::text::LayoutJob::default();
    job.append(name, 0.0, egui::TextFormat { font_id: font, color: t.text, italics: italic, ..Default::default() });
    job.wrap = egui::text::TextWrapping::truncate_at_width(w - 60.0);
    let galley = ui.painter().layout_job(job);
    ui.painter().galley(pos2(r.min.x + 14.0, r.center().y - galley.size().y / 2.0), galley, t.text);
    if size.is_finite() && size > 0.0 {
        let s = if size.fract() == 0.0 { format!("{size:.0}") } else { format!("{size:.1}") };
        ui.painter().text(pos2(r.max.x - 14.0, r.center().y), Align2::RIGHT_CENTER, s, egui::FontId::monospace(10.5), t.text_dim);
    }
    resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, active, name));
    resp.context_menu(|ui| {
        if ui.button(tl!("Update to Match Selection")).clicked() {
            let _ = app.run("styles.updateToMatch", json!({"style": id}));
            ui.close();
        }
        if ui.button(tl!("Modify…")).clicked() {
            app.dialog = crate::dialogs::Dialog::modify_style(app, id);
            ui.close();
        }
    });
    if resp.on_hover_text(name).clicked() {
        let _ = app.run("para.style", json!({"style": id}));
        app.canvas.want_focus = true;
    }
}
