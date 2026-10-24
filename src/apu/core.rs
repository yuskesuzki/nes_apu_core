//! RP2A03 APU Core Coordinator (Pure Hardware Emulation Layer)
//!
//! CPU クロック (1.789773 MHz) と APU サイクル (2分周) の統括、
//! APU フレームシーケンサ ($4017: 240Hz/192Hz) によるハードウェア変調クロックの分配、
//! $4000〜$4017 レジスタマッピングを提供します。
//!
//! ※ ソフトウェア音源ドライバ（6502ドライバやNTRKローダ）には一切依存しない純粋なハードウェア層です。

use crate::apu::dmc::DmcChannel;
use crate::apu::filter::{AnalogFilterChain, NonlinearMixer};
use crate::apu::frame_counter::{FrameClockAction, FrameCounter};
use crate::apu::noise::NoiseChannel;
use crate::apu::pulse::PulseChannel;
use crate::apu::triangle::TriangleChannel;

pub const NTSC_CPU_CLOCK: f32 = 1_789_773.0;

#[derive(Debug, Clone)]
pub struct NesApuCore {
    pub pulse1: PulseChannel,
    pub pulse2: PulseChannel,
    pub triangle: TriangleChannel,
    pub noise: NoiseChannel,
    pub dmc: DmcChannel,

    /// APU フレームシーケンサ ($4017: 240Hz/192Hz)
    pub frame_counter: FrameCounter,

    /// 非線形 DAC ミキサー (抵抗ラダー)
    pub mixer: NonlinearMixer,

    /// 実機基板 3段 アナログ RC フィルターチェーン (HPF 90Hz, HPF 440Hz, LPF 14kHz)
    pub filter_chain: AnalogFilterChain,

    /// チャンネルごとの消音マスク (Pulse 1, Pulse 2, Triangle, Noise, DMC)
    pub channel_mute: [bool; 5],

    /// APU 2分周トグル (CPU 2サイクルごとに APU 1サイクル)
    pub apu_cycle_toggle: bool,

    /// 経過総 CPU サイクルカウンタ
    pub total_cpu_cycles: u64,

    /// $4015 レジスタキャッシュ
    pub status_reg: u8,
}

impl Default for NesApuCore {
    fn default() -> Self {
        Self {
            pulse1: PulseChannel::new(0),
            pulse2: PulseChannel::new(1),
            triangle: TriangleChannel::new(),
            noise: NoiseChannel::new(),
            dmc: DmcChannel::new(),
            frame_counter: FrameCounter::new(),
            mixer: NonlinearMixer::new(),
            filter_chain: AnalogFilterChain::new(44100.0),
            channel_mute: [false; 5],
            apu_cycle_toggle: false,
            total_cpu_cycles: 0,
            status_reg: 0,
        }
    }
}

impl NesApuCore {
    pub fn new() -> Self {
        Self::default()
    }

    /// 1 CPU サイクル (約 558.7 ナノ秒) を進める
    pub fn step(&mut self) {
        self.total_cpu_cycles += 1;

        // 1. Triangle, Noise, DMC は毎 CPU サイクルでタイマー減少
        self.triangle.clock_timer();
        self.noise.clock_timer();
        self.dmc.step();

        // 2. APU 2分周回路 (Pulse チャンネルは 2 CPU サイクルに 1 回進む)
        self.apu_cycle_toggle = !self.apu_cycle_toggle;
        if self.apu_cycle_toggle {
            self.pulse1.clock_timer();
            self.pulse2.clock_timer();
        }

        // 3. APU フレームシーケンサ ($4017: 240Hz / 192Hz) のステップ進行
        let action = self.frame_counter.step();
        self.dispatch_frame_action(action);
    }

    /// クォーターフレーム / ハーフフレームの変調クロックを各チャンネルへ分配
    fn dispatch_frame_action(&mut self, action: FrameClockAction) {
        match action {
            FrameClockAction::QuarterFrame => {
                // Envelope & Linear counter
                self.pulse1.clock_quarter_frame();
                self.pulse2.clock_quarter_frame();
                self.triangle.clock_quarter_frame();
                self.noise.clock_quarter_frame();
            }
            FrameClockAction::HalfFrame => {
                // Envelope & Linear counter
                self.pulse1.clock_quarter_frame();
                self.pulse2.clock_quarter_frame();
                self.triangle.clock_quarter_frame();
                self.noise.clock_quarter_frame();
                // Length counter & Sweep unit
                self.pulse1.clock_half_frame();
                self.pulse2.clock_half_frame();
                self.triangle.clock_half_frame();
                self.noise.clock_half_frame();
            }
            FrameClockAction::None => {}
        }
    }

    /// 指定した CPU サイクル数だけ一括進行
    pub fn clock_cycles(&mut self, cycles: usize) {
        for _ in 0..cycles {
            self.step();
        }
    }

    /// 指定チャンネルを消音 (Mute) / 発音 (Unmute) 設定
    pub fn mute_channel(&mut self, ch: usize, mute: bool) {
        if ch < 5 {
            self.channel_mute[ch] = mute;
        }
    }

    /// 指定チャンネルのみをソロ演奏 (他の全チャンネルをミュート)
    pub fn solo_channel(&mut self, ch: usize) {
        for i in 0..5 {
            self.channel_mute[i] = i != ch;
        }
    }

    /// 全チャンネルの消音を解除
    pub fn unmute_all(&mut self) {
        self.channel_mute = [false; 5];
    }

    /// 現在の 4 チャンネルの生デジタル出力値 (0..15) を取得 (Mute 反映)
    pub fn sample_raw(&self) -> (u8, u8, u8, u8) {
        (
            if self.channel_mute[0] { 0 } else { self.pulse1.output() },
            if self.channel_mute[1] { 0 } else { self.pulse2.output() },
            if self.channel_mute[2] { 0 } else { self.triangle.output() },
            if self.channel_mute[3] { 0 } else { self.noise.output() },
        )
    }

    /// 現在の全 5 チャンネルの生デジタル出力値 (Pulse1, Pulse2, Triangle, Noise, DMC) を取得 (Mute 反映)
    pub fn sample_raw_5ch(&self) -> (u8, u8, u8, u8, u8) {
        (
            if self.channel_mute[0] { 0 } else { self.pulse1.output() },
            if self.channel_mute[1] { 0 } else { self.pulse2.output() },
            if self.channel_mute[2] { 0 } else { self.triangle.output() },
            if self.channel_mute[3] { 0 } else { self.noise.output() },
            if self.channel_mute[4] { 0 } else { self.dmc.output() },
        )
    }

    /// 簡易リニア合成値 (-1.0〜+1.0) を取得
    pub fn sample_linear(&self) -> f32 {
        let (p1, p2, tri, noi, dmc) = self.sample_raw_5ch();
        let p_sum = (p1 + p2) as f32 / 30.0;
        let t_out = tri as f32 / 15.0;
        let n_out = noi as f32 / 15.0;
        let d_out = dmc as f32 / 127.0;

        let mixed = p_sum * 0.35 + t_out * 0.35 + n_out * 0.15 + d_out * 0.15;
        (mixed * 2.0 - 1.0).clamp(-1.0, 1.0)
    }

    /// 非線形 DAC ミキサー (抵抗ラダー) による瞬間合成値 (0.0..~1.0) を取得
    pub fn sample_nonlinear(&self) -> f32 {
        let (p1, p2, tri, noi, dmc) = self.sample_raw_5ch();
        self.mixer.mix(p1, p2, tri, noi, dmc)
    }

    /// IRQ 割り込み要求の有無（Frame Counter IRQ または DMC IRQ）
    pub fn irq_pending(&self) -> bool {
        self.frame_counter.irq_flag || self.dmc.irq_flag
    }

    /// APU レジスタ読み取り ($4015 のみ読み取り可能)
    /// ※ $4015 読み取り時、Frame Counter IRQ フラグは自動クリアされる実機仕様
    pub fn read_register(&mut self, addr: u16) -> u8 {
        match addr {
            0x4015 => {
                let mut status = 0u8;
                if self.pulse1.length_counter.counter > 0 {
                    status |= 0x01;
                }
                if self.pulse2.length_counter.counter > 0 {
                    status |= 0x02;
                }
                if self.triangle.length_counter.counter > 0 {
                    status |= 0x04;
                }
                if self.noise.length_counter.counter > 0 {
                    status |= 0x08;
                }
                if self.dmc.bytes_remaining > 0 {
                    status |= 0x10;
                }
                if self.frame_counter.irq_flag {
                    status |= 0x40;
                }
                if self.dmc.irq_flag {
                    status |= 0x80;
                }

                // 実機仕様: $4015 の読み取りによって Frame IRQ フラグはリセットされる
                self.frame_counter.irq_flag = false;

                status
            }
            _ => 0x00,
        }
    }

    /// オーディオ出力サンプル周期 (例: 44.1kHz / 48kHz) 分の CPU サイクルを進め、
    /// 非線形 DAC ダウンサンプル値を実機基板 3 段 RC フィルターへ通した高精度アナログ出力を生成
    pub fn generate_audio_sample(&mut self, sample_rate: f32) -> f32 {
        if (self.filter_chain.hpf1.sample_rate - sample_rate).abs() > 1.0 {
            self.filter_chain.set_sample_rate(sample_rate);
        }

        let cycles_per_sample = (NTSC_CPU_CLOCK / sample_rate).round() as usize;
        let mut accum: f32 = 0.0;

        for _ in 0..cycles_per_sample {
            self.step();
            accum += self.sample_nonlinear();
        }

        let raw_dac = if cycles_per_sample > 0 {
            accum / (cycles_per_sample as f32)
        } else {
            0.0
        };

        // 3段アナログ RC フィルター (HPF 90Hz -> HPF 440Hz -> LPF 14kHz)
        self.filter_chain.process(raw_dac)
    }

    /// 実機 APU レジスタ書き込み ($4000〜$4017)
    pub fn write_register(&mut self, addr: u16, val: u8) {
        match addr {
            // Pulse 1 ($4000..$4003)
            0x4000..=0x4003 => {
                self.pulse1.write_register((addr - 0x4000) as u8, val);
            }
            // Pulse 2 ($4004..$4007)
            0x4004..=0x4007 => {
                self.pulse2.write_register((addr - 0x4004) as u8, val);
            }
            // Triangle ($4008..$400B)
            0x4008..=0x400B => {
                self.triangle.write_register((addr - 0x4008) as u8, val);
            }
            // Noise ($400C..$400F)
            0x400C..=0x400F => {
                self.noise.write_register((addr - 0x400C) as u8, val);
            }
            // DMC ($4010..$4013)
            0x4010..=0x4013 => {
                self.dmc.write_register((addr - 0x4010) as u8, val);
            }
            // Status & Channel Enable ($4015)
            0x4015 => {
                self.status_reg = val;
                let p1_en = (val & 0x01) != 0;
                let p2_en = (val & 0x02) != 0;
                let tri_en = (val & 0x04) != 0;
                let noi_en = (val & 0x08) != 0;
                let dmc_en = (val & 0x10) != 0;

                self.pulse1.enabled = p1_en;
                self.pulse1.length_counter.set_enabled(p1_en);

                self.pulse2.enabled = p2_en;
                self.pulse2.length_counter.set_enabled(p2_en);

                self.triangle.enabled = tri_en;
                self.triangle.length_counter.set_enabled(tri_en);

                self.noise.enabled = noi_en;
                self.noise.length_counter.set_enabled(noi_en);

                self.dmc.set_enabled(dmc_en);
            }
            // Frame Counter Control ($4017)
            0x4017 => {
                let action = self.frame_counter.write_register(val);
                self.dispatch_frame_action(action);
            }
            _ => {}
        }
    }
}
