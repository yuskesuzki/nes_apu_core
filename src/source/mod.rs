//! Sound Source & Loader Layer (音源データ読み込み・再生制御層)
//!
//! NSF (NES Sound Format) ファイルの読み込みと 6502 CPU 実行、
//! およびハードウェア単体デモ（スイープ、エンベロープ、DMC 等）を統一した `SoundSource` トレイトで抽象化します。

pub mod driver;
pub mod nsf;
pub mod voices;

use crate::apu::NesApuCore;
pub use driver::{
    format_note, note_name, note_to_nes_timer, note_to_triangle_timer, NesSoundDriver,
    TrackState, NTSC_CYCLES_PER_FRAME,
};
pub use nsf::{NsfBus, NsfHeader, NsfSource};
pub use voices::NesVoiceMacro;

/// APU に対して初期レジスタ設定や V-Blank (60Hz: 29,780 CPU サイクル) ごとの制御を行う音源トレイト
pub trait SoundSource: Send {
    /// 現在の動作モード・曲名の説明文字列（ダッシュボード表示用）
    fn title(&self) -> String;

    /// 再生開始時の初期設定（初期レジスタ書き込みや DMC メモリのアタッチ等）
    fn init(&mut self, apu: &mut NesApuCore);

    /// 60Hz V-Blank (29,780 CPU サイクル) ごとに呼ばれる更新処理
    fn on_vblank(&mut self, apu: &mut NesApuCore);

    /// 6502 サウンドドライバとして動作している場合、その内部状態への参照を返す
    fn driver_state(&self) -> Option<&NesSoundDriver> {
        None
    }

    /// 6502 サウンドドライバとして動作している場合、その可変参照を返す
    fn driver_state_mut(&mut self) -> Option<&mut NesSoundDriver> {
        None
    }
}

impl SoundSource for NesSoundDriver {
    fn title(&self) -> String {
        format!("6502 演奏ドライバ同期再生: {}", self.song_title)
    }

    fn init(&mut self, _apu: &mut NesApuCore) {
        // 6502 ドライバは V-Blank 割り込みごとにレジスタを書き込むため初期化時は何もしない
    }

    fn on_vblank(&mut self, apu: &mut NesApuCore) {
        let writes = self.tick_frame();
        for (addr, val) in writes {
            apu.write_register(addr, val);
        }
    }

    fn driver_state(&self) -> Option<&NesSoundDriver> {
        Some(self)
    }

    fn driver_state_mut(&mut self) -> Option<&mut NesSoundDriver> {
        Some(self)
    }
}
