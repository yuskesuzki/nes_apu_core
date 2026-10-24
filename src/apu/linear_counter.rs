//! APU Triangle Linear Counter ($4008, $400B)
//!
//! クォーターフレーム (240Hz / 192Hz) で動作する高分解能 (約4.1ms) タイマー。
//! $4008 の 7-bit リロード値により、ミリ秒単位の極めて短い音（スタッカート、ベースのアタック）
//! を正確に制御します。

#[derive(Debug, Clone, Default)]
pub struct LinearCounter {
    /// 現在のカウンタ値 (0..127)
    pub counter: u8,
    /// リロード値 ($4008 bit 0-6)
    pub reload_value: u8,
    /// 制御フラグ ($4008 bit 7: true の間はリロードフラグの自動クリアを抑制)
    pub control_flag: bool,
    /// $400B 書き込み時にセットされるリロードフラグ
    pub reload_flag: bool,
}

impl LinearCounter {
    pub fn new() -> Self {
        Self::default()
    }

    /// $4008 (CRRR RRRR) の書き込み
    pub fn write_control(&mut self, val: u8) {
        self.control_flag = (val & 0x80) != 0;
        self.reload_value = val & 0x7F;
    }

    /// $400B 書き込み時のリロード要求
    pub fn set_reload_flag(&mut self) {
        self.reload_flag = true;
    }

    /// クォーターフレーム (240Hz / 192Hz) での更新
    pub fn clock(&mut self) {
        if self.reload_flag {
            self.counter = self.reload_value;
        } else if self.counter > 0 {
            self.counter -= 1;
        }

        if !self.control_flag {
            self.reload_flag = false;
        }
    }

    /// 三角波を進めて良い状態かどうか (カウンタが 0 のときは停止)
    pub fn is_active(&self) -> bool {
        self.counter > 0
    }
}
