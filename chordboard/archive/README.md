# Sequencer archive

The complete, runnable sequencer implementation is preserved at Git tag
`archive/chordboard-sequencer-2026-10-17` (commit
`5e37976350cfd6067f49e625fa5d4883febaeca6`). This includes its editor, engine,
persistence, tests, and the matching dependencies.

Inspect a file with `git show archive/chordboard-sequencer-2026-10-17:chordboard/src/sequencer.rs`.
Restore the complete version in a separate checkout with
`git worktree add /tmp/chordboard-sequencer archive/chordboard-sequencer-2026-10-17`.

The replacement arpeggiator deliberately does not read or migrate sequencer presets.
