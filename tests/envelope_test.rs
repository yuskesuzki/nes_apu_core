use nes_apu_core::envelope::EnvelopeGenerator;

#[test]
fn test_envelope_constant_volume() {
    let mut env = EnvelopeGenerator::new();
    // 固定音量モード ($4000 bit 4 = 1), 音量 = 9
    env.write_control(0x19);
    assert_eq!(env.output_volume(), 9);

    // クロックが進んでも音量は一定
    for _ in 0..50 {
        env.clock();
        assert_eq!(env.output_volume(), 9);
    }
}

#[test]
fn test_envelope_decay_and_restart() {
    let mut env = EnvelopeGenerator::new();
    // 減衰モード (bit 4 = 0), 分周周期 = 1 (V=1: 2クロックごとに1減衰), ループなし (bit 5 = 0)
    env.write_control(0x01);
    // 音符トリガー (第4レジスタ書き込みでリスタート)
    env.restart();

    // 最初は 15
    env.clock();
    assert_eq!(env.output_volume(), 15);

    // 分周周期 V=1 なので、2回目のクロックで 14 に減衰
    env.clock();
    assert_eq!(env.output_volume(), 15);
    env.clock();
    assert_eq!(env.output_volume(), 14);

    // 0 まで減衰させる
    for _ in 0..100 {
        env.clock();
    }
    assert_eq!(env.output_volume(), 0, "Non-loop envelope must stay at 0 after decaying");

    // 再度リスタートフラグを立てると即座に 15 に復帰
    env.restart();
    env.clock();
    assert_eq!(env.output_volume(), 15);
}

#[test]
fn test_envelope_loop() {
    let mut env = EnvelopeGenerator::new();
    // 減衰モード, V=0 (毎クロック減衰), ループ有効 (bit 5 = 1)
    env.write_control(0x20);
    env.restart();

    env.clock();
    assert_eq!(env.output_volume(), 15);

    // 15 回クロックで 14..0 へ
    for i in (0..15).rev() {
        env.clock();
        assert_eq!(env.output_volume(), i);
    }

    // 次のクロックでループして 15 にリロード
    env.clock();
    assert_eq!(env.output_volume(), 15, "Loop envelope must reload to 15 after reaching 0");
}
