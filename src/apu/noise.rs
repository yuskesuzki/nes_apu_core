//! Noise Channel ($400C, $400E, $400F)
//!
//! ハードウェア仕様:
//! - 15-bit LFSR (初期値 1)
//! - 通常モード (M=0: 32,767 ステップ) / ショートモード (M=1: 93 ステップ金属音)
//! - 16段階 NTSC 周期テーブル (CPU サイクル数) によるタイマー減算
//! - ハードウェア変調ユニット内蔵:
//!   - Envelope Generator (クォーターフレーム 240Hz/192Hz 減衰)
//!   - Length Counter (ハーフフレーム 120Hz/96Hz 音長自動カット)
//! - 出力判定: LFSR の bit 0 が 0 のとき音量を出力、bit 0 が 1 のときは消音

use crate::apu::envelope::EnvelopeGenerator;
use crate::apu::length_counter::LengthCounter;

pub const NOISE_PERIOD_TABLE_NTSC: [u16; 16] = [
    4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068,
];

#[derive(Debug, Clone)]
pub struct NoiseChannel {
    /// 15-bit シフトレジスタ (LFSR) - 電源投入時は 1
    pub shift_register: u16,

    /// 周期インデックス (0..15)
    pub period_index: usize,

    /// 現在のダウンカウンタ値
    pub timer: u16,

    /// モードフラグ ($400E bit 7: false = 通常32767ステップ, true = 短周期93ステップ金属音)
    pub short_mode: bool,

    /// 有効化フラグ ($4015 bit 3)
    pub enabled: bool,

    /// ハードウェア変調器群
    pub envelope: EnvelopeGenerator,
    pub length_counter: LengthCounter,
}

impl Default for NoiseChannel {
    fn default() -> Self {
        Self {
            shift_register: 1, // 実機リセット初期値
            period_index: 0,
            timer: NOISE_PERIOD_TABLE_NTSC[0],
            short_mode: false,
            enabled: false,
            envelope: EnvelopeGenerator::new(),
            length_counter: LengthCounter {
                counter: 254,
                halt: true,
                enabled: true,
            },
        }
    }
}

impl NoiseChannel {
    pub fn new() -> Self {
        Self::default()
    }

    /// 現在の音量値を取得
    pub fn volume(&self) -> u8 {
        self.envelope.output_volume()
    }

    /// 音量を設定 (テスト用固定音量)
    pub fn set_volume(&mut self, vol: u8) {
        self.envelope.constant_volume = true;
        self.envelope.volume_reload = vol & 0x0F;
    }

    /// 毎 CPU サイクル (1.789773 MHz) のタイマー進行
    pub fn clock_timer(&mut self) {
        if self.timer == 0 {
            self.timer = NOISE_PERIOD_TABLE_NTSC[self.period_index.min(15)];

            let other_bit = if self.short_mode {
                (self.shift_register >> 6) & 1
            } else {
                (self.shift_register >> 1) & 1
            };
            let feedback = (self.shift_register & 1) ^ other_bit;
            self.shift_register = (self.shift_register >> 1) | (feedback << 14);
        } else {
            self.timer -= 1;
        }
    }

    /// クォーターフレーム (240Hz / 192Hz): エンベロープの更新
    pub fn clock_quarter_frame(&mut self) {
        self.envelope.clock();
    }

    /// ハーフフレーム (120Hz / 96Hz): レングスカウンタの更新
    pub fn clock_half_frame(&mut self) {
        self.length_counter.clock();
    }

    /// 現在の 4-bit デジタル出力値 (0..15) を取得
    pub fn output(&self) -> u8 {
        if !self.enabled || self.length_counter.is_silenced() {
            return 0;
        }

        if (self.shift_register & 1) == 0 {
            self.envelope.output_volume().min(15)
        } else {
            0
        }
    }

    /// レジスタ書き込み ($400C, $400E, $400F)
    pub fn write_register(&mut self, offset: u8, val: u8) {
        match offset {
            0 => {
                // $400C: --LC VVVV
                self.envelope.write_control(val);
                self.length_counter.halt = (val & 0x20) != 0;
            }
            2 => {
                // $400E: M--- PPPP
                self.short_mode = (val & 0x80) != 0;
                self.period_index = (val & 0x0F) as usize;
            }
            3 => {
                // $400F: LLLL L--- (Length counter load & Envelope restart)
                self.envelope.restart();
                self.length_counter.load((val >> 3) & 0x1F);
            }
            _ => {}
        }
    }

    /// シフトレジスタのリセット (テストおよび電源投入時用)
    pub fn reset_lfsr(&mut self) {
        self.shift_register = 1;
    }
}
