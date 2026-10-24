//! APU Sweep Unit (Pulse 1: $4001, Pulse 2: $4005)
//!
//! ハーフフレーム (120Hz / 96Hz) でタイマー周期を自動的に加算/減算し、
//! 周波数スライド（スイープ）を行うハードウェア回路。
//!
//! 【ファミコン実機の有名な非対称バグ挙動の再現】:
//! - 減算時 (Negate=1):
//!   - Pulse 1 (チャンネル0): 1の補数減算 (target = period - change - 1)
//!   - Pulse 2 (チャンネル1): 2の補数減算 (target = period - change)
//! - ミュート条件:
//!   - period < 8、または target > 0x7FF (2047) の場合、ハードウェアにより即座に強制消音される。

#[derive(Debug, Clone)]
pub struct SweepUnit {
    /// チャンネルID (0 = Pulse 1: 1の補数, 1 = Pulse 2: 2の補数)
    pub channel_id: usize,

    /// スイープ有効フラグ (bit 7)
    pub enabled: bool,
    /// スイープ分周周期 (bit 4-6: P -> P+1 回のハーフフレームで発火)
    pub period: u8,
    /// 減算フラグ (bit 3: 0 = 加算/周波数低下, 1 = 減算/周波数上昇)
    pub negate: bool,
    /// シフト量 (bit 0-2: S, 0..7)
    pub shift: u8,

    /// 分周カウンタ
    pub divider: u8,
    /// レジスタ書き込み時にセットされるリロードフラグ
    pub reload_flag: bool,
}

impl SweepUnit {
    pub fn new(channel_id: usize) -> Self {
        Self {
            channel_id,
            enabled: false,
            period: 0,
            negate: false,
            shift: 0,
            divider: 0,
            reload_flag: false,
        }
    }

    /// $4001 / $4005 レジスタ (EPPP NSSS) の書き込み
    pub fn write_register(&mut self, val: u8) {
        self.enabled = (val & 0x80) != 0;
        self.period = (val >> 4) & 0x07;
        self.negate = (val & 0x08) != 0;
        self.shift = val & 0x07;
        self.reload_flag = true;
    }

    /// 目標タイマー周期 (target period) を計算
    pub fn calculate_target(&self, current_period: u16) -> u16 {
        let change = current_period >> self.shift;
        if self.negate {
            if self.channel_id == 0 {
                // Pulse 1: 1の補数減算 (実機の加算器にキャリーインが入らないため 1 多く減算される)
                current_period.saturating_sub(change).saturating_sub(1)
            } else {
                // Pulse 2: 2の補数減算 (通常の減算)
                current_period.saturating_sub(change)
            }
        } else {
            current_period + change
        }
    }

    /// スイープによる強制消音（ミュート）判定
    /// 実機仕様: タイマーが 8 未満、または目標タイマーが 0x7FF (2047) を超えたとき消音
    pub fn is_muting(&self, current_period: u16) -> bool {
        if current_period < 8 {
            return true;
        }
        let target = self.calculate_target(current_period);
        target > 0x07FF
    }

    /// ハーフフレームクロック (120Hz / 96Hz) での更新
    /// 新しいタイマー周期を適用すべき場合は Some(new_period) を返す
    pub fn clock(&mut self, current_period: u16) -> Option<u16> {
        let mut new_period = None;

        if self.divider == 0 && self.enabled && self.shift > 0 && !self.is_muting(current_period) {
            new_period = Some(self.calculate_target(current_period));
        }

        if self.divider == 0 || self.reload_flag {
            self.divider = self.period;
            self.reload_flag = false;
        } else {
            self.divider -= 1;
        }

        new_period
    }
}
