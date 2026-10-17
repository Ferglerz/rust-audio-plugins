# Chordboard

A MIDI-only harmony and strumming effect with Pleasant UI. VST3 and CLAP; no internal sound engine. Place Chordboard before an instrument and enable MIDI output/routing in the host. Stereo audio passes through unchanged.

## Play

Click the chord keyboard to focus the editor. QWERTY captures playing keys only while focused. The number row (`1 2 3 4 5 6 7 8 9 0 - +`) selects major chords, `Q W E R T Y U I O P [ ]` selects minor chords, and `A S D F G H J K L ; ' Enter` selects dominant seventh chords. The `+` position uses the physical `=/+` key without requiring Shift; Enter is labelled `ENT`. In **Chromatic** and **Fifths**, each staggered row includes all twelve roots; the layout changes their order. **Degrees** puts diatonic scale triads on Q, one brighter chord quality on the number row, and one darker quality on A: diminished → minor → major → augmented, clamped at the ends. Pentatonic scales use triads from their parent major or natural minor scale. **Passing** keeps Q and A, but places a chromatic approach chord immediately below each Q-row degree on the number key just before it, inheriting the preceding degree’s quality. In C major, 2=C♯, 3=D♯, 5=F♯, 6=G♯, and 7=A♯; key 1 and natural semitone gaps remain inactive. Wider scale gaps use the approach note below the next degree. Every layout retains the same physical row stagger. The bottom row (`Z X C V B N M , . /`) recalls the ten memories; Shift plus a memory key captures the current chord. Memory shortcuts follow the same QWERTY focus rules. Transpose adjusts the playing register; Key sets the first chord button and keyboard key’s root, along with the selected **Scale**. Scale changes highlighting in Chromatic/Fifths and the actual chord mapping in Degrees/Passing. The Key menu offers explicit sharp/flat tonic spellings; scale notes keep their correct letter names (for example E♯ in F♯ major and C♭ in G♭ major).

The default MIDI control octave is C0–B0 (MIDI 12–23). Hover its octave on the full keyboard to reveal a red border and **Learn octave** label, click the highlighted keys, then press C in the desired octave (or another key to transpose the control layout). The twelve keys select chords by interval from that learned tonic: C major, D♭ b9, D sus2, E♭ minor, E major, F sus4, F♯ diminished, G power, A♭ augmented, A sixth, B♭ dominant seventh, B major seventh. The learned range is highlighted on the full MIDI keyboard; hover its keys to reveal the red Learn border and see each assignment. Control keys select harmony silently: Note On, Note Off, zero-velocity Note On and per-note pressure are consumed, without starting a chord or strum. Notes below the bass split go directly to the bass output; other notes outside the control octave are performance notes. Learn can be cancelled with a second click.

The first performance note anchors the chord. A second note adds or replaces a degree: C+D gives Csus2, C+F gives Csus4, C+F# gives C–E–F#, C+G gives C5 (C–G), and C+B♭ gives C7 (C–E–G–B♭). Third and later notes add octave-independent pitch classes: ♭9, 9, ♯9, 11, ♯11, ♭13 and 13, plus remaining chord tones. Multiple extension variants coexist; later notes add rather than replacing the second-note behavior. With **Latch off**, releasing the second input removes its alteration. When only one key remains held, it becomes the root of a fresh chord, including a key pressed early while the previous chord was still held. Computer-keyboard chords retain the root key’s original quality. Additional notes preserve the established strike velocity and root expression ownership.

With **Latch on**, releasing any or all inputs retains the complete altered and extended chord, regardless of release order or timing. Releasing just D in C+D keeps Csus2. Re-press C to return to the base C chord; while C remains down, pressing another alteration replaces the retained one. Releases do not restart the strum or arpeggiator. Turning Latch off returns to the physically held inputs.

Harmony combines twelve base qualities with the second-note alteration and additive pitch classes from later notes. A second note a tone above the root still produces sus2; a later D adds 9 while retaining the third. A later F♯ adds ♯11 while retaining the fifth. Adding ♭9 to sus2 replaces the suspended second: C, then D, then D♭ produces C5(add♭9), voiced C–G–D♭. Releasing D♭ with Latch off restores Csus2. Seventh and sixth variants retain those tones and use no3 labels; sus4(add♭9) retains its fourth. Generated voicings hold up to twelve distinct notes. Memories persist the full recipe, including all additions.

Up/Down arrows cycle inversions within the original register, wrapping to root position. They do not accumulate octave changes. Input key repeats are ignored. Focus loss releases computer-keyboard notes without releasing unrelated MIDI inputs.

## Split keyboard and sustained layers

**Bass below** sets a separate bass-only boundary. Its default **Control octave** follows the bottom of the learned control octave. Drag the gold handle beneath the piano to either side of the control octave, or edit the Bass below value. Enter **Auto** to follow the control octave again. Notes below this boundary play only the selected bass MIDI channel, with sustain and note releases preserved. Control-octave notes always select qualities silently, even inside the bass range.

The Melody module’s power/bypass button enables **Key split**, which separates the remaining chord-selection keys from right-hand melody. Notes below the split choose the chord; the split note and higher notes pass through unchanged on their incoming MIDI channels. The default boundary is C4 (MIDI 60). Drag the split marker, click a key on the bottom piano when split is enabled, or click the split readout and type a note such as C4 or 60. The learned control octave is always consumed before the split is applied. Left-hand selection remains available after its keys are released.

**Play Always** in Bass holds the selected root. **Chord on Select** in Manual Strum holds the resolved chord tones, replacing Chords’ former Play Always button. They start when a right-hand key is held and stop when the last right-hand key is released. With **Latch** on, the accompaniment stays after release and changes to the next selected chord. Melody notes always follow their own releases. Bass Play Always and Chord on Select can be enabled without doubling shared tones; ordinary strum tails retain their note lengths. With Key split off, held performance keys gate these layers instead. New instances start with Bass Play Always off and Strum Only selected.

The full MIDI 0–127 piano shows control keys, the independent bass and right-hand splits, held input keys, sounding output notes, and selected voicing. Voice-leading paths animate above the keys, with exact note movements available on hover. Changing the split releases current notes to prevent stranded note ownership.

## Performance

The full MIDI keyboard animates each voice-leading transition: hollow starting dots above the split handles connect directly to destination keys, with dots animating along the paths during each transition. Upward, downward and stationary voices use distinct colors. The latest paths remain visible; hover the selection strip above the split handles for exact note names and semitone changes.

Changing chords preserves ringing manual and delay tails. Chord on Select updates its sustained layer to the new voicing. Panic, routing changes, stop, and seek release owned notes. Arp bypass releases automatic arp notes while manual playing stays available.

- **Manual Strum** uses the XY pad or CC1 mod wheel mapped to X. Both directions sweep the live voicing; stationary controls do not retrigger.
- **Arpeggiator** adds an independent automatic pattern. Once plays one cycle, Loop restarts on chord changes, Sync Loop retains rhythmic phase, and Aligned Loop follows the DAW grid.
- **MIDI Delay** repeats selected note sources and can run with arp power off.

X crosses virtual strings determined by chord tones and Octaves. Y controls manual strike velocity by default, or gate/pressure/timbre/bend/custom CC. Touching a string plucks it. Learned controllers seed their initial position before sweeping. Chord latch and CC64 retain harmony independently of physical input slots.

Click the chevron on the X or Y modulator tile to edit that controller directly. Choose Off, CC (automatic 7/14-bit detection), or Pitch bend, then select the CC number or bend channel. For paired CC use the MSB number 0–31; receiving its LSB (number+32) enables 14-bit resolution independently on that MIDI channel. Reset All Controllers (CC121) clears detected resolution. Saved explicit 14-bit mappings and channel filters remain compatible. X/Y MIDI Learn has been removed; use the modulator routing controls to assign other sources. Calibrate X with the Manual Strum pad’s grips (25% minimum span), calibrate Y with its horizontal grips, and reverse either axis in its controller editor. Grips stay visible; MIN / MAX labels appear while hovering the pad or dragging a grip, with numeric values on the grip being dragged. Labels sit above or below their handles and flip sides to avoid collisions. Pitch-bend faders must use a channel outside the incoming MPE member zone. No hardware-specific motor feedback is emitted.

## Osmose / MPE

Use **Osmose Play / Port 1**, External MIDI mode **MPE**. Enable **MPE on** and open Play. Select an MPE-capable destination instrument and enable MPE in that instrument/host. Configure matching bend ranges: default member ±48 semitones, master ±2.

The first accepted note's pressure, CC74 and bend fan out to every generated voice. The second input affects harmony only. Each output voice receives its own member channel, with expression initialized before Note On, including later strummed notes. Master expression remains separate. Incoming RPN bend sensitivity and zone configuration are recognized. Released source channels cannot take expression ownership when reused.

MPE output has a single **MPE on / MPE off** switch and defaults off. The switch pulses gold when MPE input is detected while output is off. Detection recognizes announced zones, host per-note tuning/brightness, and member-channel expression; notes alone do not trigger the warning. Existing saved protocol choices (including legacy Auto) remain compatible; clicking the switch sets an explicit on/off choice. MPE output uses the lower zone by default: master channel 1, members 2–16. Upper-zone and reduced member counts are configurable. Standard single-channel MIDI is also available. In MPE mode, generated chord voices retain member-channel allocation. Direct bass-zone notes use the chosen bass channel, which is reserved from generated MPE voices while a bass zone is enabled. An engaged Y assignment overrides only its chosen expression dimension.

Actual Osmose feel, downstream patch response, and host-specific MIDI routing require the hardware acceptance checks in `docs/ACCEPTANCE.md`.

**Affect Chords** in Melody adds held melody pitch classes to the selected chord. Chord tones that match an exact held MIDI note move to another available octave within one octave of the chord register, or are omitted if no octave fits. Releasing melody restores the selected harmony; disabling the option preserves ordinary chord behavior. The option is saved with presets and defaults off.

## Discover and recall

Compatible chords are always highlighted and Roman numerals are always displayed. Select a key and scale from the menus in the Chords header to update them. Outside chords remain playable. Highlighting considers all chord tones, not only the root. Major, natural/harmonic minor, Dorian, Mixolydian, Lydian and major/minor pentatonic scales are available.

Ten memory keys form the fourth staggered row inside the chord keyboard module and store the current harmonic recipe: root, alteration, additive pitch classes, quality, inversion, transpose and spread. Click an empty **+** card to save, or Shift-click a card to save or replace it. Saving requires a valid current chord. Shift-click and Shift plus a memory key remain capture shortcuts. The interface confirms a save only after the stored slot updates; an identical chord reports that it is already stored, and an unconfirmed request asks you to try again. Click a filled card to recall it into the current performance mode. Recall remains available for strumming until a new performance input replaces it. Empty cards have a dashed outline. Hover any card to reveal its slot number; filled cards also reveal their trash icon. Chord names and keyboard shortcuts remain visible. Drag a filled card onto an empty card to move it, or onto another filled card to swap the two. Release outside the cards to cancel a drag. Slots persist with project state and host presets.

Standard MIDI can route bass and upper voices to separate channels. MPE retains its member-channel allocator.

The ten slots are a manual progression sketchpad, not a step sequencer. MIDI drag-out, constrained chord search and a separate previous/current comparison view are follow-ons.

## Selection and modulation

Play has a three-way selector above the strings: **Strum Only → Root on Select → Chord on Select**. Strum Only waits for a gesture. Root on Select sounds the transposed root immediately for Note length. Chord on Select sustains the resolved voicing while its gate or latch is held. These choices work independently of Strumfield and arpeggiator power.

The **Modulators** column on the right meters pitch wheel, mod wheel (CC1), expression (CC11), breath (CC2), pressure, timbre (CC74), final note velocity after contour and humanization, and raw strum X/Y. MIDI expression meters use the latest incoming value across channels. Pitch fills outward from its neutral center. The Velocity meter updates before each generated note is sent; velocity routes to note length and gate use that note’s final velocity. Routes that shape velocity itself use raw input velocity to avoid feedback. **Drag a source meter onto a control** to link it. Source cards highlight in place on hover, with routing guidance in the footer; dragging shows eligible destinations, a cable and the proposed link. Clicking a meter opens its route settings; clicking the selected meter again returns to the chordboard. The selected meter pulses with a single strong border, and one inset dot marks each assigned route. Drop elsewhere or press Escape during the drag to cancel. Hovering a linked source highlights its visible destinations.

A successful drop slides that link’s range and curve editor into the Chords module; Performance remains usable. Close it with × or Escape. Hovering a modulator shows clickable destination chips below it, including disabled links, and wires leave the top or bottom edge toward each destination. Dropping onto an already linked destination reuses the effective route and preserves its range and curve; the preview says when the drop will replace its source. Repeating the same link opens it without adding duplicates. MIDI output controls are always visible in three equal-width modules below the keyboard; open a controller editor to drop onto its local controls. The editor shows assigned destinations as chips with **+ Add route**, plus categorized source and destination menus. The separate Routes button and X/Y controller popovers are removed. Existing controller assignments are preserved. Click a destination overlay’s **×** to show **Are you sure?**; only its check mark removes the route.

Each slot has an enable switch, source, destination, and a compact interactive graph. Drag the left/right endpoints vertically to set minimum/maximum; drag the center node vertically to bend the curve. Destination readouts use musical units (for example, Strings 3–12 or Voicing Close–Drop 3). Minimum greater than maximum reverses the mapping. **Linear ↺** resets the curve. The small live dot and input/output readout show the current modulation.

Sources can drive multiple destinations across performance, voicing, expression and MIDI-output controls. Routes replace the destination within their range; when several routes target the same parameter, the highest-numbered enabled route wins. Routed controls show teal live values and their driving source. Attempts to edit them identify the controlling route; disable or remove it to restore manual control. Disabling a route restores that base setting (or recalled voicing). Routes and their settings save with the host project and can be automated. Existing X/Y MIDI assignments remain stored alongside these routes. Touch-range X/Y minimum and maximum are not modulation destinations; their grips remain available for direct calibration. Saved routes to those four bounds are inactive. Strum X / sweep remains routable without feeding its result back into modulation sources.

## Interface

The 1811 × 996 interface balances the chord keyboard and voicing controls with the performance surface. Manual Strum includes Flattery’s shared graph zoom icon, which grows the strum field over the plugin body below the title header. The module edge uses the shared quarter-second quintic slide: performance width and position interpolate while the chord surfaces shrink, matching Composure’s envelope expansion. The inward icon returns the field. Both sizes center the graph and its handles together, with matching 14 px margins at the left, top and right handle edges, with the zoom button anchored at the top-right and Hover strum directly to its left. Both plugins draw these icons from `pleasant-ui/src/draw/icons.rs::Draw::graph_zoom_button`. The Arpeggiator and keyboard remain visible while the Strumfield is expanded. Hover the keyboard voicing for a border and click to cycle **Close / Open / Wide / Drop 2 / Drop 3**. The voice-leading selection strip sits above the split handles and keeps its original dimensions; paths descend from the previous chord directly to the keyboard keys. Nearest and Furthest show a persistent purple border and mode label; click to cycle **Nearest / Furthest / Off**. Bass stays left-aligned, Melody right-aligned, and the purple Chord module sits immediately left of Melody. Melody dims when bypassed. Bass, Chords and Melody place all controls in aligned buttons with small rounded corners inline with their titles; switching to MPE keeps those dimensions and omits separate bend-range controls. Neutral Key, Scale and layout controls share the main CHORDS header with a dark grey segmented transpose control above the QWERTY board. Key and Scale gain an accent while their menus are open. Latch sits beside the memory row as a larger button: neutral when off, gold when on, with its Shift shortcut always visible. Dropdowns dismiss by choosing an option, clicking outside or pressing Escape. Chord on Select uses the Chords module’s purple highlight. Buttons throughout the editor use Damian’s shared 5 px corner radius, with selection accents drawn as rounded backgrounds offset 2 px below the opaque button face. The Performance modes share its header, with a neutral Host/Manual tempo selector and BPM in the plugin header. The shared Auto/Light/Dark control aligns with the right module edge, 16 px from the window edge. Channel selectors remain in the keyboard submodule headers. Bass and Melody both have bypass buttons and **Learn Split**: play a MIDI note or click a key to set a boundary. Crossed learned boundaries swap, and dragging preserves one chord key between Bass and Melody. Bass bypass disables its direct routing and Play Always layer. Split handles use the shared EQ nodes and selection rings. Voice-leading animation descends from previous notes toward the new notes at the keyboard, whose selected voicing has a yellow fill and border. Compact inversion controls sit above the keyboard; the full keyboard below shows the resulting voicing. Spread affects playback voicing. The MIDI control octave selects chord quality silently. Hover its keys for **Learn octave**, or use **Control octave** when no control octave is assigned. Computer-keyboard playing is always enabled while focused; the former enable control is removed.

The Arpeggiator module offers a four-state repeat dropdown and seven orderings. Tempo sync selects beat divisions; Free uses milliseconds. Each pattern step appears as an adjustable string. Arp Octaves is independent of Manual Strum Octaves. Each modulation route retains its own Contour, bounds, curve, and smoothing.

Modulators fill the vertical column beside the performance and keyboard surfaces. Taller cards align their icons left and names right, with space above the meters. The output protocol and MIDI controls live in the wide, single-row Bass, Chord and Melody submodules above the keyboard. Menus and controller popovers have explicit close controls. Clicking a modulator opens its routing editor; clicking it again closes it. While open, its meter and visible connections continue pulsing even when the pointer leaves. Route chips use inset stroke icons: X asks for confirmation and the check removes the link. Manual CC assignments accept any input channel; MIDI Learn locks the assignment to the learned channel. The field’s Y axis controls velocity by default; the external Y controller starts unassigned. CC11 (Expression) changes velocity only when explicitly mapped or routed. Existing saved controller assignments are preserved. The X and Y MIN / MAX grips retain direct range calibration on the strum surface. In Manual Strum, the touch-icon **Hover strum** control plays the strum field on pointer hover so a mouse or trackpad does not need to click and hold. The lock-icon **Latch** tile retains the selected chord after note release; these controls have independent roles.

The compact Chords module uses 66 × 78 px tiles with the original row stagger. The four tile rows are vertically centered beneath the header, and the narrower Chords module reduces the full editor and keyboard width by 128 px. Performance retains room for its full control rows; Velocity lives directly on each playback page, and Keyboard is shortened to match the single-row controls. The gap above the Z memory bank is 10 px; row gaps are 8 px. Module padding, performance graph height and piano key height are tightened without increasing text sizes. The default editor opens at native scale. Playing shortcuts retain their physical keys across all four layouts. Hover memories, meters or controls for contextual detail. The interface uses shared colors, appearance preferences and controls. Key depression, pressure glow, string pulses, gesture trails and expression meters follow engine state; visuals never control MIDI timing. The strum field, trails, timing and calibration behavior are preserved. Continuous controls can be dragged from anywhere in their bounds or scrolled; click a value without dragging for text entry (Shift-drag makes fine adjustments). Values display meaningful units such as percentages, milliseconds and semitones, and typed input accepts the displayed units. On/off settings are buttons.

Use the host's preset and project features to save custom setups. Host state persists mappings, chord memories and editor settings. Runtime held notes are never saved. Appearance follows the application's preference file.

## Build and install

From the parent `audio-plugins` workspace:

```sh
cargo check -p chordboard
cargo test -p chordboard --quiet
cargo clippy -p chordboard
chordboard/scripts/install.sh
```

The installer rebuilds, bundles, signs and verifies both Chordboard formats in the user's plugin directories. Running the bundler alone does not install them.

This repository is a full-history subtree at `chordboard/`. Standalone checkout builds need the sibling Pleasant libraries and the parent workspace's patched NIH-plug/baseview/Vizia sources under `Damian Channel Strip/vendor`, matching existing plugins. Prefer the integrated workspace; subtree export does not carry those dependencies or the parent installer.

Interactive native preview (uses the real plugin with a dummy backend by default):

```sh
cargo run -p chordboard --features ui-preview --bin chordboard-preview
```

Click the mode controls and protocol dropdown to change performance mode and output protocol. This uses NIH-plug's standalone wrapper, including parameter updates and the real processing loop. The default dummy backend opens no audio/MIDI devices, so MIDI Learn requires an explicitly selected MIDI-capable backend (`-- --help` lists options). On macOS it requires a graphical session and OpenGL access. No REAPER automation is used.

## Architecture and reference

`harmony.rs` resolves fixed-capacity voicings. `engine/` owns input admission, lifecycle, timing, controllers and MPE. `lib.rs` adapts sample-timed NIH-plug events. `bridge.rs` provides bounded lock-free UI queues; UI overflow requests Panic rather than losing releases. `params.rs` defines stable host IDs and versioned saved fields. `ui/` renders and handles focused input.

All audio-thread storage is bounded: held-input and addition tables, twelve-note chord voicings, fifteen generated voices, sixty-four pending strikes, a 512-event outgoing batch and bounded GUI queues. An overflowing batch is discarded atomically and previously sounding notes are released; a new performance input resumes playback. Note Off ownership is independent of the pending-strike queue. Channel exhaustion ends the oldest voice before reuse.

Original software inspired by the interaction model of Benjamin Poilvé's [minichord](https://github.com/BenjaminPoilve/minichord); firmware was consulted as a reference, not copied or linked. Discovery features were inspired by KawaChord2. Chordboard does not include their branding, firmware, presets or artwork. GPL-3.0-or-later, consistent with the shared project libraries.

Y destination, custom CC and Reverse Y share the Y controller editor; Reverse X is in the X controller editor. Compact vertical pressure, timbre and bend meters sit at bottom right. Control-octave Learn is on the main interface. Transpose buttons in Chordboard step by −12, −1, +1 or +12 semitones, with Reset returning to zero. Escape closes an open menu or popover and releases editor-owned notes.

Playback pages use the shared quarter-second slide animation. QWERTY playing is always enabled while the editor is focused, and key labels stay visible. The chord-name readout beside the memory cards remains visible after release. Swing supports −75% to +75%, reversing the long/short step pattern below zero.

HOST SYNC follows DAW tempo (120 BPM fallback if unavailable); disable it to set manual BPM. The arpeggiator’s Tempo sync toggle selects beat divisions instead of milliseconds for both Once and Loop, using the same tempo source.

Sync Loop preserves phase across chord changes and live voicing edits; Once and Loop restart on new chord selections. Page requests take effect at the next rhythmic boundary. Aligned Loop follows host beat zero while host tempo, beat position, and tempo-synced rate are available; its status indicates Sync Loop fallback otherwise. Stop and seek release owned notes, and Aligned Loop resumes the selected chord at the new host position.

### Smart voice leading

Click the dedicated voice-leading button to cycle **Nearest resolution → Furthest dominant resolution → Off** (default). Nearest chooses the inversion and octave placement with the least ordered voice movement from the previous chord. Furthest dominant favors the most movement for major/dominant chords resolving down a fifth to major or minor, and uses nearest movement elsewhere. Candidates stay within one octave of the requested voicing and retain its chord tones; note filters apply afterward.

The first chord uses the requested inversion/register. Repeated chords retain their resolved voicing across note releases. Panic/reset clears the history; changing inversion, transpose, spread, playback mode, or voice-leading mode starts a fresh voicing. The mode is saved and automatable as `voice_leading`; existing presets default to Off.

Chord quality follows the MIDI control-octave selection. Hover and click the voice-leading dots to choose the mode; click the voicing on the piano to cycle spread.

## Live values and layout

Chords and Performance sit above the full MIDI keyboard, with the Modulators column on the right. The full MIDI keyboard has voice-leading paths and split handles above its keys, with equal-width Bass, Chords and Melody settings below. Transpose has semitone/octave steps and an explicit Reset button that resets the saved base value; active modulation continues to override that base. Routed button choices show a teal live highlight, and sliders retain the saved base marker.

Module headers follow Pleasant UI typography, including the shared 24-unit title. Chordboard retains its 1811 × 740 design size. Fresh windows fit within 90% of screen width with room for host chrome; saved instance sizes, remembered user sizes and manual resizing override this initial fit. The bottom-right handle resizes independently of the default screen allowance. Older presets gain the default X route in a free slot while preserving existing links. A full legacy matrix keeps its original direct X behavior.

The current held or latched chord keeps its persistent pulsing highlight and dot on the chordboard, including chords played over MIDI or recalled from memory. Chord tiles use simpler static framing while retaining key depression, pressure glow and all animated highlights. Split-learning key previews use gold for Bass and teal for Melody. Click any modulator and select a route to edit that route’s Contour.

## Arpeggiator

Arp power enables automatic playing independently of Manual Strum, Root on Select, Chord on Select, Bass, and Melody. Eight Pages store complete variations: repeat mode, ordering, octaves, synced/free rate, swing, humanize, adjustment lanes, and MIDI Delay. Switching Pages keeps phase in continuous modes; requesting a Page after Once finishes starts that variation. Arp power and modulation routing stay global. Page changes use the shared slide animation.

Every string represents one occurrence in the pattern. For C → E → G → E, the two E strings have independent adjustments. Choose **Velocity**, Gate, Chance, Octave, Timing, Mod 1, or Mod 2 from the sliding lane chooser. Its large buttons use the existing icon set; ordering uses the earlier direction diagrams. Drag vertically to adjust, sweep across strings to paint, and hold Shift for a straight ramp. Cmd/Ctrl-click resets one adjustment; Cmd/Ctrl-drag resets crossed strings. Swing shifts the visible string positions and displays as a whole percentage from −100% to +100%. Timing adds individual sideways offsets with a dotted reference at each swung position.

Click the button below a string to cycle **Play → Mute → Skip**. Mute leaves a rhythmic rest; Skip removes that occurrence from the cycle. The buttons stay available on every lane. Cmd/Ctrl-click restores Play. Gate directly specifies a percentage of the step interval. Note choice comes from the ordering; per-step note overrides, repeats, and speed adjustments are omitted.

Choose **Once / Loop / Sync Loop / Aligned Loop** in the repeat dropdown. New Pages default to Sync Loop. Sync Loop keeps the step counter and rhythmic clock when chords change, wrapping into the new pattern length. Aligned Loop additionally uses the DAW beat grid and retains swing. All-skipped patterns remain silent.

## MIDI Delay

Click **Delay** at the arp module’s top right to open its slide-over. Closing the editor leaves playback running. Delay has independent power and source switches for Arp, Manual Strum, Chords, Bass, and Melody; Arp is selected initially.

Basic controls set synced/free delay time, echo velocity, and feedback. Advanced controls expose up to eight taps and ping pong. Move a tap’s top handle sideways to change its time. Select Velocity, Feedback, Pitch, Fine Pitch, Pan, or Output Channel to edit vertically; painting, Shift ramps, and Cmd/Ctrl reset work like the arp lanes. Pitch offsets apply to every echo without accumulating.

Each tap repeats at multiples of its interval and decays according to its own feedback. Taps never feed one another. Ping pong alternates MIDI CC10 pan on successive repeats. Delay preserves note duration and attack expression. Chord and Page changes retain existing tails; explicitly bypassing Delay or using panic, stop, seek, or preset replacement releases echoes.

Automatic routing isolates echoes on available MIDI channels so their pan and fine pitch do not affect dry notes. Configure the receiving instrument to accept these channels. Per-tap output channels can be selected explicitly; occupied incompatible channels drop the new echo. Dry notes have priority. Defaults are one synced eighth-note tap, 75% velocity, 35% feedback, neutral pitch/pan, and automatic channel allocation, with Delay off.

## Modulation and shortcuts

**Arp Position** is a unipolar 0–100% source across the played cycle. It counts muted and chance-failed steps, excludes skipped steps, and returns to zero when inactive. **Mod 1 / Mod 2** are bipolar adjustment lanes (−100…+100%) with routable meters in the Modulators column. They update on muted and chance-failed steps too. Their routes add offsets to editable base values; zero leaves the base unchanged. Arp Position uses ordinary unipolar routing.

Modulator chains resolve before parameter routes, independently of route slot order. Circular links are bounded and read the original input at the back edge. Route smoothing supports 0–2000 ms. Removing expression routes restores the live controller baseline.

**Latch:** click the tile or tap Shift to toggle. Holding Shift temporarily inverts latch and restores it on release (250 ms threshold). Shift used during editing remains a modifier. Focus loss restores the prior latch state; text entry does not trigger the shortcut.

The former sequencer is preserved at Git tag `archive/chordboard-sequencer-2026-10-17`; see [archive notes](archive/README.md). Its presets are not migrated into this development version.
