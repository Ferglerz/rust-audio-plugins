#!/usr/bin/env python3
"""Compare actual Rust WPE windows on supplied mono speech and deterministic rooms.

Needs NumPy, SciPy, soundfile, and a release Rust toolchain. Audio stays in --out;
no recordings are committed or downloaded. The report is an engineering comparison,
not a perceptual listening test. Run from any directory; see README for usage.
"""
from pathlib import Path
import argparse
import json
import hashlib
import math
import os
import subprocess
import numpy as np
import scipy.signal as ss
import soundfile as sf

RATE = 48000

def metrics(y,target,tail):
 keep=slice(RATE*2,None)
 a=y[keep];b=target[keep]; dot=np.dot(a,b); ref=np.dot(b,b); gain=dot/ref
 error=np.dot(a-gain*b,a-gain*b);sisdr=10*np.log10(max(dot*dot/ref,1e-30)/max(error,1e-30))
 tail_db=10*np.log10(np.mean(y[tail]**2)+1e-30)
 _,_,a_spec=ss.stft(a,fs=RATE,nperseg=960,noverlap=480);_,_,b_spec=ss.stft(b,fs=RATE,nperseg=960,noverlap=480)
 active=np.mean(abs(b_spec)**2,axis=0)>np.max(np.mean(abs(b_spec)**2,axis=0))*1e-3
 bands=(np.fft.rfftfreq(960,1/RATE)>200)&(np.fft.rfftfreq(960,1/RATE)<8000)
 floor=np.max(abs(b_spec))*1e-4
 lsd=np.sqrt(np.mean((20*np.log10(np.maximum(abs(a_spec[bands][:,active]),floor)/np.maximum(abs(b_spec[bands][:,active]),floor)))**2))
 return dict(sisdr=float(sisdr),gain_db=float(20*np.log10(max(abs(gain),1e-20))),tail_db=float(tail_db),lsd=float(lsd))

def fixtures(voices):
 for voice, path in enumerate(voices, 1):
  x,rate=sf.read(path)
  if x.ndim != 1: raise ValueError(f'{path}: expected mono audio')
  divisor=math.gcd(RATE,rate);x=ss.resample_poly(x,RATE//divisor,rate//divisor)[:RATE*10]
  if len(x)<RATE*7 or not np.all(np.isfinite(x)) or max(abs(x))==0: raise ValueError('Each fixture needs at least seven seconds of finite, nonzero speech')
  # Four seconds to settle, then three-second utterance and a one-second tail.
  x=np.concatenate([x[:RATE*4], np.zeros(RATE//2),x[RATE*4:RATE*7],np.zeros(RATE)])
  x=x*(.5/max(abs(x)))
  for room,rt,seed in [('clean',0.,0),('small',.25,91),('medium',.55,92),('large',.9,93)]:
   if rt:
    rng=np.random.default_rng(seed);t=np.arange(round(rt*RATE*1.3))/RATE
    ir=ss.lfilter([.2,.3,.5],[1.],rng.standard_normal(len(t)))*np.exp(-np.log(1000)*t/rt)
    ir[:round(.035*RATE)]=0
    ir*=.7/np.linalg.norm(ir)
    ir[0]=1
    for delay,amp in [(.006,.25),(.014,-.18),(.024,.12)]:ir[round(delay*RATE)]+=amp
    early=ir.copy();early[round(.035*RATE):]=0
    wet=ss.fftconvolve(x,ir)[:len(x)];target=ss.fftconvolve(x,early)[:len(x)]
   else:wet=x;target=x
   tail=np.arange(len(x))>len(x)-int(.85*RATE)
   yield f'v{voice}-{room}',wet,target,tail


def main():
 parser=argparse.ArgumentParser(description=__doc__)
 parser.add_argument('--voices', nargs='+', type=Path, required=True)
 parser.add_argument('--out', type=Path, required=True)
 parser.add_argument('--skip-render', action='store_true', help='Score existing Rust renders')
 args=parser.parse_args(); root=args.out.resolve();root.mkdir(parents=True,exist_ok=True)
 cases=list(fixtures(args.voices))
 for name,x,target,tail in cases:
  (root/f'{name}-input.f32').write_bytes(x.astype('<f4').tobytes())
  sf.write(root/f'{name}-input.wav',x,RATE,subtype='FLOAT')
  sf.write(root/f'{name}-target.wav',target,RATE,subtype='FLOAT')
 if not args.skip_render:
  with (root/'render.log').open('w') as log:
   subprocess.run(['cargo','test','-p','dereverb','render_wpe_window_study','--release','--quiet','--offline','--','--ignored','--nocapture'],
    cwd=Path(__file__).resolve().parents[2], env={**os.environ,'DEREVERB_WINDOW_STUDY':str(root)}, stdout=log,stderr=log,check=True)
 rows=[]
 for name,x,target,tail in cases:
  rows.append(dict(fixture=name,window_ms=0,memory_seconds=0,**metrics(x,target,tail)))
  for fft,memory in [(960,0),(720,0),(480,0),(360,0),(240,0),(120,0),(480,.5),(240,.5),(120,.5)]:
   y=np.fromfile(root/f'{name}-rust-{fft}-{memory}.f32',dtype='<f4').astype(float)
   if len(y)!=len(x) or not np.all(np.isfinite(y)):raise ValueError(f'Invalid render: {name}')
   rows.append(dict(fixture=name,window_ms=fft/48,memory_seconds=memory,**metrics(y,target,tail)))
 report=dict(voice_sha256=[hashlib.sha256(p.read_bytes()).hexdigest() for p in args.voices],sample_rate=RATE,room_rt60_seconds=[0,.25,.55,.9],warmup_excluded_seconds=2,
  target='Direct sound plus the first 35 ms of the impulse response',
  note='Two metrics cannot prove perceptual equivalence. Lower tail dB and higher SI-SDR are better; clean speech measures collateral distortion.',results=rows)
 (root/'report.json').write_text(json.dumps(report,indent=2)+'\n')
 baseline={r['fixture']:r for r in rows if r['window_ms']==20}
 short=[r for r in rows if r['window_ms']==10 and r['memory_seconds']==.5]
 rooms=[r for r in short if not r['fixture'].endswith('clean')]
 print(f"10 ms: mean SI-SDR change {np.mean([r['sisdr']-baseline[r['fixture']]['sisdr'] for r in rooms]):+.3f} dB; worst tail increase {max(r['tail_db']-baseline[r['fixture']]['tail_db'] for r in rooms):+.3f} dB")
 # Same time ranges and one common gain for comparison; no per-output matching.
 previews={label:[] for label in ['input','original-20ms','short-10ms']}
 for name,x,_,_ in cases:
  if not name.endswith('medium'):continue
  sounds=[x,np.fromfile(root/f'{name}-rust-960-0.f32',dtype='<f4'),np.fromfile(root/f'{name}-rust-480-0.5.f32',dtype='<f4')]
  for label,y in zip(previews,sounds):previews[label].append(np.r_[y[int(4.5*RATE):],np.zeros(RATE//2)])
 peak=max(np.max(abs(np.concatenate(y))) for y in previews.values())
 for label,parts in previews.items():sf.write(root/f'compare-{label}.wav',np.concatenate(parts)*(.9/peak),RATE,subtype='PCM_24')
 print(f'Report and aligned listening comparisons: {root}')

if __name__=='__main__':
 main()
