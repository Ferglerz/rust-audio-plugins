# Tape Stop

An audio tape brake with MIDI-triggered slowdown and return.

## Play pitch with expression

Set **Pos CC** in the bottom row to your controller number, such as **11** for an
expression pedal. Click the value to type a number or drag to adjust it. The
assignment is saved with the preset and defaults to **Off**.

With audio playing, move the pedal to take over the brake curve, even during a
stop or return. Higher values move toward the stopped end. The midpoint plays
at half speed (one octave down). The Curve control sets how pitch responds
between the endpoints. The head holds its pitch until you move again. Returning
to zero releases the pedal and crossfades back to live audio, or starts a new
timed brake if its trigger is still held. A standard 7-bit controller works immediately.
Controllers 0–31 also accept their matching 14-bit fine messages. With Affect MIDI
enabled, Tape Stop consumes the position CC so an expression pedal controls
pitch without also changing the downstream instrument’s volume.

## Affect MIDI

**Affect MIDI** in the bottom settings row is on by default and saved with the preset. Place
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
