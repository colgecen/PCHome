use eframe::egui;

/// PChome Neon-Mavi tasarım jetonları (TEK tasarım dili).
/// Mobil taraftaki `colors.xml` ile aynı hex değerleri kullanır; iki platform
/// farklı teknoloji (egui / Material3) kullandığı için "tek kütüphane" kuralı
/// platform başına geçerlidir: desktop'ta yalnız egui, mobilde yalnız Material3.
pub const BG_DEEP: &str = "#050B14";
pub const BG_PANEL: &str = "#0A1626";
pub const CARD: &str = "#0D1B2E";
pub const PRIMARY: &str = "#00E5FF";
pub const SECONDARY: &str = "#0A84FF";
pub const TEXT: &str = "#E6F4FF";
pub const MUTED: &str = "#7A8CA3";
pub const BORDER: &str = "#1B2F4A";
pub const DANGER: &str = "#FF2A55";
pub const SUCCESS: &str = "#00FFA3";

pub fn hex(s: &str) -> egui::Color32 {
    egui::Color32::from_hex(s).unwrap_or(egui::Color32::WHITE)
}

pub fn bg_deep() -> egui::Color32 {
    hex(BG_DEEP)
}
pub fn bg_panel() -> egui::Color32 {
    hex(BG_PANEL)
}
pub fn card() -> egui::Color32 {
    hex(CARD)
}
pub fn primary() -> egui::Color32 {
    hex(PRIMARY)
}
pub fn secondary() -> egui::Color32 {
    hex(SECONDARY)
}
pub fn text() -> egui::Color32 {
    hex(TEXT)
}
pub fn muted() -> egui::Color32 {
    hex(MUTED)
}
pub fn border() -> egui::Color32 {
    hex(BORDER)
}
pub fn danger() -> egui::Color32 {
    hex(DANGER)
}
pub fn success() -> egui::Color32 {
    hex(SUCCESS)
}

/// Koyu neon-mavi görselleri her karede uygular.
pub fn apply(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = bg_deep();
    visuals.window_fill = bg_panel();
    visuals.extreme_bg_color = bg_deep();
    visuals.faint_bg_color = card();
    visuals.widgets.noninteractive.bg_fill = card();
    visuals.widgets.noninteractive.fg_stroke.color = text();
    visuals.widgets.inactive.bg_fill = card();
    visuals.widgets.inactive.fg_stroke.color = text();
    visuals.widgets.hovered.bg_fill = hex("#12263D");
    visuals.widgets.active.bg_fill = hex("#14324F");
    visuals.selection.bg_fill = secondary();
    ctx.set_visuals(visuals);
}

/// Neon çerçeveli kart.
pub fn card_frame() -> egui::Frame {
    egui::Frame::group(&egui::Style::default())
        .fill(card())
        .stroke(egui::Stroke::new(1.0_f32, border()))
        .rounding(egui::Rounding::same(10.0))
        .inner_margin(egui::Margin::same(12.0))
}

/// Kart başlığı: ikon + neon başlık, tek satır.
pub fn card_title(ui: &mut egui::Ui, icon: &str, title: &str) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(format!("{} {}", icon, title)).color(primary()).strong().size(14.0));
    });
    ui.add_space(4.0);
}

fn draw_button(
    ui: &mut egui::Ui,
    label: &str,
    fill: egui::Color32,
    hover_fill: egui::Color32,
    press_fill: egui::Color32,
    text_color: egui::Color32,
    border: egui::Color32,
    full_width: bool,
) -> egui::Response {
    let width = if full_width { ui.available_width() } else { 0.0 };
    let (rect, resp) = ui.allocate_exact_size(
        egui::vec2(width, 36.0),
        egui::Sense::click(),
    );
    let bg = if resp.is_pointer_button_down_on() {
        press_fill
    } else if resp.hovered() {
        hover_fill
    } else {
        fill
    };
    if ui.is_rect_visible(rect) {
        ui.painter().rect_filled(rect, 10.0, bg);
        ui.painter()
            .rect_stroke(rect, 10.0, egui::Stroke::new(1.0_f32, border));
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(14.0),
            text_color,
        );
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Birincil neon buton (tam genişlik, cyan dolgu + hover parlaması).
pub fn neon_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    draw_button(
        ui,
        label,
        primary(),
        hex("#4DEBFF"),
        secondary(),
        hex("#04121A"),
        secondary(),
        true,
    )
}

/// Tehlikeli işlem butonu (kırmızı dolgu, beyaz yazı).
pub fn danger_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    draw_button(
        ui,
        label,
        danger(),
        hex("#FF5C7A"),
        hex("#C40F38"),
        egui::Color32::WHITE,
        hex("#FF5C7A"),
        true,
    )
}

/// Hayalet buton (koyu zemin, neon yazı + çerçeve; ikincil işler için).
pub fn ghost_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    draw_button(
        ui,
        label,
        hex("#0F2036"),
        hex("#16294A"),
        hex("#0A1828"),
        primary(),
        border(),
        true,
    )
}

/// İçerik kadar genişlikte varyantlar (üst bar / kart içi satırlar için).
pub fn neon_button_auto(ui: &mut egui::Ui, label: &str) -> egui::Response {
    draw_button(ui, label, primary(), hex("#4DEBFF"), secondary(), hex("#04121A"), secondary(), false)
}

/// İçerik kadar genişlikte varyantlar (üst bar / kart içi satırlar için).
pub fn danger_button_auto(ui: &mut egui::Ui, label: &str) -> egui::Response {
    draw_button(
        ui,
        label,
        danger(),
        hex("#FF5C7A"),
        hex("#C40F38"),
        egui::Color32::WHITE,
        hex("#FF5C7A"),
        false,
    )
}

/// İçerik kadar genişlikte varyantlar (üst bar / kart içi satırlar için).
pub fn ghost_button_auto(ui: &mut egui::Ui, label: &str) -> egui::Response {
    draw_button(
        ui,
        label,
        hex("#0F2036"),
        hex("#16294A"),
        hex("#0A1828"),
        primary(),
        border(),
        false,
    )
}
