//! APU Frame Counter ($4017)
//!
//! NTSC CPU クロック (1.789773 MHz) から 240Hz (4ステップ) / 192Hz (5ステップ) の
//! ハードウェア変調クロック (クォーターフレーム / ハーフフレーム) を生成するシーケンサ。
//!
//! 【ステップのタイミング (NTSC CPU サイクル)】:
//! - 4-step モード ($4017 bit 7 = 0):
//!   - Step 1 ( 7,457 サイクル): クォーターフレーム (Envelope, Linear)
//!   - Step 2 (14,913 サイクル): ハーフフレーム (Envelope, Linear, Length, Sweep)
//!   - Step 3 (22,371 サイクル): クォーターフレーム (Envelope, Linear)
//!   - Step 4 (29,828 サイクル): ハーフフレーム + Frame IRQ (bit 6 = 0 の場合)
//!   - リセット (29,829 サイクル)
//! - 5-step モード ($4017 bit 7 = 1):
//!   - Step 1 ( 7,457 サイクル): クォーターフレーム
//!   - Step 2 (14,913 サイクル): ハーフフレーム
//!   - Step 3 (22,371 サイクル): クォーターフレーム
//!   - Step 4 (29,828 サイクル): 何もしない
//!   - Step 5 (37,281 サイクル): ハーフフレーム
//!   - リセット (37,282 サイクル, IRQなし)

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameClockAction {
    None,
    QuarterFrame,
    HalfFrame,
}

#[derive(Debug, Clone)]
pub struct FrameCounter {
    /// 5-step モードフラグ (false = 4-step 240Hz, true = 5-step 192Hz)
    pub mode_5_step: bool,
    /// 割り込み抑制フラグ (true: Frame IRQ を発生させない)
    pub interrupt_inhibit: bool,
    /// Frame IRQ 発生フラグ
    pub irq_flag: bool,

    /// サイクルカウンタ (0..29829 または 0..37282)
    pub cycle_counter: usize,

    /// 直前のステップ番号 (表示用: 1..4 または 1..5)
    pub current_step: usize,
}

impl Default for FrameCounter {
    fn default() -> Self {
        Self {
            mode_5_step: false,
            interrupt_inhibit: false,
            irq_flag: false,
            cycle_counter: 0,
            current_step: 0,
        }
    }
}

impl FrameCounter {
    pub fn new() -> Self {
        Self::default()
    }

    /// $4017 レジスタ (MI-- ----) の書き込み
    /// モード変更時は直ちにクォーター/ハーフフレームのアクションが発生する場合がある
    pub fn write_register(&mut self, val: u8) -> FrameClockAction {
        self.mode_5_step = (val & 0x80) != 0;
        self.interrupt_inhibit = (val & 0x40) != 0;

        if self.interrupt_inhibit {
            self.irq_flag = false;
        }

        self.cycle_counter = 0;
        self.current_step = 0;

        // 5-step モードに切り替えられた瞬間、実機は即座にハーフフレーム処理を発火する
        if self.mode_5_step {
            FrameClockAction::HalfFrame
        } else {
            FrameClockAction::None
        }
    }

    /// 1 CPU サイクル進める
    /// クォーターフレーム、ハーフフレームの発生タイミングを判定して返す
    pub fn step(&mut self) -> FrameClockAction {
        self.cycle_counter += 1;

        if !self.mode_5_step {
            // --- 4-step モード (240Hz) ---
            match self.cycle_counter {
                7457 => {
                    self.current_step = 1;
                    FrameClockAction::QuarterFrame
                }
                14913 => {
                    self.current_step = 2;
                    FrameClockAction::HalfFrame
                }
                22371 => {
                    self.current_step = 3;
                    FrameClockAction::QuarterFrame
                }
                29828 => {
                    self.current_step = 4;
                    if !self.interrupt_inhibit {
                        self.irq_flag = true;
                    }
                    FrameClockAction::HalfFrame
                }
                29829 => {
                    self.cycle_counter = 0;
                    FrameClockAction::None
                }
                _ => FrameClockAction::None,
            }
        } else {
            // --- 5-step モード (192Hz) ---
            match self.cycle_counter {
                7457 => {
                    self.current_step = 1;
                    FrameClockAction::QuarterFrame
                }
                14913 => {
                    self.current_step = 2;
                    FrameClockAction::HalfFrame
                }
                22371 => {
                    self.current_step = 3;
                    FrameClockAction::QuarterFrame
                }
                29828 => {
                    self.current_step = 4;
                    FrameClockAction::None // 5-step の step 4 はスキップ
                }
                37281 => {
                    self.current_step = 5;
                    FrameClockAction::HalfFrame
                }
                37282 => {
                    self.cycle_counter = 0;
                    FrameClockAction::None
                }
                _ => FrameClockAction::None,
            }
        }
    }
}
