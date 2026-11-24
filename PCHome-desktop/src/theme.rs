use eframe::egui;
use egui::epaint::Shadow;
use egui::{Align2, Color32, FontFamily, FontId, Margin, Response, RichText, Rounding, Stroke};
use std::sync::Once;

// PCHome masaüstü — nötr kömür paleti, tek sakin vurgu.
// Neon/camgöbeği yok; yeşil yalnızca gerçek "bağlı" göstergesinde.
pub const BG: &str = "#0F0F11";
pub const SIDEBAR: &str = "#121214";
pub const PANEL: &str = "#16161A";
pub const FIELD: &str = "#18181C";
pub const STAGE: &str = "#09090B";
pub const TEXT: &str = "#EDEDEF";
pub const TEXT_2: &str = "#A1A1AA";
pub const MUTED: &str = "#71717A";
pub const ACCENT: &str = "#6E9BFF";
pub const GREEN: &str = "#4ADE80";
pub const RED: &str = "#F87171";
pub const AMBER: &str = "#FBBF24";
pub const SILVER: &str = "#E8E8EA";
pub const INK: &str = "#0F0F11";

// Phosphor Regular (MIT) codepoints — ince çizgili ikon seti.
pub const I_POWER: &str = "\u{e3da}";
pub const I_REFRESH: &str = "\u{e094}";
pub const I_SETTINGS: &str = "\u{e272}";
pub const I_COPY: &str = "\u{e1ca}";
pub const I_EYE: &str = "\u{e220}";
pub const I_EYE_OFF: &str = "\u{e224}";
pub const I_PHONE: &str = "\u{e1e0}";
pub const I_WIFI: &str = "\u{e4ea}";
pub const I_TRASH: &str = "\u{e4a6}";
pub const I_CLOSE: &str = "\u{e4f6}";
pub const I_CHECK: &str = "\u{e182}";
pub const I_MONITOR: &str = "\u{e32e}";
pub const I_DESKTOP: &str = "\u{e560}";
pub const I_MOUSE: &str = "\u{e33a}";
pub const I_BOLT: &str = "\u{e2de}";
pub const I_TERMINAL: &str = "\u{eae8}";
pub const I_INFO: &str = "\u{e2ce}";
pub const I_LOCK: &str = "\u{e2fe}";
pub const I_KEYBOARD: &str = "\u{e2d8}";
pub const I_CARET_RIGHT: &str = "\u{e13a}";
pub const I_CARET_DOWN: &str = "\u{e136}";
pub const I_DOWNLOAD: &str = "\u{e20c}";
pub const I_SCAN: &str = "\u{e0a2}";
pub const I_CHECK_CIRCLE: &str = "\u{e184}";
pub const I_WARNING: &str = "\u{e4e2}";
pub const I_GAUGE: &str = "\u{e628}";

pub fn hex(s: &str) -> Color32 {
    Color32::from_hex(s).unwrap_or(Color32::WHITE)
}

pub fn alpha(c: Color32, a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}

pub fn bg() -> Color32 {
    hex(BG)
}
pub fn sidebar() -> Color32 {
    hex(SIDEBAR)
}
pub fn panel() -> Color32 {
    hex(PANEL)
}
pub fn field_bg() -> Color32 {
    hex(FIELD)
}
pub fn stage() -> Color32 {
    hex(STAGE)
}
pub fn text() -> Color32 {
    hex(TEXT)
}
pub fn text2() -> Color32 {
    hex(TEXT_2)
}
pub fn muted() -> Color32 {
    hex(MUTED)
}
pub fn accent() -> Color32 {
    hex(ACCENT)
}
pub fn success() -> Color32 {
    hex(GREEN)
}
pub fn danger() -> Color32 {
    hex(RED)
}
pub fn warning() -> Color32 {
    hex(AMBER)
}
pub fn silver() -> Color32 {
    hex(SILVER)
}
pub fn ink() -> Color32 {
    hex(INK)
}
pub fn border() -> Color32 {
    Color32::from_rgba_unmultiplied(255, 255, 255, 15)
}
pub fn border_strong() -> Color32 {
    Color32::from_rgba_unmultiplied(255, 255, 255, 26)
}
pub fn hover() -> Color32 {
    Color32::from_rgba_unmultiplied(255, 255, 255, 9)
}
pub fn pressed() -> Color32 {
    Color32::from_rgba_unmultiplied(255, 255, 255, 16)
}

// Eski çağrılar için uyumluluk adları.
pub fn bg_deep() -> Color32 {
    bg()
}
pub fn bg_panel() -> Color32 {
    panel()
}
pub fn card() -> Color32 {
    panel()
}
pub fn card_hi() -> Color32 {
    field_bg()
}
pub fn primary() -> Color32 {
    accent()
}
pub fn secondary() -> Color32 {
    accent()
}
pub fn indigo() -> Color32 {
    muted()
}
pub fn dim() -> Color32 {
    muted()
}

fn mono(size: f32) -> FontId {
    FontId::new(size, FontFamily::Monospace)
}
pub fn mono_font(size: f32) -> FontId {
    mono(size)
}
pub fn mono_bold_font(size: f32) -> FontId {
    mono(size)
}
pub fn icon_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("icons".into()))
}

static FONTS: Once = Once::new();

fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "inter".to_owned(),
        egui::FontData::from_static(include_bytes!("../../assets/fonts/Inter-Variable.ttf")),
    );
    fonts.font_data.insert(
        "mono".to_owned(),
        egui::FontData::from_static(include_bytes!(
            "../../assets/fonts/JetBrainsMono-Variable.ttf"
        )),
    );
    fonts.font_data.insert(
        "icons".to_owned(),
        egui::FontData::from_static(include_bytes!("../../assets/fonts/Phosphor-Regular.ttf")),
    );
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "inter".to_owned());
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, "mono".to_owned());
    fonts
        .families
        .insert(FontFamily::Name("icons".into()), vec!["icons".to_owned()]);
    ctx.set_fonts(fonts);
}

pub fn apply(ctx: &egui::Context) {
    FONTS.call_once(|| setup_fonts(ctx));

    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = bg();
    visuals.window_fill = panel();
    visuals.extreme_bg_color = stage();
    visuals.faint_bg_color = panel();
    visuals.window_stroke = Stroke::new(1.0_f32, border());
    visuals.window_rounding = Rounding::same(12.0);
    visuals.window_shadow = Shadow {
        offset: egui::vec2(0.0, 10.0),
        blur: 30.0,
        spread: 0.0,
        color: Color32::from_black_alpha(120),
    };
    visuals.selection.bg_fill = alpha(accent(), 60);
    visuals.selection.stroke = Stroke::new(1.0_f32, accent());
    visuals.widgets.noninteractive.bg_fill = panel();
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, text());
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, border());
    visuals.widgets.inactive.bg_fill = field_bg();
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, text());
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, border());
    visuals.widgets.hovered.bg_fill = hex("#1C1C21");
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, text());
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, border_strong());
    visuals.widgets.active.bg_fill = hex("#232329");
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, text());
    visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, border_strong());
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style
        .text_styles
        .insert(egui::TextStyle::Body, FontId::proportional(13.5));
    style
        .text_styles
        .insert(egui::TextStyle::Small, FontId::proportional(11.5));
    style
        .text_styles
        .insert(egui::TextStyle::Button, FontId::proportional(13.0));
    style
        .text_styles
        .insert(egui::TextStyle::Heading, FontId::proportional(18.0));
    style
        .text_styles
        .insert(egui::TextStyle::Monospace, FontId::monospace(12.5));
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(12.0, 7.0);
    style.spacing.interact_size = egui::vec2(34.0, 30.0);
    style.spacing.scroll = egui::style::ScrollStyle::thin();
    ctx.set_style(style);
}

/// Modal / ayar penceresi çerçevesi.
pub fn panel_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(panel())
        .stroke(Stroke::new(1.0_f32, border_strong()))
        .rounding(Rounding::same(12.0))
        .inner_margin(Margin::same(22.0))
        .shadow(Shadow {
            offset: egui::vec2(0.0, 12.0),
            blur: 36.0,
            spread: 0.0,
            color: Color32::from_black_alpha(150),
        })
}

pub fn card_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(panel())
        .stroke(Stroke::new(1.0_f32, border()))
        .rounding(Rounding::same(12.0))
        .inner_margin(Margin::same(16.0))
}

pub fn section_label(ui: &mut egui::Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .color(muted())
            .size(9.5)
            .strong()
            .family(FontFamily::Proportional),
    );
}

pub fn dot(ui: &mut egui::Ui, color: Color32, diameter: f32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(diameter, diameter), egui::Sense::hover());
    ui.painter()
        .circle_filled(rect.center(), diameter / 2.0, color);
}

pub fn icon_button(ui: &mut egui::Ui, glyph: &str, tooltip: &str, color: Color32) -> Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(rect, Rounding::same(6.0), hover());
    }
    let c = if resp.hovered() { text() } else { color };
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        glyph,
        icon_font(16.0),
        c,
    );
    resp.on_hover_text(tooltip)
        .on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub fn filled_icon_button(
    ui: &mut egui::Ui,
    glyph: &str,
    tooltip: &str,
    fill: Color32,
    fg: Color32,
) -> Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::click());
    let bg = if resp.is_pointer_button_down_on() {
        fill.gamma_multiply(0.85)
    } else if resp.hovered() {
        fill.gamma_multiply(0.92)
    } else {
        fill
    };
    ui.painter().rect_filled(rect, Rounding::same(6.0), bg);
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        glyph,
        icon_font(16.0),
        fg,
    );
    resp.on_hover_text(tooltip)
        .on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn draw_button(
    ui: &mut egui::Ui,
    icon: Option<&str>,
    label: &str,
    fill: Color32,
    hover_fill: Color32,
    press_fill: Color32,
    text_color: Color32,
    border: Color32,
    full_width: bool,
) -> Response {
    let font = FontId::proportional(12.5);
    let icon_w = if icon.is_some() { 20.0 } else { 0.0 };
    let width = if full_width {
        ui.available_width()
    } else {
        let galley = ui.fonts(|f| f.layout_no_wrap(label.to_owned(), font.clone(), text_color));
        galley.size().x + icon_w + 28.0
    };
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, 34.0), egui::Sense::click());
    let bg = if resp.is_pointer_button_down_on() {
        press_fill
    } else if resp.hovered() {
        hover_fill
    } else {
        fill
    };
    if ui.is_rect_visible(rect) {
        if bg != Color32::TRANSPARENT {
            ui.painter().rect_filled(rect, Rounding::same(8.0), bg);
        }
        if border != Color32::TRANSPARENT {
            ui.painter()
                .rect_stroke(rect, Rounding::same(8.0), Stroke::new(1.0_f32, border));
        }
        let text_x = if let Some(glyph) = icon {
            ui.painter().text(
                egui::pos2(rect.left() + 12.0, rect.center().y),
                Align2::LEFT_CENTER,
                glyph,
                icon_font(15.0),
                text_color,
            );
            rect.left() + 12.0 + 20.0
        } else {
            rect.center().x
                - ui.fonts(|f| f.layout_no_wrap(label.to_owned(), font.clone(), text_color))
                    .size()
                    .x
                    / 2.0
        };
        ui.painter().text(
            egui::pos2(text_x, rect.center().y),
            if icon.is_some() {
                Align2::LEFT_CENTER
            } else {
                Align2::CENTER_CENTER
            },
            label,
            font,
            text_color,
        );
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Beyaz/gümüş birincil düğme (sade, nötr).
pub fn neon_button(ui: &mut egui::Ui, label: &str) -> Response {
    draw_button(
        ui,
        None,
        label,
        silver(),
        hex("#F5F5F7"),
        hex("#D4D4D8"),
        ink(),
        Color32::TRANSPARENT,
        true,
    )
}

pub fn danger_button(ui: &mut egui::Ui, label: &str) -> Response {
    draw_button(
        ui,
        None,
        label,
        Color32::TRANSPARENT,
        alpha(danger(), 18),
        alpha(danger(), 28),
        danger(),
        alpha(danger(), 90),
        true,
    )
}

pub fn ghost_button(ui: &mut egui::Ui, label: &str) -> Response {
    draw_button(
        ui,
        None,
        label,
        Color32::TRANSPARENT,
        hover(),
        pressed(),
        text2(),
        border(),
        true,
    )
}

pub fn neon_button_auto(ui: &mut egui::Ui, label: &str) -> Response {
    button_auto(ui, None, label, silver(), ink(), Color32::TRANSPARENT)
}

pub fn ghost_button_auto(ui: &mut egui::Ui, label: &str) -> Response {
    button_auto(ui, None, label, Color32::TRANSPARENT, text2(), border())
}

pub fn icon_button_auto(
    ui: &mut egui::Ui,
    glyph: &str,
    label: &str,
    fill: Color32,
    fg: Color32,
) -> Response {
    button_auto(ui, Some(glyph), label, fill, fg, Color32::TRANSPARENT)
}

fn button_auto(
    ui: &mut egui::Ui,
    icon: Option<&str>,
    label: &str,
    fill: Color32,
    fg: Color32,
    border: Color32,
) -> Response {
    draw_button(
        ui,
        icon,
        label,
        fill,
        if fill == Color32::TRANSPARENT {
            hover()
        } else {
            fill.gamma_multiply(1.08)
        },
        if fill == Color32::TRANSPARENT {
            pressed()
        } else {
            fill.gamma_multiply(0.88)
        },
        fg,
        border,
        false,
    )
}

pub fn danger_button_auto(ui: &mut egui::Ui, label: &str) -> Response {
    draw_button(
        ui,
        None,
        label,
        Color32::TRANSPARENT,
        alpha(danger(), 18),
        alpha(danger(), 28),
        danger(),
        alpha(danger(), 90),
        false,
    )
}

pub fn divider(ui: &mut egui::Ui) {
    ui.add_space(6.0);
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, 0.0, border());
    ui.add_space(6.0);
}

pub fn field(ui: &mut egui::Ui, label: &str, value: &mut String, hint: &str, monospace: bool) {
    ui.label(RichText::new(label).color(muted()).size(9.5).strong());
    ui.add_space(3.0);
    egui::Frame::none()
        .fill(field_bg())
        .stroke(Stroke::new(1.0_f32, border()))
        .rounding(Rounding::same(8.0))
        .inner_margin(Margin::symmetric(10.0, 7.0))
        .show(ui, |ui| {
            let mut edit = egui::TextEdit::singleline(value)
                .frame(false)
                .hint_text(RichText::new(hint).color(muted()))
                .text_color(text())
                .desired_width(f32::INFINITY);
            if monospace {
                edit = edit.font(mono(12.0));
            }
            ui.add(edit);
        });
}

/// Eski çağrı uyumluluğu: arka plan artık düz koyu; parıltı yok.
pub fn paint_background(ui: &egui::Ui, rect: egui::Rect) {
    ui.painter().rect_filled(rect, 0.0, bg());
}
