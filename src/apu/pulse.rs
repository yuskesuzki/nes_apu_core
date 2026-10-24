//! Pulse (Square Wave) Channel - Pulse 1 ($4000..$4003) & Pulse 2 ($4004..$4007)
//!
//! ハードウェア仕様:
//! - 11-bit ダウンカウンタタイマー (t)
//! - APU サイクル (2 CPU サイクルに1回) ごとにタイマーをデクリメント
//! - 8ステップ逆行シーケンサ (0, 7, 6, 5, 4, 3, 2, 1) による4種類のデューティ比
//! - 1波形の全周期 = 16 * (t + 1) CPU サイクル
//! - ハードウェア変調ユニット内蔵:
//!   - Envelope Generator (クォーターフレーム 240Hz/192Hz 減衰)
//!   - Sweep Unit (ハーフフレーム 120Hz/96Hz 周波数自動スライド、1の補数/2の補数)
//!   - Length Counter (ハーフフレーム 120Hz/96Hz 音長自動カット)
//! - ハードウェアミュート条件: t < 8 または sweep target > 0x7FF

use crate::apu::envelope::EnvelopeGenerator;
use crate::apu::sweep::SweepUnit;
use crate::apu::length_counter::LengthCounter;

pub const PULSE_DUTY_TABLE: [[u8; 8]; 4] = [
    [0, 1, 0, 0, 0, 0, 0, 0], // Duty 0: 12.5%
    [0, 1, 1, 0, 0, 0, 0, 0], // Duty 1: 25.0%
    [0, 1, 1, 1, 1, 0, 0, 0], // Duty 2: 50.0%
    [1, 0, 0, 1, 1, 1, 1, 1], // Duty 3: 75.0% (反転25%)
];

#[derive(Debug, Clone)]
pub struct PulseChannel {
    /// チャンネル番号 (0 = Pulse 1, 1 = Pulse 2)
    pub channel_id: usize,

    /// 11-bit タイマーリロード値 (0..2047)
    pub timer_period: u16,
    /// 11-bit 現在のダウンカウンタ値
    pub timer: u16,

    /// 8ステップ シーケンサ位置 (0..7)
    pub sequencer_step: usize,

    /// デューティ比インデックス (0..3: 12.5%, 25%, 50%, 75%)
    pub duty: usize,

    /// 有効化フラグ ($4015 bit 0/1)
    pub enabled: bool,

    /// ハードウェア変調器群
    pub envelope: EnvelopeGenerator,
    pub sweep: SweepUnit,
    pub length_counter: LengthCounter,
}

impl PulseChannel {
    pub fn new(channel_id: usize) -> Self {
        Self {
            channel_id,
            timer_period: 0,
            timer: 0,
            sequencer_step: 0,
            duty: 0,
            enabled: false,
            envelope: EnvelopeGenerator::new(),
            sweep: SweepUnit::new(channel_id),
            length_counter: LengthCounter {
                counter: 254,
                halt: true,
                enabled: true,
            },
        }
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

    /// APU サイクル (2 CPU サイクルに1回) のタイマー進行
    pub fn clock_timer(&mut self) {
        if self.timer == 0 {
            self.timer = self.timer_period;
            self.sequencer_step = (self.sequencer_step + 7) & 7;
        } else {
            self.timer -= 1;
        }
    }

    /// クォーターフレーム (240Hz / 192Hz): エンベロープの更新
    pub fn clock_quarter_frame(&mut self) {
        self.envelope.clock();
    }

    /// ハーフフレーム (120Hz / 96Hz): レングスカウンタ & スイープの更新
    pub fn clock_half_frame(&mut self) {
        self.length_counter.clock();
        if let Some(new_period) = self.sweep.clock(self.timer_period) {
            self.timer_period = new_period;
        }
    }

    /// 現在の 4-bit デジタル出力値 (0..15) を取得
    pub fn output(&self) -> u8 {
        if !self.enabled || self.length_counter.is_silenced() {
            return 0;
        }
        // 実機ハードウェア仕様: タイマー周期が 8 未満、またはスイープ目標が 0x7FF 超過時は消音
        if self.timer_period < 8 || self.sweep.is_muting(self.timer_period) {
            return 0;
        }

        let duty_bit = PULSE_DUTY_TABLE[self.duty & 3][self.sequencer_step & 7];
        if duty_bit == 1 {
            self.envelope.output_volume().min(15)
        } else {
            0
        }
    }

    /// レジスタ書き込み ($4000〜$4003 または $4004〜$4007)
    pub fn write_register(&mut self, offset: u8, val: u8) {
        match offset {
            0 => {
                // $4000 / $4004: DDLC VVVV
                self.duty = ((val >> 6) & 0x03) as usize;
                self.length_counter.halt = (val & 0x20) != 0;
                self.envelope.write_control(val);
            }
            1 => {
                // $4001 / $4005: EPPP NSSS (Sweep control)
                self.sweep.write_register(val);
            }
            2 => {
                // $4002 / $4006: TTTT TTTT (Timer Low 8-bit)
                self.timer_period = (self.timer_period & 0x0700) | (val as u16);
            }
            3 => {
                // $4003 / $4007: LLLL LTTT (Length counter load & Timer High 3-bit)
                self.timer_period = (self.timer_period & 0x00FF) | (((val & 0x07) as u16) << 8);
                // 実機仕様: 第4レジスタへの書き込みでシーケンサをリセットし、エンベロープを再起動
                self.sequencer_step = 0;
                self.envelope.restart();
                self.length_counter.load((val >> 3) & 0x1F);
            }
            _ => {}
        }
    }

    /// 理論上の出力周波数 (Hz) を計算
    pub fn calculate_frequency(&self, cpu_clock: f32) -> f32 {
        if self.timer_period < 8 {
            0.0
        } else {
            cpu_clock / (16.0 * (self.timer_period as f32 + 1.0))
        }
    }
}
