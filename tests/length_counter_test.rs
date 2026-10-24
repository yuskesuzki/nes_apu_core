use nes_apu_core::length_counter::LengthCounter;

#[test]
fn test_length_counter_table_loading() {
    let mut lc = LengthCounter::new();
    lc.set_enabled(true);

    // インデックス 0 -> 10
    lc.load(0);
    assert_eq!(lc.counter, 10);

    // インデックス 1 -> 254
    lc.load(1);
    assert_eq!(lc.counter, 254);

    // インデックス 31 -> 30
    lc.load(31);
    assert_eq!(lc.counter, 30);
    assert!(!lc.is_silenced());
}

#[test]
fn test_length_counter_countdown_and_mute() {
    let mut lc = LengthCounter::new();
    lc.set_enabled(true);
    lc.load(3); // 2 カウント

    assert_eq!(lc.counter, 2);
    assert!(!lc.is_silenced());

    lc.clock();
    assert_eq!(lc.counter, 1);
    assert!(!lc.is_silenced());

    lc.clock();
    assert_eq!(lc.counter, 0);
    assert!(lc.is_silenced(), "Counter at 0 must silence the channel");

    // 0 からはこれ以上デクリメントしない
    lc.clock();
    assert_eq!(lc.counter, 0);
}

#[test]
fn test_length_counter_halt() {
    let mut lc = LengthCounter::new();
    lc.set_enabled(true);
    lc.load(0); // 10
    lc.halt = true;

    // halt=true の間はクロックが来ても減算されない
    for _ in 0..20 {
        lc.clock();
        assert_eq!(lc.counter, 10);
    }
}
