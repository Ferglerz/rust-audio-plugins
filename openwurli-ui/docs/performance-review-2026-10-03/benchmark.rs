use openwurli_dsp::{CircuitMode, WurliEngine};
use std::{hint::black_box, time::Instant};
fn quantile(x: &mut [f64], q: f64) -> f64 { x.sort_by(f64::total_cmp); x[((x.len()-1) as f64*q) as usize] }
fn notes(n: usize) -> Vec<u8> { if n <= 6 { [48,55,60,63,67,70][..n].to_vec() } else { (0..n).map(|i| 33+i as u8).collect() } }
fn main() {
 let args: Vec<String> = std::env::args().collect();
 if args.get(1).is_some_and(|x|x=="stages") { stages(); return; }
 let rounds: usize = args.get(1).and_then(|x|x.parse().ok()).unwrap_or(5);
 let sr = 48000.0;
 println!("mode,voices,depth,workload,median_ns_per_sample,p99_block_us,max_block_us,checksum,nan,amp_clamps,amp_maxiter");
 for n in [0,1,6,32,64] { for depth in [0.0,0.5] { for mode in [CircuitMode::Fast,CircuitMode::Heavy] {
 let mut costs=vec![]; let mut blocks=vec![]; let mut hash=0u64; let mut nan=0; let mut diag=(0,0,0.0);
 for _ in 0..rounds {
 let mut e=WurliEngine::new_with_circuit_mode(sr, mode); e.set_tremolo_depth(depth); e.set_noise_enabled(false); e.warm_up();
 for note in notes(n) {e.note_on(note,0.95);}
 let mut b=[0.0f32;256]; let mut elapsed=0.0; let mut count=0;
 for _ in 0..94 {let t=Instant::now(); e.render(black_box(&mut b));let dt=t.elapsed().as_secs_f64(); elapsed+=dt;blocks.push(dt*1e6);count+=256;for s in &b {hash=hash.wrapping_mul(16777619)^s.to_bits() as u64;}}
 costs.push(elapsed*1e9/count as f64);nan+=e.nan_guard_fires();diag=e.power_amp_diag();
 }
 let med=quantile(&mut costs,0.5);let p99=quantile(&mut blocks,0.99);let max=*blocks.last().unwrap();
 println!("{mode:?},{n},{depth},held,{med:.2},{p99:.2},{max:.2},{hash},{nan},{},{}",diag.0,diag.1);
 }}}
}
fn stages() {
 use openwurli_dsp::{voice::Voice, oversampler::Oversampler,tremolo::Tremolo,preamp::PreampModel,dk_preamp::{FastPreamp,HeavyPreamp},power_amp::{FastPowerAmp,HeavyPowerAmp},tables};
 let sr=48000.0;let os=96000.0;let len=48000usize;
 println!("stage,depth,median_ns_per_host_sample");
 for depth in [0.0,0.5] {
 let mut sum=vec![0.0;len];let mut scratch=vec![0.0;len];
 let mut vt=vec![];
 for _ in 0..7 {sum.fill(0.0);let mut voices:Vec<_>=notes(6).iter().enumerate().map(|(i,&note)|Voice::note_on(note,0.95,sr,(note as u32).wrapping_mul(2654435761).wrapping_add(i as u32+1),false)).collect();let t=Instant::now();for v in &mut voices {for block in scratch.chunks_mut(256){v.render(black_box(block));}for(a,b)in sum.iter_mut().zip(&scratch){*a+=*b;}}vt.push(t.elapsed().as_secs_f64()*1e9/len as f64);}
 println!("voices_6,{depth},{:.2}",quantile(&mut vt,0.5));
 let mut input=vec![0.0;len*2];let mut up=Oversampler::new();up.upsample_2x(&sum,&mut input);
 let mut ldr=vec![0.0;len*2];let mut times=vec![];
 for _ in 0..7 {let mut tr=Tremolo::new(depth,os);for _ in 0..57600 {black_box(tr.process());}let t=Instant::now();for r in &mut ldr {*r=tr.process();}times.push(t.elapsed().as_secs_f64()*1e9/len as f64);black_box(&ldr);}
 println!("tremolo,{depth},{:.2}",quantile(&mut times,0.5));
 for heavy in [false,true] {
 let mut output=vec![0.0;input.len()];let mut pt=vec![];let mut at=vec![];
 for _ in 0..7 {
 let mut pre:Box<dyn PreampModel>=if heavy {Box::new(HeavyPreamp::new(os))} else {Box::new(FastPreamp::new(os))};
 for &r in ldr.iter().take(57600) {pre.set_ldr_resistance(r);black_box(pre.process_sample(0.0));}
 let t=Instant::now();for ((&x,&r),y) in input.iter().zip(&ldr).zip(&mut output) {pre.set_ldr_resistance(r);*y=pre.process_sample(x)*tables::FIXED_CIRCUIT_DRIVE;}pt.push(t.elapsed().as_secs_f64()*1e9/len as f64);black_box(&output);
 if heavy {let mut amp=HeavyPowerAmp::new_at_sample_rate(os);for _ in 0..57600 {black_box(amp.process(0.0));}let t=Instant::now();for &x in &output {black_box(amp.process(x));}at.push(t.elapsed().as_secs_f64()*1e9/len as f64);}else{let mut amp=FastPowerAmp::new_at_sample_rate(os);let t=Instant::now();for &x in &output {black_box(amp.process(x));}at.push(t.elapsed().as_secs_f64()*1e9/len as f64);}
 }
 let m=if heavy{"heavy"}else{"fast"};println!("{m}_preamp,{depth},{:.2}",quantile(&mut pt,0.5));println!("{m}_amp,{depth},{:.2}",quantile(&mut at,0.5));
 }
 }
}
