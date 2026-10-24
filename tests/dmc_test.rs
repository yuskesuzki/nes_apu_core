use nes_apu_core::{DmcChannel, DMC_NTSC_RATE_TABLE};

#[test]
fn test_dmc_direct_load_dac() {
    let mut dmc = DmcChannel::new();
    assert_eq!(dmc.output(), 0);

    // Write $4011: 7-bit value (0..127)
    dmc.write_register(1, 64);
    assert_eq!(dmc.output(), 64);

    dmc.write_register(1, 127);
    assert_eq!(dmc.output(), 127);

    // Values with bit 7 set should be masked to 7-bit
    dmc.write_register(1, 0xFF);
    assert_eq!(dmc.output(), 127);
}

#[test]
fn test_dmc_rate_table_and_periods() {
    let mut dmc = DmcChannel::new();

    // Check default rate index 0
    assert_eq!(dmc.rate_period, DMC_NTSC_RATE_TABLE[0]);
    assert_eq!(DMC_NTSC_RATE_TABLE[0], 428);
    assert_eq!(DMC_NTSC_RATE_TABLE[15], 54);

    // Set rate index to 15
    dmc.write_register(0, 0x0F);
    assert_eq!(dmc.rate_index, 15);
    assert_eq!(dmc.rate_period, 54);
}

#[test]
fn test_dmc_delta_modulation_increment_and_decrement() {
    let mut dmc = DmcChannel::new();
    dmc.write_register(1, 10); // DAC = 10
    assert_eq!(dmc.output(), 10);

    // Setup 1 byte sample in memory: 0b00000011 (bits 0, 1 are 1, rest are 0)
    // Bit 0 = 1 (+2 -> 12)
    // Bit 1 = 1 (+2 -> 14)
    // Bit 2 = 0 (-2 -> 12)
    // Bit 3 = 0 (-2 -> 10)
    let mut mem = vec![0u8; 0x10000];
    mem[0xC000] = 0b0000_0011;
    dmc.attach_memory(mem);

    // $4012: address $C000 (A = 0)
    dmc.write_register(2, 0x00);
    // $4013: length = 1 byte (L = 0 -> 1 byte)
    dmc.write_register(3, 0x00);
    // $4010: rate index 15 (54 cycles)
    dmc.write_register(0, 0x0F);

    // Enable DMC
    dmc.set_enabled(true);
    assert!(dmc.sample_buffer.is_some());

    // Step until timer expires to clock first bit (bit 0 = 1 -> +2)
    while dmc.timer > 0 {
        dmc.step();
    }
    dmc.step();
    assert_eq!(dmc.output(), 12);

    // Step next rate_period (bit 1 = 1 -> +2)
    while dmc.timer > 0 {
        dmc.step();
    }
    dmc.step();
    assert_eq!(dmc.output(), 14);

    // Step next rate_period (bit 2 = 0 -> -2)
    while dmc.timer > 0 {
        dmc.step();
    }
    dmc.step();
    assert_eq!(dmc.output(), 12);

    // Step next rate_period (bit 3 = 0 -> -2)
    while dmc.timer > 0 {
        dmc.step();
    }
    dmc.step();
    assert_eq!(dmc.output(), 10);
}

#[test]
fn test_dmc_dac_clamping() {
    let mut dmc = DmcChannel::new();
    dmc.write_register(1, 126); // DAC = 126

    let mut mem = vec![0u8; 0x10000];
    mem[0xC000] = 0xFF; // All 1s -> should add +2 but clamp at 126 or 127
    dmc.attach_memory(mem);
    dmc.write_register(2, 0x00);
    dmc.write_register(3, 0x00);
    dmc.write_register(0, 0x0F);
    dmc.set_enabled(true);

    for _ in 0..=54 {
        dmc.step();
    }
    // Since dac_level was 126, adding 2 would exceed 127, so it does not add
    assert_eq!(dmc.output(), 126);

    // Now test downward clamping at 0
    dmc.write_register(1, 1);
    let mut mem2 = vec![0u8; 0x10000];
    mem2[0xC000] = 0x00; // All 0s -> should subtract 2 but clamp if < 2
    dmc.attach_memory(mem2);
    dmc.restart();

    for _ in 0..=54 {
        dmc.step();
    }
    assert_eq!(dmc.output(), 1); // 1 >= 2 is false, so it remains 1
}

#[test]
fn test_dmc_loop_and_irq() {
    let mut dmc = DmcChannel::new();
    let mem = vec![0xAA; 0x10000];
    dmc.attach_memory(mem);
    dmc.write_register(2, 0x00); // $C000
    dmc.write_register(3, 0x00); // 1 byte

    // Non-looping, IRQ enabled
    dmc.write_register(0, 0x8F); // IRQ enable = 1, Loop = 0, Rate = 15
    dmc.set_enabled(true);

    // Run until sample finishes (8 bits * 54 cycles = ~432 cycles)
    for _ in 0..600 {
        dmc.step();
    }

    assert!(dmc.irq_flag, "DMC IRQ flag should be set when sample completes without loop");

    // Clear IRQ via $4015 or $4010
    dmc.write_register(0, 0x0F); // IRQ disabled
    assert!(!dmc.irq_flag);

    // Now test Loop mode
    dmc.write_register(0, 0x4F); // Loop = 1
    dmc.set_enabled(true);
    for _ in 0..1000 {
        dmc.step();
    }
    assert_eq!(dmc.bytes_remaining, 1, "Bytes remaining should reload automatically in loop mode");
    assert!(!dmc.irq_flag, "Loop mode should never trigger IRQ");
}
