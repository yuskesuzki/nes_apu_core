//! Triangle Channel ($4008, $400A, $400B)
//!
//! ハードウェア仕様:
//! - 11-bit ダウンカウンタタイマー (毎 CPU サイクル減少)
//! - 32ステップ階段波形 (15 -> 0 -> 15)
//! - 音量レジスタなし (常に 4-bit DAC 階段電圧)
//! - ハードウェア変調ユニット内蔵:
//!   - Linear Counter (クォーターフレーム 240Hz/192Hz 高分解能ミリ秒タイマー)
//!   - Length Counter (ハーフフレーム 120Hz/96Hz 音長自動カット)
//! - 停止時は 0 に戻らず「止まった位置の電圧 (0〜15) を保持」する (実機特有のDCオフセット挙動)

use crate::apu::linear_counter::LinearCounter;
use crate::apu::length_counter::LengthCounter;

pub const TRIANGLE_SEQUENCE: [u8; 32] = [
    15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0,
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
];

#[derive(Debug, Clone)]
pub struct TriangleChannel {
    /// 11-bit タイマーリロード値 (0..2047)
    pub timer_period: u16,
    /// 11-bit 現在のダウンカウンタ値
    pub timer: u16,

    /// 32ステップ シーケンサ位置 (0..31)
    pub sequencer_step: usize,

    /// 有効化フラグ ($4015 bit 2)
    pub enabled: bool,

    /// ハードウェア変調器群
    pub linear_counter: LinearCounter,
    pub length_counter: LengthCounter,
}

impl Default for TriangleChannel {
    fn default() -> Self {
        Self {
            timer_period: 0,
            timer: 0,
            sequencer_step: 0,
            enabled: false,
            linear_counter: LinearCounter {
                counter: 127,
                reload_value: 127,
                control_flag: true,
                reload_flag: true,
            },
            length_counter: LengthCounter {
                counter: 254,
                halt: true,
                enabled: true,
            },
        }
    }
}

impl TriangleChannel {
    pub fn new() -> Self {
        Self::default()
    }

    /// 毎 CPU サイクル (1.789773 MHz) のタイマー進行
    pub fn clock_timer(&mut self) {
        if self.timer == 0 {
            self.timer = self.timer_period;
            // リニアカウンタとレングスカウンタの双方が有効な場合のみシーケンサを進める
            if self.enabled && self.timer_period >= 2 && self.linear_counter.is_active() && !self.length_counter.is_silenced() {
                self.sequencer_step = (self.sequencer_step + 1) & 31;
            }
        } else {
            self.timer -= 1;
        }
    }

    /// クォーターフレーム (240Hz / 192Hz): リニアカウンタの更新
    pub fn clock_quarter_frame(&mut self) {
        self.linear_counter.clock();
    }

    /// ハーフフレーム (120Hz / 96Hz): レングスカウンタの更新
    pub fn clock_half_frame(&mut self) {
        self.length_counter.clock();
    }

    /// 現在の 4-bit デジタル出力値 (0..15) を取得
    /// 実機仕様: 停止時も 0 にリセットされず、停止位置の階段電圧 (0〜15) を出力し続ける
    /// (DC オフセットは実機アナログ HPF フィルターによって自然に減衰・直流カットされる)
    pub fn output(&self) -> u8 {
        TRIANGLE_SEQUENCE[self.sequencer_step & 31]
    }

    /// 停止時に保持されている生のDC出力値 (実機挙動再現用)
    pub fn raw_output(&self) -> u8 {
        TRIANGLE_SEQUENCE[self.sequencer_step & 31]
    }

    /// レジスタ書き込み ($4008, $400A, $400B)
    pub fn write_register(&mut self, offset: u8, val: u8) {
        match offset {
            0 => {
                // $4008: CRRR RRRR
                self.linear_counter.write_control(val);
                self.length_counter.halt = (val & 0x80) != 0;
            }
            2 => {
                // $400A: TTTT TTTT (Timer Low 8-bit)
                self.timer_period = (self.timer_period & 0x0700) | (val as u16);
            }
            3 => {
                // $400B: LLLL LTTT (Length counter load & Timer High 3-bit)
                self.timer_period = (self.timer_period & 0x00FF) | (((val & 0x07) as u16) << 8);
                self.linear_counter.set_reload_flag();
                self.length_counter.load((val >> 3) & 0x1F);
            }
            _ => {}
        }
    }

    /// 理論上の出力周波数 (Hz) を計算
    pub fn calculate_frequency(&self, cpu_clock: f32) -> f32 {
        if self.timer_period < 2 {
            0.0
        } else {
            cpu_clock / (32.0 * (self.timer_period as f32 + 1.0))
        }
    }
}
