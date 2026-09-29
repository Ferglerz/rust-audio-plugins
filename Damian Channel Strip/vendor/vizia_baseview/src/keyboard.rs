//! Native key ownership for the patched Vizia text-input backend.
use std::{cell::Cell, collections::HashSet, rc::Rc};

use baseview::{Event, EventStatus};
use vizia_core::{backend::BackendContext, prelude::*};
use vizia_input::{Code, Key, KeyState, KeyboardModifiers};

/// Report a keyboard event that a custom Vizia view consumed.
#[derive(Clone, Copy, Debug)]
pub struct KeyboardEventCaptured;

pub(crate) struct TextInputKeyboard {
    handled: Rc<Cell<bool>>,
    editor: Rc<Cell<Option<Entity>>>,
    captured: HashSet<Code>,
}

impl TextInputKeyboard {
    pub(crate) fn new(cx: &mut Context) -> Self {
        let handled = Rc::new(Cell::new(false));
        let listener_handled = handled.clone();
        let editor = Rc::new(Cell::new(None));
        let listener_editor = editor.clone();
        cx.add_global_listener(move |_, event| {
            event.map(|_: &KeyboardEventCaptured, _| listener_handled.set(true));
            event.map(|event: &TextEvent, meta| {
                match event {
                    TextEvent::StartEdit => listener_editor.set(Some(meta.target)),
                    TextEvent::EndEdit if listener_editor.get() == Some(meta.target) => {
                        listener_editor.set(None);
                    }
                    _ => {}
                }
            });
        });
        Self { handled, editor, captured: HashSet::new() }
    }

    pub(crate) fn begin_event(&self) {
        self.handled.set(false);
    }

    pub(crate) fn finish_event(&mut self, event: &Event, status: EventStatus) -> EventStatus {
        if cfg!(target_os = "macos") && self.handled.get() {
            if let Event::Keyboard(key) = event {
                if key.state == KeyState::Down {
                    self.captured.insert(key.code);
                }
                return EventStatus::Captured;
            }
        }
        status
    }

    pub(crate) fn status(&mut self, cx: &mut Context, event: &Event) -> EventStatus {
        // Other platforms keep the upstream routing behavior.
        if !cfg!(target_os = "macos") {
            return EventStatus::Ignored;
        }
        if matches!(event, Event::Window(baseview::WindowEvent::Unfocused)) {
            self.captured.clear();
        }
        let Event::Keyboard(key) = event else {
            return EventStatus::Ignored;
        };
        // Pair releases with captured presses even if Enter/Escape ended editing.
        if key.state == KeyState::Up {
            return if self.captured.remove(&key.code) {
                EventStatus::Captured
            } else {
                EventStatus::Ignored
            };
        }
        let focused = BackendContext::new(cx).focused();
        let editing = self.editor.get() == Some(focused) && {
            let cx = EventContext::new_with_current(cx, focused);
            cx.is_checked() && !cx.is_disabled()
        };
        if self.captured.contains(&key.code)
            || (editing && text_editing_key(key.code, &key.key, key.modifiers))
        {
            self.captured.insert(key.code);
            EventStatus::Captured
        } else {
            EventStatus::Ignored
        }
    }
}

fn text_editing_key(code: Code, key: &Key, modifiers: KeyboardModifiers) -> bool {
    use Code::*;
    // Match the pinned Textbox implementation's macOS clipboard shortcuts.
    if modifiers.contains(KeyboardModifiers::META) {
        return modifiers == KeyboardModifiers::META && matches!(code, KeyA | KeyC | KeyV | KeyX);
    }
    if matches!(code, Enter | NumpadEnter | Escape | Tab | Backspace | Delete | ArrowLeft | ArrowRight
        | ArrowUp | ArrowDown | Home | End | PageUp | PageDown)
    {
        return true;
    }
    // Option is a text-input modifier on macOS. Control combinations remain host shortcuts.
    !modifiers.contains(KeyboardModifiers::CONTROL) && matches!(key, Key::Character(_))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typing_and_navigation_belong_to_the_editor() {
        for modifiers in [KeyboardModifiers::empty(), KeyboardModifiers::SHIFT,
            KeyboardModifiers::ALT, KeyboardModifiers::ALT | KeyboardModifiers::SHIFT]
        {
            assert!(text_editing_key(Code::KeyE, &Key::Character("é".into()), modifiers));
        }
        for code in [Code::Enter, Code::Escape, Code::Tab, Code::Backspace, Code::Delete,
            Code::ArrowLeft, Code::ArrowRight, Code::Home, Code::End]
        {
            assert!(text_editing_key(code, &Key::Unidentified, KeyboardModifiers::empty()));
        }
    }

    #[test]
    fn host_shortcuts_are_not_text_input() {
        for code in [Code::KeyS, Code::KeyQ, Code::KeyZ, Code::Space] {
            assert!(!text_editing_key(code, &Key::Character("s".into()), KeyboardModifiers::META));
        }
        assert!(!text_editing_key(Code::KeyS, &Key::Character("s".into()), KeyboardModifiers::CONTROL));
        assert!(!text_editing_key(Code::F1, &Key::F1, KeyboardModifiers::empty()));
        for code in [Code::KeyA, Code::KeyC, Code::KeyV, Code::KeyX] {
            assert!(text_editing_key(code, &Key::Character("a".into()), KeyboardModifiers::META));
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn start_and_end_edit_track_the_editor() {
        let mut cx = Context::default();
        let keyboard = TextInputKeyboard::new(&mut cx);
        let target = Entity::root();
        cx.emit_to(target, TextEvent::StartEdit);
        BackendContext::new_with_event_manager(&mut cx).process_events();
        assert_eq!(keyboard.editor.get(), Some(target));
        cx.emit_to(target, TextEvent::EndEdit);
        BackendContext::new_with_event_manager(&mut cx).process_events();
        assert_eq!(keyboard.editor.get(), None);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn unfocused_keys_pass_through_and_captured_releases_stay_captured() {
        let mut cx = Context::default();
        let mut keyboard = TextInputKeyboard::new(&mut cx);
        // Build the native event through the type inferred from Event::Keyboard.
        let mut key = match Event::Keyboard(Default::default()) {
            Event::Keyboard(key) => key,
            _ => unreachable!(),
        };
        key.code = Code::Enter;
        key.state = KeyState::Down;
        assert_eq!(keyboard.status(&mut cx, &Event::Keyboard(key.clone())), EventStatus::Ignored);
        keyboard.captured.insert(Code::Enter);
        key.state = KeyState::Up;
        assert_eq!(keyboard.status(&mut cx, &Event::Keyboard(key.clone())), EventStatus::Captured);
        assert_eq!(keyboard.status(&mut cx, &Event::Keyboard(key)), EventStatus::Ignored);
    }
}
