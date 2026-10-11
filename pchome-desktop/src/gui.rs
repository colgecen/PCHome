use crate::state::SharedState;
use crate::theme;
use eframe::egui;
use std::sync::atomic::Ordering;
use std::time::Duration;

/// Masaüstü GUI giriş noktası (ana iş parçacığında winit döngüsünü bloklar).
pub fn run(state: SharedState) {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([980.0, 700.0])
            .with_icon(app_icon()),
        ..Default::default()
    };
    let _ = eframe::run_native(
        "PChome Desktop",
        options,
        Box::new(|_cc| {
            let signal_url = crate::config::Config::from_env()
                .map(|c| c.signal_url)
                .unwrap_or_else(|_| "ws://localhost:8080".to_string());
            let is_default = signal_url.trim() == "ws://localhost:8080";
            if is_default {
                *state.status.lock().unwrap() = "SİNYAL: AYARLARDAN ADRESİ GİRİN".to_string();
            }
            *state.update_status.lock().unwrap() =
                format!("v{} — güncellemeler denetleniyor…", crate::updater::current_version());
            let updater_state = state.clone();
            crate::updater::spawn_background_checker(updater_state);
            Ok(Box::new(DesktopGui {
                state,
                signal_url,
                show_settings: is_default,
                show_password: false,
                new_password: String::new(),
                new_pc_name: String::new(),
                settings_msg: String::new(),
            }))
        }),
    );
}

/// Paketlenmiş duvar kağıdından pencere/görev çubuğu simgesi.
fn app_icon() -> egui::IconData {
    const ICON_JPG: &[u8] = include_bytes!("../../assets/PCHome-Wallpaper.jpg");
    let img = image::load_from_memory(ICON_JPG).expect("paketli simge geçerli bir JPEG olmalı");
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
    signal_url: String,
    show_settings: bool,
    show_password: bool,
    new_password: String,
    new_pc_name: String,
    settings_msg: String,
}

impl eframe::App for DesktopGui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.state.terminate.load(Ordering::SeqCst) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        theme::apply(ctx);
        let glow = theme::primary();
        let danger = theme::danger();
        let ok = theme::success();
        let muted = theme::muted();

        // Üst şerit: durum + telemetri + güncelleme bandı.
        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    egui::RichText::new(self.state.status.lock().unwrap().clone())
                        .color(glow)
                        .strong(),
                );
                ui.separator();
                ui.label(format!("YEREL: {}", self.state.local_ip.lock().unwrap()));
                ui.label(format!("UZAK: {}", self.state.remote_ip.lock().unwrap()));
                ui.label(format!("PING: {} ms", self.state.ping_ms.load(Ordering::SeqCst)));
                ui.label(format!("FPS: {}", self.state.fps.load(Ordering::SeqCst)));
            });
            let upd = self.state.update_status.lock().unwrap().clone();
            if upd.contains("yeni sürüm") || upd.contains("kuruldu") {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(format!("⬆ {}", upd)).color(ok).strong());
                    if ui.button("Şimdi Güncelle").clicked() {
                        let st = self.state.clone();
                        std::thread::spawn(move || {
                            if let Err(e) = crate::updater::install_update(&st) {
                                *st.update_status.lock().unwrap() =
                                    format!("güncelleme kurulamadı: {}", e);
                            }
                        });
                    }
                });
            } else {
                ui.label(egui::RichText::new(upd).color(muted).small());
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                // Kimlik kartı: kalıcı ID + sabit şifre (telefona bunlar girilir).
                theme::card_frame().show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.heading(egui::RichText::new("PC KİMLİĞİ").color(glow).strong());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("📋 Kopyala").clicked() {
                                let id = self.state.pc_id.lock().unwrap().clone();
                                let pw = self.state.password.lock().unwrap().clone();
                                ctx.copy_text(format!("ID:{} Şifre:{}", id, pw));
                            }
                        });
                    });
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new("PC ID (sabit)").color(muted).small());
                            ui.label(
                                egui::RichText::new(self.state.pc_id.lock().unwrap().clone())
                                    .color(glow)
                                    .monospace()
                                    .size(40.0),
                            );
                        });
                        ui.separator();
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new("Şifre (sabit, 6 hane)").color(muted).small());
                            let shown = if self.show_password {
                                self.state.password.lock().unwrap().clone()
                            } else {
                                "••••••".to_string()
                            };
                            ui.label(
                                egui::RichText::new(shown).monospace().size(40.0).color(theme::text()),
                            );
                            ui.horizontal(|ui| {
                                let eye = if self.show_password { "🙈 Gizle" } else { "👁 Göster" };
                                if ui.small_button(eye).clicked() {
                                    self.show_password = !self.show_password;
                                }
                                if ui.small_button("🔄 Yenile").clicked() {
                                    match crate::identity::Identity::load_or_create() {
                                        Ok(mut id) => match id.regenerate_password() {
                                            Ok(()) => {
                                                *self.state.password.lock().unwrap() =
                                                    id.password.clone();
                                                self.state.push_event(format!(
                                                    "şifre yenilendi"
                                                ));
                                            }
                                            Err(e) => {
                                                self.settings_msg =
                                                    format!("şifre yenilenemedi: {}", e)
                                            }
                                        },
                                        Err(e) => {
                                            self.settings_msg = format!("kimlik açılamadı: {}", e)
                                        }
                                    }
                                }
                            });
                        });
                    });
                    if !self.settings_msg.is_empty() {
                        ui.label(egui::RichText::new(&self.settings_msg).color(danger).small());
                    }
                    ui.separator();
                    ui.horizontal_wrapped(|ui| {
                        ui.label(format!("Ad: {}", self.state.pc_name.lock().unwrap()));
                        ui.separator();
                        ui.label(format!(
                            "Çözünürlük: {}",
                            self.state.resolution.lock().unwrap()
                        ));
                        ui.separator();
                        ui.label(format!(
                            "Kip: {}",
                            self.state.mode.lock().unwrap()
                        ));
                    });
                });

                // Canlı önizleme + giriş yöneticisi.
                ui.columns(2, |cols| {
                    theme::card_frame().show(&mut cols[0], |ui| {
                        ui.heading(egui::RichText::new("CANLI ÖNİZLEME").color(glow));
                        let rect = egui::Rect::from_min_size(
                            ui.cursor().min,
                            egui::vec2(ui.available_width(), 200.0),
                        );
                        ui.painter().rect_filled(rect, 6.0, theme::bg_panel());
                        let res = self.state.resolution.lock().unwrap().clone();
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            format!("{}  •  YAYINDA", res),
                            egui::FontId::proportional(16.0),
                            glow,
                        );
                        ui.allocate_rect(rect, egui::Sense::hover());
                    });
                    theme::card_frame().show(&mut cols[1], |ui| {
                        let uinput = if self.state.uinput_active.load(Ordering::SeqCst) {
                            "ETKİN"
                        } else {
                            "KAPALI"
                        };
                        ui.heading(egui::RichText::new("GİRİŞ YÖNETİCİSİ").color(glow));
                        ui.label(
                            egui::RichText::new(format!("uinput: {}", uinput)).color(
                                if self.state.uinput_active.load(Ordering::SeqCst) {
                                    ok
                                } else {
                                    danger
                                },
                            ),
                        );
                        ui.separator();
                        ui.label(egui::RichText::new("CANLI OLAY AKIŞI").monospace().small());
                        egui::ScrollArea::vertical().max_height(140.0).show(ui, |ui| {
                            let events = self.state.events.lock().unwrap();
                            for line in events.iter().rev().take(20) {
                                ui.label(egui::RichText::new(line).monospace().small());
                            }
                        });
                    });
                });
            });
        });

        egui::TopBottomPanel::bottom("footer").show(ctx, |ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(egui::RichText::new("AYARLAR").color(glow)).clicked() {
                    self.new_pc_name = self.state.pc_name.lock().unwrap().clone();
                    self.new_password.clear();
                    self.show_settings = !self.show_settings;
                }
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("OTURUMU KAPAT").color(danger).strong(),
                        )
                        .fill(theme::card()),
                    )
                    .clicked()
                {
                    self.state.terminate.store(true, Ordering::SeqCst);
                }
            });
        });

        if self.show_settings {
            egui::Window::new("Ayarlar").resizable(false).show(ctx, |ui| {
                ui.label(egui::RichText::new("Sinyal Sunucusu").strong());
                ui.text_edit_singleline(&mut self.signal_url);
                ui.separator();
                ui.label(egui::RichText::new("PC Adı").strong());
                ui.text_edit_singleline(&mut self.new_pc_name);
                ui.separator();
                ui.label(egui::RichText::new("Yeni Şifre (6 hane)").strong());
                ui.text_edit_singleline(&mut self.new_password);
                ui.separator();
                ui.label(egui::RichText::new("Güncelleme").strong());
                ui.label(self.state.update_status.lock().unwrap().clone());
                ui.horizontal(|ui| {
                    if ui.button("Güncellemeyi Denetle").clicked() {
                        let st = self.state.clone();
                        std::thread::spawn(move || crate::updater::check_once(&st));
                    }
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Kaydet").clicked() {
                        let url = self.signal_url.trim().to_string();
                        if !url.is_empty() {
                            let _ = crate::config::Config::save_signal_url(&url);
                            *self.state.status.lock().unwrap() =
                                "SİNYAL: yeniden başlatınca uygulanır".to_string();
                        }
                        if !self.new_pc_name.trim().is_empty() {
                            if let Ok(mut id) = crate::identity::Identity::load_or_create() {
                                if id.set_pc_name(self.new_pc_name.trim()).is_ok() {
                                    *self.state.pc_name.lock().unwrap() =
                                        self.new_pc_name.trim().to_string();
                                }
                            }
                        }
                        if !self.new_password.trim().is_empty() {
                            match crate::identity::Identity::load_or_create() {
                                Ok(mut id) => match id.set_password(self.new_password.trim()) {
                                    Ok(()) => {
                                        *self.state.password.lock().unwrap() = id.password.clone();
                                        self.settings_msg = "şifre güncellendi".to_string();
                                    }
                                    Err(e) => {
                                        self.settings_msg = format!("şifre hatası: {}", e)
                                    }
                                },
                                Err(e) => {
                                    self.settings_msg = format!("kimlik açılamadı: {}", e)
                                }
                            }
                        }
                        self.show_settings = false;
                    }
                    if ui.button("Vazgeç").clicked() {
                        self.show_settings = false;
                    }
                });
            });
        }

        ctx.request_repaint_after(Duration::from_millis(250));
    }
}
