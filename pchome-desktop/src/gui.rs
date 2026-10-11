use crate::state::SharedState;
use crate::theme;
use eframe::egui;
use std::sync::atomic::Ordering;
use std::time::Duration;

/// Masaüstü GUI giriş noktası (ana iş parçacığında winit döngüsünü bloklar).
pub fn run(state: SharedState) {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1080.0, 720.0])
            .with_min_inner_size([900.0, 600.0])
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

/// Olay satırının tonu: hata kırmızı, başarı yeşil, diğer soluk.
fn event_color(line: &str) -> egui::Color32 {
    let l = line.to_lowercase();
    if l.contains("yanlış") || l.contains("hata") || l.contains("kilit") || l.contains("başarısız") {
        theme::danger()
    } else if l.contains("doğrulandı")
        || l.contains("bağlan")
        || l.contains("güncelleme bulundu")
        || l.contains("kuruldu")
    {
        theme::success()
    } else {
        theme::muted()
    }
}

impl DesktopGui {
    /// İnce üst bar: logo + sürüm + durum hapı + telemetri + kapat.
    fn top_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("topbar")
            .exact_height(44.0)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.label(egui::RichText::new("⬢ PChome").color(theme::primary()).strong().size(17.0));
                    ui.label(
                        egui::RichText::new(format!("v{}", crate::updater::current_version()))
                            .color(theme::muted())
                            .small(),
                    );
                    ui.separator();
                    let status = self.state.status.lock().unwrap().clone();
                    let pill = if status.contains("BAĞLI") {
                        theme::success()
                    } else if status.contains("DENENİYOR") || status.contains("denetlen") {
                        theme::secondary()
                    } else {
                        theme::danger()
                    };
                    egui::Frame::none()
                        .fill(theme::card())
                        .stroke(egui::Stroke::new(1.0_f32, pill))
                        .rounding(egui::Rounding::same(8.0))
                        .inner_margin(egui::Margin::symmetric(10.0, 3.0))
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new(status).color(pill).strong().small());
                        });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("⏻ Kapat").color(theme::danger()).strong(),
                                )
                                .fill(theme::card()),
                            )
                            .clicked()
                        {
                            self.state.terminate.store(true, Ordering::SeqCst);
                        }
                        if ui.small_button("⚙ Ayarlar").clicked() {
                            self.new_pc_name = self.state.pc_name.lock().unwrap().clone();
                            self.new_password.clear();
                            self.settings_msg.clear();
                            self.show_settings = !self.show_settings;
                        }
                        ui.label(
                            egui::RichText::new(format!(
                                "PING {}ms • {} FPS",
                                self.state.ping_ms.load(Ordering::SeqCst),
                                self.state.fps.load(Ordering::SeqCst)
                            ))
                            .color(theme::muted())
                            .monospace()
                            .small(),
                        );
                    });
                });
            });
    }

    /// Sol pano: kimlik + cihaz + sinyal + güncelleme kartları.
    fn side_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("side")
            .resizable(false)
            .exact_width(322.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    // ——— Kimlik kartı ———
                    theme::card_frame().show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("PC KİMLİĞİ").color(theme::primary()).strong());
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.small_button("📋").on_hover_text("ID + şifreyi kopyala").clicked() {
                                    let id = self.state.pc_id.lock().unwrap().clone();
                                    let pw = self.state.password.lock().unwrap().clone();
                                    ctx.copy_text(format!("ID:{} Şifre:{}", id, pw));
                                }
                            });
                        });
                        ui.add_space(2.0);
                        ui.label(egui::RichText::new("PC ID — telefondaki ID alanına").color(theme::muted()).small());
                        ui.label(
                            egui::RichText::new(self.state.pc_id.lock().unwrap().clone())
                                .color(theme::primary())
                                .monospace()
                                .size(38.0),
                        );
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new("Şifre — 6 hane, sabit").color(theme::muted()).small());
                        ui.horizontal(|ui| {
                            let shown = if self.show_password {
                                self.state.password.lock().unwrap().clone()
                            } else {
                                "••••••".to_string()
                            };
                            ui.label(
                                egui::RichText::new(shown).monospace().size(28.0).color(theme::text()),
                            );
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let eye = if self.show_password { "🙈" } else { "👁" };
                                if ui.small_button(eye).on_hover_text("Göster/Gizle").clicked() {
                                    self.show_password = !self.show_password;
                                }
                                if ui.small_button("🔄").on_hover_text("Şifreyi yenile").clicked() {
                                    self.refresh_password();
                                }
                            });
                        });
                        if !self.settings_msg.is_empty() {
                            ui.label(egui::RichText::new(&self.settings_msg).color(theme::danger()).small());
                        }
                    });
                    ui.add_space(8.0);

                    // ——— Cihaz kartı ———
                    theme::card_frame().show(ui, |ui| {
                        ui.label(egui::RichText::new("CİHAZ").color(theme::primary()).strong());
                        ui.add_space(2.0);
                        let rows = [
                            ("Ad", self.state.pc_name.lock().unwrap().clone()),
                            ("Yerel IP", self.state.local_ip.lock().unwrap().clone()),
                            ("Uzak IP", self.state.remote_ip.lock().unwrap().clone()),
                            ("Çözünürlük", self.state.resolution.lock().unwrap().clone()),
                            ("Kip", self.state.mode.lock().unwrap().clone()),
                        ];
                        egui::Grid::new("device_grid").num_columns(2).spacing([8.0, 3.0]).show(ui, |ui| {
                            for (k, v) in rows {
                                ui.label(egui::RichText::new(k).color(theme::muted()).small());
                                ui.label(egui::RichText::new(v).monospace().small().color(theme::text()));
                                ui.end_row();
                            }
                        });
                        ui.add_space(2.0);
                        let uinput = self.state.uinput_active.load(Ordering::SeqCst);
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Giriş (uinput)").color(theme::muted()).small());
                            ui.label(
                                egui::RichText::new(if uinput { "● ETKİN" } else { "○ KAPALI" })
                                    .color(if uinput { theme::success() } else { theme::danger() })
                                    .strong()
                                    .small(),
                            );
                        });
                    });
                    ui.add_space(8.0);

                    // ——— Sinyal kartı ———
                    theme::card_frame().show(ui, |ui| {
                        ui.label(egui::RichText::new("SİNYAL").color(theme::primary()).strong());
                        ui.add_space(2.0);
                        ui.label(egui::RichText::new(self.signal_url.clone()).monospace().small().color(theme::text()));
                        ui.add_space(4.0);
                        if ui.small_button("✎ Adresi değiştir").clicked() {
                            self.new_pc_name = self.state.pc_name.lock().unwrap().clone();
                            self.new_password.clear();
                            self.show_settings = true;
                        }
                    });
                    ui.add_space(8.0);

                    // ——— Güncelleme kartı ———
                    theme::card_frame().show(ui, |ui| {
                        ui.label(egui::RichText::new("GÜNCELLEME").color(theme::primary()).strong());
                        ui.add_space(2.0);
                        let upd = self.state.update_status.lock().unwrap().clone();
                        let has_update = upd.contains("yeni sürüm") || upd.contains("kuruldu");
                        ui.label(
                            egui::RichText::new(upd)
                                .small()
                                .color(if has_update { theme::success() } else { theme::muted() }),
                        );
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            if ui.small_button("🔍 Denetle").clicked() {
                                let st = self.state.clone();
                                std::thread::spawn(move || crate::updater::check_once(&st));
                            }
                            if has_update && ui.button("⬆ Şimdi Güncelle").clicked() {
                                let st = self.state.clone();
                                std::thread::spawn(move || {
                                    if let Err(e) = crate::updater::install_update(&st) {
                                        *st.update_status.lock().unwrap() =
                                            format!("güncelleme kurulamadı: {}", e);
                                    }
                                });
                            }
                        });
                    });
                });
            });
    }

    /// Orta alan: canlı önizleme + olay akışı (pencereye sığar, taşan kayar).
    fn central(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Canlı önizleme
            theme::card_frame().show(ui, |ui| {
                ui.horizontal(|ui| {
                    let t = ctx.input(|i| i.time);
                    let pulse = ((t * 3.0).sin() * 0.5 + 0.5) as f32;
                    let dot = egui::Color32::from_rgb(
                        0,
                        (180.0 + 75.0 * pulse) as u8,
                        (200.0 + 55.0 * pulse) as u8,
                    );
                    ui.label(egui::RichText::new("●").color(dot).strong().size(16.0));
                    ui.label(egui::RichText::new("CANLI ÖNİZLEME").color(theme::primary()).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "{} • {} FPS • {}ms",
                                self.state.resolution.lock().unwrap(),
                                self.state.fps.load(Ordering::SeqCst),
                                self.state.ping_ms.load(Ordering::SeqCst)
                            ))
                            .monospace()
                            .small()
                            .color(theme::muted()),
                        );
                    });
                });
                ui.add_space(4.0);
                let avail = ui.available_width();
                let h = (ui.available_height() - 170.0).clamp(220.0, 420.0);
                let rect = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(avail, h));
                ui.painter().rect_filled(rect, 8.0, theme::bg_panel());
                ui.painter().rect_stroke(
                    rect,
                    8.0,
                    egui::Stroke::new(1.0_f32, theme::border()),
                );
                // Köşe süsleri
                for corner in [rect.left_top(), rect.right_top(), rect.left_bottom(), rect.right_bottom()] {
                    ui.painter().circle_filled(corner, 3.0, theme::primary());
                }
                let res = self.state.resolution.lock().unwrap().clone();
                ui.painter().text(
                    rect.center() + egui::vec2(0.0, -12.0),
                    egui::Align2::CENTER_CENTER,
                    "⬢",
                    egui::FontId::proportional(44.0),
                    theme::primary(),
                );
                ui.painter().text(
                    rect.center() + egui::vec2(0.0, 26.0),
                    egui::Align2::CENTER_CENTER,
                    format!("{}  •  YAYINDA", res),
                    egui::FontId::proportional(17.0),
                    theme::text(),
                );
                ui.allocate_rect(rect, egui::Sense::hover());
            });
            ui.add_space(8.0);

            // Olay akışı
            theme::card_frame().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("OLAY AKIŞI").color(theme::primary()).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("🗑 Temizle").clicked() {
                            self.state.clear_events();
                        }
                    });
                });
                ui.add_space(2.0);
                let lines: Vec<String> =
                    self.state.events.lock().unwrap().iter().rev().take(30).cloned().collect();
                egui::ScrollArea::vertical().max_height(150.0).auto_shrink([false, true]).show(
                    ui,
                    |ui| {
                        if lines.is_empty() {
                            ui.label(
                                egui::RichText::new("Henüz olay yok — telefon bağlanınca burada görünür.")
                                    .small()
                                    .color(theme::muted()),
                            );
                        }
                        for line in lines {
                            ui.label(
                                egui::RichText::new(format!("› {}", line))
                                    .monospace()
                                    .small()
                                    .color(event_color(&line)),
                            );
                        }
                    },
                );
            });
        });
    }

    /// Ayarlar penceresi: sinyal + ad + şifre + güncelleme.
    fn settings_window(&mut self, ctx: &egui::Context) {
        if !self.show_settings {
            return;
        }
        egui::Window::new("⚙ Ayarlar")
            .resizable(false)
            .collapsible(false)
            .min_width(380.0)
            .show(ctx, |ui| {
                ui.label(egui::RichText::new("Sinyal Sunucusu").strong().color(theme::primary()));
                ui.text_edit_singleline(&mut self.signal_url);
                ui.add_space(6.0);
                ui.label(egui::RichText::new("PC Adı").strong().color(theme::primary()));
                ui.text_edit_singleline(&mut self.new_pc_name);
                ui.add_space(6.0);
                ui.label(egui::RichText::new("Yeni Şifre (6 hane, boş bırak = değişmesin)").strong().color(theme::primary()));
                ui.text_edit_singleline(&mut self.new_password);
                if !self.settings_msg.is_empty() {
                    ui.label(egui::RichText::new(&self.settings_msg).color(theme::danger()).small());
                }
                ui.add_space(8.0);
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button(egui::RichText::new("💾 Kaydet").strong()).clicked() {
                        self.save_settings();
                    }
                    if ui.button("Vazgeç").clicked() {
                        self.show_settings = false;
                    }
                });
            });
    }

    fn refresh_password(&mut self) {
        match crate::identity::Identity::load_or_create() {
            Ok(mut id) => match id.regenerate_password() {
                Ok(()) => {
                    *self.state.password.lock().unwrap() = id.password.clone();
                    self.state.push_event("şifre yenilendi".to_string());
                    self.settings_msg.clear();
                }
                Err(e) => self.settings_msg = format!("şifre yenilenemedi: {}", e),
            },
            Err(e) => self.settings_msg = format!("kimlik açılamadı: {}", e),
        }
    }

    fn save_settings(&mut self) {
        let url = self.signal_url.trim().to_string();
        if !url.is_empty() {
            let _ = crate::config::Config::save_signal_url(&url);
            *self.state.status.lock().unwrap() =
                "SİNYAL: yeniden başlatınca uygulanır".to_string();
        }
        if !self.new_pc_name.trim().is_empty() {
            if let Ok(mut id) = crate::identity::Identity::load_or_create() {
                if id.set_pc_name(self.new_pc_name.trim()).is_ok() {
                    *self.state.pc_name.lock().unwrap() = self.new_pc_name.trim().to_string();
                }
            }
        }
        if !self.new_password.trim().is_empty() {
            match crate::identity::Identity::load_or_create() {
                Ok(mut id) => match id.set_password(self.new_password.trim()) {
                    Ok(()) => {
                        *self.state.password.lock().unwrap() = id.password.clone();
                        self.settings_msg = "şifre güncellendi".to_string();
                        self.state.push_event("şifre güncellendi".to_string());
                    }
                    Err(e) => self.settings_msg = format!("şifre hatası: {}", e),
                },
                Err(e) => self.settings_msg = format!("kimlik açılamadı: {}", e),
            }
        }
        self.show_settings = false;
    }
}

impl eframe::App for DesktopGui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.state.terminate.load(Ordering::SeqCst) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        theme::apply(ctx);
        self.top_bar(ctx);
        self.side_panel(ctx);
        self.central(ctx);
        self.settings_window(ctx);
        ctx.request_repaint_after(Duration::from_millis(300));
    }
}
