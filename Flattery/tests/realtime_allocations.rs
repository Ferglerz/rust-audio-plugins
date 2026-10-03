// The debug-alloc feature supplies its own global allocator through NIH-plug.
#![cfg(not(feature = "debug-alloc"))]

use flattery::{
    dsp::{Engine, Shared},
    params::FlatteryParams,
    strength::StrengthNode,
};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Barrier,
    },
    thread,
};

#[path = "support/legacy_strength.rs"]
mod legacy_strength;

struct CountingAllocator;

thread_local! {
    static COUNTS: Cell<(bool, usize, usize)> = const { Cell::new((false, 0, 0)) };
}

fn record(allocation: bool, deallocation: bool) {
    let _ = COUNTS.try_with(|counts| {
        let (enabled, allocated, freed) = counts.get();
        if enabled {
            counts.set((
                true,
                allocated + usize::from(allocation),
                freed + usize::from(deallocation),
            ));
        }
    });
}

// SAFETY: Every operation delegates the original pointer/layout unchanged to
// System; the thread-local bookkeeping neither allocates nor touches user memory.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record(true, false);
        // SAFETY: The caller supplies the valid allocation layout.
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record(true, false);
        // SAFETY: The caller supplies the valid allocation layout.
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        record(false, true);
        // SAFETY: The caller supplies a live System allocation and its layout.
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        record(true, true);
        // SAFETY: The caller supplies a live System allocation and valid new size.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn allocations_and_frees(f: impl FnOnce()) -> (usize, usize) {
    struct StopCounting;
    impl Drop for StopCounting {
        fn drop(&mut self) {
            COUNTS.with(|counts| {
                let (_, allocated, freed) = counts.get();
                counts.set((false, allocated, freed));
            });
        }
    }
    COUNTS.with(|counts| counts.set((true, 0, 0)));
    let stop = StopCounting;
    f();
    drop(stop);
    let (_, allocated, freed) = COUNTS.with(Cell::get);
    (allocated, freed)
}

fn assert_no_allocations_or_frees(f: impl FnOnce()) {
    let (allocated, freed) = allocations_and_frees(f);
    assert_eq!(
        (allocated, freed),
        (0, 0),
        "audio thread allocated or reclaimed heap memory"
    );
}

#[test]
#[ignore = "release allocation comparison; run explicitly with --release --ignored --nocapture"]
fn benchmark_legacy_and_prepared_curve_allocations() {
    assert!(!cfg!(debug_assertions), "benchmark requires --release");
    let nodes = nodes(2);
    let prepared = flattery::strength::PreparedStrengthCurve::new(Arc::clone(&nodes));
    let mut weights = vec![0.0; 4096];
    let mut radii = vec![0; 4096];
    let bin_hz = 48000.0 / 8192.0;
    let legacy_counts = allocations_and_frees(|| {
        for _ in 0..2 {
            legacy_strength::fill_bin_weights(&nodes, 4096, bin_hz, 10.0, 22050.0, &mut weights);
            legacy_strength::fill_bin_radii(&nodes, 4096, bin_hz, 3, &mut radii);
            std::hint::black_box((&weights, &radii));
        }
    });
    let prepared_counts = allocations_and_frees(|| {
        for _ in 0..2 {
            prepared.fill_bin_weights(4096, bin_hz, &mut weights);
            prepared.fill_bin_radii(4096, bin_hz, 3, &mut radii);
            std::hint::black_box((&weights, &radii));
        }
    });
    let shared = Arc::new(Shared::new());
    let mut engine = Engine::new(Arc::clone(&shared), 48000.0);
    shared.publish_nodes(true, Arc::clone(&nodes));
    shared.publish_nodes(false, nodes);
    let mut settings = FlatteryParams::default().process_settings();
    settings.fft_size = 8192;
    let engine_counts = allocations_and_frees(|| {
        for _ in 0..8192 {
            std::hint::black_box(engine.tick(0.2, -0.1, &settings));
        }
    });
    assert_eq!(legacy_counts, (8192, 8192));
    assert_eq!(prepared_counts, (0, 0));
    assert_eq!(engine_counts, (0, 0));
    println!("FFT 8192, two 2-node curves, allocations/frees: legacy hop {legacy_counts:?}; prepared rebuild {prepared_counts:?}; complete engine with FFT/node changes {engine_counts:?}");
}

fn nodes(count: usize) -> Arc<[StrengthNode]> {
    (0..count)
        .map(|i| {
            let mut node = StrengthNode::new(i as u64, 100.0 + ((i * 331) % 19000) as f64, 0.7);
            node.radius = i % 12 + 1;
            node
        })
        .collect::<Vec<_>>()
        .into()
}

#[test]
fn hops_curve_replacements_and_fft_changes_do_not_allocate_or_free() {
    let shared = Arc::new(Shared::new());
    let mut engine = Engine::new(Arc::clone(&shared), 48000.0);
    let mut settings = FlatteryParams::default().process_settings();
    engine.set_display_enabled(true);
    // Publish outside the measured audio region; snapshots may contain any node count.
    for count in [2, 1024, 0, 7] {
        shared.publish_nodes(true, nodes(count));
        shared.publish_nodes(false, nodes(count));
        assert_no_allocations_or_frees(|| {
            for _ in 0..settings.fft_size * 2 {
                std::hint::black_box(engine.tick(0.2, -0.1, &settings));
            }
        });
    }
    shared.publish_nodes(true, nodes(2));
    shared.publish_nodes(false, nodes(2));
    for fft_size in [128, 256, 512, 1024, 2048, 4096, 8192, 128] {
        settings.fft_size = fft_size;
        assert_no_allocations_or_frees(|| {
            engine.set_sample_rate(96000.0);
            engine.set_display_enabled(false);
            for _ in 0..fft_size {
                std::hint::black_box(engine.tick(0.2, -0.1, &settings));
            }
            engine.set_sample_rate(44100.0);
            engine.set_display_enabled(true);
            for _ in 0..fft_size {
                std::hint::black_box(engine.tick(0.2, -0.1, &settings));
            }
        });
    }
}

#[test]
fn a_full_retirement_queue_keeps_current_snapshot_without_reclamation() {
    let shared = Shared::new();
    let mut readers: Vec<_> = (0..9)
        .map(|_| shared.node_snapshot(true).unwrap())
        .collect();
    shared.publish_nodes(true, nodes(16));
    assert_no_allocations_or_frees(|| {
        for reader in &mut readers[..8] {
            assert!(shared.refresh_node_snapshot(true, reader));
        }
        assert!(!shared.refresh_node_snapshot(true, &mut readers[8]));
    });
    shared.publish_nodes(true, nodes(32));
    assert_no_allocations_or_frees(|| {
        assert!(shared.refresh_node_snapshot(true, &mut readers[8]));
    });
    assert_eq!(readers[8].nodes().len(), 32);
}

#[test]
fn racing_publication_reclaims_snapshots_only_on_the_publisher() {
    let shared = Arc::new(Shared::new());
    let mut current = shared.node_snapshot(true).unwrap();
    let barrier = Arc::new(Barrier::new(2));
    let done = Arc::new(AtomicBool::new(false));
    let publisher = {
        let shared = Arc::clone(&shared);
        let barrier = Arc::clone(&barrier);
        let done = Arc::clone(&done);
        thread::spawn(move || {
            barrier.wait();
            for count in 0..2000 {
                shared.publish_nodes(true, nodes(count % 31 + 1));
                thread::yield_now();
            }
            done.store(true, Ordering::Release);
        })
    };
    barrier.wait();
    assert_no_allocations_or_frees(|| {
        while !done.load(Ordering::Acquire) {
            shared.refresh_node_snapshot(true, &mut current);
            std::hint::spin_loop();
        }
        shared.refresh_node_snapshot(true, &mut current);
    });
    publisher.join().unwrap();
    assert!(!current.nodes().is_empty());
}
