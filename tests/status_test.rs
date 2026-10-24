use nes_apu_core::NesApuCore;

#[test]
fn test_status_channel_enable_and_length_counter() {
    let mut apu = NesApuCore::new();

    // Enable Pulse 1 and Triangle only via $4015
    apu.write_register(0x4015, 0b0000_0101); // Pulse 1 (bit 0) and Triangle (bit 2)

    assert!(apu.pulse1.enabled);
    assert!(!apu.pulse2.enabled);
    assert!(apu.triangle.enabled);
    assert!(!apu.noise.enabled);
    assert!(!apu.dmc.enabled);

    // Load length counter for Pulse 1 ($4003) and Pulse 2 ($4007)
    apu.write_register(0x4003, 0x08); // load length
    apu.write_register(0x4007, 0x08);

    // Pulse 1 length is active, Pulse 2 was disabled so its length counter remains 0
    let status = apu.read_register(0x4015);
    assert_eq!(status & 0x01, 0x01, "Pulse 1 length counter flag should be set");
    assert_eq!(status & 0x02, 0x00, "Pulse 2 length counter flag should be clear");

    // Disabling Pulse 1 via $4015 should immediately clear its length counter
    apu.write_register(0x4015, 0x00);
    let status2 = apu.read_register(0x4015);
    assert_eq!(status2 & 0x01, 0x00, "Pulse 1 length counter should be zeroed after disabling");
}

#[test]
fn test_status_frame_irq_read_clears_flag() {
    let mut apu = NesApuCore::new();

    // 4-step mode with IRQ enabled ($4017 = 0x00)
    apu.write_register(0x4017, 0x00);

    // Clock APU to trigger frame counter IRQ (cycle 29828)
    apu.clock_cycles(29828);

    assert!(apu.frame_counter.irq_flag, "Frame counter IRQ flag should be set");
    assert!(apu.irq_pending());

    // Reading $4015 should return bit 6 = 1
    let status = apu.read_register(0x4015);
    assert_eq!(status & 0x40, 0x40, "Bit 6 of $4015 should indicate Frame IRQ");

    // CRITICAL HARDWARE QUIRK: Reading $4015 must clear Frame IRQ flag
    assert!(!apu.frame_counter.irq_flag, "Reading $4015 must clear Frame IRQ flag");
    assert!(!apu.irq_pending());

    // Second read of $4015 should now have bit 6 = 0
    let status2 = apu.read_register(0x4015);
    assert_eq!(status2 & 0x40, 0x00, "Bit 6 of $4015 should now be 0 after being acknowledged");
}

#[test]
fn test_status_dmc_irq_and_active_flag() {
    let mut apu = NesApuCore::new();
    let mem = vec![0x55; 0x10000];
    apu.dmc.attach_memory(mem);

    // Setup DMC: IRQ enable = 1, Loop = 0, Rate = 15 (54 cycles)
    apu.write_register(0x4010, 0x8F);
    apu.write_register(0x4012, 0x00); // $C000
    apu.write_register(0x4013, 0x01); // (1 * 16) + 1 = 17 bytes

    // Start DMC via $4015 bit 4
    apu.write_register(0x4015, 0x10);

    // While playing, $4015 bit 4 should be 1
    let status_active = apu.read_register(0x4015);
    assert_eq!(status_active & 0x10, 0x10, "DMC active bit 4 should be set");
    assert_eq!(status_active & 0x80, 0x00, "DMC IRQ bit 7 should not be set yet");

    // Run until sample finishes (17 bytes * 8 bits * 54 cycles = ~7,344 cycles)
    apu.clock_cycles(10000);

    // DMC should now be finished and DMC IRQ bit 7 should be set
    let status_irq = apu.read_register(0x4015);
    assert_eq!(status_irq & 0x80, 0x80, "DMC IRQ bit 7 should be set upon sample completion");
    assert_eq!(status_irq & 0x10, 0x00, "DMC active bit 4 should now be 0");

    // Unlike Frame IRQ, reading $4015 does NOT clear DMC IRQ
    let status_irq2 = apu.read_register(0x4015);
    assert_eq!(status_irq2 & 0x80, 0x80, "Reading $4015 must NOT clear DMC IRQ");

    // Writing $4015 clears DMC IRQ
    apu.write_register(0x4015, 0x00);
    let status_cleared = apu.read_register(0x4015);
    assert_eq!(status_cleared & 0x80, 0x00, "Writing $4015 must clear DMC IRQ flag");
}
