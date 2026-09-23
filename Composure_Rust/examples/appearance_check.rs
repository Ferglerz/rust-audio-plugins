pub use composure::{dsp, graph_store, params};
#[path = "../src/ui/preview_temp.rs"]
mod ui;
fn main() {
    ui::create(std::sync::Arc::new(params::ComposureParams::default()), std::sync::Arc::new(ui::UiDisplay::default()));
}
