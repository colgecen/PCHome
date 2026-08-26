use crate::state::SharedState;
use eframe::egui;
use std::sync::atomic::Ordering;
use std::time::Duration;

/// Entry point for the desktop GUI. Blocks on the winit event loop (must be
/// called from the main thread).
pub fn run(state: SharedState) {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([960.0, 640.0])
            .with_icon(app_icon()),
        ..Default::default()
    };
    let _ = eframe::run_native(
        "PChome Desktop",
        options,
        Box::new(|_cc| Ok(Box::new(DesktopGui { state }))),
    );
}

/// Decodes the bundled wallpaper into the window/taskbar icon at startup.
fn app_icon() -> egui::IconData {
    const ICON_JPG: &[u8] = include_bytes!("../../assets/PCHome-Wallpaper.jpg");
    let img = image::load_from_memory(ICON_JPG).expect("bundled icon is a valid JPEG");
    let img = img.resize_exact(128, 128, image::imageops::FilterType::Lanczos3);
    let rgba = img.to_rgba8();
    egui::IconData {
        width: rgba.width(),
        height: rgba.height(),
        rgba: rgba.into_raw(),
    }
}

struct DesktopGui {
    state: SharedState,
}

impl eframe::App for DesktopGui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.state.terminate.load(Ordering::SeqCst) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = egui::Color32::from_hex("#090C10").unwrap_or(egui::Color32::BLACK);
        visuals.window_fill = egui::Color32::from_hex("#0D1117").unwrap_or(egui::Color32::BLACK);
        visuals.extreme_bg_color = egui::Color32::from_hex("#090C10").unwrap_or(egui::Color32::BLACK);
        ctx.set_visuals(visuals);

        let glow = egui::Color32::from_hex("#00F4FF").unwrap_or(egui::Color32::WHITE);
        let danger = egui::Color32::from_hex("#FF2A55").unwrap_or(egui::Color32::RED);
        let panel = egui::Color32::from_hex("#0D1117").unwrap_or(egui::Color32::BLACK);

        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    egui::RichText::new(self.state.status.lock().unwrap().clone())
                        .color(glow)
                        .strong(),
                );
                ui.separator();
                ui.label(format!("LOCAL: {}", self.state.local_ip.lock().unwrap()));
                ui.label(format!("REMOTE: {}", self.state.remote_ip.lock().unwrap()));
                ui.label(format!("PING: {} ms", self.state.ping_ms.load(Ordering::SeqCst)));
                ui.label(format!("FPS: {}", self.state.fps.load(Ordering::SeqCst)));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(format!("PIN: {:06}", self.state.pin.load(Ordering::SeqCst)))
                            .color(glow)
                            .strong()
                            .monospace(),
                    );
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                // Top panel: live preview placeholder (resolution + status).
                ui.group(|ui| {
                    ui.heading("LIVE PREVIEW");
                    let preview_rect = egui::Rect::from_min_size(
                        ui.cursor().min,
                        egui::vec2(ui.available_width(), 240.0),
                    );
                    ui.painter().rect_filled(preview_rect, 6.0, panel);
                    let res = self.state.resolution.lock().unwrap().clone();
                    ui.painter().text(
                        preview_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        format!("{}  •  STREAMING", res),
                        egui::FontId::proportional(18.0),
                        glow,
                    );
                    ui.allocate_rect(preview_rect, egui::Sense::hover());
                });

                // Bottom panel: input manager + live event stream.
                ui.group(|ui| {
                    let uinput = if self.state.uinput_active.load(Ordering::SeqCst) {
                        "ACTIVE"
                    } else {
                        "INACTIVE"
                    };
                    ui.label(
                        egui::RichText::new(format!("INPUT MANAGER: {}", uinput))
                            .color(glow),
                    );
                    ui.label(format!(
                        "ACTIVE MODE: {}",
                        self.state.mode.lock().unwrap()
                    ));
                    ui.separator();
                    ui.label(egui::RichText::new("LIVE EVENT STREAM").monospace());
                    let events = self.state.events.lock().unwrap();
                    for line in events.iter().rev().take(16) {
                        ui.label(egui::RichText::new(line).monospace().small());
                    }
                });
            });
        });

        egui::TopBottomPanel::bottom("footer").show(ctx, |ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("TERMINATE SESSION")
                                .color(danger)
                                .strong(),
                        )
                        .fill(panel),
                    )
                    .clicked()
                {
                    self.state.terminate.store(true, Ordering::SeqCst);
                }
            });
        });

        ctx.request_repaint_after(Duration::from_millis(120));
    }
}
