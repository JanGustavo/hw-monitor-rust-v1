//! Regras de saúde visual: classifica leituras numéricas em níveis de cor.
//! Limites são genéricos e indicativos, não diagnóstico de hardware.

use eframe::egui::Color32;

// ── Paleta ────────────────────────────────────────────────────────────────────
pub const BG: Color32     = Color32::from_rgb(5, 12, 26);
pub const PANEL: Color32  = Color32::from_rgb(16, 35, 63);
pub const LINE: Color32   = Color32::from_rgb(40, 85, 130);
pub const CYAN: Color32   = Color32::from_rgb(89, 220, 255);
pub const WHITE: Color32  = Color32::from_rgb(235, 246, 255);
pub const MUTED: Color32  = Color32::from_rgb(165, 190, 214);
pub const GREEN: Color32  = Color32::from_rgb(99, 221, 161);
pub const AMBER: Color32  = Color32::from_rgb(255, 208, 116);
pub const RED: Color32    = Color32::from_rgb(255, 104, 124);

// ── Níveis ────────────────────────────────────────────────────────────────────
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Health { Normal, Good, Watch, Critical, Unknown }

impl Health {
    pub fn color(self) -> Color32 {
        match self {
            Self::Normal   => WHITE,
            Self::Good     => GREEN,
            Self::Watch    => AMBER,
            Self::Critical => RED,
            Self::Unknown  => MUTED,
        }
    }
}

/// Classifica valores onde alto é ruim (ex.: temperatura, uso de CPU).
pub fn high(v: Option<f64>, good: f64, watch: f64, critical: f64) -> Health {
    match v {
        None                         => Health::Unknown,
        Some(x) if x >= critical     => Health::Critical,
        Some(x) if x >= watch        => Health::Watch,
        Some(x) if x < good          => Health::Good,
        _                            => Health::Normal,
    }
}

/// Classifica valores onde baixo é favorável (ex.: uso ocioso de GPU/CPU).
pub fn low(v: Option<f64>, below: f64) -> Health {
    match v {
        None              => Health::Unknown,
        Some(x) if x < below => Health::Good,
        _                 => Health::Normal,
    }
}
