//! Presentation / Renderer Layer (画面描画層)
//!
//! エミュレータ本体（`ApuSystem` / `NesApuCore`）から状態スナップショットを受け取り、
//! ターミナルダッシュボードや 1 サイクル刻みのハードウェアトレース表、
//! および波形グラフ（スパークライン／オシロスコープ）を描画します。

pub mod dashboard;
pub mod oscilloscope;
pub mod sparkline;
pub mod trace;
pub mod waveform_buffer;

use crate::apu::NesApuCore;
use crate::emulator::ApuSystem;
use crate::source::NesSoundDriver;
pub use oscilloscope::{format_oscilloscope_panel, render_waveform_grid};
pub use sparkline::{format_channel_sparkline, render_sparkline};
pub use waveform_buffer::ChannelWaveformBuffer;

/// 描画用のシステム状態スナップショット（オーディオロックを最小限にするためクローンで保持）
#[derive(Debug, Clone)]
pub struct SystemSnapshot {
    pub apu: NesApuCore,
    pub mode_title: String,
    pub driver: Option<NesSoundDriver>,
    pub vblank_cycle_counter: usize,
    pub elapsed_real_secs: f64,
}

impl SystemSnapshot {
    /// 現在の `ApuSystem` から描画用スナップショットを抽出します
    pub fn capture(sys: &ApuSystem, elapsed_real_secs: f64) -> Self {
        Self {
            apu: sys.apu.clone(),
            mode_title: sys.source.title(),
            driver: sys.source.driver_state().cloned(),
            vblank_cycle_counter: sys.vblank_cycle_counter,
            elapsed_real_secs,
        }
    }
}
