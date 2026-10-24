use nes_apu_core::{NonlinearMixer, HighPassFilter, LowPassFilter, AnalogFilterChain};

#[test]
fn test_nonlinear_mixer_pulse_table() {
    let mixer = NonlinearMixer::new();

    // Sum = 0 -> 0.0
    assert_eq!(mixer.mix_pulse(0, 0), 0.0);

    // Single channel output
    let p1_only = mixer.mix_pulse(15, 0);
    assert!(p1_only > 0.0 && p1_only < 0.35);

    // Sum of two channels: 15 + 15 = 30
    let p_both = mixer.mix_pulse(15, 15);
    // Non-linear compression: 2 channels at 15 is LESS than 2 * single channel
    assert!(p_both < 2.0 * p1_only);
    // Exact formula check: 95.88 / ((8128.0 / 30.0) + 100.0) = 95.88 / (270.933 + 100) ≈ 0.2585
    let expected = 95.88 / ((8128.0 / 30.0) + 100.0);
    assert!((p_both - expected).abs() < 1e-4);
}

#[test]
fn test_nonlinear_mixer_tnd() {
    let mixer = NonlinearMixer::new();

    // All zero -> 0.0
    assert_eq!(mixer.mix_tnd(0, 0, 0), 0.0);

    let tri_only = mixer.mix_tnd(15, 0, 0);
    let noi_only = mixer.mix_tnd(0, 15, 0);
    let dmc_only = mixer.mix_tnd(0, 0, 127);

    assert!(tri_only > 0.0);
    assert!(noi_only > 0.0);
    assert!(dmc_only > 0.0);

    // When all three are sounding, output compresses non-linearly
    let all_sound = mixer.mix_tnd(15, 15, 127);
    assert!(all_sound < tri_only + noi_only + dmc_only);
    assert!(all_sound > 0.0 && all_sound < 1.0);
}

#[test]
fn test_high_pass_filter_dc_blocking() {
    // 90 Hz HPF at 44.1 kHz sample rate
    let mut hpf = HighPassFilter::new(44100.0, 90.0);

    // Feed a constant DC voltage of 1.0 for 2,000 samples (~45ms)
    let mut last_y = 0.0;
    for _ in 0..2000 {
        last_y = hpf.process(1.0);
    }

    // DC must be almost completely blocked (decayed towards 0.0)
    assert!(last_y.abs() < 0.05, "HPF should block constant DC input, got {}", last_y);
}

#[test]
fn test_low_pass_filter_high_frequency_attenuation() {
    // 14 kHz LPF at 44.1 kHz sample rate
    let mut lpf = LowPassFilter::new(44100.0, 14000.0);

    // Feed a slow signal (constant 1.0) -> output should reach 1.0
    for _ in 0..100 {
        lpf.process(1.0);
    }
    assert!((lpf.process(1.0) - 1.0).abs() < 0.01);

    // Reset and feed Nyquist frequency alternating signal (+1.0, -1.0)
    lpf.reset();
    let mut output_mag = 0.0;
    for i in 0..100 {
        let x = if i % 2 == 0 { 1.0 } else { -1.0 };
        output_mag = lpf.process(x).abs();
    }
    // High frequencies must be attenuated
    assert!(output_mag < 0.5, "LPF should attenuate high frequency alternating input, got {}", output_mag);
}

#[test]
fn test_analog_filter_chain_end_to_end() {
    let mut chain = AnalogFilterChain::new(44100.0);

    // Process 4,000 samples of a DC offset square wave (0.0 to 0.8)
    let mut out = 0.0;
    for i in 0..4000 {
        let val = if (i / 50) % 2 == 0 { 0.8 } else { 0.0 };
        out = chain.process(val);
    }

    // Output should be AC-coupled (centered around 0.0)
    assert!(out.abs() <= 1.0);

    // Test bypass mode
    chain.bypass = true;
    let bypassed = chain.process(0.8);
    // Bypassed formula: (0.8 - 0.5) * 2.0 = 0.6
    assert!((bypassed - 0.6).abs() < 1e-4);
}
