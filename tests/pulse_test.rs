use nes_apu_core::pulse::{PulseChannel, PULSE_DUTY_TABLE};
use nes_apu_core::NTSC_CPU_CLOCK;

#[test]
fn test_pulse_timer_reload_and_countdown() {
    let mut pulse = PulseChannel::new(0);
    pulse.enabled = true;
    pulse.set_volume(15);

    // Period t = 10
    pulse.write_register(0, 0x8F); // Duty 2 (50%), Constant volume 15
    pulse.write_register(2, 10);   // Timer Low = 10
    pulse.write_register(3, 0);    // Timer High = 0, reset sequencer_step = 0

    assert_eq!(pulse.timer_period, 10);
    assert_eq!(pulse.sequencer_step, 0);

    // 1 APU サイクルで timer が 10 -> 9 にデクリメント
    pulse.clock_timer();
    assert_eq!(pulse.timer, 10); // 最初は timer=0 だったので 10 にリロードされ、step が進む
    assert_eq!(pulse.sequencer_step, 7); // 0 -> 7

    // 次の 10 回で 9 -> 8 -> ... -> 0
    for i in (0..10).rev() {
        pulse.clock_timer();
        assert_eq!(pulse.timer, i);
    }

    // 次のクロックで再び 0 -> 10 にリロードされ、step が 7 -> 6 に進む
    pulse.clock_timer();
    assert_eq!(pulse.timer, 10);
    assert_eq!(pulse.sequencer_step, 6);
}

#[test]
fn test_pulse_all_duty_cycles() {
    // 4種類のデューティ比のパターンを確認
    for duty in 0..4 {
        let mut pulse = PulseChannel::new(0);
        pulse.enabled = true;
        pulse.set_volume(12);
        pulse.timer_period = 10;
        pulse.duty = duty;
        pulse.sequencer_step = 0;

        // 8ステップ分の出力を記録
        let mut observed_bits = Vec::new();
        for _ in 0..8 {
            let out = pulse.output();
            observed_bits.push(if out > 0 { 1 } else { 0 });
            // タイマーをゼロにしてステップ進行
            pulse.timer = 0;
            pulse.clock_timer();
        }

        // 実機の逆順シーケンサ (0, 7, 6, 5, 4, 3, 2, 1) と一致することを確認
        let expected: Vec<u8> = [0, 7, 6, 5, 4, 3, 2, 1]
            .iter()
            .map(|&step| PULSE_DUTY_TABLE[duty][step])
            .collect();

        assert_eq!(observed_bits, expected, "Duty {} sequence mismatch", duty);

        // 1の割合を確認
        let ones: usize = observed_bits.iter().filter(|&&b| b == 1).count();
        match duty {
            0 => assert_eq!(ones, 1, "Duty 0 must be 12.5% (1/8)"),
            1 => assert_eq!(ones, 2, "Duty 1 must be 25.0% (2/8)"),
            2 => assert_eq!(ones, 4, "Duty 2 must be 50.0% (4/8)"),
            3 => assert_eq!(ones, 6, "Duty 3 must be 75.0% (6/8)"),
            _ => unreachable!(),
        }
    }
}

#[test]
fn test_pulse_hardware_mute_period_under_8() {
    let mut pulse = PulseChannel::new(0);
    pulse.enabled = true;
    pulse.set_volume(15);
    pulse.duty = 2; // 50%
    pulse.sequencer_step = 1; // 出力 HIGH の位置

    // t < 8 のとき、実機ハードウェアは強制消音 (0) を出力する
    for t in 0..8 {
        pulse.timer_period = t;
        assert_eq!(pulse.output(), 0, "Period {} must be hardware muted", t);
    }

    // t >= 8 では正常に音量が出力される
    pulse.timer_period = 8;
    assert_eq!(pulse.output(), 15);
}

#[test]
fn test_pulse_frequency_cycle_accuracy() {
    let mut pulse = PulseChannel::new(0);
    pulse.enabled = true;
    pulse.set_volume(15);
    pulse.duty = 2;

    // A4 (440Hz): t = 253
    // 1周期 = 16 * (t + 1) = 16 * 254 = 4064 CPU サイクル (= 2032 APU サイクル)
    let t = 253u16;
    pulse.timer_period = t;
    pulse.timer = t;
    pulse.sequencer_step = 0;

    // 8ステップすべてが1巡するのに必要な APU サイクル数をカウント
    let mut apu_cycles = 0;
    let initial_step = pulse.sequencer_step;

    loop {
        pulse.clock_timer();
        apu_cycles += 1;
        if pulse.sequencer_step == initial_step && pulse.timer == t {
            break;
        }
    }

    // 8ステップ * (253 + 1) = 2032 APU サイクル
    assert_eq!(apu_cycles, 8 * (t as usize + 1));
    assert_eq!(apu_cycles * 2, 4064, "Total CPU cycles must be 4064");

    // 計算周波数が ~440.4 Hz であることを確認
    let freq = pulse.calculate_frequency(NTSC_CPU_CLOCK);
    assert!((freq - 440.4).abs() < 0.5);
}
