#!/usr/bin/env python3
"""Small CLAP ABI integration test; no DAW, audio device, or external packages.

Declarations follow the clap-sys ABI used by the workspace's NIH-plug.
Usage: python3 chordboard/scripts/verify-clap.py /path/to/Chordboard.clap [--arpeggiator | --midi-delay | --play-contract | --held-chord | --extensions]
"""
import ctypes as C
import json
import pathlib
import sys

P, U, I, D, B = C.c_void_p, C.c_uint32, C.c_int32, C.c_double, C.c_bool
F = C.CFUNCTYPE
class Version(C.Structure):
    _fields_ = [(n, U) for n in ('major', 'minor', 'revision')]
class Header(C.Structure):
    _fields_ = [('size',U),('time',U),('space',C.c_uint16),('type',C.c_uint16),('flags',U)]
class Transport(C.Structure):
    _fields_ = [('header',Header),('flags',U),('beats',C.c_int64),('seconds',C.c_int64),
                ('tempo',D),('tempo_inc',D),('loop_start_beats',C.c_int64),('loop_end_beats',C.c_int64),
                ('loop_start_seconds',C.c_int64),('loop_end_seconds',C.c_int64),('bar_start',C.c_int64),
                ('bar_number',I),('tsig_num',C.c_uint16),('tsig_denom',C.c_uint16)]
class Midi(C.Structure):
    _fields_ = [('header',Header),('port',C.c_uint16),('data',C.c_uint8*3)]
class Note(C.Structure):
    _fields_ = [('header',Header),('id',I),('port',C.c_int16),('channel',C.c_int16),('note',C.c_int16),('velocity',D)]
class ParamEvent(C.Structure):
    _fields_ = [('header',Header),('id',U),('cookie',P),('note_id',I),('port',C.c_int16),('channel',C.c_int16),('key',C.c_int16),('value',D)]
class InEvents(C.Structure):
    _fields_ = [('ctx',P),('size',F(U,P)),('get',F(P,P,U))]
class OutEvents(C.Structure):
    _fields_ = [('ctx',P),('push',F(B,P,P))]
class Audio(C.Structure):
    _fields_ = [('data32',C.POINTER(C.POINTER(C.c_float))),('data64',P),('channels',U),('latency',U),('constant',C.c_uint64)]
class Process(C.Structure):
    _fields_ = [('steady',C.c_int64),('frames',U),('transport',P),('inputs',C.POINTER(Audio)),('outputs',C.POINTER(Audio)),('input_count',U),('output_count',U),('in_events',C.POINTER(InEvents)),('out_events',C.POINTER(OutEvents))]
class Plugin(C.Structure):
    _fields_ = [('descriptor',P),('data',P),('init',F(B,P)),('destroy',F(None,P)),('activate',F(B,P,D,U,U)),('deactivate',F(None,P)),('start',F(B,P)),('stop',F(None,P)),('reset',F(None,P)),('process',F(I,P,C.POINTER(Process))),('extension',F(P,P,C.c_char_p)),('main',F(None,P))]
class Host(C.Structure):
    _fields_ = [('version',Version),('data',P),('name',C.c_char_p),('vendor',C.c_char_p),('url',C.c_char_p),('release',C.c_char_p),('extension',F(P,P,C.c_char_p)),('restart',F(None,P)),('process',F(None,P)),('callback',F(None,P))]
class Entry(C.Structure):
    _fields_ = [('version',Version),('init',F(B,C.c_char_p)),('deinit',F(None)),('factory',F(P,C.c_char_p))]
class Factory(C.Structure):
    _fields_ = [('count',F(U,P)),('descriptor',F(P,P,U)),('create',F(P,P,C.POINTER(Host),C.c_char_p))]
class ParamInfo(C.Structure):
    _fields_ = [('id',U),('flags',U),('cookie',P),('name',C.c_char*256),('module',C.c_char*1024),('minimum',D),('maximum',D),('default',D)]
class InputStream(C.Structure):
    _fields_ = [('ctx',P),('read',F(C.c_int64,P,P,C.c_uint64))]
class OutputStream(C.Structure):
    _fields_ = [('ctx',P),('write',F(C.c_int64,P,P,C.c_uint64))]
class StateExtension(C.Structure):
    _fields_ = [('save',F(B,P,C.POINTER(OutputStream))),('load',F(B,P,C.POINTER(InputStream)))]
class Params(C.Structure):
    _fields_ = [('count',F(U,P)),('info',F(B,P,U,C.POINTER(ParamInfo))),('value',P),('to_text',P),('from_text',P),('flush',F(None,P,C.POINTER(InEvents),C.POINTER(OutEvents)))]

def midi(time,status,a,b=0):
    return Midi(Header(C.sizeof(Midi),time,0,10,0),0,(C.c_uint8*3)(status,a,b))

def decode(blob):
    header=Header.from_buffer_copy(blob)
    if header.type in (0,1):
        n=Note.from_buffer_copy(blob)
        return ('on' if header.type==0 else 'off',header.time,n.channel,n.note,n.velocity)
    if header.type==10:
        m=Midi.from_buffer_copy(blob); status,a,b=m.data
        kind=status & 0xf0
        if kind in (0x90,0x80):
            return ('on' if kind==0x90 and b else 'off',header.time,status&15,a,b/127)
        return ('midi',header.time,status,a,b)
    return ('other',header.time,header.type)

class Harness:
    def __init__(self,path,plugin_id=b'com.fergler.chordboard'):
        self.library=C.CDLL(str(path));self.entry=Entry.in_dll(self.library,'clap_entry')
        assert self.entry.init(str(path).encode())
        self.factory_ptr=self.entry.factory(b'clap.plugin-factory')
        self.factory=C.cast(self.factory_ptr,C.POINTER(Factory)).contents
        self.host=Host(Version(1,2,0),None,b'Chordboard test host',b'Fergler',b'',b'1',F(P,P,C.c_char_p)(lambda *_:None),F(None,P)(lambda *_:None),F(None,P)(lambda *_:None),F(None,P)(lambda *_:None))
        self.ptr=self.factory.create(self.factory_ptr,C.byref(self.host),plugin_id);assert self.ptr
        self.plugin=C.cast(self.ptr,C.POINTER(Plugin)).contents;assert self.plugin.init(self.ptr)
        self.params=C.cast(self.plugin.extension(self.ptr,b'clap.params'),C.POINTER(Params)).contents
        self.ids={}
        for index in range(self.params.count(self.ptr)):
            info=ParamInfo();assert self.params.info(self.ptr,index,C.byref(info));self.ids[info.name.decode()]=info.id
        assert self.plugin.activate(self.ptr,48000.,1,512);assert self.plugin.start(self.ptr)
        self.time=0
        self.transport=None
        self.baseline_state=self.save_state()
    def close(self):
        self.plugin.stop(self.ptr);self.plugin.deactivate(self.ptr);self.plugin.destroy(self.ptr);self.entry.deinit()
    def save_state(self):
        extension=C.cast(self.plugin.extension(self.ptr,b'clap.state'),C.POINTER(StateExtension)).contents
        chunks=[]
        def write(_,data,size):
            chunks.append(C.string_at(data,size));return size
        stream=OutputStream(None,F(C.c_int64,P,P,C.c_uint64)(write))
        assert extension.save(self.ptr,C.byref(stream))
        blob=b''.join(chunks)
        assert int.from_bytes(blob[:8],'little')==len(blob)-8
        return json.loads(blob[8:])
    def load_state(self,state):
        self.plugin.stop(self.ptr);self.plugin.deactivate(self.ptr)
        extension=C.cast(self.plugin.extension(self.ptr,b'clap.state'),C.POINTER(StateExtension)).contents
        payload=json.dumps(state).encode();blob=len(payload).to_bytes(8,'little')+payload;offset=0
        def read(_,dest,size):
            nonlocal offset
            chunk=blob[offset:offset+size];C.memmove(dest,chunk,len(chunk));offset+=len(chunk);return len(chunk)
        stream=InputStream(None,F(C.c_int64,P,P,C.c_uint64)(read))
        assert extension.load(self.ptr,C.byref(stream))
        assert self.plugin.activate(self.ptr,48000.,1,512);assert self.plugin.start(self.ptr)
        self.time=0
    def parameter(self,name,value,time=0):
        return ParamEvent(Header(C.sizeof(ParamEvent),time,0,5,0),self.ids[name],None,-1,-1,-1,-1,value)
    def run(self,events=(),frames=256):
        events=sorted(events,key=lambda e:e.header.time);captured=[]
        inputs=InEvents(None,F(U,P)(lambda _:len(events)),F(P,P,U)(lambda _,i:C.addressof(events[i])))
        def push(_,event):
            h=C.cast(event,C.POINTER(Header)).contents
            captured.append(C.string_at(event,h.size));return True
        outputs=OutEvents(None,F(B,P,P)(push))
        samples=[(C.c_float*frames)(*(0.125 if ch==0 else -0.25 for _ in range(frames))) for ch in range(2)]
        rendered=[(C.c_float*frames)() for _ in range(2)]
        in_ptrs=(C.POINTER(C.c_float)*2)(*samples);out_ptrs=(C.POINTER(C.c_float)*2)(*rendered)
        audio_in=Audio(in_ptrs,None,2,0,0);audio_out=Audio(out_ptrs,None,2,0,0)
        process=Process(self.time,frames,C.addressof(self.transport) if self.transport is not None else None,C.pointer(audio_in),C.pointer(audio_out),1,1,C.pointer(inputs),C.pointer(outputs))
        assert self.plugin.process(self.ptr,C.byref(process))!=0
        self.time+=frames
        assert list(rendered[0])==list(samples[0]) and list(rendered[1])==list(samples[1]),'audio passthrough changed'
        result=list(map(decode,captured))
        assert all(0<=e[1]<frames for e in result),'out-of-block output event'
        return result

def run_tests(path):
    h=Harness(path)
    try:
        # Exercise Chord on Select independently of automatic arp power.
        h.run([h.parameter('Always play full chord',1),h.parameter('Output protocol',1),
               h.parameter('Arpeggiator',0)])
        events=h.run([midi(0,0xd1,90),midi(0,0xb1,74,80),midi(12,0x91,60,100)])
        ons=[e for e in events if e[0]=='on']
        assert [e[3] for e in ons]==[60,64,67],ons
        assert all(e[1]==12 for e in ons),('note timing',ons)
        assert [e[2] for e in ons]==[1,2,3],ons
        for note in ons:
            index=events.index(note);ch=note[2]
            assert any(e[0]=='midi' and e[2]==0xd0+ch and e[3]==90 for e in events[:index]),('missing pressure initialization',note)
            assert any(e[0]=='midi' and e[2]==0xb0+ch and e[3:]==(74,80) for e in events[:index]),('missing CC74 initialization',note)
        events=h.run([midi(20,0xd1,100)])
        assert sum(e[0]=='midi' and e[2] in (0xd1,0xd2,0xd3) and e[3]==100 for e in events)==3
        events=h.run([h.parameter('Output protocol',2,48)])
        offs=[e for e in events if e[0]=='off']
        assert len(offs)==3 and all(e[1]==48 for e in offs),('routing automation timing',events)
        h.plugin.reset(h.ptr)
        events=h.run()
        assert len([e for e in events if e[0]=='off'])==3,'reset lost owned note offs'
        assert not [e for e in h.run() if e[0]=='on'],'reset restored held notes'
        h.run([midi(0,0x91,60,100)])
        events=h.run([midi(i,0xd1,50+i%70) for i in range(200)])
        assert len([e for e in events if e[0]=='off'])==3,'event storm lost existing note releases'
        assert not [e for e in h.run() if e[0]=='on'],'event storm restored held notes'
        h.plugin.reset(h.ptr)
        h.run([h.parameter('Always play full chord',0),h.parameter('Note filter',3),h.parameter('Inversion',1)])
        h.run([midi(16,0x91,60,100),midi(32,0xb0,1,0)])
        events=h.run([midi(64,0xb0,1,127)])
        events.extend(h.run([midi(64,0xb0,1,0)]))
        ons=[e for e in events if e[0]=='on']
        assert {e[3] for e in ons}=={64,72,76},('filtered strum',ons)
        assert all(e[1]==64 for e in ons),('controller event timing',ons)
        print('CLAP integration passed: passthrough, sample timing, MPE initialization/fan-out, routing automation, filters, reset, and bounded event-storm recovery.')
    finally:
        h.close()

def run_held_chord_tests(path):
    for root_first,gap in [(False,0),(True,0),(False,1),(True,1),(False,None)]:
        h=Harness(path)
        try:
            # Manual selection and Latch operate independently of arp playback.
            h.run([h.parameter('Latch',1),h.parameter('Output protocol',2)])
            h.run([midi(0,0x91,60,100),midi(32,0x92,62,100)])
            order=[(0x81,60),(0x82,62)] if root_first else [(0x82,62),(0x81,60)]
            releases=[midi(64,*order[0],50)]
            if gap is not None:
                releases.append(midi(64+gap,*order[1],50))
            h.run(releases)
            h.run([midi(0,0xb0,1,0)])
            events=h.run([midi(16,0xb0,1,127)])
            events.extend(h.run([midi(16,0xb0,1,0)]))
            pitches={e[3]%12 for e in events if e[0]=='on'}
            assert pitches=={0,2,7},('sus2 lost after release',root_first,gap,pitches)
            if gap is None:
                h.run([midi(0,0x81,60,50)])
            h.run([midi(0,0x91,60,100)])
            h.run([midi(0,0xb0,1,0)])
            events=h.run([midi(16,0xb0,1,127)])
            events.extend(h.run([midi(16,0xb0,1,0)]))
            pitches={e[3]%12 for e in events if e[0]=='on'}
            assert pitches=={0,4,7},('root re-trigger did not reset',pitches)
        finally:
            h.close()
    print('CLAP Hold chord passed: simultaneous and adjacent-sample releases in both orders, alteration-only release, retained sus2 MIDI, root re-trigger reset.')

def run_extension_tests(path):
    for latch in (False, True):
        h=Harness(path)
        try:
            h.run([h.parameter('Latch',int(latch)),
                   h.parameter('Output protocol',2),h.parameter('Arpeggiator',0)])
            # C + Bb retains the existing C7 gesture; later keys add both ninths,
            # sharp eleven and thirteen, independently of their played octave.
            h.run([midi(0,0x91,60,100),midi(8,0x92,70,100),
                   midi(16,0x93,61,40),midi(24,0x94,74,90),
                   midi(32,0x95,66,60),midi(40,0x96,69,70)])
            def sweep():
                h.run([midi(0,0xb0,1,0)])
                events=h.run([midi(16,0xb0,1,127)])
                events.extend(h.run([midi(16,0xb0,1,0)]))
                return {e[3]%12 for e in events if e[0]=='on'}
            pitches=sweep()
            assert pitches=={0,1,2,4,6,7,9,10},('combined extensions',latch,pitches)
            h.run([midi(0,0x97,73,50)]) # duplicate b9 on another channel/octave
            h.run([midi(0,0x83,61,50)])
            assert 1 in sweep(),'one duplicate release removed the other b9 owner'
            h.run([midi(0,0x87,73,50)])
            pitches=sweep()
            assert (1 in pitches)==latch,('extension release/latch',latch,pitches)
            h.run([midi(0,0xb0,123,0)])
        finally:
            h.close()
    print('CLAP extensions passed: combined ninths through thirteenths, octave independence, duplicate ownership, release and latch.')

def run_strum_tests(path):
    h=Harness(path)
    try:
        h.run([h.parameter('Manual strum octaves',3),
               h.parameter('Inversion',1),h.parameter('Output protocol',2),
               h.parameter('Arpeggiator',0)])
        events=h.run([midi(16,0x90,63,100)])
        assert not any(e[0]=='on' for e in events),'Strum Only sounded on selection'
        expected=[67,70,75,79,82,87,91,94,99,103,106,111]
        events=h.run([midi(0,0xb0,1,0)])
        assert not any(e[0]=='on' for e in events),'initial controller position struck a string'
        events=h.run([midi(16,0xb0,1,127)])
        assert [e[3] for e in events if e[0]=='on']==expected[1:],('up sweep',events)
        events=h.run([midi(16,0xb0,1,0)])
        assert [e[3] for e in events if e[0]=='on']==list(reversed(expected[:-1])),('down sweep',events)
        assert not any(e[0]=='on' for e in h.run([midi(0,0xb0,1,0)])),'stationary controller retriggered'
        events=h.run([midi(8,0x80,63,50)])
        assert not any(e[0]=='off' for e in events),'release cut ringing strings'
        h.run([h.parameter('Manual strum octaves',0)])
        events=h.run([midi(16,0x90,65,100)])
        assert not any(e[0]=='off' and e[3] in expected for e in events),'chord change cut tails'
        h.run([midi(0,0xb0,1,0)])
        events=h.run([midi(16,0xb0,1,127)])
        events.extend(h.run([midi(16,0xb0,1,0)]))
        assert {e[3] for e in events if e[0]=='on'}=={69,72,77},events
        h.run([midi(8,0x80,65,50)])
        expired=[]
        for _ in range(40):
            expired.extend(h.run(frames=512))
        assert set(expected+[69,72,77]) <= {e[3] for e in expired if e[0]=='off'},'tails did not expire'
        print('CLAP strum controls passed: twelve strings, initial-position priming, both sweep directions, stationary input, chord changes, and tail release.')
    finally:
        h.close()

def run_split_tests(path):
    for full,latch,affect in [(full,latch,affect)
                              for full,latch in [(False,False),(True,False),(True,True)]
                              for affect in [False,True]]:
        h=Harness(path)
        try:
            h.run([h.parameter('Output protocol',2),
                   h.parameter('Key split',1),h.parameter('First right-hand note',60),
                   h.parameter('Always play bass',1),h.parameter('Always play full chord',int(full)),
                   h.parameter('Melody affects chords',int(affect)),h.parameter('Latch',int(latch))])
            # Chord keys sound immediately, before any melody key is held.
            events=h.run([midi(0,0x90,48,100)])
            chord=[48,52,55] if full else [48]
            assert [e[3] for e in events if e[0]=='on']==chord,events
            for note in [72,76]:
                events=h.run([midi(0,0x94,note,100)])
                assert [(e[2],e[3]) for e in events if e[0]=='on']==[(4,note)],events
            if full and affect:
                events=h.run([midi(0,0x94,74,100)])
                assert {e[3] for e in events if e[0]=='on'}=={50,74},events
                h.run([midi(0,0x84,74,50)])
            events=[]
            for _ in range(40):
                events.extend(h.run(frames=512))
            assert not any(e[0]=='off' and e[3] in chord for e in events),'held layer timed out'
            events=h.run([midi(0,0x80,48,50)])
            if latch:
                assert not any(e[0]=='off' and e[3] in chord for e in events),'latch released layer'
            else:
                assert {e[3] for e in events if e[0]=='off'} >= set(chord),events
            for note in [72,76]:
                events=h.run([midi(0,0x84,note,50)])
                assert any(e[0]=='off' and e[2:4]==(4,note) for e in events),events
                assert not any(e[0]=='on' for e in events),'melody release restarted chord'
            if latch:
                events=h.run([midi(0,0x90,50,100)])
                assert {e[3] for e in events if e[0]=='off'}==set(chord),events
                assert [e[3] for e in events if e[0]=='on']==[50,54,57],events
            else:
                events=h.run([midi(0,0x94,76,100)])
                assert [(e[2],e[3]) for e in events if e[0]=='on']==[(4,76)],events
            h.run([midi(0,0xb0,123,0)])
        finally:
            h.close()
    h=Harness(path)
    try:
        h.run([h.parameter('Output protocol',2),
               h.parameter('Key split',1),h.parameter('Always play full chord',1)])
        h.run([midi(0,0x90,48,100),midi(1,0x94,72,100)])
        events=h.run([midi(i,0xd0,50+i%70) for i in range(200)])
        assert {(0,48),(0,52),(0,55),(4,72)} <= {(e[2],e[3]) for e in events if e[0]=='off'},'overflow lost split-owned releases'
        assert sum(e[0]=='midi' and e[2]&0xf0==0xb0 and e[3:]==(64,0) for e in events)==16,'overflow lost pedal release'
    finally:
        h.close()
    # Verify chord-hand extensions through the real plugin's MIDI output.
    for second, expected in [(58, {48,52,55,58}), (46, {48,52,55,58}), (55, {48,55})]:
        h=Harness(path)
        try:
            h.run([h.parameter('Output protocol',2),
                   h.parameter('Key split',1),h.parameter('Always play full chord',1)])
            events=h.run([midi(0,0x90,48,100),midi(1,0x94,72,100),midi(32,0x90,second,100)])
            sounding=set()
            for event in events:
                if event[0]=='on' and event[2]==0:
                    sounding.add(event[3])
                elif event[0]=='off' and event[2]==0:
                    sounding.discard(event[3])
            assert sounding==expected,('chord-hand interval',second,sounding,expected)
        finally:
            h.close()
    print('CLAP split controls passed: melody passthrough, sustained bass/full chord, chord-key gate, independent melody, melody harmony influence, latch replacement, and overflow recovery.')

def state_value(state,key,value):
    """Edit the typed value already present in NIH-plug's saved state."""
    assert key in state['params'],('missing state parameter',key)
    tagged=state['params'][key]
    assert len(tagged)==1,(key,tagged)
    state['params'][key]={next(iter(tagged)):value}

def array_parameter(state,base,index,count=8):
    # Discover the actual serialized IDs. Array parameters have repeated display
    # names, so Harness.ids must never choose an arp page by its name alone.
    prefix=base+'_'
    candidates={int(key[len(prefix):]):key for key in state['params']
                if key.startswith(prefix) and key[len(prefix):].isdigit()}
    assert sorted(candidates)==list(range(1,count+1)),('array state IDs',base,sorted(candidates))
    return candidates[index+1]

def page_value(state,base,value,page=0):
    state_value(state,array_parameter(state,base,page),value)

def arp_state(h,enabled=True):
    state=json.loads(json.dumps(h.baseline_state))
    assert 'arpeggiator-v1' in state['fields'],'installed plugin lacks the new arpeggiator state'
    program=json.loads(state['fields']['arpeggiator-v1'])
    assert len(program['pages'])==8 and all(len(p['loops'])==4 and all(len(loop['steps'])==24 for loop in p['loops']) for p in program['pages'])
    for key,value in [('arp_enabled',enabled),('arp_page',0),('output_mode',2),
                      ('voice_leading',0),('root_on_select',False),('always_chord',False),
                      ('always_bass',False),('tempo_sync',False),('tempo',120.0),('latch',False)]:
        state_value(state,key,value)
    for key,value in [('arp_mode',0),('arp_pattern',0),('arp_octaves',1),('arp_sync',True),
                      ('rate',0.25),('swing',0.0),('humanize',0.0)]:
        page_value(state,key,value)
    # Preserve the manual controller mapping; eliminate unrelated optional links.
    for slot in range(1,16):
        state_value(state,array_parameter(state,'route_source',slot,16),0)
    for page in program['pages']:
        page['initialized']=4
    return state,program

def load_program(h,state,program):
    state['fields']['arpeggiator-v1']=json.dumps(program)
    h.load_state(state)

def timeline(h,samples,events=(),block=256):
    """Collect absolute sample times while processing ordinary small callbacks."""
    result=[];first=True
    while samples:
        frames=min(block,samples);origin=h.time
        current=h.run(events if first else (),frames)
        result.extend((e[0],origin+e[1],*e[2:]) for e in current)
        first=False;samples-=frames
    return result

def attacks(events):
    return [e for e in events if e[0]=='on']

def assert_released(events):
    active={}
    for event in events:
        if event[0] in ('on','off'):
            key=event[2:4]
            if event[0]=='on':active[key]=active.get(key,0)+1
            elif key in active:
                active[key]-=1
                if not active[key]:del active[key]
    assert not active,('stranded emitted notes',active)

def run_arpeggiator_tests(path):
    h=Harness(path)
    try:
        state,program=arp_state(h)
        page_value(state,'arp_pattern',2) # C -> E -> G -> E, four occurrences
        for step,velocity in zip(program['pages'][0]['loops'][0]['steps'],[100,50,125,25]):
            step['velocity']=velocity
        # Distinct values on the repeated E prove indexing is by occurrence.
        program['pages'][1]['loops'][0]['steps'][0]['velocity']=67
        program['pages'][1]['delay']['taps'][0]['feedback']=0.5
        page_value(state,'arp_mode',1,page=1)
        page_value(state,'arp_octaves',3,page=1)
        load_program(h,state,program)
        events=timeline(h,25000,[midi(16,0x90,60,100)])
        ons=attacks(events)
        assert [(e[1],e[3]) for e in ons]==[(16,60),(6016,64),(12016,67),(18016,64)],('arp cycle/timing',ons)
        assert [round(e[4]*127) for e in ons]==[100,50,125,25],('per-occurrence velocity',ons)
        assert_released(events)
        saved=h.save_state()
        assert json.loads(saved['fields']['arpeggiator-v1'])==program,'arp/Delay Pages changed on save'
        h.load_state(saved)
        assert h.save_state()['params']==saved['params'],'page parameter roundtrip changed'
        assert json.loads(h.save_state()['fields']['arpeggiator-v1'])==program,'arp/Delay roundtrip changed'
        for status,expected in [('Mute',[(16,60),(12016,67)]),('Skip',[(16,60),(6016,67)])]:
            page_value(state,'arp_pattern',0)
            program['pages'][0]['loops'][0]['steps'][1]['state']=status
            load_program(h,state,program)
            events=timeline(h,20000,[midi(16,0x90,60,100)])
            assert [(e[1],e[3]) for e in attacks(events)]==expected,(status,attacks(events))
            assert_released(events)
        for step in program['pages'][0]['loops'][0]['steps'][:3]:step['state']='Skip'
        load_program(h,state,program)
        assert not attacks(timeline(h,12000,[midi(16,0x90,60,100)])),'all-skipped cycle emitted notes'
        # Four independent loop records must match the installed engine's cycle.
        state,program=arp_state(h)
        page_value(state,'arp_octaves',4)
        for loop,octave,velocity in zip(program['pages'][0]['loops'],[0,1,-1,0],[100,50,75,25]):
            loop['octave']=octave
            loop['steps'][0]['velocity']=velocity
        expected=[60,64,67,72,76,79,48,52,55,60,64,67]
        load_program(h,state,program)
        events=timeline(h,75000,[midi(16,0x90,60,100)])
        ons=attacks(events)
        assert [(e[1],e[3]) for e in ons]==[(16+i*6000,note) for i,note in enumerate(expected)],('four loop ordering',ons)
        assert [round(ons[i][4]*127) for i in [0,3,6,9]]==[100,50,75,25],('loop-local adjustments',ons)
        assert_released(events)
        # The four loop octave destinations are appended to the routing catalog.
        final_target=None
        for index in range(h.params.count(h.ptr)):
            info=ParamInfo();assert h.params.info(h.ptr,index,C.byref(info))
            if info.name.decode()=='Destination':final_target=int(info.maximum)
        assert final_target is not None
        state_value(state,array_parameter(state,'route_source',1,16),2)
        state_value(state,array_parameter(state,'route_target',1,16),final_target-3)
        state_value(state,array_parameter(state,'route_min',1,16),0.75)
        state_value(state,array_parameter(state,'route_max',1,16),0.75)
        load_program(h,state,program)
        events=timeline(h,75000,[midi(16,0x90,60,100)])
        assert [e[3] for e in attacks(events)]==[84,88,91]+expected[3:],('independent loop octave routing',attacks(events))
        assert_released(events)
        print('CLAP arpeggiator passed: repeated-note occurrences, exact 1/16 timing, Once completion, Mute versus Skip, all-skipped silence, eight-Page state roundtrip, four loop variations, and independent octave routing.')
    finally:
        h.close()

def single_note_delay(h,feedback=0.5,gate=25,free_ms=2.0):
    state,program=arp_state(h)
    page_value(state,'arp_sync',False)
    page_value(state,'arp_free_ms',free_ms)
    for step in program['pages'][0]['loops'][0]['steps'][1:3]:step['state']='Skip'
    program['pages'][0]['loops'][0]['steps'][0]['gate']=gate
    delay=program['pages'][0]['delay']
    delay.update(enabled=True,sync=False,sources=1,count=1)
    delay['taps'][0].update(time=10.0,velocity=0.75,feedback=feedback)
    return state,program,delay

def run_midi_delay_tests(path):
    h=Harness(path)
    try:
        state,program,delay=single_note_delay(h)
        load_program(h,state,program)
        events=timeline(h,5000,[midi(16,0x90,60,100)])
        ons=attacks(events);echoes=ons[1:]
        assert ons[0][1:4]==(16,0,60),('dry attack',ons)
        assert [e[1] for e in echoes]==[16+480*n for n in range(1,8)],('10 ms feedback train',echoes)
        # CLAP carries fractional velocity; compare the train itself instead of
        # imposing integer rounding at a half-velocity boundary.
        assert all(abs(e[4] - 75/127 * 0.5**i) < 1e-6 for i,e in enumerate(echoes)),('feedback velocity decay',echoes)
        assert all(e[2]!=0 and e[3]==60 for e in echoes),'echo changed dry routing/pitch'
        for on in ons:
            assert any(e[0]=='off' and e[1]==on[1]+24 and e[2:4]==on[2:4] for e in events),('captured duration',on)
        assert_released(events)
        # Same clock, independent tap intervals, static transposition, captured
        # pressure/timbre/bend and isolated per-tap pan/fine pitch channels.
        state,program,delay=single_note_delay(h,feedback=0.0,gate=50,free_ms=200.0)
        delay['count']=2
        delay['taps'][0].update(pitch=12,fine=25.0,pan=-0.5)
        delay['taps'][1].update(time=15.0,velocity=0.5,feedback=0.0,pitch=7,fine=-25.0,pan=0.5)
        load_program(h,state,program)
        events=timeline(h,6000,[midi(0,0xd0,90),midi(0,0xb0,74,80),midi(0,0xe0,0,80),midi(16,0x90,60,100)])
        ons=attacks(events);echoes=ons[1:]
        assert [(e[1],e[3]) for e in echoes]==[(496,72),(736,67)],('independent taps',echoes)
        assert len({e[2] for e in ons})==3,('incompatible simultaneous taps shared a channel',ons)
        for echo,pan,bend in zip(echoes,[32,95],[11264,9216]):
            prefix=events[:events.index(echo)];channel=echo[2]
            at=[e for e in prefix if e[0]=='midi' and e[1]==echo[1]]
            assert any(e[2]==0xb0+channel and e[3]==10 and abs(e[4]-pan)<=1 for e in at),('tap pan',echo,at)
            assert any(e[2]==0xd0+channel and e[3]==90 for e in at),('captured pressure',echo,at)
            assert any(e[2]==0xb0+channel and e[3:]==(74,80) for e in at),('captured timbre',echo,at)
            assert any(e[2]==0xe0+channel and abs(e[3]+128*e[4]-bend)<=2 for e in at),('captured bend plus detune',echo,at)
        assert_released(events)
        # A high-gain train remains above velocity 1 at generation 64. The
        # explicit generation cap, rather than decay, must end it here.
        state,program,delay=single_note_delay(h,feedback=0.95,gate=1,free_ms=1.0)
        delay['taps'][0].update(time=1.0,velocity=2.0)
        load_program(h,state,program)
        events=timeline(h,4000,[midi(16,0x90,60,100)])
        assert len(attacks(events))==65,('64-generation bound',len(attacks(events)))
        assert_released(events)
        # Ping pong changes only echo pan; panic cancels pending generations.
        state,program,delay=single_note_delay(h,feedback=0.5)
        delay['ping_pong']=True
        load_program(h,state,program)
        events=timeline(h,1700,[midi(16,0x90,60,100)])
        for index,echo in enumerate(attacks(events)[1:]):
            expected=0 if index%2==0 else 127
            assert any(e[0]=='midi' and e[1]==echo[1] and e[2]==0xb0+echo[2] and e[3:]==(10,expected) for e in events),('ping pong',index,echo)
        h.run([midi(0,0xb0,123,0)])
        assert not attacks(timeline(h,6000)),'panic retained future echoes'
        # Filter excluded sources without disabling dry playback.
        for source in [0,2,4,8,16]:
            state,program,delay=single_note_delay(h,feedback=0.0)
            delay['sources']=source
            load_program(h,state,program)
            events=timeline(h,2000,[midi(16,0x90,60,100)])
            assert len(attacks(events))==1,('source filter accepted arp',source,attacks(events))
        # Exercise the remaining four source paths with arp independently off.
        for source in [2,4,8,16]:
            state,program,delay=single_note_delay(h,feedback=0.0)
            state_value(state,'arp_enabled',False)
            delay['sources']=source
            gesture=[midi(16,0x90,60,100)]
            if source==2:
                gesture=[midi(0,0x90,60,100),midi(8,0xb0,1,0),midi(16,0xb0,1,127)]
            elif source==4:state_value(state,'always_chord',True)
            elif source==8:state_value(state,'always_bass',True)
            else:
                state_value(state,'key_split',True)
                state_value(state,'split_note',60)
                gesture=[midi(16,0x94,72,100)]
            load_program(h,state,program)
            events=timeline(h,1000,gesture)
            dry=[e for e in attacks(events) if e[1]==16]
            echoes=[e for e in attacks(events) if e[1]==496]
            assert dry and sorted(e[3] for e in echoes)==sorted(e[3] for e in dry),('source capture with arp off',source,dry,echoes)
            assert len(attacks(events))==len(dry)+len(echoes),('source fed another train',source,attacks(events))
            panic=h.run([midi(0,0xb0,123,0)])
            assert_released(events+panic)
            assert not attacks(timeline(h,2000)),('source panic',source)
        # A real stopped host supplies the same beat cursor on every callback.
        state,program,delay=single_note_delay(h,feedback=0.0)
        delay.update(sync=True)
        delay['taps'][0]['time']=0.5
        load_program(h,state,program)
        h.transport=Transport(header=Header(C.sizeof(Transport),0,0,9,0),flags=3,beats=8*(1<<31),tempo=120.0)
        events=timeline(h,14000,[midi(16,0x90,60,100)])
        assert [(e[1],e[3]) for e in attacks(events)]==[(16,60),(12016,60)],('stopped host synced delay',attacks(events))
        assert_released(events)
        print('CLAP MIDI Delay passed: exact repeats including stopped-host sync, feedback threshold/cap, independent taps, duration, isolated pitch/pan/expression, ping pong, all five sources, and panic.')
    finally:
        h.close()

def run_play_contract_tests(path):
    for arp in (False,True):
        for strumming in (False,True):
            for chord_on_select in (False,True):
                h=Harness(path)
                try:
                    state,program=arp_state(h,enabled=arp)
                    state_value(state,'strum_enabled',strumming)
                    state_value(state,'always_chord',chord_on_select)
                    program['pages'][0]['loops'][0]['steps'][0]['octave']=1
                    load_program(h,state,program)
                    events=h.run([midi(16,0x90,60,100)])
                    expected={60,64,67} if chord_on_select else set()
                    if arp:expected.add(72)
                    assert {e[3] for e in attacks(events)}==expected,('independent power',arp,strumming,chord_on_select,events)
                    bypass=h.run([h.parameter('Arpeggiator',0),h.parameter('Strumfield enabled',0)])
                    if chord_on_select:
                        assert not any(e[0]=='off' and e[3] in (60,64,67) for e in bypass),'arp bypass released Chord on Select'
                    assert not attacks(timeline(h,12000)),'arp bypass produced automatic attacks'
                    h.run([midi(0,0xb0,123,0)])
                    assert not attacks(timeline(h,12000)),'panic restored held selection'
                finally:
                    h.close()
    for protocol in (1,2):
        for selection in range(3):
            h=Harness(path)
            try:
                state,program=arp_state(h,enabled=False)
                state_value(state,'output_mode',protocol)
                state_value(state,'root_on_select',selection==1)
                state_value(state,'always_chord',selection==2)
                load_program(h,state,program)
                h.run([midi(0,0x90,60,100),midi(8,0xb0,1,0)])
                for _ in range(2):
                    up=h.run([midi(64,0xb0,1,127)])
                    down=h.run([midi(64,0xb0,1,0)])
                    assert [e[3] for e in attacks(up)]==[64,67],('selection mode strum up',protocol,selection,attacks(up))
                    assert [e[3] for e in attacks(down)]==[64,60],('selection mode strum down',protocol,selection,attacks(down))
                h.run([midi(0,0xb0,123,0)])
                assert not attacks(timeline(h,12000)),'panic restored selection-mode strum'
            finally:
                h.close()
    print('CLAP Play contract passed: independent power, all three selection modes strum in regular MIDI and MPE, arp bypass ownership and panic.')

if __name__=='__main__':
    assert len(sys.argv)>1,__doc__
    path=pathlib.Path(sys.argv[1]).expanduser().resolve()
    if path.is_dir():path=path/'Contents'/'MacOS'/'Chordboard'
    suites={
        '--extensions':run_extension_tests,
        '--arpeggiator':run_arpeggiator_tests,
        '--midi-delay':run_midi_delay_tests,
        '--play-contract':run_play_contract_tests,
        '--split-controls':run_split_tests,
        '--strum-controls':run_strum_tests,
        '--held-chord':run_held_chord_tests,
    }
    requested=sys.argv[2:]
    assert all(flag in suites for flag in requested),('unknown verification option',requested)
    for run in [suites[flag] for flag in requested] or [run_tests]:run(path)
