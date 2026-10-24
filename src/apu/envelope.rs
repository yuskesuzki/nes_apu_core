//! APU Envelope Generator
//!
//! クォーターフレーム (240Hz / 192Hz) で動作する 4-bit ハードウェア音量減衰回路。
//! 音符のアタック直後に 15 から 0 へ自動的に音量を減衰させ、
//! ループフラグによりノコギリ波状の連続エンベロープも生成可能です。

#[derive(Debug, Clone)]
pub struct EnvelopeGenerator {
    /// 4番目レジスタ書き込みでセットされるリスタートフラグ
    pub start_flag: bool,
    /// 減衰周期分周カウンタ (V から 0 へカウントダウン)
    pub divider: u8,
    /// 4-bit 減衰カウンタ (15 から 0 へ減少)
    pub decay_counter: u8,

    /// 固定音量フラグ (true: 音量値を直接出力, false: ハードウェア減衰を出力)
    pub constant_volume: bool,
    /// ループフラグ (true: 0 に達したとき 15 にリロードしてループ)
    pub loop_flag: bool,
    /// 音量 / 分周周期リロード値 (0..15)
    pub volume_reload: u8,
}

impl Default for EnvelopeGenerator {
    fn default() -> Self {
        Self {
            start_flag: false,
            divider: 0,
            decay_counter: 0,
            constant_volume: true,
            loop_flag: false,
            volume_reload: 0,
        }
    }
}

impl EnvelopeGenerator {
    pub fn new() -> Self {
        Self::default()
    }

    /// レジスタ 0 ($4000, $4004, $400C: --LC VVVV) の書き込み
    pub fn write_control(&mut self, val: u8) {
        self.loop_flag = (val & 0x20) != 0;
        self.constant_volume = (val & 0x10) != 0;
        self.volume_reload = val & 0x0F;
    }

    /// レジスタ 3 ($4003, $4007, $400F) への書き込み時にエンベロープを再起動
    pub fn restart(&mut self) {
        self.start_flag = true;
    }

    /// クォーターフレームクロック (240Hz / 192Hz) での更新
    pub fn clock(&mut self) {
        if self.start_flag {
            self.start_flag = false;
            self.decay_counter = 15;
            self.divider = self.volume_reload;
        } else if self.divider == 0 {
            self.divider = self.volume_reload;
            if self.decay_counter > 0 {
                self.decay_counter -= 1;
            } else if self.loop_flag {
                self.decay_counter = 15;
            }
        } else {
            self.divider -= 1;
        }
    }

    /// 現在の出力音量 (0..15) を取得
    pub fn output_volume(&self) -> u8 {
        if self.constant_volume {
            self.volume_reload
        } else {
            self.decay_counter
        }
    }
}
