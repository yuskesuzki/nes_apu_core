use nes_apu_core::{
    ApuSystem, NesApuCore, NesSoundDriver, note_to_nes_timer, note_to_triangle_timer,
    NTSC_CPU_CLOCK, NTSC_CYCLES_PER_FRAME,
};

#[test]
fn test_note_to_nes_timer_pitch() {
    // A4 = 440 Hz -> timer period = 253
    let timer_a4 = note_to_nes_timer(45); // A4 (MIDI 69 - 24 = 45)
    assert_eq!(timer_a4, 253);

    // C4 = ~261.63 Hz -> timer period = 427
    let timer_c4 = note_to_nes_timer(36); // C4 (MIDI 60 - 24 = 36)
    assert_eq!(timer_c4, 427);

    // A2 = 110 Hz -> timer period = 1016
    let timer_a2 = note_to_nes_timer(21);
    assert_eq!(timer_a2, 1016);
}

#[test]
fn test_voice_macro_modulation() {
    let mut driver = NesSoundDriver::default();
    // Track 0 has Note 48 (C5) with Voice 6 (duty_sweep_lead)
    driver.bytecode[0] = vec![
        0x86, // Voice 6: duty_macro = [0, 0, 1, 1, 2, 2, 1, 1]
        48, 10, // Note 48 for 10 frames
        0xFF,
    ];
    driver.reset();

    // Frame 1
    let writes1 = driver.tick_frame();
    // Duty should be 0 (12.5%), Vol should be 14 ($4000 = (0 << 6) | 0x30 | 14 = 0x3E)
    let w_4000 = writes1.iter().find(|(a, _)| *a == 0x4000).unwrap().1;
    assert_eq!(w_4000 & 0xC0, 0x00, "Frame 1 duty should be 0 (12.5%)");
    assert_eq!(w_4000 & 0x0F, 14, "Frame 1 volume should be 14");

    // Frame 2
    let writes2 = driver.tick_frame();
    let w_4000_2 = writes2.iter().find(|(a, _)| *a == 0x4000).unwrap().1;
    assert_eq!(w_4000_2 & 0xC0, 0x00, "Frame 2 duty should be 0 (12.5%)");
    assert_eq!(w_4000_2 & 0x0F, 13, "Frame 2 volume should decay to 13");

    // Frame 3
    let writes3 = driver.tick_frame();
    let w_4000_3 = writes3.iter().find(|(a, _)| *a == 0x4000).unwrap().1;
    assert_eq!(w_4000_3 & 0xC0, 0x40, "Frame 3 duty should rotate to 1 (25%)");
    assert_eq!(w_4000_3 & 0x0F, 12, "Frame 3 volume should decay to 12");
}

#[test]
fn test_cycle_accurate_vblank_trigger() {
    let mut driver = NesSoundDriver::default();
    driver.bytecode[0] = vec![
        0x80,
        48, 10,
        0xFF,
    ];
    let mut sys = ApuSystem::new(Box::new(driver));

    // Initial state: Pulse 1 enabled is false or volume 0
    assert_eq!(sys.apu.total_cpu_cycles, 0);

    // Run 29,779 CPU cycles (1 cycle before V-Blank)
    sys.clock_cycles(NTSC_CYCLES_PER_FRAME - 1);
    assert_eq!(sys.vblank_cycle_counter, NTSC_CYCLES_PER_FRAME - 1);
    // Driver should not have ticked yet (frame_counter == 0)
    assert_eq!(sys.source.driver_state().unwrap().frame_counter, 0);

    // Exactly on cycle 29,780, V-Blank occurs!
    sys.step();
    assert_eq!(sys.vblank_cycle_counter, 0);
    assert_eq!(sys.source.driver_state().unwrap().frame_counter, 1);

    // Pulse 1 should now be actively programmed and enabled
    assert!(sys.apu.pulse1.enabled);
    assert_eq!(sys.apu.pulse1.timer_period, note_to_nes_timer(48));
}

#[test]
fn test_pulse_no_phase_reset_on_sustained_note() {
    let mut driver = NesSoundDriver::default();
    driver.bytecode[0] = vec![
        0x80,
        45, 10, // A4 (440Hz) sustained for 10 frames
        0xFF,
    ];
    driver.reset();

    // Frame 1 (Note-on): must write $4003 to load timer high & initialize sequencer
    let writes1 = driver.tick_frame();
    assert!(
        writes1.iter().any(|(a, _)| *a == 0x4003),
        "Note-on frame must write $4003"
    );

    // Frame 2..10 (Sustain with unchanged high timer byte): must NOT write $4003
    // so that the 8-step duty sequencer is not forcibly reset at 60Hz
    for f in 2..=10 {
        let writes = driver.tick_frame();
        assert!(
            !writes.iter().any(|(a, _)| *a == 0x4003),
            "Sustained frame {} must not write $4003 to avoid 60Hz phase-reset distortion",
            f
        );
    }
}

#[test]
fn test_triangle_timer_pitch_accuracy() {
    // A2 = 110 Hz (note 21): Triangle is 32-step CPU-clocked, so timer = 507 (~110.1 Hz)
    let timer_tri_a2 = note_to_triangle_timer(21);
    assert_eq!(timer_tri_a2, 507);

    let mut apu = NesApuCore::new();
    apu.write_register(0x4015, 0x04);
    apu.write_register(0x4008, 0xFF);
    apu.write_register(0x400A, (timer_tri_a2 & 0xFF) as u8);
    apu.write_register(0x400B, ((timer_tri_a2 >> 8) & 0x07) as u8);

    let freq = apu.triangle.calculate_frequency(NTSC_CPU_CLOCK);
    assert!((freq - 110.0).abs() < 0.5, "Triangle A2 frequency should be ~110Hz, got {}", freq);
}
