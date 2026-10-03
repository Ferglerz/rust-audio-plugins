use std::{hint::black_box,time::Instant};
fn main(){
 let mut bits=0u64;let mut entries=0usize;
 for sr in [44100.0,48000.0,88200.0,96000.0,192000.0] {
 let mut bytes=0; let mut normal=0;
 for note in 0..92u8 {let (rates,ramp)=params(note,sr);let table=bake(rates,ramp);bytes+=table.len()*7*8;if note>=33 {normal+=table.len()*7*8;}
 let mut a=[1.0f64;7];let mut b=a;let mut t=0.0;
 for row in &table {t+=1.0;for i in 0..7 {let inst_rate=black_box(rates[i])*t/ramp;a[i]*=(-inst_rate).exp();b[i]*=row[i];bits+=(a[i].to_bits()!=b[i].to_bits())as u64;entries+=1;}}
 }
 println!("rate={sr}, bytes_extended={bytes}, bytes_normal={normal}");
 }
 println!("compared_envelope_steps={entries}, differing_bits={bits}");
 for note in [36,60,84] {let(rates,ramp)=params(note,48000.0);let table=bake(rates,ramp);let mut direct=vec![];let mut cached=vec![];
 for rep in 0..11 {for cache in if rep%2==0 {[false,true]}else{[true,false]} {let rates=black_box(rates);let ramp=black_box(ramp);let start=Instant::now();for _ in 0..512 {let mut e=black_box([1.0f64;7]);let mut t=0.0;for row in &table {t+=1.0;for i in 0..7 {e[i]*=if cache {row[i]}else{{let inst_rate=rates[i]*t/ramp;(-inst_rate).exp()}};}black_box(e);}black_box(e);}let ns=start.elapsed().as_secs_f64()*1e9/(512*table.len())as f64;if cache {cached.push(ns)}else{direct.push(ns)}}}
 direct.sort_by(f64::total_cmp);cached.sort_by(f64::total_cmp);println!("note={note}, original_ns_per_ramp_sample={}, baked_ns_per_ramp_sample={}",direct[5],cached[5]);}
}
fn params(note:u8,sr:f64)->([f64;7],f64){let base=(55.0*2.0_f64.powf((note as f64-60.0)/24.0)).max(0.5);let rates=std::array::from_fn(|m|(base*3.0_f64.powi(m as i32)).min(2000.0)/sr);let time=if note<48 {0.050}else if note<72 {0.025}else{0.008};(rates,time*sr)}
fn bake(rates:[f64;7],ramp:f64)->Vec<[f64;7]>{let mut t=0.0;let mut rows=vec![];loop{t+=1.0;if t>ramp{break;}rows.push(std::array::from_fn(|i|{let rate=rates[i]*t/ramp;(-rate).exp()}));}rows}
