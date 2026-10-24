use nes_apu_core::core::NesApuCore;

#[test]
fn test_apu_register_configuration() {
    let mut apu = NesApuCore::new();

    // Pulse 1 設定 ($4000: Duty 2 (50%), Vol 12)
    apu.write_register(0x4000, 0x9C);
    // Pulse 1 Timer ($4002, $4003): Period 253 (A4 440Hz)
    apu.write_register(0x4002, 253);
    apu.write_register(0x4003, 0);

    // Triangle 設定 ($4008, $400A, $400B): Period 507 (A2 110Hz)
    apu.write_register(0x4008, 0x80);
    apu.write_register(0x400A, (507 & 0xFF) as u8);
    apu.write_register(0x400B, (507 >> 8) as u8);

    // Noise 設定 ($400C: Vol 10, $400E: Short mode, Period index 4)
    apu.write_register(0x400C, 0x1A);
    apu.write_register(0x400E, 0x84);

    // 全チャンネル有効化 ($4015: P1, P2, Tri, Noise = 0x0F)
    apu.write_register(0x4015, 0x0F);

    assert!(apu.pulse1.enabled);
    assert_eq!(apu.pulse1.duty, 2);
    assert_eq!(apu.pulse1.volume(), 12);
    assert_eq!(apu.pulse1.timer_period, 253);

    assert!(apu.triangle.enabled);
    assert_eq!(apu.triangle.timer_period, 507);

    assert!(apu.noise.enabled);
    assert_eq!(apu.noise.volume(), 10);
    assert!(apu.noise.short_mode);
    assert_eq!(apu.noise.period_index, 4);
}

#[test]
fn test_apu_clock_stepping_and_divider() {
    let mut apu = NesApuCore::new();
    apu.write_register(0x4015, 0x0F);
    apu.write_register(0x4000, 0x9F); // P1 Vol 15
    apu.write_register(0x4002, 10);
    apu.write_register(0x4003, 0);

    apu.write_register(0x4008, 0xFF); // Tri
    apu.write_register(0x400A, 10);
    apu.write_register(0x400B, 0);

    assert_eq!(apu.total_cpu_cycles, 0);

    // 1 CPU サイクル実行
    apu.step();
    assert_eq!(apu.total_cpu_cycles, 1);
    assert!(apu.apu_cycle_toggle); // APU サイクル発火

    // 次の 1 CPU サイクル実行
    apu.step();
    assert_eq!(apu.total_cpu_cycles, 2);
    assert!(!apu.apu_cycle_toggle); // APU サイクル非発火 (2分周)

    // 1000 サイクル進行
    apu.clock_cycles(1000);
    assert_eq!(apu.total_cpu_cycles, 1002);
}

#[test]
fn test_audio_sample_generation() {
    let mut apu = NesApuCore::new();
    apu.write_register(0x4015, 0x0F);
    apu.write_register(0x4000, 0x9F); // P1 Vol 15
    apu.write_register(0x4002, 253);
    apu.write_register(0x4003, 0);

    // 44100 Hz のオーディオサンプルを 100 個生成
    for _ in 0..100 {
        let sample = apu.generate_audio_sample(44100.0);
        assert!(sample >= -1.0 && sample <= 1.0);
    }
}
