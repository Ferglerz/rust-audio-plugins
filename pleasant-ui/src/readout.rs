//! Readout interaction decisions; parameter mapping and event ownership stay local.

use crate::{pointer::ValuePress, typed_char, ValueEdit};
use nih_plug_vizia::{vizia::prelude::*, widgets::util::ModifiersExt};

#[derive(Clone, Copy, Debug)]
pub enum PressAction<T> {
    Inactive,
    Blocked,
    BeginDrag(ValuePress<T>),
    BeginEdit(ValuePress<T>),
    Released,
    Cancelled,
}

/// Resolve a pending click versus drag without borrowing the owning view.
pub fn handle_press<T: Copy>(
    pending: &mut Option<ValuePress<T>>,
    event: &WindowEvent,
    pointer: (f32, f32),
) -> PressAction<T> {
    let Some(press) = pending.as_mut() else {
        return PressAction::Inactive;
    };
    match event {
        WindowEvent::MouseMove(_, _) if press.update(pointer.0, pointer.1) => {
            PressAction::BeginDrag(pending.take().unwrap())
        }
        WindowEvent::MouseUp(MouseButton::Left) => {
            let click = press.released_as_click(pointer.0, pointer.1);
            let press = pending.take().unwrap();
            if click {
                PressAction::BeginEdit(press)
            } else {
                PressAction::Released
            }
        }
        WindowEvent::FocusOut
        | WindowEvent::KeyDown(Code::Escape, _)
        | WindowEvent::MouseDown(MouseButton::Right) => {
            *pending = None;
            PressAction::Cancelled
        }
        _ => PressAction::Blocked,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditAction {
    Blocked,
    Handled,
    Commit,
    /// Focus loss commits, then continues through the view's focus handling.
    CommitAndContinue,
    Cancel,
}

/// Apply common editing keys and return operations that require the owning view.
pub fn handle_edit<T>(
    edit: &mut ValueEdit<T>,
    cx: &mut EventContext,
    event: &WindowEvent,
    pointer: (f32, f32),
) -> EditAction {
    match event {
        WindowEvent::CharInput(c) => {
            if !cx.modifiers().command() && c.is_ascii() && !c.is_control() {
                edit.insert(&c.to_string());
            }
            EditAction::Handled
        }
        WindowEvent::KeyDown(Code::Enter | Code::NumpadEnter, _) => EditAction::Commit,
        WindowEvent::KeyDown(Code::Escape, _) => EditAction::Cancel,
        WindowEvent::KeyDown(code, key) => {
            edit.handle_key(cx, *code);
            let has_char = matches!(key, Some(Key::Character(_)));
            if !has_char && !cx.modifiers().command() {
                if let Some(c) = typed_char(*code, cx.modifiers().shift()) {
                    edit.insert(&c.to_string());
                }
            }
            EditAction::Handled
        }
        WindowEvent::MouseDown(MouseButton::Left) => {
            let (x, y, w, h) = edit.rect;
            if pointer.0 >= x && pointer.0 <= x + w && pointer.1 >= y && pointer.1 <= y + h {
                edit.handle_mouse_down(pointer.0);
                EditAction::Handled
            } else {
                EditAction::Commit
            }
        }
        WindowEvent::MouseDoubleClick(MouseButton::Left) => {
            edit.select_all();
            EditAction::Handled
        }
        WindowEvent::FocusOut => EditAction::CommitAndContinue,
        _ => EditAction::Blocked,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nih_plug_vizia::vizia::backend::BackendContext;

    fn press() -> Option<ValuePress<u8>> {
        Some(ValuePress::new(7, (0.0, 0.0, 80.0, 20.0), (20.0, 10.0)))
    }

    #[test]
    fn pending_press_distinguishes_click_drag_and_cancellation() {
        let mut pending = press();
        assert!(matches!(
            handle_press(
                &mut pending,
                &WindowEvent::MouseMove(0.0, 0.0),
                (21.0, 11.0)
            ),
            PressAction::Blocked
        ));
        assert!(matches!(
            handle_press(
                &mut pending,
                &WindowEvent::MouseUp(MouseButton::Left),
                (21.0, 11.0)
            ),
            PressAction::BeginEdit(ValuePress { target: 7, .. })
        ));
        assert!(pending.is_none());
        let mut pending = press();
        assert!(matches!(
            handle_press(
                &mut pending,
                &WindowEvent::MouseMove(0.0, 0.0),
                (24.0, 10.0)
            ),
            PressAction::BeginDrag(_)
        ));
        assert!(pending.is_none());
        let mut pending = press();
        assert!(matches!(
            handle_press(
                &mut pending,
                &WindowEvent::MouseUp(MouseButton::Left),
                (70.0, 10.0)
            ),
            PressAction::Released
        ));
        assert!(pending.is_none());
        for event in [
            WindowEvent::FocusOut,
            WindowEvent::KeyDown(Code::Escape, None),
            WindowEvent::MouseDown(MouseButton::Right),
        ] {
            let mut pending = press();
            assert!(matches!(
                handle_press(&mut pending, &event, (20.0, 10.0)),
                PressAction::Cancelled
            ));
            assert!(pending.is_none());
        }
    }

    #[test]
    fn editing_keeps_character_delivery_and_commit_policies_distinct() {
        let mut context = Context::default();
        let target = Element::new(&mut context).entity();
        let mut cx = EventContext::new_with_current(&mut context, target);
        let mut edit = ValueEdit::new((), (0.0, 0.0, 80.0, 20.0), "12".into());
        assert_eq!(
            handle_edit(
                &mut edit,
                &mut cx,
                &WindowEvent::KeyDown(Code::Digit3, Some(Key::Character("3".into()))),
                (0.0, 0.0)
            ),
            EditAction::Handled
        );
        assert_eq!(edit.text, "12");
        handle_edit(&mut edit, &mut cx, &WindowEvent::CharInput('3'), (0.0, 0.0));
        handle_edit(
            &mut edit,
            &mut cx,
            &WindowEvent::KeyDown(Code::Digit4, None),
            (0.0, 0.0),
        );
        assert_eq!(edit.text, "34");
        handle_edit(&mut edit, &mut cx, &WindowEvent::CharInput('é'), (0.0, 0.0));
        assert_eq!(edit.text, "34");
        assert_eq!(
            handle_edit(
                &mut edit,
                &mut cx,
                &WindowEvent::KeyDown(Code::Enter, None),
                (0.0, 0.0)
            ),
            EditAction::Commit
        );
        assert_eq!(
            handle_edit(
                &mut edit,
                &mut cx,
                &WindowEvent::KeyDown(Code::Escape, None),
                (0.0, 0.0)
            ),
            EditAction::Cancel
        );
        assert_eq!(
            handle_edit(&mut edit, &mut cx, &WindowEvent::FocusOut, (0.0, 0.0)),
            EditAction::CommitAndContinue
        );
        assert_eq!(
            handle_edit(
                &mut edit,
                &mut cx,
                &WindowEvent::MouseDown(MouseButton::Left),
                (90.0, 10.0)
            ),
            EditAction::Commit
        );
        assert_eq!(
            handle_edit(
                &mut edit,
                &mut cx,
                &WindowEvent::MouseDown(MouseButton::Left),
                (4.0, 10.0)
            ),
            EditAction::Handled
        );
        assert_eq!((edit.cursor, edit.anchor), (0, 0));
        handle_edit(
            &mut edit,
            &mut cx,
            &WindowEvent::MouseDoubleClick(MouseButton::Left),
            (4.0, 10.0),
        );
        assert_eq!(edit.selection(), 0..2);
    }

    #[test]
    fn command_keys_do_not_insert_fallback_characters() {
        let mut context = Context::default();
        let target = Element::new(&mut context).entity();
        // The native command modifier is Super on macOS and Control elsewhere.
        BackendContext::new(&mut context).modifiers().set(
            if cfg!(target_os = "macos") {
                Modifiers::LOGO
            } else {
                Modifiers::CTRL
            },
            true,
        );
        let mut cx = EventContext::new_with_current(&mut context, target);
        let mut edit = ValueEdit::new((), (0.0, 0.0, 80.0, 20.0), "12".into());
        handle_edit(
            &mut edit,
            &mut cx,
            &WindowEvent::KeyDown(Code::KeyA, None),
            (0.0, 0.0),
        );
        handle_edit(&mut edit, &mut cx, &WindowEvent::CharInput('a'), (0.0, 0.0));
        assert_eq!(edit.text, "12");
        assert_eq!(edit.selection(), 0..2);
    }
}
