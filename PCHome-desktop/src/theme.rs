use eframe::egui;
use egui::epaint::Shadow;
use egui::{Align2, Color32, FontFamily, FontId, Margin, Response, RichText, Rounding, Stroke};
use std::sync::Once;

// PCHome "Neon Glass" masaüstü tasarım jetonları. Mobil `colors.xml` ile
// aynı aile: derin lacivert, elektrik camgöbeği, yumuşak indigo.
pub const BG_DEEP: &str = "#05070E";
pub const BG_PANEL: &str = "#0B1422";
pub const CARD: &str = "#0D1727";
pub const CARD_HI: &str = "#122033";
pub const PRIMARY: &str = "#22D3EE";
pub const SECONDARY: &str = "#3B82F6";
pub const INDIGO: &str = "#6366F1";
pub const TEXT: &str = "#F1F5F9";
pub const MUTED: &str = "#94A3B8";
pub const DIM: &str = "#64748B";
pub const BORDER: &str = "#1E2A3E";
pub const DANGER: &str = "#FB7185";
pub const SUCCESS: &str = "#34D399";
pub const WARNING: &str = "#FBBF24";

// Material Icons (Apache-2.0) codepoints.
pub const I_POWER: &str = "\u{e8ac}";
pub const I_REFRESH: &str = "\u{e5d5}";
pub const I_SETTINGS: &str = "\u{e8b8}";
pub const I_MONITOR: &str = "\u{e30a}";
pub const I_UPDATE: &str = "\u{e8d7}";
pub const I_COPY: &str = "\u{e14d}";
pub const I_EYE: &str = "\u{e8f4}";
pub const I_EYE_OFF: &str = "\u{e8f5}";
pub const I_TRASH: &str = "\u{e872}";
pub const I_PHONE: &str = "\u{e32c}";
pub const I_LOCK: &str = "\u{e897}";
pub const I_WIFI: &str = "\u{e63e}";
pub const I_CLOSE: &str = "\u{e5cd}";
pub const I_CHECK: &str = "\u{e86c}";
pub const I_INFO: &str = "\u{e88e}";
pub const I_SENSORS: &str = "\u{e51e}";
pub const I_KEY: &str = "\u{e0da}";
pub const I_CHEVRON: &str = "\u{e5cc}";
pub const I_KEYBOARD: &str = "\u{e312}";
pub const I_MOUSE: &str = "\u{e323}";
pub const I_DESKTOP: &str = "\u{e30c}";
pub const I_TERMINAL: &str = "\u{eb8e}";
pub const I_BOLT: &str = "\u{ea0b}";

pub fn hex(s: &str) -> Color32 {
    Color32::from_hex(s).unwrap_or(Color32::WHITE)
}

pub fn bg_deep() -> Color32 {
    hex(BG_DEEP)
}
pub fn bg_panel() -> Color32 {
    hex(BG_PANEL)
}
pub fn card() -> Color32 {
    hex(CARD)
}
pub fn card_hi() -> Color32 {
    hex(CARD_HI)
}
pub fn primary() -> Color32 {
    hex(PRIMARY)
}
pub fn secondary() -> Color32 {
    hex(SECONDARY)
}
pub fn indigo() -> Color32 {
    hex(INDIGO)
}
pub fn text() -> Color32 {
    hex(TEXT)
}
pub fn muted() -> Color32 {
    hex(MUTED)
}
pub fn dim() -> Color32 {
    hex(DIM)
}
pub fn border() -> Color32 {
    hex(BORDER)
}
pub fn danger() -> Color32 {
    hex(DANGER)
}
pub fn success() -> Color32 {
    hex(SUCCESS)
}
pub fn warning() -> Color32 {
    hex(WARNING)
}

/// Saydam renk: aynı tonun düşük alfa varyantı.
pub fn alpha(c: Color32, a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}

fn mono(size: f32) -> FontId {
    FontId::new(size, FontFamily::Monospace)
}
fn mono_bold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("mono-bold".into()))
}
pub fn mono_font(size: f32) -> FontId {
    mono(size)
}
pub fn mono_bold_font(size: f32) -> FontId {
    mono_bold(size)
}
pub fn icon_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("icons".into()))
}

static FONTS: Once = Once::new();

fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "manrope".to_owned(),
        egui::FontData::from_static(include_bytes!("../../assets/fonts/Manrope-Variable.ttf")),
    );
    fonts.font_data.insert(
        "mono".to_owned(),
        egui::FontData::from_static(include_bytes!(
            "../../assets/fonts/NotoSansMono-Regular.ttf"
        )),
    );
    fonts.font_data.insert(
        "mono-bold".to_owned(),
        egui::FontData::from_static(include_bytes!("../../assets/fonts/NotoSansMono-Bold.ttf")),
    );
    fonts.font_data.insert(
        "icons".to_owned(),
        egui::FontData::from_static(include_bytes!(
            "../../assets/fonts/MaterialIcons-Regular.ttf"
        )),
    );
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "manrope".to_owned());
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, "mono".to_owned());
    fonts.families.insert(
        FontFamily::Name("mono-bold".into()),
        vec!["mono-bold".to_owned()],
    );
    fonts
        .families
        .insert(FontFamily::Name("icons".into()), vec!["icons".to_owned()]);
    ctx.set_fonts(fonts);
}

/// Koyu neon-cam görselleri ve tipografiyi uygular.
pub fn apply(ctx: &egui::Context) {
    FONTS.call_once(|| setup_fonts(ctx));

    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = bg_deep();
    visuals.window_fill = card();
    visuals.extreme_bg_color = bg_deep();
    visuals.faint_bg_color = bg_panel();
    visuals.window_stroke = Stroke::new(1.0_f32, border());
    visuals.window_rounding = Rounding::same(18.0);
    visuals.window_shadow = Shadow {
        offset: egui::vec2(0.0, 10.0),
        blur: 32.0,
        spread: 0.0,
        color: Color32::from_black_alpha(150),
    };
    visuals.selection.bg_fill = alpha(primary(), 70);
    visuals.selection.stroke = Stroke::new(1.0_f32, primary());
    visuals.widgets.noninteractive.bg_fill = card();
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, text());
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, border());
    visuals.widgets.inactive.bg_fill = card_hi();
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, text());
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, border());
    visuals.widgets.hovered.bg_fill = hex("#16263D");
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.2_f32, primary());
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, alpha(primary(), 140));
    visuals.widgets.active.bg_fill = hex("#1B2E48");
    visuals.widgets.active.fg_stroke = Stroke::new(1.2_f32, primary());
    visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, primary());
    visuals.widgets.open.bg_fill = card_hi();
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style
        .text_styles
        .insert(egui::TextStyle::Body, FontId::proportional(15.0));
    style
        .text_styles
        .insert(egui::TextStyle::Small, FontId::proportional(12.5));
    style
        .text_styles
        .insert(egui::TextStyle::Button, FontId::proportional(14.5));
    style
        .text_styles
        .insert(egui::TextStyle::Heading, FontId::proportional(22.0));
    style
        .text_styles
        .insert(egui::TextStyle::Monospace, FontId::monospace(13.5));
    style.spacing.item_spacing = egui::vec2(10.0, 8.0);
    style.spacing.button_padding = egui::vec2(14.0, 9.0);
    style.spacing.interact_size = egui::vec2(38.0, 34.0);
    style.spacing.scroll = egui::style::ScrollStyle::thin();
    ctx.set_style(style);
}

/// Neon çerçeveli, yumuşak gölgeli kart.
pub fn card_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(card())
        .stroke(Stroke::new(1.0_f32, border()))
        .rounding(Rounding::same(20.0))
        .inner_margin(Margin::same(18.0))
        .shadow(Shadow {
            offset: egui::vec2(0.0, 6.0),
            blur: 20.0,
            spread: 0.0,
            color: Color32::from_black_alpha(90),
        })
}

/// Modal / pencere çerçevesi.
pub fn window_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(card())
        .stroke(Stroke::new(1.0_f32, border()))
        .rounding(Rounding::same(20.0))
        .inner_margin(Margin::same(24.0))
        .shadow(Shadow {
            offset: egui::vec2(0.0, 12.0),
            blur: 40.0,
            spread: 0.0,
            color: Color32::from_black_alpha(170),
        })
}

/// Sadece yazıdan oluşan, küçük ve harf aralıklı bölüm etiketi.
pub fn section_label(ui: &mut egui::Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .color(dim())
            .size(10.0)
            .strong()
            .family(FontFamily::Proportional),
    );
}

pub fn dot(ui: &mut egui::Ui, color: Color32, diameter: f32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(diameter, diameter), egui::Sense::hover());
    ui.painter()
        .circle_filled(rect.center(), diameter / 2.0, color);
}

/// Renkli nokta + metin hapı (durum göstergesi).
pub fn status_pill(ui: &mut egui::Ui, label: &str, color: Color32) {
    egui::Frame::none()
        .fill(alpha(color, 20))
        .stroke(Stroke::new(1.0_f32, alpha(color, 90)))
        .rounding(Rounding::same(100.0))
        .inner_margin(Margin::symmetric(12.0, 6.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                dot(ui, color, 7.0);
                ui.label(RichText::new(label).color(color).size(11.0).strong());
            });
        });
}

/// Küçük metrik hapı: etiket + monospace değer.
pub fn metric_pill(ui: &mut egui::Ui, label: &str, value: &str) {
    egui::Frame::none()
        .fill(alpha(hex("#FFFFFF"), 8))
        .stroke(Stroke::new(1.0_f32, border()))
        .rounding(Rounding::same(100.0))
        .inner_margin(Margin::symmetric(11.0, 5.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(label).color(dim()).size(9.5).strong());
                ui.label(RichText::new(value).color(text()).font(mono(12.0)));
            });
        });
}

/// Renkli, ikonlu metin (satır içi).
pub fn icon_text(ui: &mut egui::Ui, glyph: &str, size: f32, color: Color32) {
    ui.label(RichText::new(glyph).font(icon_font(size)).color(color));
}

/// Yalnız ikonlu yuvarlak düğme.
pub fn icon_button(ui: &mut egui::Ui, glyph: &str, tooltip: &str, color: Color32) -> Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(34.0, 34.0), egui::Sense::click());
    let bg = if resp.is_pointer_button_down_on() {
        alpha(color, 34)
    } else if resp.hovered() {
        alpha(color, 20)
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, Rounding::same(10.0), bg);
    if resp.hovered() {
        ui.painter().rect_stroke(
            rect,
            Rounding::same(10.0),
            Stroke::new(1.0_f32, alpha(color, 120)),
        );
    }
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        glyph,
        icon_font(18.0),
        color,
    );
    resp.on_hover_text(tooltip)
        .on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Dolgulu, ikonlu yuvarlak düğme (birincil eylemler).
pub fn filled_icon_button(
    ui: &mut egui::Ui,
    glyph: &str,
    tooltip: &str,
    fill: Color32,
    fg: Color32,
) -> Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(34.0, 34.0), egui::Sense::click());
    let bg = if resp.is_pointer_button_down_on() {
        fill.gamma_multiply(0.82)
    } else if resp.hovered() {
        fill.gamma_multiply(0.92)
    } else {
        fill
    };
    ui.painter().rect_filled(rect, Rounding::same(10.0), bg);
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        glyph,
        icon_font(18.0),
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
    let font = FontId::proportional(13.5);
    let icon_w = if icon.is_some() { 24.0 } else { 0.0 };
    let width = if full_width {
        ui.available_width()
    } else {
        let galley = ui.fonts(|f| f.layout_no_wrap(label.to_owned(), font.clone(), text_color));
        galley.size().x + icon_w + 34.0
    };
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, 40.0), egui::Sense::click());
    let bg = if resp.is_pointer_button_down_on() {
        press_fill
    } else if resp.hovered() {
        hover_fill
    } else {
        fill
    };
    if ui.is_rect_visible(rect) {
        ui.painter().rect_filled(rect, Rounding::same(12.0), bg);
        ui.painter()
            .rect_stroke(rect, Rounding::same(12.0), Stroke::new(1.0_f32, border));
        let text_x = if let Some(glyph) = icon {
            let icon_pos = egui::pos2(rect.left() + 16.0, rect.center().y);
            ui.painter().text(
                icon_pos,
                Align2::LEFT_CENTER,
                glyph,
                icon_font(17.0),
                text_color,
            );
            rect.left() + 16.0 + 24.0
        } else {
            rect.center().x
                - ui.fonts(|f| f.layout_no_wrap(label.to_owned(), font.clone(), text_color))
                    .size()
                    .x
                    / 2.0
        };
        let align = if icon.is_some() {
            Align2::LEFT_CENTER
        } else {
            Align2::CENTER_CENTER
        };
        ui.painter().text(
            egui::pos2(text_x, rect.center().y),
            align,
            label,
            font,
            text_color,
        );
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub fn neon_button(ui: &mut egui::Ui, label: &str) -> Response {
    draw_button(
        ui,
        None,
        label,
        primary(),
        hex("#67E8F9"),
        hex("#0EA5C6"),
        hex("#041018"),
        alpha(hex("#FFFFFF"), 40),
        true,
    )
}

pub fn danger_button(ui: &mut egui::Ui, label: &str) -> Response {
    draw_button(
        ui,
        None,
        label,
        hex("#E11D48"),
        hex("#F43F5E"),
        hex("#BE123C"),
        Color32::WHITE,
        alpha(Color32::WHITE, 40),
        true,
    )
}

pub fn ghost_button(ui: &mut egui::Ui, label: &str) -> Response {
    draw_button(
        ui,
        None,
        label,
        alpha(hex("#FFFFFF"), 6),
        alpha(hex("#FFFFFF"), 12),
        alpha(hex("#FFFFFF"), 18),
        text(),
        border(),
        true,
    )
}

pub fn neon_button_auto(ui: &mut egui::Ui, label: &str) -> Response {
    button_auto(ui, None, label, primary(), hex("#041018"))
}

pub fn ghost_button_auto(ui: &mut egui::Ui, label: &str) -> Response {
    button_auto(ui, None, label, alpha(hex("#FFFFFF"), 6), text())
}

/// İkonlu, içerik kadar geniş buton.
pub fn icon_button_auto(
    ui: &mut egui::Ui,
    glyph: &str,
    label: &str,
    fill: Color32,
    fg: Color32,
) -> Response {
    button_auto(ui, Some(glyph), label, fill, fg)
}

fn button_auto(
    ui: &mut egui::Ui,
    icon: Option<&str>,
    label: &str,
    fill: Color32,
    fg: Color32,
) -> Response {
    draw_button(
        ui,
        icon,
        label,
        fill,
        fill.gamma_multiply(1.12),
        fill.gamma_multiply(0.85),
        fg,
        if fill == Color32::TRANSPARENT {
            border()
        } else {
            alpha(Color32::WHITE, 30)
        },
        false,
    )
}

pub fn danger_button_auto(ui: &mut egui::Ui, label: &str) -> Response {
    draw_button(
        ui,
        None,
        label,
        hex("#E11D48"),
        hex("#F43F5E"),
        hex("#BE123C"),
        Color32::WHITE,
        alpha(Color32::WHITE, 30),
        false,
    )
}

/// Gradient logo kutusu: camgöbeği → mavi → indigo, ortada monitör ikonu.
pub fn logo(ui: &mut egui::Ui, size: f32) -> Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    let rounding = Rounding::same(size * 0.26);
    let mut mesh = egui::Mesh::default();
    let i = mesh.vertices.len() as u32;
    mesh.colored_vertex(rect.left_top(), primary());
    mesh.colored_vertex(rect.right_top(), secondary());
    mesh.colored_vertex(rect.left_bottom(), secondary());
    mesh.colored_vertex(rect.right_bottom(), indigo());
    mesh.add_triangle(i, i + 1, i + 2);
    mesh.add_triangle(i + 1, i + 3, i + 2);
    ui.painter().add(egui::Shape::mesh(mesh));
    ui.painter().rect_stroke(
        rect,
        rounding,
        Stroke::new(1.0_f32, alpha(Color32::WHITE, 55)),
    );
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        I_MONITOR,
        icon_font(size * 0.52),
        Color32::WHITE,
    );
    resp
}

/// Orta alanın arka planı: lacivert degrade + iki yumuşak neon parıltı.
pub fn paint_background(ui: &egui::Ui, rect: egui::Rect) {
    let mut mesh = egui::Mesh::default();
    let i = mesh.vertices.len() as u32;
    mesh.colored_vertex(rect.left_top(), hex("#05070E"));
    mesh.colored_vertex(rect.right_top(), hex("#07141C"));
    mesh.colored_vertex(rect.left_bottom(), hex("#080C18"));
    mesh.colored_vertex(rect.right_bottom(), hex("#05070E"));
    mesh.add_triangle(i, i + 1, i + 2);
    mesh.add_triangle(i + 1, i + 3, i + 2);
    ui.painter().add(egui::Shape::mesh(mesh));

    let cyan = alpha(primary(), 3);
    let center = egui::pos2(rect.right() - 60.0, rect.top() + 20.0);
    for k in 0..18 {
        let r = 320.0 - k as f32 * 15.0;
        ui.painter().circle_filled(center, r, cyan);
    }
    let violet = alpha(indigo(), 3);
    let center2 = egui::pos2(rect.left() + 30.0, rect.bottom() - 20.0);
    for k in 0..16 {
        let r = 360.0 - k as f32 * 18.0;
        ui.painter().circle_filled(center2, r, violet);
    }
}

/// Etiketli, koyu zeminli metin alanı (modal formları).
pub fn field(ui: &mut egui::Ui, label: &str, value: &mut String, hint: &str, monospace: bool) {
    ui.label(RichText::new(label).color(muted()).size(11.0).strong());
    egui::Frame::none()
        .fill(bg_panel())
        .stroke(Stroke::new(1.0_f32, border()))
        .rounding(Rounding::same(12.0))
        .inner_margin(Margin::symmetric(12.0, 9.0))
        .show(ui, |ui| {
            let mut edit = egui::TextEdit::singleline(value)
                .frame(false)
                .hint_text(RichText::new(hint).color(dim()))
                .text_color(text())
                .desired_width(f32::INFINITY);
            if monospace {
                edit = edit.font(mono(13.0));
            }
            ui.add(edit);
        });
}

/// İnce ayırıcı çizgi.
pub fn divider(ui: &mut egui::Ui) {
    ui.add_space(2.0);
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, 0.0, border());
    ui.add_space(2.0);
}
