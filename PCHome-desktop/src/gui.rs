use crate::local_input::LocalClicker;
use crate::state::SharedState;
use crate::theme;
use crate::updater::UpdateInfo;
use eframe::egui;
use egui::{Align2, Color32, FontId, RichText};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Masaüstü GUI giriş noktası (ana iş parçacığında winit döngüsünü bloklar).
pub fn run(state: SharedState) {
    let focus_request = Arc::new(AtomicBool::new(false));
    #[cfg(unix)]
    {
        use signal_hook::consts::SIGUSR1;
        if let Err(e) = signal_hook::flag::register(SIGUSR1, Arc::clone(&focus_request)) {
            log::warn!("SIGUSR1 dinleyicisi kurulamadı: {}", e);
        }
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([1000.0, 680.0])
            .with_icon(app_icon()),
        ..Default::default()
    };
    let _ = eframe::run_native(
        "PCHome — Uzak Masaüstü",
        options,
        Box::new(|cc| {
            theme::apply(&cc.egui_ctx);
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
            *state.update_status.lock().unwrap() = format!(
                "v{} — güncellemeler denetleniyor…",
                crate::updater::current_version()
            );
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
                preview_seq: 0,
                clicker: None,
                press_start: None,
                press_fired: false,
                update_modal: UpdateModalState::Closed,
                update_tx,
                update_rx,
                settings_rect: None,
                update_rect: None,
                was_peer_connected: false,
                events_open: false,
                focus_request,
            }))
        }),
    );
}

/// Kullanıcının verdiği PCHome simgesi (yuvarlak içinde telefon).
fn app_icon() -> egui::IconData {
    const ICON_PNG: &[u8] = include_bytes!("../../assets/icon.png");
    let img = image::load_from_memory(ICON_PNG).expect("paketli simge geçerli bir PNG olmalı");
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
    preview_seq: u64,
    clicker: Option<LocalClicker>,
    press_start: Option<(Instant, egui::Pos2)>,
    press_fired: bool,
    update_modal: UpdateModalState,
    update_tx: Sender<UpdateEvent>,
    update_rx: Receiver<UpdateEvent>,
    settings_rect: Option<egui::Rect>,
    update_rect: Option<egui::Rect>,
    was_peer_connected: bool,
    events_open: bool,
    focus_request: Arc<AtomicBool>,
}

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
        matches!(
            self,
            UpdateModalState::Checking | UpdateModalState::Busy { .. }
        )
    }
}

enum UpdateEvent {
    Checked(Result<(Option<UpdateInfo>, String), String>),
    Installed(Result<PathBuf, String>),
}

fn fmt_clock(epoch: u32) -> String {
    if epoch == 0 {
        return "—".to_string();
    }
    chrono::DateTime::from_timestamp(epoch as i64, 0)
        .map(|dt| dt.with_timezone(&chrono::Local).format("%H:%M").to_string())
        .unwrap_or_else(|| "—".to_string())
}

fn fmt_id(id: &str) -> String {
    let clean: String = id.chars().filter(|c| c.is_ascii_digit()).collect();
    if clean.len() == 6 {
        format!("{} {}", &clean[..3], &clean[3..])
    } else {
        clean
    }
}

fn status_of(status: &str) -> (&'static str, Color32) {
    if status.contains("BAĞLI") {
        ("Bağlı", theme::success())
    } else if status.contains("DENEN") || status.contains("DEĞİŞTİ") {
        ("Bağlanıyor", theme::warning())
    } else if status.contains("BAŞARISIZ") || status.contains("YOK") {
        ("Bağlantı yok", theme::danger())
    } else {
        ("Hazırlanıyor", theme::muted())
    }
}

fn event_color(line: &str) -> egui::Color32 {
    let l = line.to_lowercase();
    if l.contains("yanlış") || l.contains("hata") || l.contains("kilit") || l.contains("başarısız")
    {
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

fn sidebar_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(10.5).color(theme::muted()));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                RichText::new(value)
                    .font(theme::mono_font(10.0))
                    .color(theme::text2()),
            );
        });
    });
    ui.add_space(4.0);
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, 0.0, theme::border());
    ui.add_space(4.0);
}

fn status_sep(ui: &mut egui::Ui) {
    ui.label(RichText::new("·").size(10.0).color(theme::muted()));
}

fn text_link(ui: &mut egui::Ui, label: &str) -> egui::Response {
    let font = FontId::proportional(10.0);
    let galley = ui.fonts(|f| f.layout_no_wrap(label.to_owned(), font.clone(), theme::muted()));
    let (rect, resp) =
        ui.allocate_exact_size(galley.size() + egui::vec2(6.0, 4.0), egui::Sense::click());
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        font,
        if resp.hovered() {
            theme::accent()
        } else {
            theme::muted()
        },
    );
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn dash(s: String) -> String {
    if s.trim().is_empty() {
        "—".to_string()
    } else {
        s
    }
}

fn small_metric(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.label(RichText::new(label).size(8.5).color(theme::muted()));
    ui.label(
        RichText::new(value)
            .font(theme::mono_font(10.5))
            .color(theme::text2()),
    );
}

impl DesktopGui {
    fn top_bar(&mut self, ctx: &egui::Context) {
        let status = self.state.status.lock().unwrap().clone();
        let (label, color) = status_of(&status);
        egui::TopBottomPanel::top("topbar")
            .exact_height(52.0)
            .frame(
                egui::Frame::none()
                    .fill(theme::bg())
                    .inner_margin(egui::Margin::symmetric(20.0, 0.0)),
            )
            .show(ctx, |ui| {
                let r = ui.max_rect();
                ui.painter().rect_filled(
                    egui::Rect::from_min_max(
                        egui::pos2(r.left(), r.bottom() - 1.0),
                        r.right_bottom(),
                    ),
                    0.0,
                    theme::border(),
                );
                ui.horizontal_centered(|ui| {
                    ui.label(RichText::new("PCHome").size(15.0).color(theme::text()));
                    ui.add_space(5.0);
                    ui.label(RichText::new("DESKTOP").size(8.5).color(theme::muted()));
                    ui.add_space(16.0);
                    theme::dot(ui, color, 6.0);
                    ui.label(RichText::new(label).size(11.0).color(theme::text2()));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if theme::icon_button(ui, theme::I_POWER, "Kapat", theme::danger())
                            .clicked()
                        {
                            if self.update_modal.busy() {
                                self.state
                                    .push_event("kurulum bitmeden kapatılamaz — bekle".to_string());
                            } else {
                                self.state.terminate.store(true, Ordering::SeqCst);
                            }
                        }
                        if theme::icon_button(ui, theme::I_SETTINGS, "Ayarlar", theme::muted())
                            .clicked()
                        {
                            self.new_pc_name = self.state.pc_name.lock().unwrap().clone();
                            self.new_password.clear();
                            self.settings_msg.clear();
                            self.show_settings = !self.show_settings;
                        }
                        if theme::icon_button(
                            ui,
                            theme::I_REFRESH,
                            "Ekran yakalamayı yenile",
                            theme::muted(),
                        )
                        .clicked()
                        {
                            self.state.restart_capture.fetch_add(1, Ordering::SeqCst);
                            self.state
                                .push_event("ekran yakalama yenileniyor…".to_string());
                        }
                        ui.add_space(10.0);
                        ui.label(
                            RichText::new(format!("v{}", crate::updater::current_version()))
                                .font(theme::mono_font(9.5))
                                .color(theme::muted()),
                        );
                    });
                });
            });
    }

    fn side_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("side")
            .resizable(false)
            .exact_width(318.0)
            .frame(
                egui::Frame::none()
                    .fill(theme::sidebar())
                    .inner_margin(egui::Margin {
                        left: 20.0,
                        right: 18.0,
                        top: 18.0,
                        bottom: 14.0,
                    }),
            )
            .show(ctx, |ui| {
                let r = ui.max_rect();
                ui.painter().rect_filled(
                    egui::Rect::from_min_max(
                        egui::pos2(r.right() + 17.0, r.top() - 18.0),
                        egui::pos2(r.right() + 18.0, r.bottom() + 14.0),
                    ),
                    0.0,
                    theme::border(),
                );
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        self.sidebar_identity(ctx, ui);
                        theme::divider(ui);
                        theme::section_label(ui, "CİHAZ");
                        ui.add_space(6.0);
                        self.sidebar_device(ui);
                        theme::divider(ui);
                        self.sidebar_peer(ui);
                        theme::divider(ui);
                        self.sidebar_server(ui);
                    });
            });
    }

    fn sidebar_identity(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        theme::section_label(ui, "PC KİMLİĞİ");
        ui.add_space(3.0);
        let id = self.state.pc_id.lock().unwrap().clone();
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(fmt_id(&id))
                    .font(theme::mono_bold_font(36.0))
                    .color(theme::text()),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if theme::icon_button(ui, theme::I_COPY, "ID + şifreyi kopyala", theme::muted())
                    .clicked()
                {
                    let pw = self.state.password.lock().unwrap().clone();
                    ctx.copy_text(format!("ID: {}  Şifre: {}", id, pw));
                    self.state.push_event("ID ve şifre kopyalandı".to_string());
                }
            });
        });
        ui.label(
            RichText::new("Telefondaki PC ID alanına gir")
                .size(10.0)
                .color(theme::muted()),
        );
        ui.add_space(14.0);

        theme::section_label(ui, "ŞİFRE");
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            let shown = if self.show_password {
                self.state.password.lock().unwrap().clone()
            } else {
                "••••••".to_string()
            };
            ui.label(
                RichText::new(shown)
                    .font(theme::mono_font(17.0))
                    .color(theme::text2()),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if theme::icon_button(ui, theme::I_REFRESH, "Şifreyi yenile", theme::muted())
                    .clicked()
                {
                    self.refresh_password();
                }
                if theme::icon_button(ui, theme::I_COPY, "ID + şifreyi kopyala", theme::muted())
                    .clicked()
                {
                    let pw = self.state.password.lock().unwrap().clone();
                    ctx.copy_text(format!("ID: {}  Şifre: {}", id, pw));
                    self.state.push_event("ID ve şifre kopyalandı".to_string());
                }
                let eye = if self.show_password {
                    theme::I_EYE_OFF
                } else {
                    theme::I_EYE
                };
                if theme::icon_button(ui, eye, "Göster / gizle", theme::muted()).clicked() {
                    self.show_password = !self.show_password;
                }
            });
        });
        ui.label(
            RichText::new("6 haneli sabit şifre")
                .size(9.5)
                .color(theme::muted()),
        );
        if !self.settings_msg.is_empty() {
            ui.add_space(4.0);
            let ok = self.settings_msg.contains("güncellendi")
                || self.settings_msg.contains("kaydedildi");
            ui.label(RichText::new(&self.settings_msg).size(10.0).color(if ok {
                theme::success()
            } else {
                theme::danger()
            }));
        }
    }

    fn sidebar_device(&mut self, ui: &mut egui::Ui) {
        let rows = [
            ("Ad", dash(self.state.pc_name.lock().unwrap().clone())),
            (
                "Yerel IP",
                dash(self.state.local_ip.lock().unwrap().clone()),
            ),
            (
                "Çözünürlük",
                dash(self.state.resolution.lock().unwrap().clone()),
            ),
            (
                "Yakalama",
                dash(self.state.capture_status.lock().unwrap().clone()),
            ),
            ("Kip", dash(self.state.mode.lock().unwrap().clone())),
        ];
        for (k, v) in rows {
            sidebar_row(ui, k, &v);
        }
    }

    fn sidebar_peer(&mut self, ui: &mut egui::Ui) {
        theme::section_label(ui, "BAĞLI CİHAZ");
        ui.add_space(5.0);
        let connected = self.state.peer_connected.load(Ordering::SeqCst);
        let peer_name = self.state.peer_name.lock().unwrap().clone();
        let seen = self.state.seen_devices.lock().unwrap().clone();
        ui.horizontal(|ui| {
            theme::dot(
                ui,
                if connected {
                    theme::success()
                } else {
                    theme::muted()
                },
                6.0,
            );
            ui.label(
                RichText::new(if connected { "Bağlı" } else { "Bekleniyor" })
                    .size(11.5)
                    .color(theme::text2()),
            );
        });
        if connected {
            if !peer_name.is_empty() {
                ui.label(RichText::new(peer_name).size(13.0).color(theme::text()));
            }
            let since = self.state.peer_since.load(Ordering::SeqCst);
            let last = self.state.peer_last_auth.load(Ordering::SeqCst);
            ui.label(
                RichText::new(format!(
                    "{}'den beri  •  son doğrulama {}",
                    fmt_clock(since),
                    fmt_clock(last)
                ))
                .size(9.5)
                .color(theme::muted()),
            );
        } else {
            ui.label(
                RichText::new("Henüz bağlı cihaz yok")
                    .size(11.0)
                    .color(theme::text2()),
            );
            ui.label(
                RichText::new("Telefondan bu ID ve şifreyle bağlan.")
                    .size(9.5)
                    .color(theme::muted()),
            );
        }
        if !seen.is_empty() {
            ui.add_space(8.0);
            theme::section_label(ui, "SON CİHAZLAR");
            for device in seen.iter().take(2) {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&device.name).size(10.0).color(theme::text2()));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(crate::device::format_seen(device.last_seen))
                                .font(theme::mono_font(9.0))
                                .color(theme::muted()),
                        );
                    });
                });
            }
        }
    }

    fn sidebar_server(&mut self, ui: &mut egui::Ui) {
        let upd = self.state.update_status.lock().unwrap().clone();
        let has_update = upd.contains("yeni sürüm") || upd.contains("kuruldu");
        theme::section_label(ui, "SUNUCU");
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(&self.signal_url)
                    .font(theme::mono_font(9.0))
                    .color(theme::text2()),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if theme::icon_button(ui, theme::I_SETTINGS, "Adresi düzenle", theme::muted())
                    .clicked()
                {
                    self.new_pc_name = self.state.pc_name.lock().unwrap().clone();
                    self.new_password.clear();
                    self.settings_msg.clear();
                    self.show_settings = true;
                }
            });
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("v{}", crate::updater::current_version()))
                    .font(theme::mono_font(9.0))
                    .color(theme::muted()),
            );
            if has_update {
                theme::dot(ui, theme::success(), 5.0);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let resp = text_link(ui, "Güncelle");
                if resp.on_hover_text(upd).clicked() {
                    self.update_modal = UpdateModalState::Idle;
                }
            });
        });
    }

    fn central(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(theme::bg())
                    .inner_margin(egui::Margin {
                        left: 20.0,
                        right: 20.0,
                        top: 14.0,
                        bottom: 14.0,
                    }),
            )
            .show(ctx, |ui| {
                let avail = ui.available_size();
                let header_h = 16.0;
                let status_h = 30.0;
                let drawer_h = if self.events_open { 186.0 } else { 30.0 };
                let stage_h =
                    (avail.y - header_h - 6.0 - status_h - 6.0 - drawer_h - 6.0).max(200.0);

                ui.allocate_ui_with_layout(
                    egui::vec2(avail.x, header_h),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        let t = ctx.input(|i| i.time);
                        let pulse = ((t * 2.4).sin() * 0.5 + 0.5) as f32;
                        let connected = self.state.peer_connected.load(Ordering::SeqCst);
                        theme::dot(
                            ui,
                            if connected {
                                theme::success()
                            } else {
                                theme::alpha(theme::muted(), 140)
                            },
                            5.0,
                        );
                        ui.add_space(2.0);
                        ui.label(
                            RichText::new("CANLI ÖNİZLEME")
                                .size(9.5)
                                .color(theme::muted())
                                .strong(),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let res = self.state.resolution.lock().unwrap().clone();
                            ui.label(
                                RichText::new(res)
                                    .font(theme::mono_font(9.5))
                                    .color(theme::muted()),
                            );
                            let _ = pulse;
                        });
                    },
                );
                ui.add_space(6.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(avail.x, stage_h),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        self.preview_area(ctx, ui);
                    },
                );
                ui.add_space(4.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(avail.x, status_h),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        let ping = self.state.ping_ms.load(Ordering::SeqCst);
                        let fps = self.state.fps.load(Ordering::SeqCst);
                        let uinput = self.state.uinput_active.load(Ordering::SeqCst);
                        small_metric(ui, "GECİKME", &format!("{} ms", ping));
                        status_sep(ui);
                        small_metric(ui, "FPS", &format!("{}", fps));
                        status_sep(ui);
                        small_metric(ui, "GİRİŞ", if uinput { "AKTİF" } else { "KAPALI" });
                        if uinput {
                            theme::dot(ui, theme::success(), 5.0);
                        }
                    },
                );
                ui.add_space(4.0);
                self.events_drawer(ui, drawer_h);
            });
    }

    fn clicker_available(&mut self) -> bool {
        if self.clicker.is_none() {
            self.clicker = Some(LocalClicker::open(self.cap_w, self.cap_h));
        }
        self.clicker
            .as_ref()
            .map(|c| c.available())
            .unwrap_or(false)
    }

    fn preview_area(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let area_w = ui.available_width().max(200.0);
        let area_h = ui.available_height().max(160.0);
        let area = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(area_w, area_h));
        ui.painter()
            .rect_filled(area, egui::Rounding::same(10.0), theme::stage());
        ui.painter().rect_stroke(
            area,
            egui::Rounding::same(10.0),
            egui::Stroke::new(1.0_f32, theme::border()),
        );

        let frame = self.state.preview_jpeg.lock().unwrap().clone();
        let seq = self.state.preview_seq.load(Ordering::SeqCst);
        if let Some((bytes, _fw, _fh)) = frame {
            if seq != self.preview_seq {
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
                    self.preview_seq = seq;
                }
            }
            if let Some(tex) = &self.tex {
                let fit = aspect_fit(area, self.tex_dims.0 as f32, self.tex_dims.1 as f32);
                ui.painter().image(
                    tex.id(),
                    fit,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
                if let Some((cx, cy)) = *self.state.preview_cursor.lock().unwrap() {
                    let p =
                        egui::pos2(fit.left() + cx * fit.width(), fit.top() + cy * fit.height());
                    let s = 11.0;
                    let arrow = vec![
                        p,
                        p + egui::vec2(0.0, s),
                        p + egui::vec2(3.4, s - 3.6),
                        p + egui::vec2(5.2, s - 1.2),
                    ];
                    ui.painter().add(egui::Shape::convex_polygon(
                        arrow,
                        Color32::WHITE,
                        egui::Stroke::new(1.0_f32, Color32::BLACK),
                    ));
                }
                if self.state.peer_connected.load(Ordering::SeqCst) {
                    let since = self.state.peer_since.load(Ordering::SeqCst);
                    let bar = egui::Rect::from_min_size(
                        fit.left_top() + egui::vec2(10.0, 10.0),
                        egui::vec2(150.0, 24.0),
                    );
                    ui.painter().rect_filled(
                        bar,
                        egui::Rounding::same(6.0),
                        egui::Color32::from_black_alpha(180),
                    );
                    ui.painter().circle_filled(
                        bar.left_center() + egui::vec2(11.0, 0.0),
                        3.5,
                        theme::success(),
                    );
                    ui.painter().text(
                        bar.left_center() + egui::vec2(22.0, 0.0),
                        egui::Align2::LEFT_CENTER,
                        format!("Bağlandı  {}", fmt_clock(since)),
                        egui::FontId::proportional(9.5),
                        theme::text2(),
                    );
                }
                self.preview_click(ctx, ui, area, fit);
            } else {
                self.waiting_screen(ui, area);
            }
        } else {
            self.waiting_screen(ui, area);
        }
    }

    fn waiting_screen(&mut self, ui: &mut egui::Ui, area: egui::Rect) {
        let alive = self.state.preview_alive.load(Ordering::SeqCst);
        let peer_since = self.state.peer_since.load(Ordering::SeqCst);
        let peer_connected = self.state.peer_connected.load(Ordering::SeqCst);
        let (title, msg, accent) = if peer_since > 0 && !peer_connected {
            (
                "Bağlantı kesildi",
                "Cihaz ayrıldı — yeniden bekleniyor…",
                theme::danger(),
            )
        } else if peer_since == 0 {
            (
                "Telefondan bağlanın",
                "PCHome mobil uygulamasında bu ID ve şifreyi girin.",
                theme::muted(),
            )
        } else if alive {
            (
                "İlk kare bekleniyor…",
                "Ekran akışı hazırlanıyor.",
                theme::muted(),
            )
        } else {
            (
                "Telefondan bağlanın",
                "PCHome mobil uygulamasında bu ID ve şifreyi girin.",
                theme::muted(),
            )
        };
        let t = ui.input(|i| i.time);
        let center = area.center() - egui::vec2(0.0, 26.0);
        for i in 1..=3 {
            let r = 20.0 + i as f32 * 14.0;
            ui.painter()
                .circle_stroke(center, r, egui::Stroke::new(1.0_f32, theme::border()));
        }
        let angle = t as f32 * 0.9;
        let end = center + egui::vec2(angle.cos() * 62.0, angle.sin() * 62.0);
        ui.painter().line_segment(
            [center, end],
            egui::Stroke::new(1.0_f32, theme::alpha(accent, 80)),
        );
        ui.painter().circle_filled(end, 2.0, accent);
        ui.painter().circle_filled(center, 2.0, theme::muted());

        ui.painter().text(
            area.center() + egui::vec2(0.0, 42.0),
            Align2::CENTER_CENTER,
            title,
            FontId::proportional(13.5),
            if title == "Bağlantı kesildi" {
                theme::danger()
            } else {
                theme::text()
            },
        );
        ui.painter().text(
            area.center() + egui::vec2(0.0, 60.0),
            Align2::CENTER_CENTER,
            msg,
            FontId::proportional(10.5),
            theme::muted(),
        );
        ui.allocate_rect(area, egui::Sense::hover());
    }

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
                let r = if right {
                    c.right_click_at(x, y)
                } else {
                    c.click_at(x, y)
                };
                match r {
                    Ok(()) => self
                        .state
                        .push_event(format!("YEREL {} tık {} {}", label, x, y)),
                    Err(e) => self.state.push_event(format!("yerel tık hatası: {}", e)),
                }
            }
            None => self.state.push_event("yerel tıklayıcı yok".to_string()),
        }
    }

    fn events_drawer(&mut self, ui: &mut egui::Ui, height: f32) {
        let (rect, resp) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 28.0), egui::Sense::click());
        ui.painter().rect_filled(
            egui::Rect::from_min_max(
                egui::pos2(rect.left(), rect.top()),
                egui::pos2(rect.right(), rect.top() + 1.0),
            ),
            0.0,
            theme::border(),
        );
        ui.painter().text(
            egui::pos2(rect.left(), rect.center().y),
            Align2::LEFT_CENTER,
            "OLAYLAR",
            FontId::proportional(9.0),
            theme::muted(),
        );
        let count = self.state.events.lock().unwrap().len();
        if resp.hovered() {
            ui.painter()
                .rect_filled(rect, egui::Rounding::same(4.0), theme::hover());
        }
        ui.painter().text(
            egui::pos2(rect.right() - 16.0, rect.center().y),
            Align2::RIGHT_CENTER,
            count.to_string(),
            FontId::proportional(9.5),
            theme::muted(),
        );
        ui.painter().text(
            egui::pos2(rect.right(), rect.center().y),
            Align2::RIGHT_CENTER,
            if self.events_open {
                theme::I_CARET_DOWN
            } else {
                theme::I_CARET_RIGHT
            },
            theme::icon_font(13.0),
            theme::muted(),
        );
        let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
        if resp.clicked() {
            self.events_open = !self.events_open;
        }

        if self.events_open {
            ui.add_space(4.0);
            let lines: Vec<String> = self
                .state
                .events
                .lock()
                .unwrap()
                .iter()
                .rev()
                .take(80)
                .cloned()
                .collect();
            egui::ScrollArea::vertical()
                .max_height((height - 38.0).max(40.0))
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if lines.is_empty() {
                        ui.label(
                            RichText::new("Henüz olay yok.")
                                .size(10.0)
                                .color(theme::muted()),
                        );
                    }
                    for line in lines {
                        let color = event_color(&line);
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing.x = 6.0;
                            theme::dot(ui, color, 4.0);
                            ui.add(
                                egui::Label::new(
                                    RichText::new(line)
                                        .font(theme::mono_font(9.5))
                                        .color(theme::text2()),
                                )
                                .wrap(),
                            );
                        });
                    }
                });
        }
    }

    fn settings_window(&mut self, ctx: &egui::Context) {
        if !self.show_settings {
            self.settings_rect = None;
            return;
        }
        let was_open = self.settings_rect.is_some();
        let mut signal_url = self.signal_url.clone();
        let mut pc_name = self.new_pc_name.clone();
        let mut new_password = self.new_password.clone();
        let mut close = false;
        let mut save = false;

        let resp = egui::Window::new("ayarlar")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .frame(theme::panel_frame())
            .anchor(Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.set_width(380.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Ayarlar").size(16.0).color(theme::text()));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if theme::icon_button(ui, theme::I_CLOSE, "Kapat", theme::muted()).clicked()
                        {
                            close = true;
                        }
                    });
                });
                ui.label(
                    RichText::new("Bağlantı ve kimlik bilgileri")
                        .size(10.0)
                        .color(theme::muted()),
                );
                ui.add_space(14.0);
                theme::field(
                    ui,
                    "SİNYAL SUNUCUSU",
                    &mut signal_url,
                    "wss://sunucu-adresi/ws",
                    true,
                );
                ui.add_space(8.0);
                theme::field(ui, "PC ADI", &mut pc_name, "SistemBey", false);
                ui.add_space(8.0);
                theme::field(
                    ui,
                    "YENİ ŞİFRE",
                    &mut new_password,
                    "6 hane • boş bırak = değişmesin",
                    true,
                );
                if !self.settings_msg.is_empty() {
                    ui.add_space(6.0);
                    let ok = self.settings_msg.contains("güncellendi")
                        || self.settings_msg.contains("kaydedildi");
                    ui.label(RichText::new(&self.settings_msg).size(10.0).color(if ok {
                        theme::success()
                    } else {
                        theme::danger()
                    }));
                }
                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    if theme::ghost_button_auto(ui, "Vazgeç").clicked() {
                        close = true;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if theme::neon_button_auto(ui, "Kaydet").clicked() {
                            save = true;
                        }
                    });
                });
            });
        if let Some(r) = resp {
            self.settings_rect = Some(r.response.rect);
        }
        if save {
            self.signal_url = signal_url;
            self.new_pc_name = pc_name;
            self.new_password = new_password;
            self.save_settings();
        } else if close || (was_open && outside_tap(ctx, self.settings_rect)) {
            self.show_settings = false;
            self.settings_rect = None;
        }
    }

    fn update_modal_window(&mut self, ctx: &egui::Context) {
        if matches!(self.update_modal, UpdateModalState::Closed) {
            self.update_rect = None;
            return;
        }
        let was_open = self.update_rect.is_some();
        while let Ok(ev) = self.update_rx.try_recv() {
            match ev {
                UpdateEvent::Checked(Ok((info, size))) => match info {
                    Some(info) => {
                        self.state
                            .push_event(format!("güncelleme bulundu: {}", info.version));
                        self.update_modal = UpdateModalState::Available { info, size };
                    }
                    None => self.update_modal = UpdateModalState::UpToDate,
                },
                UpdateEvent::Checked(Err(msg)) => {
                    self.update_modal = UpdateModalState::Failed { msg };
                }
                UpdateEvent::Installed(Ok(path)) => {
                    self.state
                        .push_event("güncelleme kuruldu, yeniden başlatılıyor".to_string());
                    crate::updater::relaunch(&path);
                }
                UpdateEvent::Installed(Err(msg)) => {
                    self.update_modal = UpdateModalState::Failed { msg };
                }
            }
        }

        let locked = self.update_modal.busy();
        let mut close = false;
        let resp = egui::Window::new("guncelleme")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .frame(theme::panel_frame())
            .anchor(Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.set_width(390.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Güncelleme").size(16.0).color(theme::text()));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if !locked
                            && theme::icon_button(ui, theme::I_CLOSE, "Kapat", theme::muted())
                                .clicked()
                        {
                            close = true;
                        }
                    });
                });
                ui.label(
                    RichText::new("GitHub üzerindeki en son sürüm")
                        .size(10.0)
                        .color(theme::muted()),
                );
                ui.add_space(12.0);
                match &self.update_modal {
                    UpdateModalState::Closed => {}
                    UpdateModalState::Idle => {
                        ui.label(
                            RichText::new("En son sürüm denetlenecek.")
                                .size(12.0)
                                .color(theme::text2()),
                        );
                        ui.add_space(12.0);
                        if theme::neon_button_auto(ui, "Kontrol et").clicked() {
                            self.start_check();
                        }
                    }
                    UpdateModalState::Checking => {
                        ui.horizontal(|ui| {
                            ui.add(egui::Spinner::new().color(theme::accent()));
                            ui.label(
                                RichText::new("Denetleniyor…")
                                    .size(11.0)
                                    .color(theme::text2()),
                            );
                        });
                    }
                    UpdateModalState::Available { info, size } => {
                        ui.label(
                            RichText::new(format!("Yeni sürüm: {}", info.version))
                                .size(15.0)
                                .color(theme::text())
                                .strong(),
                        );
                        ui.label(
                            RichText::new(format!("İndirme boyutu: {}", size))
                                .size(10.0)
                                .color(theme::muted()),
                        );
                        if !info.notes.trim().is_empty() {
                            ui.add_space(6.0);
                            let notes: String = info.notes.chars().take(800).collect();
                            egui::ScrollArea::vertical()
                                .max_height(140.0)
                                .show(ui, |ui| {
                                    ui.label(RichText::new(notes).size(10.0).color(theme::muted()));
                                });
                        }
                        ui.add_space(12.0);
                        if theme::neon_button_auto(ui, &format!("Güncelle ({})", info.version))
                            .clicked()
                        {
                            self.start_install();
                        }
                    }
                    UpdateModalState::UpToDate => {
                        ui.horizontal(|ui| {
                            theme::dot(ui, theme::success(), 5.0);
                            ui.label(
                                RichText::new(format!(
                                    "Zaten güncel (v{})",
                                    crate::updater::current_version()
                                ))
                                .size(12.0)
                                .color(theme::text2()),
                            );
                        });
                    }
                    UpdateModalState::Busy { phase } => {
                        ui.horizontal(|ui| {
                            ui.add(egui::Spinner::new().color(theme::accent()));
                            ui.label(RichText::new(phase).size(11.0).color(theme::text2()));
                        });
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new("Kurulum bitmeden bu ekrandan çıkma!")
                                .size(10.0)
                                .color(theme::danger()),
                        );
                    }
                    UpdateModalState::Failed { msg } => {
                        ui.label(
                            RichText::new(format!("Hata: {}", msg))
                                .size(11.0)
                                .color(theme::danger()),
                        );
                        ui.add_space(12.0);
                        if theme::ghost_button_auto(ui, "Tekrar dene").clicked() {
                            self.update_modal = UpdateModalState::Idle;
                        }
                    }
                }
                if !locked {
                    ui.add_space(12.0);
                    if theme::ghost_button_auto(ui, "Kapat").clicked() {
                        close = true;
                    }
                }
            });
        if let Some(r) = resp {
            self.update_rect = Some(r.response.rect);
        }
        if close || (was_open && !locked && outside_tap(ctx, self.update_rect)) {
            self.update_modal = UpdateModalState::Closed;
            self.update_rect = None;
        }
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
            phase: "İndiriliyor…".to_string(),
        };
        let tx = self.update_tx.clone();
        let st = self.state.clone();
        std::thread::spawn(move || {
            let out = crate::updater::install_update(&st).map_err(|e| e.to_string());
            let _ = tx.send(UpdateEvent::Installed(out));
        });
    }

    fn refresh_password(&mut self) {
        match crate::identity::Identity::load_or_create() {
            Ok(mut id) => match id.regenerate_password() {
                Ok(()) => {
                    *self.state.password.lock().unwrap() = id.password.clone();
                    self.state.push_event("şifre yenilendi".to_string());
                    self.settings_msg = "şifre yenilendi".to_string();
                }
                Err(e) => self.settings_msg = format!("şifre yenilenemedi: {}", e),
            },
            Err(e) => self.settings_msg = format!("kimlik açılamadı: {}", e),
        }
    }

    fn save_settings(&mut self) {
        let raw_url = self.signal_url.trim().to_string();
        if !raw_url.is_empty() {
            let Some(url) = crate::config::Config::normalize_signal_url(&raw_url) else {
                self.settings_msg = "geçerli bir sinyal adresi gir".to_string();
                return;
            };
            let _ = crate::config::Config::save_signal_url(&url);
            self.signal_url = url.clone();
            *self.state.signal_url.lock().unwrap() = url.clone();
            *self.state.status.lock().unwrap() = "SİNYAL: ADRES GÜNCELLENDİ".to_string();
            self.state.push_event(format!("sinyal adresi: {}", url));
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

fn outside_tap(ctx: &egui::Context, rect: Option<egui::Rect>) -> bool {
    let Some(r) = rect else {
        return false;
    };
    ctx.input(|i| {
        i.pointer.button_pressed(egui::PointerButton::Primary)
            && i.pointer
                .press_origin()
                .map(|p| !r.contains(p))
                .unwrap_or(false)
    })
}

fn aspect_fit(area: egui::Rect, iw: f32, ih: f32) -> egui::Rect {
    if iw <= 0.0 || ih <= 0.0 {
        return area;
    }
    let scale = (area.width() / iw).min(area.height() / ih);
    let fw = iw * scale;
    let fh = ih * scale;
    egui::Rect::from_min_size(
        egui::pos2(
            area.left() + (area.width() - fw) / 2.0,
            area.top() + (area.height() - fh) / 2.0,
        ),
        egui::vec2(fw, fh),
    )
}

impl eframe::App for DesktopGui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.state.terminate.load(Ordering::SeqCst) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        theme::apply(ctx);
        let peer = self.state.peer_connected.load(Ordering::SeqCst);
        if (peer && !self.was_peer_connected) || self.focus_request.swap(false, Ordering::SeqCst) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(
                egui::UserAttentionType::Informational,
            ));
        }
        self.was_peer_connected = peer;
        self.top_bar(ctx);
        self.side_panel(ctx);
        self.central(ctx);
        self.settings_window(ctx);
        self.update_modal_window(ctx);
        ctx.request_repaint_after(Duration::from_millis(200));
    }
}
