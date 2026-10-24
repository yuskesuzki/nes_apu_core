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

/// トラック選択ダイアログの対象 (BGM、SE-Z、SE-X)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackDialogTarget {
    Bgm,
    Z,
    X,
}

pub type SeSelectionTarget = TrackDialogTarget;

/// トラック選択ダイアログの表示状態
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeDialogState {
    /// 選択対象 (BGM、SE-Z、SE-X)
    pub target: TrackDialogTarget,
    /// 現在ダイアログ上で選択されているトラック番号 (1-based)
    pub selected_track: u8,
    /// NSF の総トラック数
    pub total_tracks: u8,
    /// ダイアログを開いた時点の元のトラック番号（キャンセル復元用）
    pub original_track: u8,
}

/// 描画用のシステム状態スナップショット（オーディオロックを最小限にするためクローンで保持）
#[derive(Debug, Clone)]
pub struct SystemSnapshot {
    pub apu: NesApuCore,
    pub mode_title: String,
    pub driver: Option<NesSoundDriver>,
    pub vblank_cycle_counter: usize,
    pub elapsed_real_secs: f64,
    pub is_paused: bool,
    pub total_tracks: u8,
    pub current_bgm_track: u8,
    pub se_track_z: Option<u8>,
    pub se_track_x: Option<u8>,
    pub se_current_track: Option<u8>,
    pub se_dialog: Option<SeDialogState>,
}

impl SystemSnapshot {
    /// 現在の `ApuSystem` から描画用スナップショットを抽出します
    pub fn capture(
        sys: &ApuSystem,
        elapsed_real_secs: f64,
        se_dialog: Option<SeDialogState>,
    ) -> Self {
        Self {
            apu: sys.apu.clone(),
            mode_title: sys.source.title(),
            driver: sys.source.driver_state().cloned(),
            vblank_cycle_counter: sys.vblank_cycle_counter,
            elapsed_real_secs,
            is_paused: sys.paused,
            total_tracks: sys.total_tracks,
            current_bgm_track: sys.current_bgm_track,
            se_track_z: sys.se_track_z,
            se_track_x: sys.se_track_x,
            se_current_track: sys.se_current_track,
            se_dialog,
        }
    }
}
