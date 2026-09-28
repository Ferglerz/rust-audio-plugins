use super::*;
impl ChordboardView {
    pub(super) fn render(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let bounds = cx.bounds();
        if bounds.w <= 0.0 || bounds.h <= 0.0 {
            return;
        }
        if self.font.get().is_none() {
            self.font.set(canvas.add_font_mem(FONT_JETBRAINS_MONO).ok());
        }
        let mut d = Draw::new(
            canvas,
            prefs().light(),
            bounds.w / W,
            bounds.x,
            bounds.y,
            self.font.get(),
        );
        d.rounded_rect(0.0, 0.0, W, H, 0.0, BG);
        d.rounded_rect(0.0, 0.0, W, 76.0, 0.0, PANEL);
        d.line(0.0, 75.0, W, 75.0, LINE, 1.0);
        d.text(32.0, 44.0, "CHORDBOARD", 23.0, GOLD);
        d.button(
            (814.0, 24.0, 112.0, 30.0),
            "QWERTY",
            self.params.keyboard.value(),
            TEAL,
        );
        d.button(
            (936.0, 24.0, 70.0, 30.0),
            if prefs().light() { "LIGHT" } else { "DARK" },
            false,
            TEAL,
        );
        self.draw_chords(&mut d);
        self.draw_performance_header(&mut d);
        if self.arp_main() {
            self.draw_arp(&mut d);
        } else {
            self.draw_pad(&mut d);
        }
        self.draw_meters(&mut d);
        for axis in 0..3 {
            let m =
                crate::engine::Mapping::decode(self.params.mapping(axis).load(Ordering::Relaxed));
            let source = match m.kind {
                1 => format!("CC{}", m.number),
                2 => format!("CC{}/{}", m.number, m.number + 32),
                3 => "BEND".into(),
                _ => "OFF".into(),
            };
            let channel = if m.kind == 0 {
                String::new()
            } else if m.channel == 16 {
                " · ANY".into()
            } else {
                format!(" · CH{}", m.channel + 1)
            };
            let r = mapping_summary_rect(axis);
            d.text(
                r.0 + 4.0,
                r.1 + 15.0,
                &format!("{}: {source}{channel} ▾", ["X", "Y", "GATE"][axis]),
                TEXT_SMALL,
                if m.kind == 0 { MUTED } else { TEAL },
            );
        }
        d.line(32.0, 582.0, 1088.0, 582.0, LINE, 1.0);
        for (i, &(group, label)) in self.groups().iter().enumerate() {
            d.tab_button(
                group_rect(i),
                label,
                self.group == group,
                if self.setup_open { TEAL } else { GOLD },
            );
        }
        d.tab_button(
            SETUP,
            if self.setup_open {
                "CLOSE SETUP"
            } else {
                "SETUP ▾"
            },
            self.setup_open,
            TEAL,
        );
        if self.pages() > 1 {
            d.button(
                PAGE,
                &if self.group == 1 {
                    if self.page == 0 {
                        "ARP  >".into()
                    } else {
                        "STRUM  >".into()
                    }
                } else {
                    format!("{}/{}  >", self.page + 1, self.pages())
                },
                false,
                TEAL,
            );
        }
        if self.group == 6 {
            self.draw_mapping(&mut d);
        }
        if self.arp_visible() && !self.arp_main() {
            self.draw_arp(&mut d);
        }
        for (c, rect) in self.placed_controls() {
            if self.arp_visible() && matches!(c.id, "gate" | "swing") {
                continue;
            }
            let color = if self.group == 2 { TEAL } else { GOLD };
            if c.toggle {
                d.button(
                    rect,
                    &format!("{}: {}", c.name, c.value),
                    c.norm >= 0.5,
                    color,
                );
            } else if rect.3 < 54.0 {
                let name = if c.id == "length_ms" {
                    "Length ms"
                } else {
                    &c.name
                };
                d.text(rect.0 + 12.0, rect.1 + 15.0, name, TEXT_SMALL, MUTED);
                d.text_right(
                    rect.0 + rect.2 - 12.0,
                    rect.1 + 15.0,
                    &c.value,
                    TEXT_SMALL,
                    color,
                );
                d.rect(rect.0 + 12.0, rect.1 + 23.0, rect.2 - 24.0, 6.0, LINE);
                d.rect(
                    rect.0 + 12.0,
                    rect.1 + 23.0,
                    (rect.2 - 24.0) * c.norm,
                    6.0,
                    color,
                );
            } else {
                d.text(rect.0 + 12.0, rect.1 + 18.0, &c.name, TEXT_SMALL, MUTED);
                d.text_right(
                    rect.0 + rect.2 - 12.0,
                    rect.1 + 18.0,
                    &c.value,
                    TEXT_LABEL,
                    TEXT,
                );
                d.rect(rect.0 + 12.0, rect.1 + 31.0, rect.2 - 24.0, 8.0, LINE);
                d.rect(
                    rect.0 + 12.0,
                    rect.1 + 31.0,
                    (rect.2 - 24.0) * c.norm,
                    8.0,
                    color,
                );
            }
        }
        self.draw_memory_drag(&mut d);
        if let Some(edit) = &self.edit {
            d.value_edit(edit, GOLD);
        }
        let footer = if self.status.is_empty() {
            "↑ / ↓ inversions • Drag memories to move / swap • Double-click values to type • Shift-click a memory to capture"
        } else {
            &self.status
        };
        d.text(
            32.0,
            767.0,
            &footer.chars().take(145).collect::<String>(),
            TEXT_SMALL,
            MUTED,
        );
        if let Some(menu) = self.menu {
            let first = menu.option_rect(0);
            let mut right = first.0 + first.2;
            let mut bottom = first.1 + first.3;
            for i in 1..menu.items().len() {
                let rect = menu.option_rect(i);
                right = right.max(rect.0 + rect.2);
                bottom = bottom.max(rect.1 + rect.3);
            }
            let bounds = (
                first.0 - 3.0,
                first.1 - 3.0,
                right - first.0 + 6.0,
                bottom - first.1 + 6.0,
            );
            d.rect(bounds.0, bounds.1, bounds.2, bounds.3, BG);
            d.outline(bounds, TEAL);
            for (i, label) in menu.items().iter().enumerate() {
                d.button(menu.option_rect(i), label, i == self.menu_cursor, TEAL);
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn arp_controls_fit_both_layouts_without_overlapping() {
        for main in [false, true] {
            let mut rects = (0..5).map(|i| pattern_rect(i, main)).collect::<Vec<_>>();
            rects.extend((0..8).map(|i| rate_rect(i, main)));
            rects.extend((0..4).map(|i| octave_rect(i, main)));
            rects.extend(arp_controls(main).map(|(_, r)| r));
            rects.push(rate_header(main));
            for (i, r) in rects.iter().enumerate() {
                assert!(r.0 >= 0.0 && r.1 >= 0.0 && r.0 + r.2 <= W && r.1 + r.3 < 750.0);
                if main {
                    assert!(
                        r.0 >= PAD.0
                            && r.1 >= PAD.1
                            && r.0 + r.2 <= PAD.0 + PAD.2
                            && r.1 + r.3 <= PAD.1 + PAD.3
                    );
                }
                for other in rects.iter().skip(i + 1) {
                    let overlap = r.0 < other.0 + other.2
                        && other.0 < r.0 + r.2
                        && r.1 < other.1 + other.3
                        && other.1 < r.1 + r.3;
                    assert!(!overlap, "overlapping arp targets: {r:?} and {other:?}");
                }
            }
        }
    }
    #[test]
    fn keyboard_is_staggered_and_hit_boxes_do_not_overlap() {
        assert_eq!(key_rect(KEY_COLUMNS).0 - key_rect(0).0, 10.5);
        assert_eq!(key_rect(KEY_COLUMNS * 2).0 - key_rect(0).0, 31.5);
        for i in 0..KEY_COUNT {
            let r = key_rect(i);
            assert!(hit(r, r.0 + r.2 / 2.0, r.1 + r.3 / 2.0));
            for j in 0..KEY_COUNT {
                if i != j {
                    assert!(!hit(key_rect(j), r.0 + r.2 / 2.0, r.1 + r.3 / 2.0));
                }
            }
        }
    }
}
