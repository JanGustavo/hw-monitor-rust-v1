//! Cards e componentes visuais do painel — sem lógica de coleta ou regras de saúde.

use eframe::egui::{self, Color32, RichText, Stroke};
use serde_json::Value;

use crate::health::{
    high, low, Health,
    CYAN, LINE, MUTED, PANEL, WHITE, AMBER,
};

// ── Constantes de layout ───────────────────────────────────────────────────────
const SUB: Color32 = Color32::from_rgb(9, 25, 47);
pub const GAP: f32 = 12.0;

// ── Helpers de formatação ──────────────────────────────────────────────────────
pub fn n(v: &Value) -> Option<f64> { v.as_f64().filter(|x| x.is_finite()) }
pub fn as_text(v: &Value) -> String {
    if let Some(s) = v.as_str() { s.to_string() }
    else if v.is_null()         { "—".into() }
    else                        { v.to_string() }
}
pub fn number(v: Option<f64>, unit: &str) -> String {
    v.map(|x| format!("{x:.1}{unit}")).unwrap_or_else(|| "—".into())
}
pub fn pct(v: Option<f64>) -> String { number(v, "%") }
pub fn gib(v: &Value) -> String { number(n(v).map(|x| x / 1_073_741_824.0), " GiB") }
pub fn rate(v: &Value) -> String { number(n(v).map(|x| x / 1_048_576.0), " MiB/s") }
pub fn ratio(a: &Value, b: &Value) -> Option<f64> {
    let den = n(b)?; if den > 0.0 { Some(100.0 * n(a)? / den) } else { None }
}

// ── Primitivas visuais ─────────────────────────────────────────────────────────

/// Par nome/valor com linha separadora fina.
pub fn row(ui: &mut egui::Ui, name: &str, value: impl AsRef<str>, health: Health) {
    ui.columns(2, |cols| {
        cols[0].add(egui::Label::new(
            RichText::new(name).size(12.0).color(MUTED),
        ).wrap_mode(egui::TextWrapMode::Wrap));
        cols[1].with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            ui.add(egui::Label::new(
                RichText::new(value.as_ref()).size(12.0).color(health.color()).strong(),
            ).wrap_mode(egui::TextWrapMode::Wrap));
        });
    });
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter().line_segment(
        [rect.left_center(), rect.right_center()],
        Stroke::new(1.0, Color32::from_rgb(26, 49, 76)),
    );
}

/// Frame secundário com título opcional dentro de um card.
pub fn subcard(ui: &mut egui::Ui, title: &str, content: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new().fill(SUB).stroke(Stroke::new(1.0, LINE))
        .corner_radius(8).inner_margin(10).show(ui, |ui| {
            ui.set_min_width(ui.available_width().max(0.0));
            if !title.is_empty() {
                ui.label(RichText::new(title).size(11.0).color(Color32::from_rgb(128, 200, 244)).strong());
                ui.add_space(4.0);
            }
            content(ui);
        });
    ui.add_space(6.0);
}

/// Card principal com borda, título e brilho decorativo no topo.
pub fn card(ui: &mut egui::Ui, title: &str, height: f32, content: impl FnOnce(&mut egui::Ui)) {
    ui.push_id(title, |ui| {
        let response = egui::Frame::new().fill(PANEL).stroke(Stroke::new(1.0, LINE))
            .corner_radius(12).inner_margin(14).show(ui, |ui| {
                ui.set_min_width(ui.available_width().max(0.0));
                ui.label(RichText::new(title).color(Color32::from_rgb(168, 217, 251)).strong().size(12.0));
                ui.add_space(10.0);
                egui::ScrollArea::vertical()
                    .id_salt("body")
                    .max_height((height - 58.0).max(100.0))
                    .auto_shrink([false, false])
                    .show(ui, content);
            });
        let a = response.response.rect.left_top() + egui::vec2(17.0, 1.0);
        let b = a + egui::vec2(65.0, 0.0);
        // Brilho estático: sem animação ou repintura contínua.
        ui.painter().line_segment([a, b], Stroke::new(5.0, Color32::from_rgba_unmultiplied(45, 175, 255, 25)));
        ui.painter().line_segment([a, b], Stroke::new(1.2, CYAN));
    });
}

/// Barra de progresso com valor percentual grande acima.
pub fn gauge(ui: &mut egui::Ui, value: Option<f64>, health: Health, caption: &str) {
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(pct(value)).size(48.0).strong().color(health.color()));
        ui.label(RichText::new(caption).size(12.0).color(MUTED));
    });
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 6.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, 3.0, Color32::from_rgb(27, 52, 82));
    if let Some(v) = value {
        let fill = egui::Rect::from_min_size(
            rect.min,
            egui::vec2(rect.width() * (v as f32 / 100.0).clamp(0.0, 1.0), 6.0),
        );
        ui.painter().rect_filled(fill, 3.0, health.color());
    }
    ui.add_space(12.0);
}

pub fn empty(ui: &mut egui::Ui) {
    ui.label(RichText::new("Sem leitura disponível neste equipamento.").color(MUTED).size(12.0));
}

// ── Helpers de sensores ────────────────────────────────────────────────────────

pub fn sensor_health(s: &Value) -> Health {
    if n(&s["value"]).is_none() { return Health::Unknown; }
    if s["kind"] != "temp"      { return Health::Normal; }
    let chip = as_text(&s["chip"]).to_lowercase();
    if chip.contains("nvme") {
        high(n(&s["value"]), 45.0, 65.0, 75.0)
    } else if ["amdgpu", "nvidia", "nouveau", "i915", "k10temp", "coretemp"]
        .iter().any(|x| chip.contains(x))
    {
        high(n(&s["value"]), 55.0, 80.0, 90.0)
    } else {
        Health::Normal
    }
}

pub fn is_gpu_sensor(s: &Value) -> bool {
    let chip = as_text(&s["chip"]).to_lowercase();
    ["amdgpu", "nvidia", "nouveau", "i915"].iter().any(|x| chip.contains(x)) || chip == "xe"
}

fn sensor_chip(ui: &mut egui::Ui, s: &Value) {
    egui::Frame::new()
        .fill(Color32::from_rgb(17, 43, 76))
        .stroke(Stroke::new(1.0, LINE))
        .corner_radius(6).inner_margin(6)
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(
                    format!("{} · {}", as_text(&s["chip"]), as_text(&s["label"])),
                ).size(11.0).color(MUTED));
                ui.label(RichText::new(
                    format!("{} {}", number(n(&s["value"]), ""), as_text(&s["unit"])),
                ).size(12.0).strong().color(sensor_health(s).color()));
            });
        });
}

// ── Cards de métrica ───────────────────────────────────────────────────────────

fn psi_card(ui: &mut egui::Ui, m: &Value) {
    subcard(ui, "PRESSÃO · PSI AVG10", |ui| {
        for (key, label) in [("cpu", "CPU"), ("memory", "Memória"), ("io", "I/O")] {
            row(ui, label, pct(n(&m["pressure"][key])), high(n(&m["pressure"][key]), 1.0, 10.0, 25.0));
        }
    });
}

fn cpu_states(ui: &mut egui::Ui, c: &Value) {
    subcard(ui, "ESTADOS & SCHEDULER", |ui| {
        for key in ["user", "system", "iowait", "irq", "softirq", "steal"] {
            row(ui, key, pct(n(&c["states_percent"][key])), Health::Normal);
        }
        row(ui, "Context switches/s", number(n(&c["context_switches_s"]), ""), Health::Normal);
        row(ui, "Interrupções/s",      number(n(&c["interrupts_s"]), ""),       Health::Normal);
    });
}

pub fn cpu(ui: &mut egui::Ui, m: &Value, height: f32) {
    let c = &m["cpu"];
    card(ui, "01 / CPU", height, |ui| {
        ui.label(RichText::new(as_text(&c["model"])).color(MUTED).size(12.0));
        gauge(ui, n(&c["usage_percent"]), low(n(&c["usage_percent"]), 20.0), "utilização total");
        if ui.available_width() >= 480.0 {
            ui.columns(2, |cols| { cpu_states(&mut cols[0], c); psi_card(&mut cols[1], m); });
        } else {
            cpu_states(ui, c); psi_card(ui, m);
        }
        subcard(ui, "THREADS LÓGICAS · USO E CLOCK", |ui| {
            if let Some(cores) = c["per_core_percent"].as_array() {
                let count = ((ui.available_width() + 6.0) / 82.0).floor().max(1.0) as usize;
                for (batch, items) in cores.chunks(count).enumerate() {
                    ui.columns(count, |cols| {
                        for (j, value) in items.iter().enumerate() {
                            let i = batch * count + j;
                            egui::Frame::new()
                                .fill(Color32::from_rgb(18, 43, 76))
                                .corner_radius(5).inner_margin(5)
                                .show(&mut cols[j], |ui| {
                                    ui.label(RichText::new(format!("CPU {i}")).size(10.0).color(MUTED));
                                    ui.label(RichText::new(pct(n(value))).size(16.0).strong().color(low(n(value), 20.0).color()));
                                    ui.label(RichText::new(number(n(&c["clock_mhz"][i]), " MHz")).size(10.0).color(MUTED));
                                });
                        }
                    });
                    ui.add_space(4.0);
                }
            } else { empty(ui); }
        });
    });
}

pub fn gpu(ui: &mut egui::Ui, m: &Value, height: f32) {
    card(ui, "02 / GPU · DRM", height, |ui| {
        if let Some(items) = m["gpu"].as_array().filter(|a| !a.is_empty()) {
            for g in items {
                gauge(ui, n(&g["usage_percent"]), low(n(&g["usage_percent"]), 20.0), &as_text(&g["name"]));
                subcard(ui, "DISPOSITIVO & MEMÓRIA", |ui| {
                    row(ui, "Driver",             as_text(&g["driver"]),     Health::Normal);
                    row(ui, "Vendor / device",    format!("{} / {}", as_text(&g["vendor_id"]), as_text(&g["device_id"])), Health::Normal);
                    row(ui, "VRAM usada / total", format!("{} / {}", gib(&g["vram_used_bytes"]), gib(&g["vram_total_bytes"])), Health::Normal);
                    row(ui, "Clock core",         number(n(&g["core_clock_mhz"]), " MHz"), Health::Normal);
                });
            }
        } else { empty(ui); }
        subcard(ui, "SENSORES GRÁFICOS", |ui| {
            let mut found = false;
            if let Some(items) = m["sensors"].as_array() {
                ui.horizontal_wrapped(|ui| {
                    for s in items.iter().filter(|s| is_gpu_sensor(s)) { found = true; sensor_chip(ui, s); }
                });
            }
            if !found { empty(ui); }
        });
    });
}

pub fn memory(ui: &mut egui::Ui, m: &Value, height: f32) {
    let r = &m["memory"];
    card(ui, "03 / MEMÓRIA & SWAP", height, |ui| {
        let used = ratio(&r["used_bytes"], &r["total_bytes"]);
        gauge(ui, used, high(used, 60.0, 85.0, 95.0), "RAM em uso");
        subcard(ui, "ALOCAÇÃO", |ui| {
            row(ui, "Usada / total",   format!("{} / {}", gib(&r["used_bytes"]), gib(&r["total_bytes"])), Health::Normal);
            row(ui, "Disponível",      gib(&r["available_bytes"]), Health::Normal);
            for (label, a, b) in [
                ("Cache / buffers",   "cached_bytes",   "buffers_bytes"),
                ("Ativa / inativa",   "active_bytes",   "inactive_bytes"),
                ("Slab / dirty",      "slab_bytes",     "dirty_bytes"),
            ] {
                row(ui, label, format!("{} / {}", gib(&r[a]), gib(&r[b])), Health::Normal);
            }
        });
        subcard(ui, "SWAP & PAGINAÇÃO", |ui| {
            row(ui, "Swap usada / total", format!("{} / {}", gib(&r["swap_used_bytes"]), gib(&r["swap_total_bytes"])),
                high(ratio(&r["swap_used_bytes"], &r["swap_total_bytes"]), 10.0, 50.0, 80.0));
            for (label, key) in [
                ("Page faults/s",          "page_faults_s"),
                ("Major faults/s",         "major_faults_s"),
                ("Scan direto/s",          "pgscan_s"),
                ("Reclaim direto/s",       "pgsteal_s"),
                ("Compactação · stalls/s", "compact_stall_s"),
            ] {
                row(ui, label, number(n(&r[key]), ""), Health::Normal);
            }
        });
    });
}

pub fn storage(ui: &mut egui::Ui, m: &Value, height: f32) {
    card(ui, "04 / ARMAZENAMENTO", height, |ui| {
        if let Some(items) = m["filesystems"].as_array() {
            for f in items {
                subcard(ui, &format!("{} · {}", as_text(&f["mount"]), as_text(&f["kind"])), |ui| {
                    let p = ratio(&f["used_bytes"], &f["total_bytes"]);
                    row(ui, "Uso / capacidade",
                        format!("{} / {}", gib(&f["used_bytes"]), gib(&f["total_bytes"])),
                        high(p, 60.0, 80.0, 90.0));
                    ui.add(egui::ProgressBar::new(
                        (p.unwrap_or(0.0) as f32 / 100.0).clamp(0.0, 1.0),
                    ).text(pct(p)).fill(high(p, 60.0, 80.0, 90.0).color()));
                    row(ui, "Disponível", gib(&f["available_bytes"]), Health::Normal);
                });
            }
        }
        if let Some(items) = m["block"].as_array() {
            for d in items {
                subcard(ui, &format!("{} · I/O", as_text(&d["name"])), |ui| {
                    row(ui, "Leitura / escrita",   format!("{} / {}", rate(&d["read_bytes_s"]),  rate(&d["write_bytes_s"])),  Health::Normal);
                    row(ui, "IOPS leitura / escrita", format!("{} / {}", number(n(&d["read_iops"]), ""), number(n(&d["write_iops"]), "")), Health::Normal);
                    row(ui, "Ocupação / fila",     format!("{} / {}", pct(n(&d["busy_percent"])), number(n(&d["average_queue_depth"]), "")), Health::Normal);
                });
            }
        }
    });
}

pub fn network(ui: &mut egui::Ui, m: &Value, height: f32) {
    card(ui, "05 / REDE", height, |ui| {
        if let Some(items) = m["network"].as_array().filter(|a| !a.is_empty()) {
            for x in items {
                subcard(ui, &as_text(&x["name"]), |ui| {
                    let link = match x["link_up"].as_bool() { Some(true) => "Ativo", Some(false) => "Inativo", None => "—" };
                    row(ui, "Link / velocidade", format!("{} / {}", link, number(n(&x["speed_mbps"]), " Mbps")), Health::Normal);
                    row(ui, "Recebido",  rate(&x["rx_bytes_s"]), Health::Normal);
                    row(ui, "Enviado",   rate(&x["tx_bytes_s"]), Health::Normal);
                    row(ui, "Pacotes/s RX / TX", format!("{} / {}", number(n(&x["rx_packets_s"]), ""), number(n(&x["tx_packets_s"]), "")), Health::Normal);
                    row(ui, "Erros RX / TX",     format!("{} / {}", as_text(&x["rx_errors"]),    as_text(&x["tx_errors"])),    Health::Normal);
                    row(ui, "Drops RX / TX",     format!("{} / {}", as_text(&x["rx_drops"]),     as_text(&x["tx_drops"])),     Health::Normal);
                });
            }
        } else { empty(ui); }
    });
}

pub fn sensors(ui: &mut egui::Ui, m: &Value, height: f32) {
    card(ui, "06 / SENSORES FÍSICOS · HWMON", height, |ui| {
        if let Some(items) = m["sensors"].as_array().filter(|a| !a.is_empty()) {
            ui.horizontal_wrapped(|ui| { for s in items { sensor_chip(ui, s); } });
        } else { empty(ui); }
    });
}

pub fn system(ui: &mut egui::Ui, m: &Value, height: f32) {
    card(ui, "07 / SISTEMA & KERNEL", height, |ui| {
        let s = &m["system"];
        row(ui, "Host",              as_text(&m["hostname"]), Health::Normal);
        row(ui, "Distribuição",      as_text(&s["distro"]),   Health::Normal);
        row(ui, "Kernel",            as_text(&s["kernel"]),   Health::Normal);
        row(ui, "Uptime",            number(n(&s["uptime_s"]).map(|v| v / 3600.0), " h"), Health::Normal);
        if let Some(load) = s["load"].as_array() {
            row(ui, "Load 1 / 5 / 15", load.iter().map(|v| number(n(v), "")).collect::<Vec<_>>().join(" / "), Health::Normal);
        }
        row(ui, "Tarefas / executando", format!("{} / {}", as_text(&s["processes"]), as_text(&s["running"])), Health::Normal);
        row(ui, "TCP retransmitidos/s", number(n(&s["tcp_retransmits_s"]), ""), Health::Normal);
        row(ui, "Forks/s",             number(n(&s["forks_s"]), ""),            Health::Normal);
    });
}

pub fn battery(ui: &mut egui::Ui, m: &Value, height: f32) {
    card(ui, "08 / ENERGIA & BATERIA", height, |ui| {
        if let Some(items) = m["batteries"].as_array().filter(|a| !a.is_empty()) {
            for b in items {
                subcard(ui, &as_text(&b["name"]), |ui| {
                    row(ui, "Estado", as_text(&b["status"]), Health::Normal);
                    let h = match (b["ac_online"].as_bool(), n(&b["capacity_percent"])) {
                        (_, None)                          => Health::Unknown,
                        (Some(false), Some(v)) if v <= 10.0 => Health::Critical,
                        (Some(false), Some(v)) if v <= 20.0 => Health::Watch,
                        (_, Some(v)) if v >= 60.0           => Health::Good,
                        _                                  => Health::Normal,
                    };
                    row(ui, "Carga",   pct(n(&b["capacity_percent"])), h);
                    row(ui, "Potência", number(n(&b["power_w"]), " W"),   Health::Normal);
                    row(ui, "Energia",  number(n(&b["energy_wh"]), " Wh"), Health::Normal);
                    row(ui, "Tomada",   match b["ac_online"].as_bool() { Some(true) => "Sim", Some(false) => "Não", None => "—" }, Health::Normal);
                });
            }
        } else { empty(ui); }
    });
}

// ── Layout do dashboard ────────────────────────────────────────────────────────

pub fn dashboard(ui: &mut egui::Ui, m: &Value, wide: bool) {
    let width    = ui.available_width();
    let hero     = 510.0;
    let details  = 360.0;
    let auxiliary = 270.0;

    if wide && width >= 1080.0 {
        ui.columns(3, |c| { cpu(&mut c[0], m, hero); gpu(&mut c[1], m, hero); memory(&mut c[2], m, hero); });
        ui.add_space(GAP);
        if width >= 1800.0 {
            ui.columns(5, |c| {
                storage(&mut c[0], m, details); network(&mut c[1], m, details);
                sensors(&mut c[2], m, details); system(&mut c[3], m, details); battery(&mut c[4], m, details);
            });
        } else {
            ui.columns(3, |c| { storage(&mut c[0], m, details); network(&mut c[1], m, details); sensors(&mut c[2], m, details); });
            ui.add_space(GAP);
            ui.columns(2, |c| { system(&mut c[0], m, auxiliary); battery(&mut c[1], m, auxiliary); });
        }
    } else if width >= 1080.0 {
        ui.columns(2, |c| { cpu(&mut c[0], m, hero); gpu(&mut c[1], m, hero); });
        ui.add_space(GAP);
        ui.columns(3, |c| { memory(&mut c[0], m, details); storage(&mut c[1], m, details); network(&mut c[2], m, details); });
        ui.add_space(GAP);
        ui.columns(3, |c| { sensors(&mut c[0], m, auxiliary); system(&mut c[1], m, auxiliary); battery(&mut c[2], m, auxiliary); });
    } else if width >= 720.0 {
        ui.columns(2, |c| { cpu(&mut c[0], m, hero); gpu(&mut c[1], m, hero); });
        ui.add_space(GAP);
        ui.columns(2, |c| { memory(&mut c[0], m, details); storage(&mut c[1], m, details); });
        ui.add_space(GAP);
        ui.columns(2, |c| { network(&mut c[0], m, details); sensors(&mut c[1], m, details); });
        ui.add_space(GAP);
        ui.columns(2, |c| { system(&mut c[0], m, auxiliary); battery(&mut c[1], m, auxiliary); });
    } else {
        for draw in [cpu, gpu, memory, storage, network, sensors, system, battery] {
            draw(ui, m, details);
            ui.add_space(GAP);
        }
    }
}

/// Barra de status do topo: logo, indicador ao vivo, toggle ultrawide e legenda de cores.
pub fn header(ui: &mut egui::Ui, fresh: bool, ultrawide: &mut bool) {
    ui.horizontal_wrapped(|ui| {
        // Logo embutida no binário — sem dependência de caminho em runtime.
        egui::Frame::new()
            .fill(Color32::from_rgb(16, 46, 89))
            .corner_radius(8)
            .inner_margin(4)
            .show(ui, |ui| {
                ui.add(
                    egui::Image::new(egui::include_image!("../../static/logohw.png"))
                        .fit_to_exact_size(egui::vec2(40.0, 40.0))
                        .corner_radius(6),
                );
            });
        ui.vertical(|ui| {
            ui.label(RichText::new("HW / MONITOR").size(23.0).strong().color(WHITE));
            ui.label(RichText::new("LINUX  ·  DEEP TELEMETRY  ·  RUST V1").size(10.0).color(MUTED));
        });
        ui.add_space(10.0);
        ui.label(
            RichText::new(if fresh { "● AO VIVO" } else { "● AGUARDANDO DADOS" })
                .size(11.0)
                .color(if fresh { CYAN } else { AMBER }),
        );
        if ui.selectable_label(*ultrawide, RichText::new("ULTRAWIDE").size(11.0).strong())
            .on_hover_text("Três cards principais em janelas largas; cinco cards inferiores a partir de 1800 pontos.")
            .clicked()
        {
            *ultrawide = !*ultrawide;
        }
    });
    ui.add_space(7.0);
    ui.horizontal_wrapped(|ui| {
        for (text, color) in [
            ("● Normal", WHITE), ("● Favorável", crate::health::GREEN),
            ("● Atenção", AMBER), ("● Crítico", crate::health::RED),
            ("● Sem leitura", MUTED),
        ] {
            ui.label(RichText::new(text).size(11.0).color(color));
        }
    });
    ui.add_space(9.0);
}
