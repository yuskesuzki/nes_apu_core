use nes_apu_core::NesApuCore;

#[test]
fn test_channel_mute_and_solo() {
    let mut apu = NesApuCore::new();

    // Enable Pulse 1 and Triangle with constant outputs
    apu.write_register(0x4000, 0xBF); // Duty = 2 (50%), vol = 15
    apu.write_register(0x4002, 10);   // Small timer period for fast stepping
    apu.write_register(0x4003, 0x08);

    apu.write_register(0x4008, 0xFF); // Triangle
    apu.write_register(0x400A, 10);
    apu.write_register(0x400B, 0x08);

    apu.write_register(0x4015, 0x05); // P1 and Tri enable

    // Step until both are actively in high output state
    while apu.pulse1.output() == 0 || apu.triangle.output() == 0 {
        apu.step();
    }

    // Both should initially have sound output
    let (p1, _p2, tri, _noi, _dmc) = apu.sample_raw_5ch();
    assert!(p1 > 0, "Pulse 1 should be outputting");
    assert!(tri > 0, "Triangle should be outputting");

    // Mute Pulse 1 (channel 0)
    apu.mute_channel(0, true);
    let (p1_muted, _p2, tri_active, _noi, _dmc) = apu.sample_raw_5ch();
    assert_eq!(p1_muted, 0, "Pulse 1 must be 0 when muted");
    assert!(tri_active > 0, "Triangle should still be outputting");

    // Unmute Pulse 1
    apu.mute_channel(0, false);
    let (p1_unmuted, _, _, _, _) = apu.sample_raw_5ch();
    assert!(p1_unmuted > 0, "Pulse 1 output should be restored after unmute");

    // Solo Triangle (channel 2) -> Pulse 1, 2, Noise, DMC are muted
    apu.solo_channel(2);
    let (p1_s, p2_s, tri_s, noi_s, dmc_s) = apu.sample_raw_5ch();
    assert_eq!(p1_s, 0);
    assert_eq!(p2_s, 0);
    assert!(tri_s > 0, "Soloed Triangle must still output");
    assert_eq!(noi_s, 0);
    assert_eq!(dmc_s, 0);

    // Unmute all
    apu.unmute_all();
    let (p1_all, _, tri_all, _, _) = apu.sample_raw_5ch();
    assert!(p1_all > 0);
    assert!(tri_all > 0);
}
