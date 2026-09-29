//! User appearance preferences deliberately live outside host/preset state.
use pleasant_ui::AppearanceStore;
use std::sync::OnceLock;

static STORE: OnceLock<AppearanceStore> = OnceLock::new();

fn store() -> &'static AppearanceStore {
    STORE.get_or_init(|| AppearanceStore::new("Damian Channel Strip"))
}

pub fn light() -> bool {
    store().light()
}

pub fn toggle() {
    store().toggle();
}

pub fn label() -> &'static str {
    store().label()
}
