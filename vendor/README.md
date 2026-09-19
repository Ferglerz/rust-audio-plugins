# Vendored plugin GUI/host adapters

Trimmed copies of NIH-plug and baseview so we can advertise keyboard/text
input to hosts. Upstream NIH-plug reports `kNotImplemented` for VST3
`IPlugView` key/focus methods, which many DAWs treat as “this editor does
not want keys.”

- `nih-plug`: `IPlugView::onKeyDown` / `onKeyUp` / `onFocus` return success
  so the host should pass keys through instead of stealing them.
- `baseview` (macOS): `needsPanelToBecomeKey`, `canBecomeKeyView`, and
  `makeFirstResponder` on mouse down so Logic/Live plugin panels can become
  the key window.
- `baseview` (Windows): `WM_GETDLGCODE` wants all keys/chars, `SetFocus` on
  click.

Upstream revisions are in each crate’s `UPSTREAM_REV` file.
