use nes_apu_core::triangle::{TriangleChannel, TRIANGLE_SEQUENCE};
use nes_apu_core::NTSC_CPU_CLOCK;

#[test]
fn test_triangle_32_step_waveform() {
    let mut tri = TriangleChannel::new();
    tri.enabled = true;
    tri.timer_period = 2; // 最小有効周期
    tri.timer = 0;
    tri.sequencer_step = 0;

    let mut waveform = Vec::new();

    // 32ステップ分の出力をサンプリング
    for _ in 0..32 {
        waveform.push(tri.output());
        tri.timer = 0;
        tri.clock_timer();
    }

    // 15 -> 0 -> 15 の階段波形と完全一致することを確認
    assert_eq!(waveform, TRIANGLE_SEQUENCE);
    assert_eq!(waveform[0], 15);
    assert_eq!(waveform[15], 0);
    assert_eq!(waveform[16], 0);
    assert_eq!(waveform[31], 15);
}

#[test]
fn test_triangle_clocked_every_cpu_cycle() {
    let mut tri = TriangleChannel::new();
    tri.enabled = true;
    tri.timer_period = 10;
    tri.timer = 10;
    tri.sequencer_step = 0;

    // Triangle は 毎 CPU サイクル でデクリメント
    for i in (0..10).rev() {
        tri.clock_timer();
        assert_eq!(tri.timer, i);
    }

    // 次の 1 CPU サイクルで 0 -> 10 にリロードされ、ステップが 0 -> 1 に進む
    tri.clock_timer();
    assert_eq!(tri.timer, 10);
    assert_eq!(tri.sequencer_step, 1);
}

#[test]
fn test_triangle_frequency_cycle_accuracy() {
    let mut tri = TriangleChannel::new();
    tri.enabled = true;

    // A2 (110Hz): t = 507
    // 1波形の全周期 = 32 * (t + 1) CPU サイクル
    let t = 507u16;
    tri.timer_period = t;
    tri.timer = t;
    tri.sequencer_step = 0;

    let mut cpu_cycles = 0;
    let initial_step = tri.sequencer_step;

    loop {
        tri.clock_timer();
        cpu_cycles += 1;
        if tri.sequencer_step == initial_step && tri.timer == t {
            break;
        }
    }

    assert_eq!(cpu_cycles, 32 * (t as usize + 1));
    assert_eq!(cpu_cycles, 32 * 508);

    let freq = tri.calculate_frequency(NTSC_CPU_CLOCK);
    assert!((freq - 110.1).abs() < 0.5);
}

#[test]
fn test_triangle_dc_voltage_hold_when_halted() {
    let mut tri = TriangleChannel::new();
    tri.enabled = true;
    tri.timer_period = 5;
    tri.timer = 0;
    tri.sequencer_step = 0;

    // ステップ 10 まで進行 (10 のときの電圧値は TRIANGLE_SEQUENCE[10] = 5)
    for _ in 0..10 {
        tri.timer = 0;
        tri.clock_timer();
    }
    assert_eq!(tri.sequencer_step, 10);
    assert_eq!(tri.raw_output(), 5);

    // チャンネルを停止
    tri.enabled = false;

    // クロックが進行しても、ステップは 10 のまま変化せず停止位置の電圧を保持する
    for _ in 0..100 {
        tri.clock_timer();
    }
    assert_eq!(tri.sequencer_step, 10, "Halted triangle must retain its step position");
    assert_eq!(tri.raw_output(), 5, "Halted triangle must retain DC voltage level");
    assert_eq!(tri.output(), 5, "Halted triangle output() must continue holding DC voltage to avoid pop/click noise");
}
