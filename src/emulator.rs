//! NES APU System Coordinator (Emulator Layer)
//!
//! 純粋なハードウェア層である [`NesApuCore`] と、音源・演奏制御層である [`SoundSource`] を束ね、
//! 1.789773 MHz の CPU クロック進行と 29,780 サイクルごとの 60Hz V-Blank 割り込み同期を管理します。

use crate::apu::{NesApuCore, NTSC_CPU_CLOCK};
use crate::source::{SoundSource, NTSC_CYCLES_PER_FRAME};

pub struct ApuSystem {
    /// RP2A03 APU ハードウェアコア
    pub apu: NesApuCore,
    /// 現在アタッチされている音源（6502 ドライバまたは NSF プレイヤー）
    pub source: Box<dyn SoundSource>,
    /// V-Blank (60Hz) サイクルアキュムレータ (0..29,780)
    pub vblank_cycle_counter: usize,
}

impl ApuSystem {
    /// 指定された音源 (`SoundSource`) をアタッチしてシステムを初期化します
    pub fn new(mut source: Box<dyn SoundSource>) -> Self {
        let mut apu = NesApuCore::new();
        source.init(&mut apu);
        Self {
            apu,
            source,
            vblank_cycle_counter: 0,
        }
    }

    /// 新しい音源に差し替えます
    pub fn set_source(&mut self, mut source: Box<dyn SoundSource>) {
        source.init(&mut self.apu);
        self.source = source;
        self.vblank_cycle_counter = 0;
    }

    /// 1 CPU サイクル (約 558.7 ナノ秒) を進め、29,780 サイクルごとに V-Blank 更新を発火します
    pub fn step(&mut self) {
        self.apu.step();

        self.vblank_cycle_counter += 1;
        if self.vblank_cycle_counter >= NTSC_CYCLES_PER_FRAME {
            self.vblank_cycle_counter -= NTSC_CYCLES_PER_FRAME;
            self.source.on_vblank(&mut self.apu);
        }
    }

    /// 指定した CPU サイクル数だけ一括進行します
    pub fn clock_cycles(&mut self, cycles: usize) {
        for _ in 0..cycles {
            self.step();
        }
    }

    /// オーディオ出力サンプル周期 (例: 44.1kHz / 48kHz) 分の CPU サイクルを進め、
    /// V-Blank 同期を維持しながら非線形 DAC + 3 段 RC フィルター出力を生成します
    pub fn generate_audio_sample(&mut self, sample_rate: f32) -> f32 {
        if (self.apu.filter_chain.hpf1.sample_rate - sample_rate).abs() > 1.0 {
            self.apu.filter_chain.set_sample_rate(sample_rate);
        }

        let cycles_per_sample = (NTSC_CPU_CLOCK / sample_rate).round() as usize;
        let mut accum: f32 = 0.0;

        for _ in 0..cycles_per_sample {
            self.step();
            accum += self.apu.sample_nonlinear();
        }

        let raw_dac = if cycles_per_sample > 0 {
            accum / (cycles_per_sample as f32)
        } else {
            0.0
        };

        self.apu.filter_chain.process(raw_dac)
    }
}
