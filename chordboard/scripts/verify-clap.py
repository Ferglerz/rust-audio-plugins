#!/usr/bin/env python3
"""Small CLAP ABI integration test; no DAW, audio device, or external packages.

Declarations follow the clap-sys ABI used by the workspace's NIH-plug.
Usage: python3 chordboard/scripts/verify-clap.py /path/to/Chordboard.clap [--held-chord | --sequencer | --extensions]
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
    def __init__(self,path):
        self.library=C.CDLL(str(path));self.entry=Entry.in_dll(self.library,'clap_entry')
        assert self.entry.init(str(path).encode())
        self.factory_ptr=self.entry.factory(b'clap.plugin-factory')
        self.factory=C.cast(self.factory_ptr,C.POINTER(Factory)).contents
        self.host=Host(Version(1,2,0),None,b'Chordboard test host',b'Fergler',b'',b'1',F(P,P,C.c_char_p)(lambda *_:None),F(None,P)(lambda *_:None),F(None,P)(lambda *_:None),F(None,P)(lambda *_:None))
        self.ptr=self.factory.create(self.factory_ptr,C.byref(self.host),b'com.fergler.chordboard');assert self.ptr
        self.plugin=C.cast(self.ptr,C.POINTER(Plugin)).contents;assert self.plugin.init(self.ptr)
        self.params=C.cast(self.plugin.extension(self.ptr,b'clap.params'),C.POINTER(Params)).contents
        self.ids={}
        for index in range(self.params.count(self.ptr)):
            info=ParamInfo();assert self.params.info(self.ptr,index,C.byref(info));self.ids[info.name.decode()]=info.id
        assert self.plugin.activate(self.ptr,48000.,1,512);assert self.plugin.start(self.ptr)
        self.time=0
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
        process=Process(self.time,frames,None,C.pointer(audio_in),C.pointer(audio_out),1,1,C.pointer(inputs),C.pointer(outputs))
        assert self.plugin.process(self.ptr,C.byref(process))!=0
        self.time+=frames
        assert list(rendered[0])==list(samples[0]) and list(rendered[1])==list(samples[1]),'audio passthrough changed'
        result=list(map(decode,captured))
        assert all(0<=e[1]<frames for e in result),'out-of-block output event'
        return result

def run_tests(path):
    h=Harness(path)
    try:
        # Exercise current Chord on Select behavior; legacy Auto controls are inert.
        h.run([h.parameter('Always play full chord',1),h.parameter('Output protocol',1),
               h.parameter('Sequencer',0)])
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
            # CLAP's stepped values are zero-based: Manual Strum is index 1.
            h.run([h.parameter('Play mode',1),h.parameter('Latch',1),h.parameter('Output protocol',2)])
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
            h.run([h.parameter('Play mode',1),h.parameter('Latch',int(latch)),
                   h.parameter('Output protocol',2),h.parameter('Sequencer',0)])
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
        h.run([h.parameter('Play mode',1),h.parameter('Arp octaves',3),
               h.parameter('Inversion',1),h.parameter('Output protocol',2),
               h.parameter('Sequencer',0)])
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
        h.run([h.parameter('Arp octaves',0)])
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
    for full,latch in [(False,False),(True,False),(True,True)]:
        h=Harness(path)
        try:
            h.run([h.parameter('Play mode',1),h.parameter('Output protocol',2),
                   h.parameter('Key split',1),h.parameter('First right-hand note',60),
                   h.parameter('Always play bass',1),h.parameter('Always play full chord',int(full)),
                   h.parameter('Latch',int(latch))])
            # Melody passes through on its own channel; it cannot become a chord root.
            events=h.run([midi(0,0x94,72,100)])
            assert [(e[2],e[3]) for e in events if e[0]=='on']==[(4,72)],events
            events=h.run([midi(0,0x90,48,100)])
            chord=[48,52,55] if full else [48]
            assert [e[3] for e in events if e[0]=='on']==chord,events
            h.run([midi(0,0x80,48,50),midi(1,0x94,76,100)])
            events=[]
            for _ in range(40):
                events.extend(h.run(frames=512))
            assert not any(e[0]=='off' and e[3] in chord for e in events),'held layer timed out'
            events=h.run([midi(0,0x84,72,50)])
            assert any(e[0]=='off' and e[2:4]==(4,72) for e in events),events
            assert not any(e[0]=='off' and e[3] in chord for e in events),'first RH release closed gate'
            events=h.run([midi(0,0x84,76,50)])
            assert any(e[0]=='off' and e[2:4]==(4,76) for e in events),events
            if latch:
                assert not any(e[0]=='off' and e[3] in chord for e in events),'latch released layer'
                events=h.run([midi(0,0x90,50,100)])
                assert {e[3] for e in events if e[0]=='off'}==set(chord),events
                assert [e[3] for e in events if e[0]=='on']==[50,54,57],events
            else:
                assert {e[3] for e in events if e[0]=='off'} >= set(chord),events
            h.run([midi(0,0xb0,123,0)])
        finally:
            h.close()
    h=Harness(path)
    try:
        h.run([h.parameter('Play mode',1),h.parameter('Output protocol',2),
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
            h.run([h.parameter('Play mode',1),h.parameter('Output protocol',2),
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
    print('CLAP split controls passed: melody passthrough, sustained bass/full chord, last-right-key gate, latch replacement, and overflow recovery.')

def run_sequencer_tests(path):
    h=Harness(path)
    try:
        state=h.save_state()
        assert state['params']['mode']=={'i32':1},'legacy play-mode default changed'
        patterns=json.loads(state['fields']['sequencer-v1'])
        # Two-step Arp and Chords, with removed lanes restored as inert legacy data.
        for lane in range(4):
            page=patterns['lanes'][lane][0];page['length']=2;page['end']=1
        arp=patterns['lanes'][0][0]
        arp['steps'][0]['tone']=0;arp['steps'][1]['tone']=1
        arp['steps'][0]['pressure']=20;arp['steps'][0]['cc'][0]=-10
        patterns['lanes'][2][0]['steps'][0].update(enabled=True,octave=-1)
        patterns['lanes'][1][0]['steps'][1].update(enabled=True,velocity=75)
        patterns['lanes'][3][0]['steps'][0].update(enabled=True,root_offset=2)
        state['fields']['sequencer-v1']=json.dumps(patterns)
        state['params']['seq_enabled']={'bool':True}
        state['params']['mode']={'i32':3}
        state['params']['output_mode']={'i32':2}
        h.load_state(state)
        events=h.run([midi(0,0xb0,11,100),midi(16,0x90,60,100)])
        on={(e[2],e[3]) for e in events if e[0]=='on'}
        assert on=={(0,60)},('hidden legacy lanes changed the live chord',on)
        canonical=json.loads(h.save_state()['fields']['sequencer-v1'])
        assert canonical['lanes'][2][0]['steps'][0]['enabled'],'legacy data was discarded'
        assert all(e[1]==16 for e in events if e[0]=='on'),'first step timing shifted'
        assert any(e[0]=='midi' and e[2]==0xb0 and e[3:]==(11,90) for e in events),'raw CC offset wrong'
        all_events=[]
        for _ in range(25):
            origin=h.time;all_events.extend((origin+e[1],e) for e in h.run(frames=256))
        assert any(at==6016 and e[0]=='on' and e[3]==64 for at,e in all_events),'second step phase wrong'
        restored=h.save_state()
        assert json.loads(restored['fields']['sequencer-v1'])==canonical,'sequencer state roundtrip changed'
        # Existing MIDI panic and state replacement must release all lane ownership.
        events=h.run([midi(0,0xb0,123,0)])
        assert any(e[0]=='off' for e in events),'panic left sequencer notes owned'
        for _ in range(25):
            assert not any(e[0]=='on' for e in h.run()),'panic restarted an idle sequencer'
        legacy=restored;legacy['fields'].pop('sequencer-v1');legacy['params']['seq_enabled']={'bool':True}
        h.load_state(legacy)
        assert h.save_state()['params']['seq_enabled']=={'bool':False},'legacy state enabled sequencing'
        print('CLAP sequencer passed: live harmony, inert legacy lanes, exact timing, raw CC offsets, persistence, panic, and legacy fallback.')
    finally:
        h.close()

def run_play_contract_tests(path):
    for mode in (1,2,3):
        for sequencing in (False,True):
            for strumming in (False,True):
                for selection in (0,1,2):
                    h=Harness(path)
                    try:
                        state=h.save_state()
                        state['params']['mode']={'i32':mode}
                        state['params']['seq_enabled']={'bool':sequencing}
                        state['params']['strum_enabled']={'bool':strumming}
                        state['params']['root_on_select']={'bool':selection==1}
                        state['params']['always_chord']={'bool':selection==2}
                        patterns=json.loads(state['fields']['sequencer-v1'])
                        patterns['lanes'][0][0]['steps'][0]['octave']=1
                        state['fields']['sequencer-v1']=json.dumps(patterns)
                        h.load_state(state)
                        events=h.run([midi(16,0x90,60,100)])
                        ons={e[3] for e in events if e[0]=='on'}
                        expected=({60} if selection==1 else {60,64,67} if selection==2 else set())
                        if sequencing: expected.add(72)
                        assert ons==expected,('Play contract',mode,sequencing,strumming,selection,ons,expected)
                        if sequencing:
                            bypass=h.run([h.parameter('Sequencer',0),h.parameter('Strumfield enabled',0)])
                            assert not any(e[0]=='on' for e in bypass),('bypass started playback',bypass)
                            if selection in (1,2):
                                selected=(60,) if selection==1 else (60,64,67)
                                assert not any(e[0]=='off' and e[3] in selected for e in bypass),'power transition released selection playback'
                        for _ in range(30):
                            assert not any(e[0]=='on' for e in h.run()),'sequencer-off produced automatic notes'
                    finally:
                        h.close()
    print('CLAP Play contract passed: 36 combinations of Arp repeat, sequencer power, strum power, and selection playback; bypass preserves selected chords.')

if __name__=='__main__':
    path=pathlib.Path(sys.argv[1]).expanduser().resolve()
    if path.is_dir():path=path/'Contents'/'MacOS'/'Chordboard'
    if '--extensions' in sys.argv[2:]:
        run_extension_tests(path)
    elif '--sequencer' in sys.argv[2:]:
        run_sequencer_tests(path)
        run_play_contract_tests(path)
    elif '--split-controls' in sys.argv[2:]:
        run_split_tests(path)
    elif '--strum-controls' in sys.argv[2:]:
        run_strum_tests(path)
    elif '--held-chord' in sys.argv[2:]:
        run_held_chord_tests(path)
    else:
        run_tests(path)
