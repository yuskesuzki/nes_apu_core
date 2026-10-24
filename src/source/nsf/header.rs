//! NSF (NES Sound Format) ヘッダーパーサー
//!
//! NSF 仕様 (v1.61) に準拠した 128 バイトのヘッダー解析を行います。

#[derive(Debug, Clone)]
pub struct NsfHeader {
    pub version: u8,
    pub total_songs: u8,
    pub starting_song: u8,
    pub load_addr: u16,
    pub init_addr: u16,
    pub play_addr: u16,
    pub title: String,
    pub artist: String,
    pub copyright: String,
    pub ntsc_speed: u16,
    pub bankswitch_init: [u8; 8],
    pub pal_speed: u16,
    pub pal_ntsc_bits: u8,
    pub extra_chips: u8,
}

impl NsfHeader {
    /// 128 バイト以上のバイト列から NSF ヘッダーを解析します
    pub fn parse(data: &[u8]) -> Result<Self, String> {
        if data.len() < 128 {
            return Err("NSF データが短すぎます (最低128バイト必要)".to_string());
        }

        if &data[0..5] != b"NESM\x1A" {
            return Err("無効な NSF マジックナンバーです (NESM\\x1A が必要)".to_string());
        }

        let version = data[5];
        let total_songs = data[6];
        let starting_song = data[7];
        let load_addr = u16::from_le_bytes([data[8], data[9]]);
        let init_addr = u16::from_le_bytes([data[10], data[11]]);
        let play_addr = u16::from_le_bytes([data[12], data[13]]);

        let title = parse_c_string(&data[14..46]);
        let artist = parse_c_string(&data[46..78]);
        let copyright = parse_c_string(&data[78..110]);

        let ntsc_speed = u16::from_le_bytes([data[110], data[111]]);
        let mut bankswitch_init = [0u8; 8];
        bankswitch_init.copy_from_slice(&data[112..120]);

        let pal_speed = u16::from_le_bytes([data[120], data[121]]);
        let pal_ntsc_bits = data[122];
        let extra_chips = data[123];

        Ok(Self {
            version,
            total_songs,
            starting_song,
            load_addr,
            init_addr,
            play_addr,
            title,
            artist,
            copyright,
            ntsc_speed,
            bankswitch_init,
            pal_speed,
            pal_ntsc_bits,
            extra_chips,
        })
    }

    /// バンクスイッチングを使用する NSF かどうか
    pub fn uses_bankswitching(&self) -> bool {
        self.bankswitch_init.iter().any(|&b| b != 0)
    }
}

fn parse_c_string(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).trim().to_string()
}
