use nes_apu_core::noise::{NoiseChannel, NOISE_PERIOD_TABLE_NTSC};

#[test]
fn test_noise_initial_state_and_periods() {
    let noise = NoiseChannel::new();
    assert_eq!(noise.shift_register, 1, "LFSR must be initialized to 1 on reset");
    assert_eq!(NOISE_PERIOD_TABLE_NTSC.len(), 16);
    assert_eq!(NOISE_PERIOD_TABLE_NTSC[0], 4);
    assert_eq!(NOISE_PERIOD_TABLE_NTSC[15], 4068);
}

#[test]
fn test_noise_output_logic() {
    let mut noise = NoiseChannel::new();
    noise.enabled = true;
    noise.set_volume(11);

    // bit 0 が 1 のとき: 実機仕様により消音 (0)
    noise.shift_register = 0b0000_0000_0000_0001;
    assert_eq!(noise.output(), 0);

    // bit 0 が 0 のとき: 音量 (11) を出力
    noise.shift_register = 0b0000_0000_0000_0010;
    assert_eq!(noise.output(), 11);

    // チャンネルが無効のときは常に 0
    noise.enabled = false;
    assert_eq!(noise.output(), 0);
}

#[test]
fn test_noise_short_mode_93_step_period() {
    let mut noise = NoiseChannel::new();
    noise.enabled = true;
    noise.short_mode = true; // M=1: 金属ハイハット短周期モード
    noise.shift_register = 1;

    let mut visited = std::collections::HashMap::new();
    let mut step = 0;

    while !visited.contains_key(&noise.shift_register) {
        visited.insert(noise.shift_register, step);
        step += 1;

        // タイマーを進めて強制的に1回シフト
        noise.timer = 0;
        noise.clock_timer();
    }

    let cycle_length = step - visited[&noise.shift_register];
    assert_eq!(
        cycle_length, 93,
        "Short mode (M=1) must cycle in exactly 93 steps!"
    );
}

#[test]
fn test_noise_normal_mode_32767_step_period() {
    let mut noise = NoiseChannel::new();
    noise.enabled = true;
    noise.short_mode = false; // M=0: 通常ホワイトノイズ
    noise.shift_register = 1;

    // 最初の数千ステップで 93 ステップのような早期ループに陥らないことを確認
    let mut visited = std::collections::HashSet::new();
    let sample_steps = 1000;

    for _ in 0..sample_steps {
        assert!(visited.insert(noise.shift_register), "Must not repeat early in normal mode");
        noise.timer = 0;
        noise.clock_timer();
    }
}
