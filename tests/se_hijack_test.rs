//! 効果音（SE）チャンネル乗っ取り（ハイジャック）＆ BGM レストア調停の結合テスト

use nes_apu_core::apu::{NesApuCore, RegisterWriteSource};

#[test]
fn test_se_channel_hijack_and_bgm_restore() {
    let mut apu = NesApuCore::new();

    // 1. BGM が Pulse 1 と Pulse 2 を鳴らす ($4015 全有効)
    apu.write_source = RegisterWriteSource::Bgm;
    apu.write_register(0x4015, 0x03);

    // BGM Pulse 1: 音量 10, タイマー周期 200
    apu.write_register(0x4000, 0x3A); // duty 0, const vol 10
    apu.write_register(0x4002, 200 & 0xFF);
    apu.write_register(0x4003, (200 >> 8) as u8);

    // BGM Pulse 2: 音量 5, タイマー周期 400
    apu.write_register(0x4004, 0x35); // duty 0, const vol 5
    apu.write_register(0x4006, (400u16 & 0xFF) as u8);
    apu.write_register(0x4007, (400u16 >> 8) as u8);

    assert_eq!(apu.pulse1.envelope.volume_reload, 10);
    assert_eq!(apu.pulse1.timer_period, 200);
    assert_eq!(apu.pulse2.envelope.volume_reload, 5);
    assert_eq!(apu.pulse2.timer_period, 400);
    assert!(!apu.channel_hijacked[0]);
    assert!(!apu.channel_hijacked[1]);

    // 2. 効果音 (SE) が Pulse 1 だけをハイジャック (例: ジャンプ音: 音量 15, タイマー 100)
    apu.write_source = RegisterWriteSource::SoundEffect;
    apu.write_register(0x4000, 0x3F); // duty 0, const vol 15
    apu.write_register(0x4002, 100 & 0xFF);
    apu.write_register(0x4003, (100 >> 8) as u8);

    // Pulse 1 は SE の値に上書きされ、ハイジャック中フラグが立つ
    assert!(apu.channel_hijacked[0]);
    assert_eq!(apu.pulse1.envelope.volume_reload, 15);
    assert_eq!(apu.pulse1.timer_period, 100);

    // Pulse 2 は SE に使われていないのでハイジャックされず BGM のまま
    assert!(!apu.channel_hijacked[1]);
    assert_eq!(apu.pulse2.envelope.volume_reload, 5);
    assert_eq!(apu.pulse2.timer_period, 400);

    // 3. SE 再生中に、裏で BGM が次の小節へ進み Pulse 1 と Pulse 2 を書き換える
    apu.write_source = RegisterWriteSource::Bgm;
    // BGM が Pulse 1 を音量 8, タイマー 300 に更新しようとする
    apu.write_register(0x4000, 0x38); // vol 8
    apu.write_register(0x4002, (300u16 & 0xFF) as u8);
    apu.write_register(0x4003, (300u16 >> 8) as u8);

    // BGM が Pulse 2 を音量 7, タイマー 500 に更新
    apu.write_register(0x4004, 0x37); // vol 7
    apu.write_register(0x4006, (500u16 & 0xFF) as u8);
    apu.write_register(0x4007, (500u16 >> 8) as u8);

    // 検証: Pulse 1 はハイジャック中のため、SE の値 (vol 15, timer 100) が保持される
    assert_eq!(apu.pulse1.envelope.volume_reload, 15);
    assert_eq!(apu.pulse1.timer_period, 100);

    // 検証: Pulse 2 はハイジャックされていないため、BGM の新値 (vol 7, timer 500) に即座に更新される
    assert_eq!(apu.pulse2.envelope.volume_reload, 7);
    assert_eq!(apu.pulse2.timer_period, 500);

    // 4. SE が鳴り終わり、Pulse 1 のハイジャックが解除される
    apu.restore_channel_from_bgm(0);

    // 検証: Pulse 1 のハイジャックが解除され、BGM の裏で進んでいた最新値 (vol 8, timer 300) にシームレス復元！
    assert!(!apu.channel_hijacked[0]);
    assert_eq!(apu.pulse1.envelope.volume_reload, 8);
    assert_eq!(apu.pulse1.timer_period, 300);
}
