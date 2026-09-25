use eframe::egui::{IconData, ViewportBuilder};
use std::sync::Arc;
fn main() {
    let icon = IconData { rgba: vec![], width: 0, height: 0 };
    let vp = ViewportBuilder::default().with_icon(Arc::new(icon));
}
