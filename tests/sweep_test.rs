use nes_apu_core::sweep::SweepUnit;

#[test]
fn test_sweep_target_addition() {
    let mut sweep = SweepUnit::new(0); // Pulse 1
    // Enabled=1, Period=2, Negate=0, Shift=2
    sweep.write_register(0x80 | (2 << 4) | 2);

    let period = 200u16;
    // change = 200 >> 2 = 50
    // target = 200 + 50 = 250
    assert_eq!(sweep.calculate_target(period), 250);
}

#[test]
fn test_sweep_pulse1_vs_pulse2_negate_quirk() {
    // 【実機ファミコンの有名な 1の補数 vs 2の補数 スイープ非対称バグ挙動の検証】
    let mut sweep_p1 = SweepUnit::new(0); // Pulse 1 (チャンネル0)
    let mut sweep_p2 = SweepUnit::new(1); // Pulse 2 (チャンネル1)

    // Negate=1, Shift=1
    sweep_p1.write_register(0x80 | 0x08 | 1);
    sweep_p2.write_register(0x80 | 0x08 | 1);

    let period = 100u16;
    // change = 100 >> 1 = 50

    // Pulse 1 は 1の補数減算 (実機の加算器にキャリーが入らないため 1 多く減算される)
    // 100 - 50 - 1 = 49
    assert_eq!(
        sweep_p1.calculate_target(period), 49,
        "Pulse 1 must subtract using one's complement (period - change - 1)"
    );

    // Pulse 2 は 2の補数減算 (通常の減算)
    // 100 - 50 = 50
    assert_eq!(
        sweep_p2.calculate_target(period), 50,
        "Pulse 2 must subtract using two's complement (period - change)"
    );
}

#[test]
fn test_sweep_hardware_mute_conditions() {
    let mut sweep = SweepUnit::new(0);
    sweep.write_register(0x80 | 1); // Shift 1, Sweep up

    // 1. タイマーが 8 未満なら消音
    assert!(sweep.is_muting(7));
    assert!(!sweep.is_muting(8));

    // 2. 目標タイマーが 0x7FF (2047) を超えると消音
    // period = 1500 -> change = 750 -> target = 2250 > 2047
    assert!(sweep.is_muting(1500), "Target > 0x7FF must mute the channel");
}

#[test]
fn test_sweep_clock_updates_period() {
    let mut sweep = SweepUnit::new(0);
    // Enabled=1, Period=0 (毎ハーフフレーム更新), Negate=0, Shift=1
    sweep.write_register(0x80 | (0 << 4) | 1);

    let mut period = 100u16;

    // クロック発火で 100 + (100 >> 1) = 150 に更新
    let new_p = sweep.clock(period);
    assert_eq!(new_p, Some(150));
    period = 150;

    // 次のクロックで 150 + (150 >> 1) = 225 に更新
    let new_p2 = sweep.clock(period);
    assert_eq!(new_p2, Some(225));
}
