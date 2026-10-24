use nes_apu_core::source::nsf::{NsfHeader, NsfSource};
use nes_apu_core::ApuSystem;

/// 最小限の有効な NSF バイナリ（ダミーの INIT / PLAY ルーチン内蔵）
fn create_test_nsf() -> Vec<u8> {
    let mut data = vec![0u8; 128 + 32];
    // マジックナンバー
    data[0..5].copy_from_slice(b"NESM\x1A");
    data[5] = 1; // version
    data[6] = 2; // total songs = 2
    data[7] = 1; // starting song = 1

    // ロードアドレス: $8000
    data[8] = 0x00;
    data[9] = 0x80;
    // INIT アドレス: $8000
    data[10] = 0x00;
    data[11] = 0x80;
    // PLAY アドレス: $8006
    data[12] = 0x06;
    data[13] = 0x80;

    // タイトル, アーティスト
    let title = b"Test Song";
    data[14..14 + title.len()].copy_from_slice(title);
    let artist = b"Test Artist";
    data[46..46 + artist.len()].copy_from_slice(artist);

    // コード部 ($8000..)
    // $8000 (INIT):
    //   LDA #$0F
    //   STA $4015
    //   RTS
    data[128 + 0] = 0xA9; // LDA #$0F
    data[128 + 1] = 0x0F;
    data[128 + 2] = 0x8D; // STA $4015
    data[128 + 3] = 0x15;
    data[128 + 4] = 0x40;
    data[128 + 5] = 0x60; // RTS

    // $8006 (PLAY):
    //   LDA #$BF
    //   STA $4000
    //   RTS
    data[128 + 6] = 0xA9; // LDA #$BF
    data[128 + 7] = 0xBF;
    data[128 + 8] = 0x8D; // STA $4000
    data[128 + 9] = 0x00;
    data[128 + 10] = 0x40;
    data[128 + 11] = 0x60; // RTS

    data
}

#[test]
fn test_nsf_header_parser() {
    let data = create_test_nsf();
    let header = NsfHeader::parse(&data).expect("Should parse NSF header");

    assert_eq!(header.version, 1);
    assert_eq!(header.total_songs, 2);
    assert_eq!(header.starting_song, 1);
    assert_eq!(header.load_addr, 0x8000);
    assert_eq!(header.init_addr, 0x8000);
    assert_eq!(header.play_addr, 0x8006);
    assert_eq!(header.title, "Test Song");
    assert_eq!(header.artist, "Test Artist");
    assert!(!header.uses_bankswitching());
}

#[test]
fn test_nsf_player_init_and_play() {
    let data = create_test_nsf();
    let nsf = NsfSource::from_bytes(&data, None).expect("Should load NSF");
    assert_eq!(nsf.current_track, 1);

    let mut sys = ApuSystem::new(Box::new(nsf));

    // INIT ルーチンにより $4015 = 0x0F が書き込まれているはず
    assert!(sys.apu.pulse1.enabled);
    assert!(sys.apu.pulse2.enabled);
    assert!(sys.apu.triangle.enabled);
    assert!(sys.apu.noise.enabled);

    // 1フレーム (29,780 サイクル) 進行すると PLAY ルーチンが呼ばれ、$4000 = 0xBF が書き込まれる
    sys.clock_cycles(29780);

    assert_eq!(sys.apu.pulse1.duty, 2); // 50%
    assert_eq!(sys.apu.pulse1.volume(), 15);
}

#[test]
fn test_nsf_dmc_memory_sync() {
    let mut data = create_test_nsf();
    // DMC サンプルデータを $C000 (offset = 128 + 0x4000) に置くためにサイズ拡張
    data.resize(128 + 0x5000, 0xAA); // 0xAA = %10101010
    let nsf = NsfSource::from_bytes(&data, None).expect("Should load NSF");
    let mut sys = ApuSystem::new(Box::new(nsf));

    // DMC メモリが 64KB 同期されていることを確認
    assert!(sys.apu.dmc.memory.is_some());
    let mem = sys.apu.dmc.memory.as_ref().unwrap();
    assert_eq!(mem.len(), 0x10000);
    assert_eq!(mem[0xC000], 0xAA);

    // DMC を直接起動してサンプルフェッチと DAC 変調が行われることを確認
    sys.apu.dmc.write_register(0, 0x0F); // 最高レート
    sys.apu.dmc.write_register(1, 0x40); // DAC = 64
    sys.apu.dmc.write_register(2, 0x00); // addr = $C000
    sys.apu.dmc.write_register(3, 0x01); // len = 17 bytes
    sys.apu.dmc.set_enabled(true);

    let initial_dac = sys.apu.dmc.dac_level;
    // 数千サイクル進めて DAC が変化するか確認
    for _ in 0..1000 {
        sys.apu.dmc.step();
    }
    assert_ne!(sys.apu.dmc.dac_level, initial_dac, "DMC DAC should modulate using synced memory");
}
