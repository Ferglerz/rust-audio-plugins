use openwurli_dsp::{gen_preamp::{self,CircuitState},tremolo::Tremolo,voice::Voice,oversampler::Oversampler};
use std::{hint::black_box,time::Instant};
fn main(){
 let sr=48000.0;let os=96000.0;let mut base=CircuitState::default();for _ in 0..176400{gen_preamp::process_sample(0.0,&mut base);}base.set_sample_rate(os);
 println!("voices,depth,main_maxiter,main_be_fallback,main_magnitude_reset,main_nan_reset,shadow_fallback,main_final_iterations_sum,rebuild_ns_per_host_sample,solve_ns_per_host_sample,input_peak,preamp_peak");
 for n in [0,1,6,32,64] {for depth in [0.0,0.5]{
 let mut main=base.clone();let mut shadow=base.clone();let mut trem=Tremolo::new(depth,os);let mut warm_r=0.0;
 for _ in 0..57600 {let r=trem.process();main.set_runtime_R_r_ldr(r);shadow.set_runtime_R_r_ldr(r);main.rebuild_pending_shared(&mut shadow);gen_preamp::process_sample(0.0,&mut main);gen_preamp::process_sample(0.0,&mut shadow);warm_r=r;}
 black_box(warm_r);main.diag_nr_max_iter_count=0;main.diag_be_fallback_count=0;main.diag_magnitude_reset_count=0;main.diag_nan_reset_count=0;shadow.diag_be_fallback_count=0;
 let notes:Vec<_>=if n<=6{[48,55,60,63,67,70][..n].to_vec()}else{(0..n).map(|i|33+i as u8).collect()};
 let mut voices:Vec<_>=notes.iter().enumerate().map(|(i,&note)|Voice::note_on(note,0.95f32 as f64,sr,(note as u32).wrapping_mul(2654435761).wrapping_add(i as u32+1),false)).collect();
 let mut up=Oversampler::new();let mut buf=[0.0;256];let mut sum=[0.0;256];let mut input=[0.0;512];let mut rebuild=0.0;let mut solve=0.0;let mut iterations=0u64;let mut peak=0.0f64;let mut outpeak=0.0f64;
 for _ in 0..94 {sum.fill(0.0);for v in &mut voices{v.render(&mut buf);for(s,&x)in sum.iter_mut().zip(&buf){*s+=x;}}up.upsample_2x(&sum,&mut input);
 for &x in &input{peak=peak.max(x.abs());let r=trem.process();let t=Instant::now();main.set_runtime_R_r_ldr(r);shadow.set_runtime_R_r_ldr(r);main.rebuild_pending_shared(&mut shadow);rebuild+=t.elapsed().as_secs_f64();let t=Instant::now();let a=gen_preamp::process_sample(x,&mut main)[0];let b=gen_preamp::process_sample(0.0,&mut shadow)[0];solve+=t.elapsed().as_secs_f64();outpeak=outpeak.max((a-b).abs());iterations+=main.last_nr_iterations as u64+1;black_box(a-b);}}
 let count=(94*256)as f64;
 println!("{n},{depth},{},{},{},{},{},{iterations},{:.2},{:.2},{peak:.6},{outpeak:.6}",main.diag_nr_max_iter_count,main.diag_be_fallback_count,main.diag_magnitude_reset_count,main.diag_nan_reset_count,shadow.diag_be_fallback_count,rebuild*1e9/count,solve*1e9/count);
 }}
}
