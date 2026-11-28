extern crate vst3_sys as vst3_com;
use libloading::Library;
use std::{
    ffi::{c_char, c_void, CStr},
    mem::{transmute, zeroed},
    ptr::null_mut,
};
use vst3_sys::{base::*, utils::StaticVstPtr, vst::*, ComInterface, VstPtr, VST3};

const BLOCK: usize = 128;
const RATE: f64 = 48000.;
const MODE: i32 = ProcessModes::kOffline as i32;

#[VST3(implements(IEventList))]
struct Events {
    entries: Vec<Event>,
}
impl IEventList for Events {
    unsafe fn get_event_count(&self) -> i32 {
        self.entries.len() as i32
    }
    unsafe fn get_event(&self, index: i32, out: *mut Event) -> tresult {
        let Some(event) = self.entries.get(index as usize) else {
            return kInvalidArgument;
        };
        *out = *event;
        kResultOk
    }
    unsafe fn add_event(&self, _event: *mut Event) -> tresult {
        kNotImplemented
    }
}

#[VST3(implements(IParamValueQueue))]
struct Queue {
    id: u32,
    offset: i32,
    value: f64,
}
impl IParamValueQueue for Queue {
    unsafe fn get_parameter_id(&self) -> u32 {
        self.id
    }
    unsafe fn get_point_count(&self) -> i32 {
        1
    }
    unsafe fn get_point(&self, index: i32, offset: *mut i32, value: *mut f64) -> tresult {
        if index != 0 {
            return kInvalidArgument;
        }
        *offset = self.offset;
        *value = self.value;
        kResultOk
    }
    unsafe fn add_point(&self, _: i32, _: f64, _: *mut i32) -> tresult {
        kNotImplemented
    }
}

#[VST3(implements(IParameterChanges))]
struct Changes {
    entries: Vec<VstPtr<dyn IParamValueQueue>>,
}
impl IParameterChanges for Changes {
    unsafe fn get_parameter_count(&self) -> i32 {
        self.entries.len() as i32
    }
    unsafe fn get_parameter_data(&self, index: i32) -> StaticVstPtr<dyn IParamValueQueue> {
        transmute(
            self.entries
                .get(index as usize)
                .map_or(null_mut(), VstPtr::as_ptr),
        )
    }
    unsafe fn add_parameter_data(
        &self,
        _: *const u32,
        _: *mut i32,
    ) -> StaticVstPtr<dyn IParamValueQueue> {
        zeroed()
    }
}

pub struct Host {
    processor: VstPtr<dyn IAudioProcessor>,
    component: VstPtr<dyn IComponent>,
    controller: VstPtr<dyn IEditController>,
    sustain_id: u32,
    _library: Library,
}

impl Host {
    unsafe fn open(path: &CStr) -> Self {
        let library = Library::new(path.to_str().unwrap()).unwrap();
        let entry = library
            .get::<unsafe extern "C" fn(*mut c_void) -> bool>(b"bundleEntry")
            .unwrap();
        assert!(entry(null_mut()));
        let factory_fn = library
            .get::<unsafe extern "system" fn() -> *mut c_void>(b"GetPluginFactory")
            .unwrap();
        let factory = VstPtr::<dyn IPluginFactory>::owned(factory_fn().cast()).unwrap();
        let mut class: PClassInfo = zeroed();
        let mut raw: *mut c_void = null_mut();
        assert_eq!(factory.get_class_info(0, &mut class), kResultOk);
        assert_eq!(
            factory.create_instance(&class.cid, &<dyn IComponent>::IID, &mut raw),
            kResultOk
        );
        let component = VstPtr::<dyn IComponent>::owned(raw.cast()).unwrap();
        let processor = component.cast::<dyn IAudioProcessor>().unwrap();
        let controller = component.cast::<dyn IEditController>().unwrap();
        let midi = component.cast::<dyn IMidiMapping>().unwrap();
        let mut sustain_id = 0;
        assert_eq!(
            midi.get_midi_controller_assignment(0, 0, 64, &mut sustain_id),
            kResultOk
        );
        assert_eq!(component.initialize(null_mut()), kResultOk);
        assert_eq!(component.activate_bus(0, 1, 0, 1), kResultOk);
        assert_eq!(component.activate_bus(1, 0, 0, 1), kResultOk);
        assert_eq!(
            processor.set_bus_arrangements(&mut 0, 0, &mut 3, 1),
            kResultOk
        );
        assert_eq!(
            processor.setup_processing(&ProcessSetup {
                process_mode: MODE,
                symbolic_sample_size: K_SAMPLE32,
                max_samples_per_block: BLOCK as i32,
                sample_rate: RATE,
            }),
            kResultOk
        );
        assert_eq!(component.set_active(1), kResultOk);
        Self {
            processor,
            component,
            controller,
            sustain_id,
            _library: library,
        }
    }

    unsafe fn render(&self, midi: &[Midi], left: &mut [f32], right: &mut [f32]) {
        assert_eq!(self.processor.set_processing(1), kResultOk);
        let mut from = 0;
        for (block, (left, right)) in left
            .chunks_mut(BLOCK)
            .zip(right.chunks_mut(BLOCK))
            .enumerate()
        {
            let start = (block * BLOCK) as i32;
            let mut to = from;
            let mut events = Vec::new();
            let mut changes = Vec::new();
            while let Some(msg) = midi
                .get(to)
                .filter(|msg| msg.frame < start + left.len() as i32)
            {
                let offset = msg.frame - start;
                let v = f32::from(msg.value) / 127.;
                let pitch = msg.note.into();
                if msg.status == 176 {
                    let queue = Queue::allocate(self.sustain_id, offset, f64::from(v));
                    changes.push(VstPtr::owned(Box::into_raw(queue).cast()).unwrap());
                } else {
                    events.push(Event {
                        bus_index: 0,
                        sample_offset: offset,
                        ppq_position: 0.,
                        flags: 0,
                        type_: u16::from(msg.status == 128),
                        event: match msg.status {
                            144 => EventData {
                                note_on: NoteOnEvent {
                                    pitch,
                                    velocity: v,
                                    note_id: -1,
                                    ..Default::default()
                                },
                            },
                            128 => EventData {
                                note_off: NoteOffEvent {
                                    pitch,
                                    velocity: v,
                                    note_id: -1,
                                    ..Default::default()
                                },
                            },
                            _ => unreachable!(),
                        },
                    });
                }
                to += 1;
            }
            from = to;
            let events =
                VstPtr::<dyn IEventList>::owned(Box::into_raw(Events::allocate(events)).cast())
                    .unwrap();
            let changes = VstPtr::<dyn IParameterChanges>::owned(
                Box::into_raw(Changes::allocate(changes)).cast(),
            )
            .unwrap();
            let mut channels = [left.as_mut_ptr().cast(), right.as_mut_ptr().cast()];
            let mut output = AudioBusBuffers {
                num_channels: 2,
                silence_flags: 0,
                buffers: channels.as_mut_ptr(),
            };
            let mut data = ProcessData {
                process_mode: MODE,
                symbolic_sample_size: K_SAMPLE32,
                num_samples: left.len() as i32,
                num_inputs: 0,
                num_outputs: 1,
                inputs: null_mut(),
                outputs: &mut output,
                input_events: transmute(events.as_ptr()),
                input_param_changes: transmute(changes.as_ptr()),
                output_events: zeroed(),
                output_param_changes: zeroed(),
                context: null_mut(),
            };
            assert_eq!(self.processor.process(&mut data), kResultOk);
        }
        assert_eq!(self.processor.set_processing(0), kResultOk);
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        unsafe {
            self.component.set_active(0);
            self.component.terminate();
        }
    }
}
#[repr(C)]
pub struct Midi {
    frame: i32,
    status: u8,
    note: u8,
    value: u8,
}

#[no_mangle]
pub unsafe extern "C" fn ep_open(path: *const c_char) -> *mut Host {
    Box::into_raw(Box::new(Host::open(CStr::from_ptr(path))))
}

#[no_mangle]
pub unsafe extern "C" fn ep_close(host: *mut Host) {
    drop(Box::from_raw(host));
}

#[no_mangle]
pub unsafe extern "C" fn ep_parameter_count(host: &Host) -> i32 {
    host.controller.get_parameter_count()
}

#[no_mangle]
pub unsafe extern "C" fn ep_parameter_info(
    host: &Host,
    index: i32,
    id: *mut u32,
    title: *mut i16,
) -> f64 {
    let mut info: ParameterInfo = zeroed();
    assert_eq!(
        host.controller.get_parameter_info(index, &mut info),
        kResultOk
    );
    *id = info.id;
    std::ptr::copy_nonoverlapping(info.title.as_ptr(), title, 128);
    host.controller.get_param_normalized(info.id)
}

#[no_mangle]
pub unsafe extern "C" fn ep_set(host: &Host, id: u32, value: f64) -> f64 {
    assert_eq!(host.controller.set_param_normalized(id, value), kResultOk);
    host.controller.get_param_normalized(id)
}

#[no_mangle]
pub unsafe extern "C" fn ep_render(
    host: &Host,
    entries: *const Midi,
    entry_count: usize,
    left: *mut f32,
    right: *mut f32,
    frames: usize,
) {
    host.render(
        std::slice::from_raw_parts(entries, entry_count),
        std::slice::from_raw_parts_mut(left, frames),
        std::slice::from_raw_parts_mut(right, frames),
    );
}
