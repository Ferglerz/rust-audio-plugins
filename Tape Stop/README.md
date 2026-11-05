# Tape Stop

An audio tape brake with MIDI-triggered slowdown and return.

## Affect MIDI

**Affect MIDI** in the header is on by default and saved with the preset. Place
Tape Stop before an instrument and route its MIDI output to that instrument.

Outgoing MIDI follows the tape speed: notes and controllers slow down, wait while
the tape is stopped, and catch up during the return. Incoming trigger notes and
the assigned speed CC still control the brake immediately. MIDI uses the average
of the left and right tape speeds when stereo divergence is enabled.

Pitch bends follow the same speed curve, including the upward catch-up on return.
Tape Stop requests a ±48-semitone range using MIDI RPN 0. The receiving instrument
must support that request; otherwise set its bend range to ±48 semitones manually.
Bends are limited to four octaves, so a fully stopped tape holds the minimum MIDI
pitch rather than silencing a sustained synth voice. Existing pitch-wheel input
is combined with the tape bend. At normal speed or when disabled, the original
bend and last received RPN bend range are restored (±2 semitones if none was sent).

Switching Affect MIDI off passes MIDI through at its original pitch and timing.
Switching modes or resetting releases held output notes and pedals and clears
delayed events. Power off and bypass also disable MIDI processing. A fixed queue
holds 16,384 events; if a prolonged stop fills it, Tape Stop releases output notes,
discards the backlog, and passes MIDI through until playback returns to normal.
