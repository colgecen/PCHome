use crate::local_input::LocalClicker;
use crate::state::SharedState;
use crate::theme;
use crate::updater::UpdateInfo;
use eframe::egui;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};

/// Masaüstü GUI giriş noktası (ana iş parçacığında winit döngüsünü bloklar).
pub fn run(state: SharedState) {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1080.0, 720.0])
            .with_min_inner_size([920.0, 620.0])
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
            let (cap_w, cap_h) = crate::config::Config::from_env()
                .map(|c| (c.capture_width, c.capture_height))
                .unwrap_or((1920, 1080));
            *state.update_status.lock().unwrap() =
                format!("v{} — güncellemeler denetleniyor…", crate::updater::current_version());
            let updater_state = state.clone();
            crate::updater::spawn_background_checker(updater_state);
            let (update_tx, update_rx) = channel();
            Ok(Box::new(DesktopGui {
                state,
                signal_url,
                show_settings: is_default,
                show_password: false,
                new_password: String::new(),
                new_pc_name: String::new(),
                settings_msg: String::new(),
                cap_w,
                cap_h,
                tex: None,
                tex_dims: (0, 0),
                tex_bytes: 0,
                clicker: None,
                press_start: None,
                press_fired: false,
                update_modal: UpdateModalState::Closed,
                update_tx,
                update_rx,
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
    cap_w: u32,
    cap_h: u32,
    tex: Option<egui::TextureHandle>,
    tex_dims: (u32, u32),
    tex_bytes: usize,
    clicker: Option<LocalClicker>,
    press_start: Option<(Instant, egui::Pos2)>,
    press_fired: bool,
    update_modal: UpdateModalState,
    update_tx: Sender<UpdateEvent>,
    update_rx: Receiver<UpdateEvent>,
}

/// Güncelleme modalının durum makinesi. Kurulum bitmeden modal
/// kapatılamaz ve uygulama sonlandırılamaz (kilit aşağıda uygulanır).
enum UpdateModalState {
    Closed,
    Idle,
    Checking,
    Available { info: UpdateInfo, size: String },
    UpToDate,
    Busy { phase: String },
    Failed { msg: String },
}

impl UpdateModalState {
    fn busy(&self) -> bool {
        matches!(self, UpdateModalState::Checking | UpdateModalState::Busy { .. })
    }
}

enum UpdateEvent {
    Checked(Result<(Option<UpdateInfo>, String), String>),
    Installed(Result<PathBuf, String>),
}

/// Epoch saniyeyi "14:32" biçimine çevirir (0 = hiç).
fn fmt_clock(epoch: u32) -> String {
    if epoch == 0 {
        return "—".to_string();
    }
    chrono::DateTime::from_timestamp(epoch as i64, 0)
        .map(|dt| {
            dt.with_timezone(&chrono::Local)
                .format("%H:%M")
                .to_string()
        })
        .unwrap_or_else(|| "—".to_string())
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
        || l.contains("ayrıldı")
    {
        theme::success()
    } else {
        theme::muted()
    }
}

impl DesktopGui {
    /// İnce üst bar: logo + sürüm + durum hapı + telemetri + ayar/kapat.
    fn top_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("topbar")
            .exact_height(54.0)
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
                        .inner_margin(egui::Margin::symmetric(8.0, 2.0))
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new(status).color(pill).strong().small());
                        });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if theme::danger_button_auto(ui, "⏻ Kapat").clicked() {
                            if self.update_modal.busy() {
                                self.state.push_event(
                                    "kurulum bitmeden kapatılamaz — bekle".to_string(),
                                );
                            } else {
                                self.state.terminate.store(true, Ordering::SeqCst);
                            }
                        }
                        if theme::ghost_button_auto(ui, "⚙ Ayarlar").clicked() {
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

    /// Sol pano: kartlar + olay akışı; panel üstten alta boydan boya dolu.
    fn side_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("side")
            .resizable(false)
            .exact_width(326.0)
            .show(ctx, |ui| {
                self.card_identity(ctx, ui);
                ui.add_space(6.0);
                self.card_device(ui);
                ui.add_space(6.0);
                self.card_signal(ui);
                ui.add_space(6.0);
                self.card_update(ui);
                ui.add_space(6.0);
                self.card_peer(ui);
                ui.add_space(6.0);
                self.card_events(ui);
            });
    }

    fn card_identity(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        theme::card_frame().show(ui, |ui| {
            theme::card_title(ui, "💻", "PC KİMLİĞİ");
            ui.label(egui::RichText::new("Telefondaki ID alanına").color(theme::muted()).small());
            ui.label(
                egui::RichText::new(self.state.pc_id.lock().unwrap().clone())
                    .color(theme::primary())
                    .monospace()
                    .size(38.0),
            );
            ui.add_space(4.0);
            ui.label(egui::RichText::new("Şifre — 6 hane, sabit").color(theme::muted()).small());
            let shown = if self.show_password {
                self.state.password.lock().unwrap().clone()
            } else {
                "••••••".to_string()
            };
            ui.label(egui::RichText::new(shown).monospace().size(28.0).color(theme::text()));
            ui.add_space(4.0);
            // Tek Göster/Gizle düğmesi + yenile/kopyala ayrı satırda.
            let eye = if self.show_password { "🙈 Gizle" } else { "👁 Göster" };
            if theme::ghost_button(ui, eye).clicked() {
                self.show_password = !self.show_password;
            }
            ui.add_space(4.0);
            egui::Grid::new("pw_buttons").num_columns(2).spacing([6.0, 4.0]).show(ui, |ui| {
                if theme::ghost_button(ui, "🔄 Yenile").on_hover_text("Şifreyi yenile").clicked() {
                    self.refresh_password();
                }
                if theme::ghost_button(ui, "📋 Kopyala").on_hover_text("ID + şifreyi kopyala").clicked() {
                    let id = self.state.pc_id.lock().unwrap().clone();
                    let pw = self.state.password.lock().unwrap().clone();
                    ctx.copy_text(format!("ID:{} Şifre:{}", id, pw));
                }
                ui.end_row();
            });
            if !self.settings_msg.is_empty() {
                ui.label(egui::RichText::new(&self.settings_msg).color(theme::danger()).small());
            }
        });
    }

    fn card_device(&mut self, ui: &mut egui::Ui) {
        theme::card_frame().show(ui, |ui| {
            theme::card_title(ui, "🖥", "CİHAZ");
            let rows = [
                ("Ad", self.state.pc_name.lock().unwrap().clone()),
                ("Yerel IP", self.state.local_ip.lock().unwrap().clone()),
                ("Çözünürlük", self.state.resolution.lock().unwrap().clone()),
                ("Kip", self.state.mode.lock().unwrap().clone()),
            ];
            // Tam genişlik satırlar: etiket solda, değer sağda.
            for (k, v) in rows {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(k).color(theme::muted()).small());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new(v).monospace().small().color(theme::text()));
                    });
                });
            }
            ui.add_space(2.0);
            let uinput = self.state.uinput_active.load(Ordering::SeqCst);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Giriş (uinput)").color(theme::muted()).small());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(if uinput { "● ETKİN" } else { "○ KAPALI" })
                            .color(if uinput { theme::success() } else { theme::danger() })
                            .strong()
                            .small(),
                    );
                });
            });
        });
    }

    fn card_signal(&mut self, ui: &mut egui::Ui) {
        theme::card_frame().show(ui, |ui| {
            theme::card_title(ui, "📡", "SİNYAL");
            ui.label(egui::RichText::new(self.signal_url.clone()).monospace().small().color(theme::text()));
            ui.add_space(6.0);
            if theme::ghost_button(ui, "✎ Adresi değiştir").clicked() {
                self.new_pc_name = self.state.pc_name.lock().unwrap().clone();
                self.new_password.clear();
                self.show_settings = true;
            }
        });
    }

    fn card_update(&mut self, ui: &mut egui::Ui) {
        theme::card_frame().show(ui, |ui| {
            theme::card_title(ui, "⬆", "GÜNCELLEME");
            let upd = self.state.update_status.lock().unwrap().clone();
            let has_update = upd.contains("yeni sürüm") || upd.contains("kuruldu");
            ui.label(
                egui::RichText::new(upd)
                    .small()
                    .color(if has_update { theme::success() } else { theme::muted() }),
            );
            ui.add_space(6.0);
            if theme::ghost_button(ui, "🔍 Güncellemeleri kontrol et").clicked() {
                self.update_modal = UpdateModalState::Idle;
            }
        });
    }

    /// Güncelleme modalı: denetle → sonuç → kur (kilitli) → otomatik yeniden başlat.
    fn update_modal_window(&mut self, ctx: &egui::Context) {
        if matches!(self.update_modal, UpdateModalState::Closed) {
            return;
        }
        // Olay pompası: arka plan iş parçacıklarından gelen sonuçlar.
        while let Ok(ev) = self.update_rx.try_recv() {
            match ev {
                UpdateEvent::Checked(Ok((info, size))) => {
                    match info {
                        Some(info) => {
                            self.state.push_event(format!("güncelleme bulundu: {}", info.version));
                            self.update_modal = UpdateModalState::Available { info, size };
                        }
                        None => self.update_modal = UpdateModalState::UpToDate,
                    }
                }
                UpdateEvent::Checked(Err(msg)) => {
                    self.update_modal = UpdateModalState::Failed { msg };
                }
                UpdateEvent::Installed(Ok(path)) => {
                    self.state.push_event("güncelleme kuruldu, yeniden başlatılıyor".to_string());
                    crate::updater::relaunch(&path);
                }
                UpdateEvent::Installed(Err(msg)) => {
                    self.update_modal = UpdateModalState::Failed { msg };
                }
            }
        }

        let locked = self.update_modal.busy();
        egui::Window::new("⬆ Güncelleme")
            .resizable(false)
            .collapsible(false)
            .min_width(430.0)
            .show(ctx, |ui| {
                match &self.update_modal {
                    UpdateModalState::Closed => {}
                    UpdateModalState::Idle => {
                        ui.label("GitHub üzerindeki en son sürüm denetlenecek.");
                        ui.add_space(8.0);
                        if theme::neon_button(ui, "🔍 Kontrol et").clicked() {
                            self.start_check();
                        }
                    }
                    UpdateModalState::Checking => {
                        ui.horizontal(|ui| {
                            ui.add(egui::Spinner::new());
                            ui.label("denetleniyor…");
                        });
                    }
                    UpdateModalState::Available { info, size } => {
                        ui.label(
                            egui::RichText::new(format!("⬆ Yeni sürüm VAR: {}", info.version))
                                .strong()
                                .size(17.0)
                                .color(theme::success()),
                        );
                        ui.label(
                            egui::RichText::new(format!("indirme boyutu: {}", size))
                                .small()
                                .color(theme::muted()),
                        );
                        if !info.notes.trim().is_empty() {
                            ui.add_space(4.0);
                            let notes: String = info.notes.chars().take(800).collect();
                            egui::ScrollArea::vertical().max_height(140.0).show(ui, |ui| {
                                ui.label(egui::RichText::new(notes).small());
                            });
                        }
                        ui.add_space(8.0);
                        if theme::neon_button(ui, &format!("⬆ Şimdi güncelle ({})", info.version)).clicked() {
                            self.start_install();
                        }
                    }
                    UpdateModalState::UpToDate => {
                        ui.label(
                            egui::RichText::new(format!(
                                "✓ Zaten güncel (v{})",
                                crate::updater::current_version()
                            ))
                            .strong()
                            .color(theme::success()),
                        );
                    }
                    UpdateModalState::Busy { phase } => {
                        ui.horizontal(|ui| {
                            ui.add(egui::Spinner::new());
                            ui.label(egui::RichText::new(phase).strong());
                        });
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new("⛔ Kurulum bitmeden bu ekrandan çıkma!")
                                .color(theme::danger())
                                .strong(),
                        );
                    }
                    UpdateModalState::Failed { msg } => {
                        ui.label(
                            egui::RichText::new(format!("❌ {}", msg))
                                .color(theme::danger())
                                .strong(),
                        );
                        ui.add_space(8.0);
                        if theme::ghost_button(ui, "↻ Tekrar dene").clicked() {
                            self.update_modal = UpdateModalState::Idle;
                        }
                    }
                }
                // Meşgulken kapatma düğmesi YOK (kilit).
                if !locked {
                    ui.add_space(8.0);
                    if theme::ghost_button(ui, "Kapat").clicked() {
                        self.update_modal = UpdateModalState::Closed;
                    }
                }
            });
    }

    fn start_check(&mut self) {
        self.update_modal = UpdateModalState::Checking;
        let tx = self.update_tx.clone();
        std::thread::spawn(move || {
            let out: Result<(Option<UpdateInfo>, String), String> = (|| {
                let info = crate::updater::check_for_update().map_err(|e| e.to_string())?;
                let size = info
                    .as_ref()
                    .and_then(|i| {
                        let url = if std::env::var_os("APPIMAGE").is_some() {
                            &i.appimage_url
                        } else {
                            &i.tarball_url
                        };
                        crate::updater::asset_size(url)
                    })
                    .map(crate::updater::human_size)
                    .unwrap_or_else(|| "bilinmiyor".to_string());
                Ok((info, size))
            })();
            let _ = tx.send(UpdateEvent::Checked(out));
        });
    }

    fn start_install(&mut self) {
        self.update_modal = UpdateModalState::Busy {
            phase: "indiriliyor…".to_string(),
        };
        let tx = self.update_tx.clone();
        let st = self.state.clone();
        std::thread::spawn(move || {
            // Kurulum tek çağrıda iner + kurulur; bitince Done → relaunch.
            let out = crate::updater::install_update(&st).map_err(|e| e.to_string());
            let _ = tx.send(UpdateEvent::Installed(out));
        });
    }

    fn card_peer(&mut self, ui: &mut egui::Ui) {
        theme::card_frame().show(ui, |ui| {
            theme::card_title(ui, "📱", "BAĞLI CİHAZ");
            let connected = self.state.peer_connected.load(Ordering::SeqCst);
            if connected {
                let since = self.state.peer_since.load(Ordering::SeqCst);
                let last = self.state.peer_last_auth.load(Ordering::SeqCst);
                let fails = self.state.peer_fails.load(Ordering::SeqCst);
                ui.label(egui::RichText::new("● BAĞLI").color(theme::success()).strong());
                ui.label(
                    egui::RichText::new(format!(
                        "{}'den beri • son doğrulama {} • başarısız: {}",
                        fmt_clock(since),
                        fmt_clock(last),
                        fails
                    ))
                    .small()
                    .color(theme::muted()),
                );
            } else {
                ui.label(egui::RichText::new("○ bağlı cihaz yok").color(theme::muted()));
                let fails = self.state.peer_fails.load(Ordering::SeqCst);
                if fails > 0 {
                    ui.label(
                        egui::RichText::new(format!("son denemeler: {} yanlış şifre", fails))
                            .small()
                            .color(theme::danger()),
                    );
                }
            }
        });
    }

    /// Orta alan: gerçek ekran aynası; kalan tüm alanı kaplar.
    fn central(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
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
                            egui::RichText::new(if self.clicker_available() {
                                "dokun = sol tık • basılı tut = sağ tık"
                            } else {
                                "izleme kipi (tıklama için /dev/uinput)"
                            })
                            .small()
                            .color(theme::muted()),
                        );
                    });
                });
                ui.add_space(4.0);
                self.preview_area(ctx, ui);
            });
        });
    }

    fn clicker_available(&mut self) -> bool {
        if self.clicker.is_none() {
            self.clicker = Some(LocalClicker::open(self.cap_w, self.cap_h));
        }
        self.clicker.as_ref().map(|c| c.available()).unwrap_or(false)
    }

    /// JPEG kareyi dokuya çevir (değişmişse) ve harf-korumalı bas + imleç çiz.
    /// Cihaz bağlı değilse ayna yerine durum ekranı basılır.
    fn preview_area(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let area_w = ui.available_width().max(200.0);
        let area_h = ui.available_height().max(200.0);
        let area = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(area_w, area_h));
        ui.painter().rect_filled(area, 8.0, theme::bg_panel());

        if !self.state.peer_connected.load(Ordering::SeqCst) {
            self.waiting_screen(ui, area);
            return;
        }
        let frame = self.state.preview_jpeg.lock().unwrap().clone();
        if let Some((bytes, fw, fh)) = frame {
            if bytes.len() != self.tex_bytes || (fw, fh) != self.tex_dims {
                if let Ok(img) = image::load_from_memory(&bytes) {
                    let rgba = img.to_rgba8();
                    let (w, h) = (rgba.width(), rgba.height());
                    let color = egui::ColorImage::from_rgba_unmultiplied(
                        [w as usize, h as usize],
                        rgba.as_raw(),
                    );
                    match self.tex.as_mut() {
                        Some(t) => t.set(color, egui::TextureOptions::LINEAR),
                        None => {
                            self.tex = Some(ctx.load_texture(
                                "onizleme",
                                color,
                                egui::TextureOptions::LINEAR,
                            ))
                        }
                    }
                    self.tex_dims = (w, h);
                    self.tex_bytes = bytes.len();
                }
            }
            if let Some(tex) = &self.tex {
                let fit = aspect_fit(area, self.tex_dims.0 as f32, self.tex_dims.1 as f32);
                ui.painter().image(
                    tex.id(),
                    fit,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
                // İmleç oku (normalize konum → kutu içi).
                if let Some((cx, cy)) = *self.state.preview_cursor.lock().unwrap() {
                    let p = egui::pos2(
                        fit.left() + cx * fit.width(),
                        fit.top() + cy * fit.height(),
                    );
                    let s = 11.0;
                    let arrow = vec![
                        p,
                        p + egui::vec2(0.0, s),
                        p + egui::vec2(3.4, s - 3.6),
                        p + egui::vec2(5.2, s - 1.2),
                    ];
                    ui.painter().add(egui::Shape::convex_polygon(
                        arrow,
                        theme::success(),
                        egui::Stroke::new(1.0_f32, egui::Color32::BLACK),
                    ));
                }
                // BAĞLANDI rozeti: cihaz bilgisi görüntünün üstünde.
                let since = self.state.peer_since.load(Ordering::SeqCst);
                let bar = egui::Rect::from_min_size(
                    fit.left_top() + egui::vec2(8.0, 8.0),
                    egui::vec2(fit.width() - 16.0, 30.0),
                );
                ui.painter().rect_filled(bar, 6.0, egui::Color32::from_black_alpha(160));
                ui.painter().text(
                    bar.left_center() + egui::vec2(10.0, 0.0),
                    egui::Align2::LEFT_CENTER,
                    format!(
                        "● BAĞLANDI  {}'den beri • {} • {}fps • {}ms",
                        fmt_clock(since),
                        self.state.resolution.lock().unwrap(),
                        self.state.fps.load(Ordering::SeqCst),
                        self.state.ping_ms.load(Ordering::SeqCst),
                    ),
                    egui::FontId::proportional(14.5),
                    theme::success(),
                );
                self.preview_click(ctx, ui, area, fit);            } else {
                // Kare var ama doku hazır değil (çözümleme patladı) → bekleme.
                self.waiting_screen(ui, area);
            }
        } else {
            self.waiting_screen(ui, area);
        }
    }

    /// Bekleme ekranı: cihaz yoksa/GİTTİyse ya da doku hazır değilse bilgi bas.
    fn waiting_screen(&mut self, ui: &mut egui::Ui, area: egui::Rect) {
        let alive = self.state.preview_alive.load(Ordering::SeqCst);
        let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some();
        let went_away = self.state.peer_since.load(Ordering::SeqCst) > 0
            && !self.state.peer_connected.load(Ordering::SeqCst);
        let (title, title_color, msg) = if went_away {
            (
                "✖ BAĞLANTI GİTTİ",
                theme::danger(),
                "cihaz ayrıldı — yeniden bekleniyor…",
            )
        } else if alive {
            ("⬢", theme::primary(), "kare bekleniyor…")
        } else if wayland {
            (
                "⬢",
                theme::primary(),
                "Wayland: önizleme spectacle/gnome-screenshot/grim'den birini kur",
            )
        } else {
            ("⬢", theme::primary(), "cihaz bekleniyor — telefondan bağlan")
        };
        ui.painter().text(
            area.center() + egui::vec2(0.0, -12.0),
            egui::Align2::CENTER_CENTER,
            title,
            egui::FontId::proportional(40.0),
            title_color,
        );
        ui.painter().text(
            area.center() + egui::vec2(0.0, 26.0),
            egui::Align2::CENTER_CENTER,
            msg,
            egui::FontId::proportional(19.0),
            theme::muted(),
        );
        ui.allocate_rect(area, egui::Sense::hover());
    }

    /// Önizleme tıklaması: dokun = sol tık, 500ms basılı = sağ tık.
    fn preview_click(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        area: egui::Rect,
        fit: egui::Rect,
    ) {
        let resp = ui.allocate_rect(area, egui::Sense::click_and_drag());
        let down = resp.is_pointer_button_down_on();
        let pos = resp.interact_pointer_pos();
        if down {
            if let Some(p) = pos {
                match self.press_start {
                    None => self.press_start = Some((Instant::now(), p)),
                    Some((t0, p0)) => {
                        if !self.press_fired
                            && t0.elapsed() >= Duration::from_millis(500)
                            && p0.distance(p) < 12.0
                        {
                            self.press_fired = true;
                            self.fire_click(ctx, fit, p, true);
                        }
                    }
                }
            }
        } else {
            if resp.clicked() && !self.press_fired {
                if let Some(p) = pos {
                    self.fire_click(ctx, fit, p, false);
                }
            }
            self.press_start = None;
            self.press_fired = false;
        }
    }

    fn fire_click(&mut self, _ctx: &egui::Context, fit: egui::Rect, p: egui::Pos2, right: bool) {
        if !fit.contains(p) {
            return;
        }
        let x = ((p.x - fit.left()) / fit.width() * self.cap_w as f32) as i32;
        let y = ((p.y - fit.top()) / fit.height() * self.cap_h as f32) as i32;
        if self.clicker.is_none() {
            self.clicker = Some(LocalClicker::open(self.cap_w, self.cap_h));
        }
        let label = if right { "SAĞ" } else { "SOL" };
        match &self.clicker {
            Some(c) => {
                let r = if right { c.right_click_at(x, y) } else { c.click_at(x, y) };
                match r {
                    Ok(()) => self.state.push_event(format!("YEREL {} tık {} {}", label, x, y)),
                    Err(e) => self.state.push_event(format!("yerel tık hatası: {}", e)),
                }
            }
            None => self.state.push_event("yerel tıklayıcı yok".to_string()),
        }
    }

    /// Alta sabit olay akışı paneli.
    /// Olay akışı kartı: sol panelin en altında, kalan boşluğu doldurur.
    fn card_events(&mut self, ui: &mut egui::Ui) {
        theme::card_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("OLAY AKIŞI").color(theme::primary()).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if theme::ghost_button_auto(ui, "🗑 Temizle").clicked() {
                        self.state.clear_events();
                    }
                });
            });
            let lines: Vec<String> =
                self.state.events.lock().unwrap().iter().rev().take(60).cloned().collect();
            // Kalan dikey boşluğun tamamı kaydırma alanına verilir.
            let h = ui.available_height().max(90.0);
            egui::ScrollArea::vertical().max_height(h).auto_shrink([false, true]).show(ui, |ui| {
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
                });
        });
    }

    /// Ayarlar penceresi: sinyal + ad + şifre.
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
                if theme::neon_button(ui, "💾 Kaydet").clicked() {
                    self.save_settings();
                }
                ui.add_space(4.0);
                if theme::ghost_button(ui, "Vazgeç").clicked() {
                    self.show_settings = false;
                }
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

/// Harf-korumalı sığdırma: dokuyu bozmadan kutunun içine ortala.
fn aspect_fit(area: egui::Rect, iw: f32, ih: f32) -> egui::Rect {
    if iw <= 0.0 || ih <= 0.0 {
        return area;
    }
    let scale = (area.width() / iw).min(area.height() / ih);
    let fw = iw * scale;
    let fh = ih * scale;
    egui::Rect::from_min_size(
        egui::pos2(area.left() + (area.width() - fw) / 2.0, area.top() + (area.height() - fh) / 2.0),
        egui::vec2(fw, fh),
    )
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
        self.update_modal_window(ctx);
        ctx.request_repaint_after(Duration::from_millis(200));
    }
}
