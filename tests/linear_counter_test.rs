use nes_apu_core::linear_counter::LinearCounter;

#[test]
fn test_linear_counter_reload_and_countdown() {
    let mut lc = LinearCounter::new();
    // リロード値 = 5, control_flag = false
    lc.write_control(5);
    assert_eq!(lc.counter, 0);
    assert!(!lc.is_active());

    // $400B 書き込み時のリロード要求
    lc.set_reload_flag();

    // クロックが来ると 5 がリロードされる
    lc.clock();
    assert_eq!(lc.counter, 5);
    assert!(lc.is_active());

    // control_flag=false のため、リロードフラグは自動クリアされ、以降は 1 ずつ減少
    lc.clock(); assert_eq!(lc.counter, 4);
    lc.clock(); assert_eq!(lc.counter, 3);
    lc.clock(); assert_eq!(lc.counter, 2);
    lc.clock(); assert_eq!(lc.counter, 1);
    lc.clock(); assert_eq!(lc.counter, 0);

    // 0 になると非アクティブ
    assert!(!lc.is_active());
}

#[test]
fn test_linear_counter_control_flag_hold() {
    let mut lc = LinearCounter::new();
    // control_flag = true ($4008 bit 7 = 1), reload_value = 10
    lc.write_control(0x80 | 10);
    lc.set_reload_flag();

    // control_flag=true の間はリロードフラグがクリアされないため、毎クロック 10 が維持される
    for _ in 0..10 {
        lc.clock();
        assert_eq!(lc.counter, 10);
    }
}
