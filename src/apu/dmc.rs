//! NES APU Delta Modulation Channel (DMC / DPCM)
//!
//! ファミコン実機（Ricoh RP2A03）の DMC チャンネルを再現します。
//! - 16 段階のレート周期テーブル（CPU サイクル数）
//! - 7-bit DAC 直接ロード ($4011)
//! - 1-bit デルタ変調 (+2 / -2、範囲 0..127 クランプ)
//! - サンプルバッファ (8-bit) および シフトレジスタ (8-bit)
//! - ループ再生、IRQ 割り込みフラグ、ステータスレジスタ ($4015) 連携

/// NTSC DMC レート周期テーブル（1 bit 出力あたりの CPU サイクル数）
pub const DMC_NTSC_RATE_TABLE: [u16; 16] = [
    428, 380, 340, 320, 286, 254, 226, 214,
    190, 160, 142, 128, 106,  84,  72,  54,
];

#[derive(Debug, Clone)]
pub struct DmcChannel {
    pub enabled: bool,

    // レジスタ $4010
    pub irq_enabled: bool,
    pub loop_flag: bool,
    pub rate_index: u8,
    pub rate_period: u16,

    // レジスタ $4011 (7-bit DAC)
    pub dac_level: u8, // 0..127

    // レジスタ $4012 (サンプル開始アドレス)
    pub sample_address: u16,  // $C000 + A * 64
    pub current_address: u16,

    // レジスタ $4013 (サンプル長)
    pub sample_length: u16,   // L * 16 + 1 バイト
    pub bytes_remaining: u16,

    // 内部タイマー
    pub timer: u16,

    // 出力ユニット
    pub shift_register: u8,
    pub bits_remaining: u8,
    pub sample_buffer: Option<u8>,
    pub silence: bool,

    // 割り込みフラグ
    pub irq_flag: bool,

    // DMA / メモリデータ（エミュレータ内蔵の 64KB メモリバッファ、またはダイレクトアクセス）
    pub memory: Option<Vec<u8>>,
}

impl DmcChannel {
    pub fn new() -> Self {
        Self {
            enabled: false,
            irq_enabled: false,
            loop_flag: false,
            rate_index: 0,
            rate_period: DMC_NTSC_RATE_TABLE[0],
            dac_level: 0,
            sample_address: 0xC000,
            current_address: 0xC000,
            sample_length: 1,
            bytes_remaining: 0,
            timer: DMC_NTSC_RATE_TABLE[0],
            shift_register: 0,
            bits_remaining: 0,
            sample_buffer: None,
            silence: true,
            irq_flag: false,
            memory: None,
        }
    }

    /// メモリバッファをアタッチ（CPU メモリ空間 $0000..$FFFF）
    pub fn attach_memory(&mut self, mem: Vec<u8>) {
        self.memory = Some(mem);
    }

    /// メモリバッファを更新（既存のメモリ領域を再利用）
    pub fn update_memory(&mut self, mem: &[u8]) {
        if let Some(ref mut existing) = self.memory {
            if existing.len() == mem.len() {
                existing.copy_from_slice(mem);
                return;
            }
        }
        self.memory = Some(mem.to_vec());
    }

    /// レジスタ書き込み ($4010..$4013)
    pub fn write_register(&mut self, reg: u8, val: u8) {
        match reg {
            0 => {
                // $4010: Flags and Rate
                self.irq_enabled = (val & 0x80) != 0;
                self.loop_flag = (val & 0x40) != 0;
                self.rate_index = val & 0x0F;
                self.rate_period = DMC_NTSC_RATE_TABLE[self.rate_index as usize];
                if !self.irq_enabled {
                    self.irq_flag = false;
                }
            }
            1 => {
                // $4011: Direct load (7-bit DAC)
                self.dac_level = val & 0x7F;
            }
            2 => {
                // $4012: Sample address
                // Address = $C000 + (A * 64)
                self.sample_address = 0xC000 + ((val as u16) << 6);
            }
            3 => {
                // $4013: Sample length
                // Length = (L * 16) + 1 bytes
                self.sample_length = ((val as u16) << 4) + 1;
            }
            _ => {}
        }
    }

    /// $4015 書き込みによる DMC 有効化/無効化制御
    pub fn set_enabled(&mut self, enable: bool) {
        self.enabled = enable;
        self.irq_flag = false;
        if enable {
            if self.bytes_remaining == 0 {
                self.restart();
            }
        } else {
            self.bytes_remaining = 0;
        }
    }

    /// サンプル再生の開始/再開
    pub fn restart(&mut self) {
        self.current_address = self.sample_address;
        self.bytes_remaining = self.sample_length;
        if self.sample_buffer.is_none() {
            self.fetch_sample_byte();
        }
    }

    /// メモリから 1 バイトをサンプルバッファに先読み
    fn fetch_sample_byte(&mut self) {
        if self.bytes_remaining > 0 && self.sample_buffer.is_none() {
            let byte = if let Some(ref mem) = self.memory {
                let addr = self.current_address as usize;
                if addr < mem.len() {
                    mem[addr]
                } else {
                    0
                }
            } else {
                0
            };

            self.sample_buffer = Some(byte);

            // アドレスインクリメント ($8000..$FFFF 内でラップアラウンド)
            if self.current_address == 0xFFFF {
                self.current_address = 0x8000;
            } else {
                self.current_address += 1;
            }

            self.bytes_remaining -= 1;
            if self.bytes_remaining == 0 {
                if self.loop_flag {
                    self.current_address = self.sample_address;
                    self.bytes_remaining = self.sample_length;
                } else if self.irq_enabled {
                    self.irq_flag = true;
                }
            }
        }
    }

    /// 1 CPU サイクル進行
    pub fn step(&mut self) {
        // サンプルバッファが空ならフェッチを試みる
        if self.sample_buffer.is_none() && self.bytes_remaining > 0 {
            self.fetch_sample_byte();
        }

        if self.timer == 0 {
            self.timer = self.rate_period;
            self.clock_output();
        } else {
            self.timer -= 1;
        }
    }

    /// 出力ユニットのクロック（1 ビット分のデルタ処理）
    fn clock_output(&mut self) {
        if self.bits_remaining == 0 {
            self.bits_remaining = 8;
            if let Some(byte) = self.sample_buffer.take() {
                self.silence = false;
                self.shift_register = byte;
                self.fetch_sample_byte();
            } else {
                self.silence = true;
            }
        }

        if !self.silence {
            if (self.shift_register & 0x01) != 0 {
                if self.dac_level <= 125 {
                    self.dac_level += 2;
                }
            } else {
                if self.dac_level >= 2 {
                    self.dac_level -= 2;
                }
            }
            self.shift_register >>= 1;
        }

        self.bits_remaining = self.bits_remaining.saturating_sub(1);
    }

    /// 現在の 7-bit DAC 出力値 (0..127)
    pub fn output(&self) -> u8 {
        self.dac_level
    }
}
