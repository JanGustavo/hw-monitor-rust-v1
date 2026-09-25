//! Alertas descritivos baseados no mesmo snapshot usado pelas duas interfaces.
use serde_json::Value;

#[derive(Clone, Debug)]
pub struct Alert {
    pub component: &'static str,
    pub detail: String,
    pub critical: bool,
}

fn reading(v: &Value) -> Option<f64> { v.as_f64().filter(|n| n.is_finite()) }
fn percent(used: &Value, total: &Value) -> Option<f64> {
    let total = reading(total)?;
    if total > 0.0 { Some(100.0 * reading(used)? / total) } else { None }
}

/// Limiares genéricos; os textos sempre mostram o valor que motivou o alerta.
pub fn alerts(m: &Value) -> Vec<Alert> {
    let mut out = Vec::new();
    if let Some(value) = percent(&m["memory"]["used_bytes"], &m["memory"]["total_bytes"]) {
        if value >= 85.0 { out.push(Alert { component: "Memória", detail: format!("RAM {:.0}% usada; atenção ≥85%, crítico ≥95% (limites genéricos)", value), critical: value >= 95.0 }); }
    }
    if let Some(items) = m["filesystems"].as_array() {
        for disk in items {
            if let Some(value) = percent(&disk["used_bytes"], &disk["total_bytes"]) {
                if value >= 80.0 { out.push(Alert { component: "Armazenamento", detail: format!("{}: {:.0}% usado; atenção ≥80%, crítico ≥90% (limites genéricos)", disk["mount"].as_str().unwrap_or("Disco"), value), critical: value >= 90.0 }); }
            }
        }
    }
    if let Some(items) = m["sensors"].as_array() {
        for sensor in items {
            if sensor["kind"] != "temp" { continue; }
            let name = sensor["chip"].as_str().unwrap_or("");
            let sensor_name = format!("{} {}", name, sensor["label"].as_str().unwrap_or("")).to_lowercase();
            let (watch, critical) = if sensor_name.contains("nvme") { (65.0, 75.0) } else if ["k10temp", "coretemp", "amdgpu", "nvidia", "nouveau", "i915", "xe", "cpu", "package", "tctl", "tdie"].iter().any(|s| sensor_name.contains(s)) { (80.0, 90.0) } else { continue };
            if let Some(value) = reading(&sensor["value"]) {
                if value >= watch { out.push(Alert { component: "Sensores", detail: format!("{} · {}: {:.1} °C; atenção ≥{watch:.0} °C, crítico ≥{critical:.0} °C (limites genéricos)", name, sensor["label"].as_str().unwrap_or("Temperatura"), value), critical: value >= critical }); }
            }
        }
    }
    out.sort_by_key(|a| !a.critical);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unavailable_is_not_healthy_or_alert() { assert!(alerts(&serde_json::json!({"memory":{}, "filesystems":[], "sensors":[]})).is_empty()); }
    #[test]
    fn shows_reason_and_threshold() {
        let m = serde_json::json!({"memory":{"used_bytes":96,"total_bytes":100}, "filesystems":[], "sensors":[]});
        assert!(alerts(&m)[0].critical);
        assert!(alerts(&m)[0].detail.contains("≥95%"));
    }
}
