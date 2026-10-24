//! APU Length Counter
//!
//! 5-bit のインデックス値から 32 通りの長さをロードし、
//! ハーフフレームクロック (120Hz / 96Hz) で 0 までカウントダウンするハードウェアカウンタ。
//! 0 に達するとチャンネルは自動的に消音されます。

pub const LENGTH_COUNTER_TABLE: [u8; 32] = [
    10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14,
    12, 16, 24, 18, 48, 20, 96, 22, 192, 24, 72, 26, 16, 28, 32, 30,
];

#[derive(Debug, Clone, Default)]
pub struct LengthCounter {
    /// 現在の残りカウント (0 で消音)
    pub counter: u8,
    /// カウント停止 / ループフラグ (true の間はカウントダウンが停止)
    pub halt: bool,
    /// $4015 によるチャンネル有効化フラグ (false になると即座に 0 にクリア)
    pub enabled: bool,
}

impl LengthCounter {
    pub fn new() -> Self {
        Self::default()
    }

    /// 5-bit インデックス (0..31) から長さをロード
    pub fn load(&mut self, index: u8) {
        if self.enabled {
            self.counter = LENGTH_COUNTER_TABLE[(index & 0x1F) as usize];
        }
    }

    /// $4015 レジスタによる有効/無効化切り替え
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.counter = 0;
        }
    }

    /// ハーフフレームクロック (120Hz / 96Hz) でのデクリメント
    pub fn clock(&mut self) {
        if !self.halt && self.counter > 0 {
            self.counter -= 1;
        }
    }

    /// 消音すべきかどうか (カウンターが 0 なら消音)
    pub fn is_silenced(&self) -> bool {
        self.counter == 0
    }
}
