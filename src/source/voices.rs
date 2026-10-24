//! ファミコン音色マクロ定義 (`NesVoiceMacro`)
//!
//! 6502 サウンドドライバが 60Hz (V-Blank) ごとに参照するソフトウェア変調マクロ（音量、デューティ比、
//! アルペジオ、ピッチベンド）のデータ構造とプリセット音色バンクを定義します。

/// ファミコン音色マクロ
#[derive(Debug, Clone)]
pub struct NesVoiceMacro {
    pub name: String,
    pub vol_macro: Vec<u8>,
    pub vol_loop: usize,
    pub duty_macro: Vec<u8>,
    pub duty_loop: usize,
    pub arp_macro: Vec<i8>,
    pub arp_loop: usize,
    pub pitch_macro: Vec<i16>,
    pub pitch_loop: usize,
}

impl NesVoiceMacro {
    /// 標準のプリセット音色バンク (Voice 0..=6) を生成します
    pub fn default_bank() -> Vec<Self> {
        vec![
            Self::classic_lead(),    // 0: 50% 矩形波リード (マリオ等の王道リード)
            Self::arp_major(),       // 1: 疑似和音Arp
            Self::pluck_lead(),      // 2: プラックリード (25%)
            Self::triangle_bass(),   // 3: 三角波ベース
            Self::noise_snare(),     // 4: ノイズスネア
            Self::noise_hihat(),     // 5: 金属ハイハット
            Self::duty_sweep_lead(), // 6: Duty変調うねりリード
        ]
    }

    /// 0. クラシック・スクエアリード (50% 純矩形波: マリオ等の実機王道リード音色、温かく太い音)
    pub fn classic_lead() -> Self {
        Self {
            name: "50%矩形波リード".into(),
            vol_macro: vec![13, 13, 12, 12, 11],
            vol_loop: 4,
            duty_macro: vec![2], // 50.0% (奇数倍音のみ、金属感のない丸い音)
            duty_loop: 0,
            arp_macro: vec![0],
            arp_loop: 0,
            pitch_macro: vec![0],
            pitch_loop: 0,
        }
    }

    /// 1. 高速アルペジオ (疑似和音)
    pub fn arp_major() -> Self {
        Self {
            name: "疑似和音Arp".into(),
            vol_macro: vec![13, 12, 11, 11, 10, 10, 9],
            vol_loop: 4,
            duty_macro: vec![2],
            duty_loop: 0,
            arp_macro: vec![0, 4, 7], // ド -> ミ -> ソ (1/60秒回転)
            arp_loop: 0,
            pitch_macro: vec![0],
            pitch_loop: 0,
        }
    }

    /// 2. プラックリード (25% デューティ比)
    pub fn pluck_lead() -> Self {
        Self {
            name: "プラックリード".into(),
            vol_macro: vec![15, 13, 10, 7, 5, 3, 2, 1, 0],
            vol_loop: 8,
            duty_macro: vec![1], // 25%
            duty_loop: 0,
            arp_macro: vec![0],
            arp_loop: 0,
            pitch_macro: vec![0],
            pitch_loop: 0,
        }
    }

    /// 3. 三角波ベース
    pub fn triangle_bass() -> Self {
        Self {
            name: "三角波ベース".into(),
            vol_macro: vec![15],
            vol_loop: 0,
            duty_macro: vec![0],
            duty_loop: 0,
            arp_macro: vec![0],
            arp_loop: 0,
            pitch_macro: vec![0],
            pitch_loop: 0,
        }
    }

    /// 4. ノイズ・スネアドラム
    pub fn noise_snare() -> Self {
        Self {
            name: "ノイズスネア".into(),
            vol_macro: vec![15, 14, 12, 9, 6, 4, 2, 0],
            vol_loop: 7,
            duty_macro: vec![0],
            duty_loop: 0,
            arp_macro: vec![0],
            arp_loop: 0,
            pitch_macro: vec![0],
            pitch_loop: 0,
        }
    }

    /// 5. ノイズ・金属ハイハット
    pub fn noise_hihat() -> Self {
        Self {
            name: "金属ハイハット".into(),
            vol_macro: vec![12, 8, 4, 1, 0],
            vol_loop: 4,
            duty_macro: vec![0],
            duty_loop: 0,
            arp_macro: vec![0],
            arp_loop: 0,
            pitch_macro: vec![0],
            pitch_loop: 0,
        }
    }

    /// 6. デューティ比モジュレーション・リード (ファミコン名曲風のうねり)
    pub fn duty_sweep_lead() -> Self {
        Self {
            name: "Duty変調リード".into(),
            vol_macro: vec![14, 13, 12, 12, 11, 11, 10],
            vol_loop: 4,
            duty_macro: vec![0, 0, 1, 1, 2, 2, 1, 1], // 12.5% -> 25% -> 50% -> 25%
            duty_loop: 0,
            arp_macro: vec![0],
            arp_loop: 0,
            pitch_macro: vec![0, 0, 1, 2, 2, 1, 0, -1, -2, -2, -1],
            pitch_loop: 0,
        }
    }

    /// 指定されたフレームにおける音量マクロ値を取得します
    pub fn sample_volume(&self, frame: usize) -> u8 {
        if self.vol_macro.is_empty() {
            return 12;
        }
        let idx = Self::macro_index(frame, self.vol_macro.len(), self.vol_loop);
        self.vol_macro[idx]
    }

    /// 指定されたフレームにおけるデューティ比マクロ値 (0..=3) を取得します
    pub fn sample_duty(&self, frame: usize) -> u8 {
        if self.duty_macro.is_empty() {
            return 2;
        }
        let idx = Self::macro_index(frame, self.duty_macro.len(), self.duty_loop);
        self.duty_macro[idx] & 3
    }

    /// 指定されたフレームにおけるアルペジオ半音オフセットを取得します
    pub fn sample_arpeggio(&self, frame: usize) -> i8 {
        if self.arp_macro.is_empty() {
            return 0;
        }
        let idx = Self::macro_index(frame, self.arp_macro.len(), self.arp_loop);
        self.arp_macro[idx]
    }

    /// 指定されたフレームにおけるピッチデチューン値を取得します
    pub fn sample_pitch(&self, frame: usize) -> i16 {
        if self.pitch_macro.is_empty() {
            return 0;
        }
        let idx = Self::macro_index(frame, self.pitch_macro.len(), self.pitch_loop);
        self.pitch_macro[idx]
    }

    #[inline]
    fn macro_index(frame: usize, len: usize, loop_start: usize) -> usize {
        if frame < len {
            frame
        } else {
            let loop_len = len.saturating_sub(loop_start).max(1);
            loop_start + (frame - len) % loop_len
        }
    }
}
