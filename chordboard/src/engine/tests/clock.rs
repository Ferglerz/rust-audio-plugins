use super::super::*;
use super::support::*;

#[test]
fn learned_splits_swap_crossings_and_consume_the_learning_key_release() {
    for (target, note, expected) in [
        (5, 80, (60, 80)),
        (6, 20, (20, 36)),
        (5, 60, (60, 61)),
        (6, 36, (36, 37)),
    ] {
        let mut e = Engine::default();
        configure(
            &mut e,
            Config {
                mode: MANUAL,
                bass_split: 36,
                split_note: 60,
                key_split: true,
                ..Config::default()
            },
        );
        send(&mut e, Command::Learn(target));
        assert!(notes(&on(&mut e, note, 0)).is_empty());
        assert_eq!((e.config.bass_split as u8, e.config.split_note), expected);
        assert_eq!(e.learn, 0);
        assert_eq!(
            LearnedSplit::decode(e.learned_split.unwrap().encode()),
            e.learned_split
        );
        assert!(off(&mut e, note, 0).is_empty());
        assert!(e.root.is_none());
    }
}
#[test]
fn bass_bypass_releases_owned_bass_and_returns_low_notes_to_performance() {
    let mut e = Engine::default();
    let config = Config {
        mode: MANUAL,
        bass_split: 36,
        always_bass: true,
        ..Config::default()
    };
    configure(&mut e, config);
    assert_eq!(notes(&on(&mut e, 24, 0)), vec![24]);
    let mut released = Vec::new();
    e.configure(
        Config {
            bass_enabled: false,
            ..config
        },
        &mut |event| released.push(event),
    );
    assert!(released
        .iter()
        .any(|event| matches!(event, Out::Off(_, 24, _))));
    assert!(e.owned_output_notes().iter().flatten().all(|on| !on));
    assert!(notes(&on(&mut e, 24, 0)).is_empty());
    assert_eq!(e.root.unwrap().note, 24);
}
#[test]
fn split_order_always_leaves_one_chord_key() {
    for bass in 0..128 {
        for melody in 0..128 {
            let (low, high) = ordered_splits(bass, melody);
            assert!(low < high && high < 128);
        }
    }
}
#[test]
fn xy_cc_auto_resolution_is_channel_local_and_resets() {
    let mut e = Engine::default();
    let mut config = Config::default();
    config.mappings = [Mapping::cc(7, 16); 2];
    configure(&mut e, config);
    e.control(0, 7, 1.0, &mut |_| {});
    assert_eq!((e.x, e.y), (1.0, 1.0));
    e.control(0, 39, 0.0, &mut |_| {});
    let expected = 16256.0 / 16383.0;
    assert!((e.x - expected).abs() < 0.00001);
    assert!((e.y - expected).abs() < 0.00001);
    e.control(0, 39, 1.0, &mut |_| {});
    assert_eq!((e.x, e.y), (1.0, 1.0));
    e.control(1, 7, 64.0 / 127.0, &mut |_| {});
    assert!((e.x - 64.0 / 127.0).abs() < 0.00001);
    e.control(0, 121, 0.0, &mut |_| {});
    e.control(0, 7, 1.0, &mut |_| {});
    assert_eq!((e.x, e.y), (1.0, 1.0));
}
