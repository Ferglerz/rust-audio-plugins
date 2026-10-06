# Premade Chord Loops & Harmonic Progressions

A comprehensive reference collection of popular, lush, high-energy, and advanced harmonic loops designed to run through **Chordboard** (as MIDI generator, voicing engine, and strummer) and instruments like **OpenWurli**, electric pianos, analog/FM synths, and acoustic keys.

---

## Technical Guide: How Chordboard Generates Harmony

To get accurate results in Chordboard, it is essential to understand how its engine resolves chords:

1. **Native Chord Qualities (`chordboard/src/harmony.rs`)**:
   Chordboard natively supports 12 base qualities:
   - `0: Major` (`[0, 4, 7]`)
   - `1: Minor` (`[0, 3, 7]`)
   - `2: 7` (`[0, 4, 7, 10]`)
   - `3: Maj7` (`[0, 4, 7, 11]`)
   - `4: Min7` (`[0, 3, 7, 10]`)
   - `5: Dim` (`[0, 3, 6]`)
   - `6: Aug` (`[0, 4, 8]`)
   - `7: 6` (`[0, 4, 7, 9]`)
   - `8: Min6` (`[0, 3, 7, 9]`)
   - `9: Dim7` (`[0, 3, 6, 9]`)
   - `10: Half dim / m7♭5` (`[0, 3, 6, 10]`)
   - `11: Power / 5` (`[0, 7]`)

2. **The 10 Memory Cards**:
   Chordboard features **10 memory slots** across the bottom row:
   `Z` (1), `X` (2), `C` (3), `V` (4), `B` (5), `N` (6), `M` (7), `,` (8), `.` (9), `/` (10).
   Press `Shift + Click` or `Shift + Memory Key` to store the active chord. Click or press the memory key anytime to recall it!

3. **Two-Note Alterations vs. Extensions**:
   - In Chordboard, playing a root + a second note an interval of **2 semitones** above replaces the 3rd with the 2nd $\to$ creating a **$\text{sus}2$** chord.
   - Playing an interval of **5 semitones** replaces the 3rd with the 4th $\to$ creating a **$\text{sus}4$** chord.
   - Playing an interval of **1 semitone** on a dominant 7th adds a minor ninth $\to$ creating a **$7(\flat 9)$** chord.
   - **Extended 9ths, 11ths, ♯11s, and 13ths** are achieved in Chordboard in two authentic ways:
     - **Upper Structures via Bass Below Split**: The left hand holds the low bass note (e.g. $C$), while the chord layer plays an upper triad (e.g. $D$ Major triad $\to$ $D/C = C^{\text{add}9\sharp 11}$; or $B\flat$ Major $\to$ $B\flat/C = C9\text{sus}4$).
     - **Melody "Affect Chords" Mode**: Enabling *Affect Chords* in the Melody module dynamically injects held right-hand melody pitch classes into the sounding chord voicing!

4. **MIDI Control Octave (Learned Octave)**:
   By default (C0–B0), the 12 chromatic keys select harmony qualities directly:
   - `C`: Major | `C♯`: ♭9 | `D`: sus2 | `D♯`: Minor | `E`: Major | `F`: sus4
   - `F♯`: Dim | `G`: Power (5) | `G♯`: Aug | `A`: 6 | `A♯`: 7 | `B`: Maj7

---

## Rhythm Grid Legend

The rhythm grids use standard measure subdivisions:
- **Duple ($4/4$)**: 16th-note resolution (`1 e & a  2 e & a  3 e & a  4 e & a`)
- **Compound ($12/8$ or Triplet Shuffle)**: Eighth-note triplet resolution (`1 & a  2 & a  3 & a  4 & a`)
- `X`: Fresh chord strike / chord change
- `—`: Held / sustained chord tone (tie)
- `.`: Rest / release / percussive muted chuck
- `↓ / ↑`: Downward vs. upward comping stroke or strum sweep direction
- `Push`: A chord triggered on the offbeat anticipating the next bar

---

## Table of Contents

1. [Jacob Collier Advanced & Micro-Harmonic Concepts](#1-jacob-collier-advanced--micro-harmonic-concepts)
   - 1.1 The Negative Harmony 2-5-1 Cadence (Fm6 – Cmaj9)
   - 1.2 Dual-Axis Negative Reharmonization (A♭maj7 – Fm6 – D♭maj7♯11 – C)
   - 1.3 Super-Ultra-Hyper-Lydian Ascending Cycle (Polychord Stacks)
   - 1.4 The "In The Bleak Midwinter" Mediant Shift (Coltrane/Collier Matrix)
   - 1.5 Neapolitan Minor-Major 9th Pivot (Fm(maj7) – D♭maj7♯11 – Cmaj9)
2. [Funk, P-Funk & Fusion](#2-funk-p-funk--fusion)
   - 2.1 The James Brown Dominant 9th Groove ("Sex Machine" / "Cold Sweat")
   - 2.2 Herbie Hancock Headhunters Vamp ("Chameleon")
   - 2.3 Stevie Wonder Motown Funk ("I Wish" – Verse & Chromatic Bridge)
   - 2.4 Stevie Wonder Clavinet Stomp ("Superstition")
   - 2.5 Earth, Wind & Fire Uplifting Funk Anthem ("September")
   - 2.6 Nile Rodgers & Chic Disco Funk ("Good Times")
   - 2.7 Prince Minneapolis Sound ("Kiss" / "Controversy")
   - 2.8 Vulfpeck / Cory Wong Modern Pocket Nu-Funk ("1612")
   - 2.9 P-Funk & G-Funk Mothership Glide ("Give Up the Funk" / "Flash Light")
   - 2.10 Tower of Power Oakland Grease ("What Is Hip?")
3. [Neo-Soul & Warm R&B](#3-neo-soul--warm-rb)
   - 3.1 The "Just Friends" D'Angelo Groove (ii9 – V13 – Imaj9 – IVmaj7)
   - 3.2 Smooth Neo-Soul Stepper (vi9 – ii9 – V11 – Imaj9)
   - 3.3 Childish Gambino / 70s Soul Glide ("Redbone")
   - 3.4 Erykah Badu & J Dilla Chromatic Passing Harmony
   - 3.5 H.E.R. & Daniel Caesar Modern Acoustic R&B ("Best Part")
4. [Lo-Fi Hip-Hop & Chillhop](#4-lo-fi-hip-hop--chillhop)
   - 4.1 Nostalgic Rain / Rainy Afternoon (ii7 – V7♭9 – Imaj7 – vi7)
   - 4.2 Melancholy Sunset (IVmaj7 – iii7 – ii9 – vi7)
   - 4.3 Midnight Rhodes Drift (♭VImaj7 – V7alt – i9 – iv7)
   - 4.4 Coffee Shop Modal Chill (i9 – IV13 Vamp)
5. [Japanese City Pop & Shibuya-kei](#5-japanese-city-pop--shibuya-kei)
   - 5.1 The Royal Road ("Oudou" Progression: IVmaj7 – V7 – iii7 – vi)
   - 5.2 Tatsuro Yamashita / Mariya Takeuchi ("Plastic Love" 8-Bar Loop)
   - 5.3 Sparkle Breezy Promenade (Imaj9 – VI7 – ii9 – V7sus4 – V7)
6. [French House, Nu-Disco & Filter Grooves](#6-french-house-nu-disco--filter-grooves)
   - 6.1 Daft / Roulé French Touch Stabs (i7 – ♭VII7 – ♭VImaj7 – ♭VII7)
   - 6.2 Upbeat Filter-Swept Soul (IVmaj9 – iii7 – vi9 – I/V)
   - 6.3 90s Deep House / Garage Vamp (i9 – v7 – ♭VImaj7 – ♭VII7)
7. [Gospel & Modern R&B Passing Harmony](#7-gospel--modern-rb-passing-harmony)
   - 7.1 Sunday Morning 2-5-1 with Secondary Dominants (12/8 Compound Meter)
   - 7.2 Tritone Substitution Walkdown (Chromatic Bassline)
8. [Indie Pop, Dream Pop & Ambient](#8-indie-pop-dream-pop--ambient)
   - 8.1 Dreamy Bedroom Pop Haze (Mac DeMarco / Boy Pablo)
   - 8.2 Shoegaze / Cocteau Bloom (Suspended Wall of Sound)
   - 8.3 Cinematic Melancholy (Film Score Ambient Minor)
9. [Chordboard Master Configuration Matrix](#9-chordboard-master-configuration-matrix)

---

## 1. Jacob Collier Advanced & Micro-Harmonic Concepts

### 1.1 The Negative Harmony 2-5-1 Cadence (Fm6 – Cmaj9)
In Ernst Levy's and Jacob Collier's Negative Harmony system, the dominant-to-tonic cadence ($G7 \to C$) is inverted across the C–G reflection axis (the $E\flat/E$ midline). Under this mirror reflection:
- $G7$ ($G - B - D - F$) reflects note-for-note into **$F\text{m}6$** ($C - A\flat - F - D$).
- $Dm7$ (the ii chord) reflects into **$B\flat$ Major** or **$B\flat 6$**.
- Resolving $F\text{m}6 \to C^{\text{maj}9}$ yields gravity-defying downward voice leading where every note pulls with intense emotional resonance.

- **Key Center**: C Major
- **Harmonic Formula**: $\text{ii}_{\text{neg}} - \text{V}_{\text{neg}} - \text{I}^{\text{maj}9}$ ($B\flat 6 \to F\text{m}6 \to C^{\text{maj}9}$)
- **Tempo**: 72 BPM | **Voicing Spread**: Drop 2 or Open

| Slot | Chord | Bass | Voicing Notes (Exact Pitches) | Chordboard Native Quality | Harmonic Role |
|:---:|:---:|:---:|:---:|:---:|:---:|
| **1** | **B♭6** | B♭1 | F3 – B♭3 – D4 – G4 | Quality 7 (`6`) on B♭ | Negative ii chord |
| **2** | **Fm6** | F1 | C3 – A♭3 – D4 – F4 | Quality 8 (`Min6`) on F | Negative V dominant mirror |
| **3** | **Cmaj9** | C2 | E3 – G3 – B3 – D4 | Quality 3 (`Maj7`) on C + D | Tonic target resolution |
| **4** | **C6/9** | C2 | E3 – A3 – D4 – G4 | Quality 7 (`6`) on C + D | Lydian-tinted acoustic finish |

#### Rhythm & Voice-Leading Notes (2-Bar Loop)
- **Bar 1**:
  - `Beat 1.1`: **B♭6** downward slow strum (beats 1–2)
  - `Beat 3.1`: **Fm6** strikes on beat 3. Notice the guide-tone resolution: $A\flat$ falls down a semitone to $G$, $D$ falls to $C$, and $F$ falls to $E$!
- **Bar 2**:
  - `Beat 1.1`: **Cmaj9** resolves with lush bloom (beats 1–3)
  - `Beat 4.1`: **C6/9** acoustic color shift
- **Chordboard Engine**: Set Voice-leading to **Nearest**. Watch the animated paths descend smoothly without octave skips!

```text
Bar 1 (Bb6 -> Fm6):
Beat: | 1   e   &   a | 2   e   &   a | 3   e   &   a | 4   e   &   a |
Chord:| Bb6 —   —   — | —   —   .   . | Fm6 —   —   — | —   —   —   — |
Comp: | ↓ (slow sweep)|               | ↓ (tender)    |               |

Bar 2 (Cmaj9 -> C6/9):
Beat: | 1   e   &   a | 2   e   &   a | 3   e   &   a | 4   e   &   a |
Chord:| Cmaj9   —   — | —   —   —   — | —   —   .   . | C6/9—   —   — |
Comp: | ↓             |               |               | ↓             |
```

---

### 1.2 Dual-Axis Negative Reharmonization (A♭maj7 – Fm6 – D♭maj7♯11 – C)
Jacob's multi-step negative turnaround: cascading subdominant minor reflections landing on an expansive Neapolitan Major 7th before melting into the tonic.

- **Key**: C Major | **Tempo**: 68–74 BPM | **Feel**: Romantic, expressive rubato

| Slot | Chord | Bass | Voicing Notes (Exact Pitches) | Chordboard Implementation | Harmonic Role |
|:---:|:---:|:---:|:---:|:---:|:---:|
| **1** | **A♭maj7** | A♭1 | G3 – C4 – E♭4 – G4 | Quality 3 (`Maj7`) on A♭ | $\flat\text{VI}^{\text{maj}7}$ negative substitute |
| **2** | **Fm6** | F1 | A♭3 – C4 – D4 – F4 | Quality 8 (`Min6`) on F | $\text{iv}^6$ negative dominant |
| **3** | **D♭maj7♯11**| D♭1 | F3 – A♭3 – C4 – G4 | Upper: Quality 0 (C) over D♭ | $\flat\text{II}^{\text{maj}7\sharp 11}$ Neapolitan bloom |
| **4** | **Cmaj7** | C2 | E3 – G3 – B3 – E4 | Quality 3 (`Maj7`) on C | $\text{I}^{\text{maj}7}$ release |

#### Rhythm & Beat Placements (2-Bar Loop)
- **Bar 1**: **A♭maj7** on `Beat 1.1` (beats 1–2) $\to$ **Fm6** on `Beat 3.1` (beats 3–4)
- **Bar 2**: **D♭maj7♯11** on `Beat 1.1` (beats 1–2) $\to$ **Cmaj7** on `Beat 3.1` (sustains through beat 4)

---

### 1.3 Super-Ultra-Hyper-Lydian Ascending Cycle (Polychord Stacks)
Jacob Collier's signature "brightness escalator": shifting through ascending major triads over independent roots to explore stacked Lydian colors ($\sharp 11, 9, 13$).

- **Key**: Modal / Ascending Brightness
- **Chordboard Setup**: Use **Bass Below** split at C3. The left hand drives the low pedal while the right hand steps up the upper-structure major triads (Quality 0)!

| Slot | Sounding Polychord | Left Hand (Bass) | Right Hand (Chordboard Triad) | Sounding Extensions | Brightness Level |
|:---:|:---:|:---:|:---:|:---:|:---:|
| **1** | **Cmaj7** | C2 | C Major (C3 – E3 – G3) | Root, 3, 5 | Base (0) |
| **2** | **D/C** ($C^{\text{add}9\sharp 11}$) | C2 | D Major (D3 – F♯3 – A3) | 9, ♯11, 13 | Bright (+2) |
| **3** | **E/C** ($C^{\text{maj}7\sharp 5}$) | C2 | E Major (E3 – G♯3 – B3) | 3, ♯5, 7 | Augmented (+4) |
| **4** | **F♯/C** ($C^{\sharp 11\flat 9}$) | C2 | F♯ Major (F♯3 – A♯3 – C♯4) | ♯11, 7, ♭9 | Extreme Tension |
| **5** | **G/C** ($C^{\text{maj}9}$) | C2 | G Major (G3 – B3 – D4) | 5, 7, 9 | Open Consonance |
| **6** | **A/C** ($C^{13\flat 9}$) | C2 | A Major (A3 – C♯4 – E4) | 13, ♭9, 3 | Gospel Blue Note |
| **7** | **B/C** ($C^{\text{maj}7\sharp 11\sharp 9}$)| C2 | B Major (B3 – D♯4 – F♯4) | 7, ♯9, ♯11 | Hyper-Lydian Peak |
| **8** | **Cmaj9** | C2 | E Minor (E3 – G3 – B3 – D4) | 3, 5, 7, 9 | Pure Resolution |

* **Voicing**: Wide or Open
* **Chordboard Playing Tip**: Store these into slots 1–8 (`Z X C V B N M ,`). Trigger each step on beat 1 of successive bars while holding a continuous low C bass drone.

---

### 1.4 The "In The Bleak Midwinter" Mediant Shift (Coltrane / Collier Matrix)
Progressing through major-third related chromatic mediants ($C \to E \to A\flat \to C$) using smooth voice-leading paths that shift the harmonic horizon without jarring tonality breaks.

- **Tempo**: 66 BPM | **Feel**: Choral, ambient, suspended

| Slot | Chord | Bass | Voicing Notes (Exact Pitches) | Chordboard Quality | Harmonic Role |
|:---:|:---:|:---:|:---:|:---:|:---:|
| **1** | **Cmaj7** | C2 | E3 – G3 – B3 – E4 | Quality 3 (`Maj7`) on C | Tonic Home |
| **2** | **Emaj7** | E2 | D♯3 – G♯3 – B3 – D♯4 | Quality 3 (`Maj7`) on E | Chromatic Mediant ($\text{III}^{\text{maj}7}$) |
| **3** | **A♭maj7** | A♭1 | C3 – E♭3 – G3 – C4 | Quality 3 (`Maj7`) on A♭ | Submediant ($\flat\text{VI}^{\text{maj}7}$) |
| **4** | **B♭7sus4** | B♭1 | D♭3 – F3 – A♭3 – E♭4 | Quality 2 (`7`) + 4th alteration | Suspended Dominant Lift |
| **5** | **Cmaj9** | C2 | E3 – G3 – B3 – D4 | Quality 3 (`Maj7`) on C | Return resolution |

#### Rhythm & Beat Placements (4-Bar Phrase)
- **Bar 1**: **Cmaj7** on `Beat 1.1` (held 4 beats)
- **Bar 2**: **Emaj7** on `Beat 1.1` (held 4 beats; listen for common-tone B held in both chords)
- **Bar 3**: **A♭maj7** on `Beat 1.1` (held 4 beats; common-tone C held!)
- **Bar 4**: **B♭7sus4** on `Beat 1.1` (beats 1–2) $\to$ **Cmaj9** on `Beat 3.1` (beats 3–4)

---

### 1.5 Neapolitan Minor-Major 9th Pivot (Fm(maj7) – D♭maj7♯11 – Cmaj9)
One of Jacob's most emotional signature cadences: using the minor-major 7th chord ($F\text{m}^{\text{maj}7}$) as a harmonic vortex that resolves outward to Neapolitan major and tonic.

- **Key**: C Minor $\to$ C Major | **Tempo**: 70 BPM

| Slot | Chord | Bass | Voicing Notes (Exact Pitches) | Chordboard Setup | Harmonic Role |
|:---:|:---:|:---:|:---:|:---:|:---:|
| **1** | **Fm(maj7)** | F2 | A♭3 – C4 – E4 – A♭4 | Quality 1 (`Min`) on F + E | The "Hitchcock" / Collier vortex |
| **2** | **Fm7** | F2 | A♭3 – C4 – E♭4 – A♭4 | Quality 4 (`Min7`) on F | Natural 7th falls to minor 7th |
| **3** | **D♭maj7♯11**| D♭2 | F3 – A♭3 – C4 – G4 | Quality 3 (`Maj7`) on D♭ + G | Lydian Neapolitan expansion |
| **4** | **Cmaj9** | C2 | E3 – G3 – B3 – D4 | Quality 3 (`Maj7`) on C | Pure resolution |

---

## 2. Funk, P-Funk & Fusion

### 2.1 The James Brown Dominant 9th Groove ("Sex Machine" / "Cold Sweat")
The definitive funk foundation: tight dominant 9th chords with chromatic half-step slides. On an electric piano or clavinet, this punches through with incredible percussive snap.

- **Key**: D Minor / D Dorian | **Tempo**: 104 BPM | **Swing**: 60% (16th-note funk shuffle)

| Slot | Chord | Bass | Voicing Notes (Exact Pitches) | Chordboard Input | Harmonic Role |
|:---:|:---:|:---:|:---:|:---:|:---:|
| **1** | **D9** | D2 | F♯3 – C4 – E4 – A4 | Quality 2 (`7` on D) + Upper E | Tonic $\text{I}^9$ funk anchor |
| **2** | **E♭9** | E♭2 | G3 – D♭4 – F4 – B♭4 | Quality 2 (`7` on E♭) + Upper F | Chromatic half-step push |
| **3** | **D9** | D2 | F♯3 – C4 – E4 – A4 | Quality 2 (`7` on D) + Upper E | Resolution back to groove |
| **4** | **G9** | G2 | B3 – F4 – A4 – D5 | Quality 2 (`7` on G) + Upper A | Subdominant $\text{IV}^9$ relief |

#### Rhythm & Beat Placements (2-Bar Loop)
- **Bar 1**:
  - `Beat 1.1`: **D9** strong downbeat stab (held for 1 beat)
  - `Beat 2.3` ("and" of 2): **D9** staccato 16th chop
  - `Beat 3.1`: **D9** accented hit
  - `Beat 4.3` ("and" of 4): **E♭9** chromatic push (anticipates bar 2)
- **Bar 2**:
  - `Beat 1.1`: **D9** landing resolution
  - `Beat 2.3` ("and" of 2): **D9** muted ghost chuck
  - `Beat 3.1`: **G9** subdominant change (sustained through beat 4)

```text
Bar 1 (D9 -> Eb9):
Beat: | 1   e   &   a | 2   e   &   a | 3   e   &   a | 4   e   &   a |
Grid: | D9  —   .   . | .   .  (D9) . | D9  —   .   . | .   .   Eb9 — |
Comp: | ↓             |         ↑     | ↓             |         ↑     |

Bar 2 (D9 -> G9):
Beat: | 1   e   &   a | 2   e   &   a | 3   e   &   a | 4   e   &   a |
Grid: | D9  —   .   . | .   .  (D9) . | G9  —   —   — | —   .   .   . |
Comp: | ↓             |         ↑     | ↓             |               |
```

---

### 2.2 Herbie Hancock Headhunters Vamp ("Chameleon")
Hypnotic two-chord Dorian vamp between the minor tonic and the dominant fourth chord over a walking synth bassline.

- **Key**: B♭ Dorian | **Tempo**: 96 BPM | **Swing**: 56%

| Slot | Chord | Bass | Voicing Notes (Exact Pitches) | Chordboard Input | Harmonic Role |
|:---:|:---:|:---:|:---:|:---:|:---:|
| **1** | **B♭m7** | B♭1 | A♭3 – D♭4 – F4 – B♭4 | Quality 4 (`Min7`) on B♭ | $\text{i}^7$ tonic pocket |
| **2** | **B♭m9** | B♭1 | A♭3 – C4 – D♭4 – F4 | Upper Fm over B♭ bass | $\text{i}^9$ extension |
| **3** | **E♭7** | E♭2 | G3 – D♭4 – F4 – B♭4 | Quality 2 (`7`) on E♭ | $\text{IV}^7$ modal lift |
| **4** | **E♭13** | E♭2 | G3 – D♭4 – F4 – C5 | Quality 2 (`7` on E♭) + C | $\text{IV}^{13}$ funk brightness |

#### Rhythm & Beat Placements (2-Bar Loop)
- **Bar 1 (B♭m Vamp)**:
  - `Beat 1.1`: **B♭m7** solid downbeat strike
  - `Beat 2.3` ("and" of 2): **B♭m7** staccato stab
  - `Beat 3.3` ("and" of 3): **B♭m9** syncopated lift (sustained across beat 4)
- **Bar 2 (E♭7 Modal Lift)**:
  - `Beat 1.3` ("and" of 1): **E♭7** syncopated entrance
  - `Beat 3.1`: **E♭13** accented chop
  - `Beat 4.3` ("and" of 4): **B♭m7** push back to start

---

### 2.3 Stevie Wonder Motown Funk ("I Wish" – Verse & Bridge)
Stevie's masterclass in E♭ minor funk: an infectious 2-chord verse vamp followed by a chromatic walking bridge.

- **Key**: E♭ Minor | **Tempo**: 108 BPM | **Swing**: 66% (triplet/shuffle feel)

| Slot | Chord | Bass | Voicing Notes (Exact Pitches) | Chordboard Input | Harmonic Role |
|:---:|:---:|:---:|:---:|:---:|:---:|
| **1** | **E♭m7** | E♭2 | G♭3 – B♭3 – D♭4 – F4 | Quality 4 (`Min7`) on E♭ | $\text{i}^7$ (Main Verse Vamp) |
| **2** | **A♭7** | A♭1 | G♭3 – C4 – E♭4 – G♭4 | Quality 2 (`7`) on A♭ | $\text{IV}^7$ (Main Verse Vamp) |
| **3** | **B♭m7** | B♭1 | A♭3 – D♭4 – F4 – B♭4 | Quality 4 (`Min7`) on B♭ | Bridge bar 1 |
| **4** | **Am7** | A1 | G3 – C4 – E4 – A4 | Quality 4 (`Min7`) on A | Chromatic passing step down |
| **5** | **A♭m7** | A♭1 | G♭3 – B3 – E♭4 – A♭4 | Quality 4 (`Min7`) on A♭ | Bridge target minor |
| **6** | **D♭9** | D♭2 | F3 – B3 – E♭4 – A♭4 | Quality 2 (`7` on D♭) + E♭ | Secondary dominant push |
| **7** | **G♭maj7** | G♭2 | F3 – B♭3 – D♭4 – F4 | Quality 3 (`Maj7`) on G♭ | Relative major release |
| **8** | **B♭7♯9** | B♭1 | A♭3 – D4 – F4 – C♯5 | Quality 2 (`7` on B♭) + C♯ | Turnaround dominant tension |

---

### 2.4 Stevie Wonder Clavinet Stomp ("Superstition")
Syncopated E♭ pentatonic minor riff with crunchy altered dominant chords on the chorus turnaround.

- **Key**: E♭ Minor | **Tempo**: 100 BPM | **Swing**: 58%

| Slot | Chord | Bass | Voicing Notes (Exact Pitches) | Chordboard Input | Harmonic Role |
|:---:|:---:|:---:|:---:|:---:|:---:|
| **1** | **E♭m7** | E♭2 | B♭3 – D♭4 – E♭4 – G♭4 | Quality 4 (`Min7`) on E♭ | Main verse stomp |
| **2** | **E♭m9** | E♭2 | B♭3 – D♭4 – F4 – G♭4 | Quality 4 (`Min7`) + F | Verse melodic lift |
| **3** | **B♭7♯9** | B♭1 | D4 – A♭4 – C5 – C♯5 | Quality 2 (`7` on B♭) + C♯ | Chorus turnaround $\text{V}^7\sharp 9$ |
| **4** | **A7♭5** | A1 | C♯4 – G4 – C5 – E♭5 | Quality 2 (`7`) + ♭5 alteration | Tritone passing chord |
| **5** | **A♭7** | A♭1 | C4 – G♭4 – B♭4 – E♭5 | Quality 2 (`7`) on A♭ | Subdominant tension |
| **6** | **B♭7alt** | B♭1 | D4 – A♭4 – B4 – D5 | Quality 2 (`7`) + B alteration | Final turnaround push |

---

### 2.5 Earth, Wind & Fire Uplifting Funk Anthem ("September")
Descending diatonic step-downs and lush major/minor 7th suspensions with a continuous driving groove.

- **Key**: A Major | **Tempo**: 120 BPM | **Feel**: Straight 16ths with light pocket groove

| Slot | Chord | Bass | Voicing Notes (Exact Pitches) | Chordboard Input | Harmonic Role |
|:---:|:---:|:---:|:---:|:---:|:---:|
| **1** | **Dmaj7** | D2 | F♯3 – A3 – C♯4 – F♯4 | Quality 3 (`Maj7`) on D | $\text{IV}^{\text{maj}7}$ opening warmth |
| **2** | **C♯m7** | C♯2 | E3 – G♯3 – B3 – E4 | Quality 4 (`Min7`) on C♯ | $\text{iii}^7$ step-down |
| **3** | **Bm7** | B1 | D3 – F♯3 – A3 – D4 | Quality 4 (`Min7`) on B | $\text{ii}^7$ arrival |
| **4** | **C♯m7** | C♯2 | E3 – G♯3 – B3 – E4 | Quality 4 (`Min7`) on C♯ | $\text{iii}^7$ bounce back |
| **5** | **F♯m7** | F♯2 | E3 – A3 – C♯4 – E4 | Quality 4 (`Min7`) on F♯ | $\text{vi}^7$ resolution |
| **6** | **Gmaj7** | G2 | F♯3 – B3 – D4 – F♯4 | Quality 3 (`Maj7`) on G | $\flat\text{VII}^{\text{maj}7}$ funk lift |
| **7** | **E7sus4** | E2 | E3 – A3 – B3 – D4 | Quality 2 (`7`) + 4th alteration | $\text{V}^{7\text{sus}4}$ suspension |
| **8** | **E7** | E2 | E3 – G♯3 – B3 – D4 | Quality 2 (`7`) on E | $\text{V}^7$ resolution to loop |

---

### 2.6 Nile Rodgers & Chic Disco Funk ("Good Times")
The blueprint of rhythm comping: three-note tight triads over a driving bassline.

- **Key**: E Minor | **Tempo**: 114 BPM | **Feel**: Straight 16th chucking

| Slot | Chord | Bass | Voicing Notes (Exact Pitches) | Chordboard Input | Harmonic Role |
|:---:|:---:|:---:|:---:|:---:|:---:|
| **1** | **Em7** | E2 | G3 – B3 – D4 – G4 | Quality 4 (`Min7`) on E | $\text{i}^7$ tonic groove |
| **2** | **Emsus4** | E2 | A3 – B3 – D4 – G4 | Quality 1 (`Min`) + 4th alteration | Rhythmic suspension stab |
| **3** | **Em7** | E2 | G3 – B3 – D4 – G4 | Quality 4 (`Min7`) on E | Tonic return |
| **4** | **A7sus4** | A1 | G3 – C4 – D4 – G4 | Quality 2 (`7`) + 4th alteration | Subdominant suspension |
| **5** | **A7** | A1 | G3 – C♯4 – E4 – A4 | Quality 2 (`7`) on A | Subdominant resolution |
| **6** | **B7♯9** | B1 | A3 – D♯4 – F♯4 – D5 | Quality 2 (`7` on B) + D5 (♯9!) | Optional turnaround hook |

> [!NOTE] Theory Correction
> Note that in **B7♯9**, the augmented ninth is **D natural** ($D5$), 3 semitones above B. (The note $C5$ is a minor ninth $\flat 9$).

---

### 2.7 Prince Minneapolis Sound ("Kiss" / "Controversy")
Stripped-down, ultra-clean funk characterized by muted dominant 9th chops with dry, percussive envelopes.

- **Key**: A Funk Blues | **Tempo**: 114 BPM | **Feel**: Ultra-tight dry staccato

| Slot | Chord | Bass | Voicing Notes (Exact Pitches) | Chordboard Input | Harmonic Role |
|:---:|:---:|:---:|:---:|:---:|:---:|
| **1** | **A9** | A1 | G3 – C♯4 – E4 – B4 | Quality 2 (`7` on A) + B | Tonic $\text{I}^9$ (bars 1–4) |
| **2** | **D9** | D2 | F♯3 – C4 – E4 – B4 | Quality 2 (`7` on D) + E | Subdominant $\text{IV}^9$ (bars 5–6) |
| **3** | **A9** | A1 | G3 – C♯4 – E4 – B4 | Quality 2 (`7` on A) + B | Tonic return (bars 7–8) |
| **4** | **E9** | E2 | G♯3 – D4 – F♯4 – C♯5 | Quality 2 (`7` on E) + F♯ | Dominant turnaround $\text{V}^9$ (bar 9) |
| **5** | **D9** | D2 | F♯3 – C4 – E4 – B4 | Quality 2 (`7` on D) + E | Step-down $\text{IV}^9$ (bar 10) |
| **6** | **A9** | A1 | G3 – C♯4 – E4 – B4 | Quality 2 (`7` on A) + B | Final hit & turnaround (bars 11–12) |

---

### 2.8 Vulfpeck / Cory Wong Modern Pocket Nu-Funk ("1612")
Tight, sophisticated neo-funk progression utilizing colorful altered dominant tensions and major ninth resolutions.

- **Key**: C Major | **Tempo**: 94 BPM | **Swing**: 58%

| Slot | Chord | Bass | Voicing Notes (Exact Pitches) | Chordboard Input | Harmonic Role |
|:---:|:---:|:---:|:---:|:---:|:---:|
| **1** | **Dm9** | D2 | F3 – A3 – C4 – E4 | Upper Fmaj7 over D bass | $\text{ii}^9$ funk anchor |
| **2** | **G13** | G1 | F3 – B3 – E4 – A4 | Quality 2 (`7` on G) + E | $\text{V}^{13}$ dominant swing |
| **3** | **Cmaj9** | C2 | E3 – G3 – B3 – D4 | Upper Em7 over C bass | $\text{I}^{\text{maj}9}$ tonic satisfaction |
| **4** | **A7♯5♯9** | A1 | G3 – C♯4 – F4 – C5 | Quality 2 (`7`) + #5 + C | $\text{VI}^{7\text{alt}}$ turnaround back to ii |

---

### 2.9 P-Funk & G-Funk Mothership Glide ("Give Up the Funk" / "Flash Light")
Heavy Parliament / George Clinton / Dr. Dre West Coast sound: warm minor seventh vamps over heavy analog Moog bass.

- **Key**: B♭ Minor | **Tempo**: 100 BPM | **Feel**: Laid-back pocket

| Slot | Chord | Bass | Voicing Notes (Exact Pitches) | Chordboard Input | Harmonic Role |
|:---:|:---:|:---:|:---:|:---:|:---:|
| **1** | **B♭m7** | B♭1 | D♭3 – F3 – A♭3 – D♭4 | Quality 4 (`Min7`) on B♭ | $\text{i}^7$ Mothership anchor |
| **2** | **E♭m7** | E♭2 | G♭3 – B♭3 – D♭4 – G♭4 | Quality 4 (`Min7`) on E♭ | $\text{iv}^7$ subdominant roll |
| **3** | **Fm7** | F2 | A♭3 – C4 – E♭4 – A♭4 | Quality 4 (`Min7`) on F | $\text{v}^7$ dominant step-up |
| **4** | **B♭m7** | B♭1 | D♭3 – F3 – A♭3 – D♭4 | Quality 4 (`Min7`) on B♭ | $\text{i}^7$ resolution |

---

### 2.10 Tower of Power Oakland Grease ("What Is Hip?")
Aggressive, hyper-syncopated Bay Area funk with altered dominant tensions and tritone horn stabs.

- **Key**: E Funk | **Tempo**: 108 BPM | **Feel**: Linear 16th funk

| Slot | Chord | Bass | Voicing Notes (Exact Pitches) | Chordboard Input | Harmonic Role |
|:---:|:---:|:---:|:---:|:---:|:---:|
| **1** | **E7♯9** | E2 | G♯3 – D4 – G4 – D5 | Quality 2 (`7` on E) + G | "Hendrix" / Oakland tonic stab |
| **2** | **E7♭9** | E2 | G♯3 – D4 – F4 – B4 | Quality 2 (`7` on E) + F | Tension variant (♭9) |
| **3** | **B♭13** | B♭1 | A♭3 – D4 – G4 – C5 | Quality 2 (`7` on B♭) + G | Tritone passing substitution |
| **4** | **A13** | A1 | G3 – C♯4 – F♯4 – B4 | Quality 2 (`7` on A) + F♯ | $\text{IV}^{13}$ turnaround hit |

---

## 3. Neo-Soul & Warm R&B

### 3.1 The "Just Friends" D'Angelo Groove
Lush, laid-back 2-5-1 with a subdominant turnaround. Perfect with Wurlitzer tremolo or a warm Rhodes.

- **Key**: F Major | **Tempo**: 82 BPM | **Swing**: 62% (laid-back / behind the beat)

| Slot | Chord | Bass | Voicing Notes (Exact Pitches) | Chordboard Input | Harmonic Role |
|:---:|:---:|:---:|:---:|:---:|:---:|
| **1** | **Gm9** | G2 | F3 – B♭3 – D4 – A4 | Upper B♭maj7 over G bass | $\text{ii}^9$ intro pocket |
| **2** | **C13** | C2 | E3 – B♭3 – D4 – A4 | Quality 2 (`7` on C) + A | $\text{V}^{13}$ silky dominant |
| **3** | **Fmaj9** | F2 | E3 – A3 – C4 – G4 | Upper Am7 over F bass | $\text{I}^{\text{maj}9}$ tonic home |
| **4** | **B♭maj7** | B♭1 | D3 – A3 – C4 – F4 | Quality 3 (`Maj7`) on B♭ | $\text{IV}^{\text{maj}7}$ subdominant warmth |

---

### 3.2 Smooth Neo-Soul Stepper
Classic mid-tempo groove found across Musiq Soulchild, Jill Scott, and Tom Misch tracks.

- **Key**: E♭ Major | **Tempo**: 92 BPM | **Swing**: 58%

| Slot | Chord | Bass | Voicing Notes (Exact Pitches) | Chordboard Input | Harmonic Role |
|:---:|:---:|:---:|:---:|:---:|:---:|
| **1** | **Cm9** | C2 | B♭3 – E♭4 – G4 – D5 | Upper E♭maj7 over C bass | $\text{vi}^9$ minor emotional start |
| **2** | **Fm9** | F2 | A♭3 – C4 – E♭4 – G4 | Upper A♭maj7 over F bass | $\text{ii}^9$ pre-dominant |
| **3** | **B♭9sus4** | B♭1 | A♭3 – C4 – E♭4 – F4 | Quality 2 (`7`) + 4th alteration | $\text{V}^{11}$ suspended dominant |
| **4** | **E♭maj9** | E♭2 | G3 – B♭3 – D4 – F4 | Upper Gm7 over E♭ bass | $\text{I}^{\text{maj}9}$ rich resolution |

---

### 3.3 Childish Gambino / 70s Soul Glide ("Redbone")
Classic Jaco Pastorius / Bootsy Collins inspired minor modal vamps.

- **Key**: D Minor (Dorian feel) | **Tempo**: 82 BPM | **Feel**: Smooth, sustained

| Slot | Chord | Bass | Voicing Notes (Exact Pitches) | Chordboard Input | Harmonic Role |
|:---:|:---:|:---:|:---:|:---:|:---:|
| **1** | **Dm9** | D2 | F3 – A3 – C4 – E4 | Upper Fmaj7 over D bass | $\text{i}^9$ tonic vibe |
| **2** | **Gm9** | G2 | B♭3 – D4 – F4 – A4 | Upper B♭maj7 over G bass | $\text{iv}^9$ subdominant float |
| **3** | **Am7** | A2 | C4 – E4 – G4 – C5 | Quality 4 (`Min7`) on A | $\text{v}^7$ crest of progression |
| **4** | **Gm9** | G2 | B♭3 – D4 – F4 – A4 | Upper B♭maj7 over G bass | $\text{iv}^9$ return glide |

---

### 3.4 Erykah Badu & J Dilla Chromatic Passing Harmony
The chromatic diminished passing chord creates an unmistakable hip-hop soul push into the minor chord.

- **Key**: D♭ Major | **Tempo**: 78 BPM | **Swing**: 64% (drunk/dilla swing)

| Slot | Chord | Bass | Voicing Notes (Exact Pitches) | Chordboard Input | Harmonic Role |
|:---:|:---:|:---:|:---:|:---:|:---:|
| **1** | **D♭maj9** | D♭2 | F3 – A♭3 – C4 – E♭4 | Upper Fm7 over D♭ bass | $\text{I}^{\text{maj}9}$ opening |
| **2** | **Fm7** | F2 | A♭3 – C4 – E♭4 – A♭4 | Quality 4 (`Min7`) on F | $\text{iii}^7$ step |
| **3** | **G♭maj7** | G♭2 | B♭3 – D♭4 – F4 – B♭4 | Quality 3 (`Maj7`) on G♭ | $\text{IV}^{\text{maj}7}$ subdominant |
| **4** | **Gdim7** | G2 | B♭3 – D♭4 – E4 – G4 | Quality 9 (`Dim7`) on G | $\sharp\text{IV}^{\circ 7}$ passing chord |
| **5** | **A♭7** | A♭2 | C4 – E♭4 – G♭4 – C5 | Quality 2 (`7`) on A♭ | $\text{V}^7$ dominant push |

---

### 3.5 H.E.R. & Daniel Caesar Modern Acoustic R&B ("Best Part")
Simple, incredibly resonant major 7th and minor 7th shapes that sound gorgeous on Wurlitzer or Rhodes.

- **Key**: D Major | **Tempo**: 76 BPM | **Feel**: Gentle acoustic ballad

| Slot | Chord | Bass | Voicing Notes (Exact Pitches) | Chordboard Input | Harmonic Role |
|:---:|:---:|:---:|:---:|:---:|:---:|
| **1** | **Dmaj7** | D2 | F♯3 – A3 – C♯4 – F♯4 | Quality 3 (`Maj7`) on D | $\text{I}^{\text{maj}7}$ warm root |
| **2** | **Am7** | A2 | G3 – C4 – E4 – A4 | Quality 4 (`Min7`) on A | $\text{v}^7$ modal drop |
| **3** | **Gmaj7** | G2 | F♯3 – B3 – D4 – G4 | Quality 3 (`Maj7`) on G | $\text{IV}^{\text{maj}7}$ release |
| **4** | **B♭maj7** | B♭1 | F3 – A3 – D4 – F4 | Quality 3 (`Maj7`) on B♭ | $\flat\text{VI}^{\text{maj}7}$ chromatic lift |

---

## 4. Lo-Fi Hip-Hop & Chillhop

### 4.1 Nostalgic Rain / Rainy Afternoon
The immortal jazz-hop loop used in countless ChilledCow / Lofi Girl beats.

- **Key**: C Major | **Tempo**: 75 BPM | **Swing**: 56%

| Slot | Chord | Bass | Voicing Notes (Exact Pitches) | Chordboard Input | Harmonic Role |
|:---:|:---:|:---:|:---:|:---:|:---:|
| **1** | **Dm7** | D2 | F3 – A3 – C4 – F4 | Quality 4 (`Min7`) on D | $\text{ii}^7$ opening statement |
| **2** | **G7♭9** | G2 | F3 – B3 – D4 – A♭4 | Quality 2 (`7` on G) + ♭9 alt | $\text{V}^7\flat 9$ nostalgic tension |
| **3** | **Cmaj7** | C2 | E3 – G3 – B3 – E4 | Quality 3 (`Maj7`) on C | $\text{I}^{\text{maj}7}$ resolution |
| **4** | **Am7** | A2 | E3 – G3 – C4 – E4 | Quality 4 (`Min7`) on A | $\text{vi}^7$ melancholic turnaround |

---

### 4.2 Melancholy Sunset
A minor-to-major descending progression with deep bittersweet emotion.

- **Key**: C Major / A Minor | **Tempo**: 72 BPM | **Feel**: Slow, relaxed

| Slot | Chord | Bass | Voicing Notes (Exact Pitches) | Chordboard Input | Harmonic Role |
|:---:|:---:|:---:|:---:|:---:|:---:|
| **1** | **Fmaj7** | F2 | E3 – A3 – C4 – E4 | Quality 3 (`Maj7`) on F | $\text{IV}^{\text{maj}7}$ |
| **2** | **Em7** | E2 | D3 – G3 – B3 – E4 | Quality 4 (`Min7`) on E | $\text{iii}^7$ step down |
| **3** | **Dm9** | D2 | C3 – F3 – A3 – E4 | Upper Fmaj7 over D bass | $\text{ii}^9$ rich suspension |
| **4** | **Am7** | A2 | C3 – E3 – G3 – C4 | Quality 4 (`Min7`) on A | $\text{vi}^7$ minor home |

---

### 4.3 Midnight Rhodes Drift
Darker, altered chords featuring the moody $\flat\text{VI}$ to $\text{V}^{\text{alt}}$ resolution.

- **Key**: C Minor | **Tempo**: 70 BPM

| Slot | Chord | Bass | Voicing Notes (Exact Pitches) | Chordboard Input | Harmonic Role |
|:---:|:---:|:---:|:---:|:---:|:---:|
| **1** | **A♭maj7** | A♭2 | G3 – C4 – E♭4 – G4 | Quality 3 (`Maj7`) on A♭ | $\flat\text{VI}^{\text{maj}7}$ dark warmth |
| **2** | **G7♭13** | G2 | F3 – B3 – E♭4 – A♭4 | Quality 2 (`7` on G) + ♭13 alt | $\text{V}^7\flat 13$ altered tension |
| **3** | **Cm9** | C2 | E♭3 – B♭3 – D4 – G4 | Upper E♭maj7 over C bass | $\text{i}^9$ deep tonic |
| **4** | **Fm7** | F2 | E♭3 – A♭3 – C4 – F4 | Quality 4 (`Min7`) on F | $\text{iv}^7$ subdominant roll |

---

### 4.4 Coffee Shop Modal Chill (i9 – IV13 Vamp)
Minimalist two-chord vamp that loops indefinitely without fatigue.

- **Key**: G Dorian | **Tempo**: 80 BPM

| Slot | Chord | Bass | Voicing Notes (Exact Pitches) | Chordboard Input | Harmonic Role |
|:---:|:---:|:---:|:---:|:---:|:---:|
| **1** | **Gm9** | G2 | F3 – B♭3 – D4 – A4 | Upper B♭maj7 over G bass | $\text{i}^9$ tonic chill |
| **2** | **C13** | C2 | E3 – B♭3 – D4 – A4 | Quality 2 (`7` on C) + A | $\text{IV}^{13}$ jazzy lift |

---

## 5. Japanese City Pop & Shibuya-kei

### 5.1 The Royal Road ("Oudou" Progression)
The legendary Japanese pop progression ($\text{IV}^{\text{maj}7} - \text{V}^7 - \text{iii}^7 - \text{vi}$), heard in countless anime themes, City Pop anthems, and modern J-Pop hits.

- **Key**: F Major | **Tempo**: 118 BPM | **Feel**: Energetic 80s driving pop

| Slot | Chord | Bass | Voicing Notes (Exact Pitches) | Chordboard Input | Harmonic Role |
|:---:|:---:|:---:|:---:|:---:|:---:|
| **1** | **B♭maj7** | B♭1 | A3 – D4 – F4 – B♭4 | Quality 3 (`Maj7`) on B♭ | $\text{IV}^{\text{maj}7}$ emotional start |
| **2** | **C7** | C2 | B♭3 – E4 – G4 – C5 | Quality 2 (`7`) on C | $\text{V}^7$ driving lift |
| **3** | **Am7** | A2 | G3 – C4 – E4 – A4 | Quality 4 (`Min7`) on A | $\text{iii}^7$ bittersweet peak |
| **4** | **Dm7** | D2 | F3 – A3 – C4 – F4 | Quality 4 (`Min7`) on D | $\text{vi}^7$ satisfying arrival |

---

### 5.2 Tatsuro Yamashita / Mariya Takeuchi ("Plastic Love" 8-Bar Loop)
The holy grail of City Pop funk. Extended dominant suspensions and half-diminished walkups that define the 1980s Tokyo sound.

- **Key**: D Minor | **Tempo**: 108 BPM | **Feel**: Chic-inspired disco-funk groove

| Slot | Chord | Bass | Voicing Notes (Exact Pitches) | Chordboard Input | Harmonic Role |
|:---:|:---:|:---:|:---:|:---:|:---:|
| **1** | **Dm7** | D2 | F3 – A3 – C4 – F4 | Quality 4 (`Min7`) on D | $\text{i}^7$ |
| **2** | **Gm9** | G2 | B♭3 – D4 – F4 – A4 | Upper B♭maj7 over G bass | $\text{iv}^9$ |
| **3** | **C7** | C2 | B♭3 – E4 – G4 – C5 | Quality 2 (`7`) on C | $\flat\text{VII}^7$ |
| **4** | **Fmaj7** | F2 | A3 – C4 – E4 – A4 | Quality 3 (`Maj7`) on F | $\text{III}^{\text{maj}7}$ |
| **5** | **B♭maj7** | B♭1 | A3 – D4 – F4 – B♭4 | Quality 3 (`Maj7`) on B♭ | $\flat\text{VI}^{\text{maj}7}$ |
| **6** | **Em7♭5** | E2 | G3 – B♭3 – D4 – G4 | Quality 10 (`Half dim`) on E | $\text{ii}^{\varnothing 7}$ |
| **7** | **A7♭9** | A1 | G3 – C♯4 – E4 – B♭4 | Quality 2 (`7` on A) + ♭9 alt | $\text{V}^7\flat 9$ |
| **8** | **Dm9** | D2 | F3 – A3 – C4 – E4 | Upper Fmaj7 over D bass | $\text{i}^9$ |

---

### 5.3 Sparkle Breezy Promenade
Bright, breezy summer vibes with secondary dominants turning into major extensions.

- **Key**: A Major | **Tempo**: 118 BPM

| Slot | Chord | Bass | Voicing Notes (Exact Pitches) | Chordboard Input | Harmonic Role |
|:---:|:---:|:---:|:---:|:---:|:---:|
| **1** | **Amaj9** | A2 | G♯3 – C♯4 – E4 – B4 | Upper C♯m7 over A bass | $\text{I}^{\text{maj}9}$ |
| **2** | **F♯7** | F♯2 | E3 – A♯3 – C♯4 – F♯4 | Quality 2 (`7`) on F♯ | $\text{VI}^7$ secondary dominant |
| **3** | **Bm9** | B2 | A3 – D4 – F♯4 – C♯5 | Upper Dmaj7 over B bass | $\text{ii}^9$ target |
| **4** | **E7sus4** | E2 | A3 – B3 – D4 – E4 | Quality 2 (`7`) + 4th alteration | $\text{V}^{7\text{sus}4}$ suspension |
| **5** | **E7** | E2 | G♯3 – B3 – D4 – E4 | Quality 2 (`7`) on E | $\text{V}^7$ resolution |

---

## 6. French House, Nu-Disco & Filter Grooves

### 6.1 Daft / Roulé French Touch Stabs
The classic repetitive French touch sample loop. Chunky minor chords with high energy.

- **Key**: G Minor | **Tempo**: 124 BPM | **Feel**: Four-on-the-floor house

| Slot | Chord | Bass | Voicing Notes (Exact Pitches) | Chordboard Input | Harmonic Role |
|:---:|:---:|:---:|:---:|:---:|:---:|
| **1** | **Gm7** | G2 | F3 – B♭3 – D4 – G4 | Quality 4 (`Min7`) on G | $\text{i}^7$ punchy stab |
| **2** | **F7** | F2 | E♭3 – A3 – C4 – F4 | Quality 2 (`7`) on F | $\flat\text{VII}^7$ step-down |
| **3** | **E♭maj7** | E♭2 | D3 – G3 – B♭3 – E♭4 | Quality 3 (`Maj7`) on E♭ | $\flat\text{VI}^{\text{maj}7}$ harmonic drop |
| **4** | **F7** | F2 | E♭3 – A3 – C4 – F4 | Quality 2 (`7`) on F | $\flat\text{VII}^7$ return step |

---

### 6.2 Upbeat Filter-Swept Soul
High-register major 9ths and minor 9ths that sparkle through a low-pass filter sweep.

- **Key**: B♭ Major | **Tempo**: 120 BPM

| Slot | Chord | Bass | Voicing Notes (Exact Pitches) | Chordboard Input | Harmonic Role |
|:---:|:---:|:---:|:---:|:---:|:---:|
| **1** | **E♭maj9** | E♭2 | G3 – B♭3 – D4 – F4 | Upper Gm7 over E♭ bass | $\text{IV}^{\text{maj}9}$ shimmering high chord |
| **2** | **Dm7** | D2 | F3 – A3 – C4 – F4 | Quality 4 (`Min7`) on D | $\text{iii}^7$ passing chord |
| **3** | **Gm9** | G2 | F3 – B♭3 – D4 – A4 | Upper B♭maj7 over G bass | $\text{vi}^9$ rich body |
| **4** | **B♭/F** | F2 | F3 – B♭3 – D4 – F4 | Quality 0 (`Major`) on B♭ over F bass | $\text{I}/\text{V}$ driving pedal |

---

### 6.3 90s Deep House / Garage Vamp
Moody, deep, hypnotic chords made famous by Todd Edwards, Kerri Chandler, and Masters at Work.

- **Key**: E Minor | **Tempo**: 124 BPM

| Slot | Chord | Bass | Voicing Notes (Exact Pitches) | Chordboard Input | Harmonic Role |
|:---:|:---:|:---:|:---:|:---:|:---:|
| **1** | **Em9** | E2 | G3 – B3 – D4 – F♯4 | Upper Gmaj7 over E bass | $\text{i}^9$ deep club tonic |
| **2** | **Bm7** | B1 | F♯3 – A3 – D4 – F♯4 | Quality 4 (`Min7`) on B | $\text{v}^7$ atmospheric step |
| **3** | **Cmaj7** | C2 | G3 – B3 – E4 – G4 | Quality 3 (`Maj7`) on C | $\flat\text{VI}^{\text{maj}7}$ uplifting surge |
| **4** | **D7** | D2 | F♯3 – C4 – D4 – A4 | Quality 2 (`7`) on D | $\flat\text{VII}^7$ driving turnaround |

---

## 7. Gospel & Modern R&B Passing Harmony

### 7.1 Sunday Morning 2-5-1 with Secondary Dominants (12/8 Compound Meter)
A richly textured gospel progression with chromatic tension, secondary dominants, and resolutions.

- **Key**: A♭ Major | **Tempo**: 68 BPM (Dotted quarter note) | **Meter**: $12/8$ Slow Gospel Ballad

| Slot | Chord | Bass | Voicing Notes (Exact Pitches) | Chordboard Input | Harmonic Role |
|:---:|:---:|:---:|:---:|:---:|:---:|
| **1** | **A♭maj9** | A♭1 | G3 – C4 – E♭4 – B♭4 | Upper Cm7 over A♭ bass | $\text{I}^{\text{maj}9}$ home tonic |
| **2** | **F7♭9** | F2 | E♭3 – A3 – C4 – G♭4 | Quality 2 (`7` on F) + ♭9 alt | $\text{VI}^7\flat 9$ secondary dominant |
| **3** | **B♭m9** | B♭1 | A♭3 – D♭4 – F4 – C5 | Upper D♭maj7 over B♭ bass | $\text{ii}^9$ rich target minor |
| **4** | **E♭7♭9** | E♭2 | D♭3 – G3 – B♭3 – E4 | Quality 2 (`7` on E♭) + ♭9 alt | $\text{V}^7\flat 9$ gospel turnaround |
| **5** | **Cm7** | C2 | B♭3 – E♭4 – G4 – C5 | Quality 4 (`Min7`) on C | $\text{iii}^7$ step-up |
| **6** | **F7** | F2 | A3 – C4 – E♭4 – A4 | Quality 2 (`7`) on F | $\text{VI}^7$ dominant cycle |

#### Compound $12/8$ Triplet Rhythm Grid
Each bar has 4 dotted-quarter main beats divided into 3 eighth-note pulses:
```text
Bar 1 (Abmaj9 -> F7b9):
Beat: | 1  &  a | 2  &  a | 3  &  a | 4  &  a |
Grid: | Abmaj9——| ————————| F7b9————| ————————|
Comp: | ↓       |         | ↓       |         |

Bar 2 (Bbm9 -> Eb7b9):
Beat: | 1  &  a | 2  &  a | 3  &  a | 4  &  a |
Grid: | Bbm9————| ————————| Eb7b9———| ————————|
Comp: | ↓       |         | ↓       |         |
```

---

### 7.2 Tritone Substitution Walkdown (Chromatic Bassline)
Sophisticated chromatic descending basslines and tritone substitutions.

- **Key**: C Major | **Tempo**: 80 BPM

| Slot | Chord | Bass | Voicing Notes (Exact Pitches) | Chordboard Input | Harmonic Role |
|:---:|:---:|:---:|:---:|:---:|:---:|
| **1** | **Cmaj9** | C2 | E3 – G3 – B3 – D4 | Upper Em7 over C bass | $\text{I}^{\text{maj}9}$ |
| **2** | **E♭7** | E♭2 | G3 – D♭4 – E♭4 – B♭4 | Quality 2 (`7`) on E♭ | $\flat\text{III}^7$ chromatic approach |
| **3** | **Dm9** | D2 | F3 – A3 – C4 – E4 | Upper Fmaj7 over D bass | $\text{ii}^9$ target minor |
| **4** | **D♭7** | D♭2 | F3 – B3 – E♭4 – A♭4 | Quality 2 (`7`) on D♭ | $\flat\text{II}^7$ tritone sub for V7 |

---

## 8. Indie Pop, Dream Pop & Ambient

### 8.1 Dreamy Bedroom Pop Haze (Mac DeMarco / Boy Pablo)
Bright, jangly, bittersweet indie pop progression.

- **Key**: G Major | **Tempo**: 92 BPM | **Feel**: Jangly, slightly lazy pocket

| Slot | Chord | Bass | Voicing Notes (Exact Pitches) | Chordboard Input | Harmonic Role |
|:---:|:---:|:---:|:---:|:---:|:---:|
| **1** | **Cmaj7** | C2 | E3 – G3 – B3 – E4 | Quality 3 (`Maj7`) on C | $\text{IV}^{\text{maj}7}$ breezy opening |
| **2** | **Gadd9** | G2 | D3 – G3 – A3 – B3 | Quality 0 (`Major`) + A | $\text{I}^{\text{add}9}$ bright chime |
| **3** | **Em7** | E2 | D3 – G3 – B3 – E4 | Quality 4 (`Min7`) on E | $\text{vi}^7$ warm melancholy |
| **4** | **Dsus4** | D2 | D3 – G3 – A3 – D4 | Quality 0 (`Major`) + 4th alt | $\text{V}^{\text{sus}4}$ floating delay |

---

### 8.2 Shoegaze / Cocteau Bloom (Suspended Wall of Sound)
Suspended, floating chords that avoid resolving quickly, creating a lush wall of sound.

- **Key**: D Major | **Tempo**: 96 BPM

| Slot | Chord | Bass | Voicing Notes (Exact Pitches) | Chordboard Input | Harmonic Role |
|:---:|:---:|:---:|:---:|:---:|:---:|
| **1** | **Dmaj7** | D2 | F♯3 – A3 – C♯4 – F♯4 | Quality 3 (`Maj7`) on D | $\text{I}^{\text{maj}7}$ floating root |
| **2** | **Gmaj7** | G2 | F♯3 – B3 – D4 – G4 | Quality 3 (`Maj7`) on G | $\text{IV}^{\text{maj}7}$ shimmering lift |
| **3** | **Bm9** | B1 | F♯3 – A3 – C♯4 – D4 | Upper Dmaj7 over B bass | $\text{vi}^9$ expansive body |
| **4** | **Asus4** | A1 | E3 – A3 – D4 – E4 | Quality 0 (`Major`) + 4th alt | $\text{V}^{\text{sus}4}$ unresolved air |

---

### 8.3 Cinematic Melancholy (Film Score Ambient Minor)
Expansive, epic minor progression common in modern film scores and emotive ambient music.

- **Key**: A Minor | **Tempo**: 65 BPM | **Feel**: Free, sustained, blooming

| Slot | Chord | Bass | Voicing Notes (Exact Pitches) | Chordboard Input | Harmonic Role |
|:---:|:---:|:---:|:---:|:---:|:---:|
| **1** | **Am9** | A1 | E3 – G3 – B3 – C4 | Upper Cmaj7 over A bass | $\text{i}^9$ moody anchor |
| **2** | **Fmaj7** | F2 | E3 – A3 – C4 – E4 | Quality 3 (`Maj7`) on F | $\flat\text{VI}^{\text{maj}7}$ expansive swell |
| **3** | **Cmaj7** | C2 | E3 – G3 – B3 – C4 | Quality 3 (`Maj7`) on C | $\text{III}^{\text{maj}7}$ cinematic light |
| **4** | **G7sus4** | G2 | D3 – F3 – G3 – C4 | Quality 2 (`7`) + 4th alt | $\flat\text{VII}^{7\text{sus}4}$ unresolved breath |

---

## 9. Chordboard Master Configuration Matrix

| Style / Genre | Voicing Mode | Strum / Playback Mode | Split Setup | Recommended Host / Synth Match |
|:---|:---|:---|:---|:---|
| **Jacob Collier Negative** | *Drop 2* or *Open* | **Manual Strum** or **Sustained** | **Voice-Leading**: *Nearest*<br>**Bass Below**: C2 | Rich Acoustic Piano, Warm Upright, Lush Wurli |
| **Collier Hyper-Lydian** | *Wide* | **Arpeggiator Once** (8 Strings) | **Bass Below**: C3 (Holds pedal)<br>**Upper**: Major triads | Shimmer Reverb, Flute/Whistle synths, OpenWurli |
| **P-Funk / Funk Stabs** | *Drop 2* | **Manual Strum** (Short Note Length ~140ms) | **Bass Below**: C3 (Auto)<br>**Play Always**: Off | OpenWurli (Hard reed/drive), Clavinet, Analog Bass |
| **Stevie Motown Funk** | *Drop 2* | **Manual Strum** (CC1 Mod Wheel mapped) | **Bass Below**: C2<br>**Play Always**: On | Clavinet / Wurlitzer with auto-wah / phaser |
| **Neo-Soul & Warm R&B** | *Drop 2* or *Open* | **Chord on Select** or **Hold Chord** | **Bass Below**: C3<br>**Key Split**: C4 | OpenWurli with smooth Tremolo, Rhodes Mark I/II |
| **Lo-Fi Hip-Hop** | *Close* or *Drop 2* | **Arpeggiator Once** (5 Strings) | **Bass Below**: Auto | Dusty Rhodes, Tape Flutter, Bitcrusher / Vinyl |
| **City Pop & Nu-Disco** | *Drop 2* | **Arpeggiator Loop** (Rate 1/16, Gate 65%) | **Bass Below**: C2 | Bright FM E-Piano (DX7), Synth Brass, Disco Bass |
| **Ambient & Dream Pop** | *Wide* | **Arpeggiator Loop** (Direction Up/Down, 2 Oct) | **Key Split**: On<br>**Affect Chords**: On | Lush Reverb / Tape Delay, Soft Piano, Pad Synth |
