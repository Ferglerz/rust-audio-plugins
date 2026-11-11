//! Read-only REAPER track metadata, shared by the CLAP and VST3 adapters.
//! API signatures: https://www.reaper.fm/sdk/reascript/reascripthelp.html

use std::ffi::{c_char, c_void, CStr};

use crate::context::track::{HardwareInput, HostTrackHandler, HostTrackUpdate};

/// The track pointer and API resolver must belong to the live host context.
/// Call only on the host control thread. No pointers are retained.
pub unsafe fn publish(
    track: *mut c_void,
    api: impl Fn(&CStr) -> *mut c_void,
    handler: &HostTrackHandler,
) {
    if track.is_null() {
        return;
    }

    let get_track_name = api(c"GetTrackName");
    if !get_track_name.is_null() {
        let get: unsafe extern "C" fn(*mut c_void, *mut c_char, i32) -> bool =
            std::mem::transmute(get_track_name);
        let mut name = [0u8; 512];
        if get(track, name.as_mut_ptr().cast(), name.len() as i32) {
            let end = name.iter().position(|&c| c == 0).unwrap_or(name.len());
            handler(HostTrackUpdate::Name(Some(
                String::from_utf8_lossy(&name[..end]).into_owned(),
            )));
        }
    }

    let get_track_value = api(c"GetMediaTrackInfo_Value");
    if get_track_value.is_null() {
        return;
    }
    let get: unsafe extern "C" fn(*mut c_void, *const c_char) -> f64 =
        std::mem::transmute(get_track_value);
    let rec_input = get(track, c"I_RECINPUT".as_ptr());
    // Audio input bits 0–9 encode the starting channel. 512+ is virtual
    // routing, and bit 12 denotes MIDI. Neither identifies hardware.
    let input = if (0.0..4096.0).contains(&rec_input) && rec_input.fract() == 0.0 {
        let code = rec_input as usize;
        let first = code & 1023;
        if first < 512 {
            let count = if code & 2048 != 0 {
                (get(track, c"I_NCHAN".as_ptr()) as usize).clamp(1, 128)
            } else if code & 1024 != 0 {
                2
            } else {
                1
            };
            let last = first + count - 1;
            let mut label = if count == 1 {
                format!("Input {}", first + 1)
            } else {
                format!("Inputs {}–{}", first + 1, last + 1)
            };
            let input_name = api(c"GetInputChannelName");
            if !input_name.is_null() {
                let name: unsafe extern "C" fn(i32) -> *const c_char =
                    std::mem::transmute(input_name);
                for index in first..=last.min(first + 1).min(511) {
                    let value = name(index as i32);
                    if !value.is_null() {
                        let value = CStr::from_ptr(value).to_string_lossy();
                        if !value.trim().is_empty() {
                            label.push_str(if index == first { " · " } else { " / " });
                            label.push_str(value.trim());
                        }
                    }
                }
            }
            Some(HardwareInput {
                first_channel: first,
                label,
            })
        } else {
            None
        }
    } else {
        None
    };
    handler(HostTrackUpdate::HardwareInput(input));
}
