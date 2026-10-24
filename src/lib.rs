//! RP2A03 NES APU (Audio Processing Unit) - Modular Architecture
//!
//! 責務ごとに以下の3層＋コーディネータ層に分離されています：
//! 1. [`apu`]: 純粋な RP2A03 ハードウェアエミュレーション層 ($4000〜$4017 レジスタ、5ch、Frame Counter、RCフィルター)
//! 2. [`source`]: 音源データ読み込み（NSF ファイル等）・6502 サウンドドライバ・ハードウェア単体デモ層
//! 3. [`ui`]: ターミナルダッシュボード描画・サイクル単位トレース出力層
//! 4. [`emulator`]: APU と SoundSource を束ねて 60Hz V-Blank (29,780 サイクル) 同期で駆動するシステムコーディネータ

pub mod apu;
pub mod audio;
pub mod cli;
pub mod emulator;
pub mod source;
pub mod ui;

// 既存テストや外部利用との互換性のためのモジュール＆型再エクスポート
pub use apu::{
    core, dmc, envelope, filter, frame_counter, length_counter, linear_counter, noise, pulse,
    sweep, triangle,
};
pub use source::driver;
pub use source::nsf;

pub use apu::{
    AnalogFilterChain, DmcChannel, EnvelopeGenerator, FrameClockAction, FrameCounter,
    HighPassFilter, LengthCounter, LinearCounter, LowPassFilter, NesApuCore, NoiseChannel,
    NonlinearMixer, PulseChannel, SweepUnit, TriangleChannel, DMC_NTSC_RATE_TABLE,
    LENGTH_COUNTER_TABLE, NOISE_PERIOD_TABLE_NTSC, NTSC_CPU_CLOCK, PULSE_DUTY_TABLE,
    TRIANGLE_SEQUENCE,
};
pub use emulator::ApuSystem;
pub use source::{
    format_note, note_name, note_to_nes_timer, note_to_triangle_timer,
    NesSoundDriver, NesVoiceMacro,
    NsfBus, NsfHeader, NsfSource, SoundSource, TrackState, NTSC_CYCLES_PER_FRAME,
};
pub use ui::SystemSnapshot;
