mod collector;
mod health;
mod self_monitor;
mod ui;

use eframe::egui::{self, RichText};
use serde_json::Value;
use std::{sync::{Arc, RwLock}, time::{Duration, Instant}};

use health::{BG, LINE, WHITE};
use self_monitor::ProcessMeter;
use ui::cards::{dashboard, header, number, pct, subcard, GAP};

struct App {
    latest:      Arc<RwLock<Option<Value>>>,
    snapshot:    Option<Value>,
    meter:       ProcessMeter,
    last_update: Instant,
    ultrawide:   bool,
}

impl App {
    fn new() -> Self {
        Self {
            latest:      collector::start(),
            snapshot:    None,
            meter:       ProcessMeter::new(),
            last_update: Instant::now() - Duration::from_secs(2),
            ultrawide:   false,
        }
    }
}

impl eframe::App for App {
    // Assinatura corrigida pelo usuário para o eframe atual.
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        ctx.request_repaint_after(Duration::from_secs(1));

        // Atualiza snapshot e custo próprio a cada segundo.
        if self.last_update.elapsed() >= Duration::from_secs(1) {
            if let Ok(guard) = self.latest.read() { self.snapshot = guard.clone(); }
            self.meter.update();
            self.last_update = Instant::now();
        }

        // Tema.
        ui.style_mut().visuals = egui::Visuals::dark();
        ui.style_mut().visuals.override_text_color = Some(WHITE);
        ui.style_mut().visuals.panel_fill = BG;
        ui.style_mut().visuals.selection.bg_fill = LINE;
        ui.spacing_mut().item_spacing = egui::vec2(GAP, 5.0);

        let available = ui.available_size();
        egui::Frame::new().fill(BG).inner_margin(18).show(ui, |ui| {
            ui.set_min_size(egui::vec2(
                (available.x - 36.0).max(0.0),
                (available.y - 36.0).max(0.0),
            ));

            // Indicador de dados frescos: timestamp dentro de 4 s.
            let fresh = self.snapshot.as_ref()
                .and_then(|m| m["timestamp"].as_u64())
                .map(|t| {
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|now| now.as_secs().saturating_sub(t) <= 4)
                        .unwrap_or(false)
                })
                .unwrap_or(false);

            header(ui, fresh, &mut self.ultrawide);

            egui::ScrollArea::vertical()
                .id_salt("dashboard")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if let Some(m) = &self.snapshot {
                        dashboard(ui, m, self.ultrawide);
                    } else {
                        ui.label(RichText::new("Aguardando a primeira amostra do coletor…")
                            .color(health::MUTED));
                    }

                    ui.add_space(GAP);

                    // Custo do próprio processo.
                    subcard(ui, "CUSTO DO PRÓPRIO MONITOR", |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new(format!("CPU  {}", pct(self.meter.cpu))).color(WHITE).strong());
                            ui.label(RichText::new(format!("RSS  {}", number(self.meter.rss_mib, " MiB"))).color(WHITE).strong());
                            ui.label(RichText::new(format!(
                                "Threads  {}",
                                self.meter.threads.map(|v| v.to_string()).unwrap_or_else(|| "—".into()),
                            )).color(WHITE).strong());
                            ui.label(RichText::new("100% CPU = um núcleo · GUI + coletor").size(11.0).color(health::MUTED));
                        });
                    });

                    ui.label(RichText::new(
                        "Limites de saúde genéricos · Atualização de 1 s · — indica leitura indisponível",
                    ).color(health::MUTED).size(10.0));
                });
        });
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1440.0, 960.0])
            .with_min_inner_size([480.0, 500.0]),
        ..Default::default()
    };
    eframe::run_native("HW Monitor", options, Box::new(|_| Ok(Box::new(App::new()))))
}
