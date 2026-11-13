//! User appearance preferences deliberately live outside host/preset state.
pleasant_ui::appearance_store!(fn store() = "Damian Channel Strip");

pub fn light() -> bool {
    store().light()
}

pub fn toggle() {
    store().toggle();
}

pub fn label() -> &'static str {
    store().label()
}
