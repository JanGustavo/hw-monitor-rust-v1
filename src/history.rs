use std::collections::VecDeque;
use serde_json::Value;
use eframe::egui::{self, Color32, RichText, Stroke};

pub struct Sample { pub at: u64, pub cpu: Option<f64>, pub ram: Option<f64>, pub gpu: Option<f64>, pub temp: Option<f64> }
pub struct History(pub VecDeque<Sample>);
impl History {
    pub fn new() -> Self { Self(VecDeque::with_capacity(900)) }
    pub fn push(&mut self, m: &Value) {
        let at = m["timestamp"].as_u64().unwrap_or(0);
        if at == 0 || self.0.back().is_some_and(|s| s.at >= at) { return }
        let n = |v: &Value| v.as_f64().filter(|x| x.is_finite());
        let total = n(&m["memory"]["total_bytes"]);
        let ram = total.filter(|x| *x > 0.0).and_then(|x| n(&m["memory"]["used_bytes"]).map(|u| 100.0 * u / x));
        let gpu = m["gpu"].as_array().and_then(|g| g.iter().find_map(|g| n(&g["usage_percent"])));
        let temp = m["sensors"].as_array().and_then(|s| s.iter().filter(|s| s["kind"] == "temp").filter_map(|s| n(&s["value"])).reduce(f64::max));
        self.0.push_back(Sample { at, cpu: n(&m["cpu"]["usage_percent"]), ram, gpu, temp });
        while self.0.len() > 900 { self.0.pop_front(); }
    }
    pub fn draw(&self, ui: &mut egui::Ui, minutes: u64) {
        let Some(last) = self.0.back() else { ui.label("Histórico aguardando amostras…"); return };
        let recent: Vec<_> = self.0.iter().filter(|s| s.at >= last.at.saturating_sub(minutes * 60)).collect();
        ui.label(RichText::new(format!("Últimos {minutes} min · {} amostras em memória", recent.len())).small());
        for (name, color, field) in [("CPU", Color32::from_rgb(89,220,255), 0), ("Memória", Color32::from_rgb(99,221,161), 1), ("GPU", Color32::from_rgb(255,208,116), 2), ("Temperatura °C", Color32::from_rgb(255,104,124), 3)] {
            let values: Vec<_> = recent.iter().map(|s| match field { 0 => s.cpu, 1 => s.ram, 2 => s.gpu, _ => s.temp }).collect();
            let valid: Vec<_> = values.iter().flatten().copied().collect();
            if valid.is_empty() { continue }
            let avg = valid.iter().sum::<f64>() / valid.len() as f64;
            ui.label(RichText::new(format!("{name} · mín {:.1} / média {:.1} / pico {:.1}", valid.iter().copied().fold(f64::INFINITY, f64::min), avg, valid.iter().copied().fold(f64::NEG_INFINITY, f64::max))).color(color).small());
            let width = ui.available_width().max(30.0);
            let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 48.0), egui::Sense::hover());
            let max = if field == 3 { 110.0 } else { 100.0 };
            let span = (values.len() - 1).max(1) as f32;
            for (i, pair) in values.windows(2).enumerate() {
                if let [Some(a), Some(b)] = pair {
                    let pos = |index: usize, value: f64| egui::pos2(rect.left() + rect.width() * index as f32 / span, rect.bottom() - rect.height() * (value / max).clamp(0.0, 1.0) as f32);
                    ui.painter().line_segment([pos(i, *a), pos(i + 1, *b)], Stroke::new(1.7, color));
                }
            }
        }
    }
}
