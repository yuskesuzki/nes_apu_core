use nes_apu_core::frame_counter::{FrameCounter, FrameClockAction};

#[test]
fn test_frame_counter_4_step_mode() {
    let mut fc = FrameCounter::new();
    fc.write_register(0x00); // 4-step mode, IRQ enabled

    let mut quarter_frames = 0;
    let mut half_frames = 0;

    // 1フレーム分 (29830 CPU サイクル) を進める
    for cycle in 1..=29830 {
        let action = fc.step();
        match action {
            FrameClockAction::QuarterFrame => {
                quarter_frames += 1;
                // Step 1: 7457, Step 3: 22371
                assert!(cycle == 7457 || cycle == 22371);
            }
            FrameClockAction::HalfFrame => {
                half_frames += 1;
                // Step 2: 14913, Step 4: 29828
                assert!(cycle == 14913 || cycle == 29828);
                if cycle == 29828 {
                    assert!(fc.irq_flag, "IRQ flag must be set at cycle 29828 in 4-step mode");
                }
            }
            FrameClockAction::None => {}
        }
    }

    assert_eq!(quarter_frames, 2, "4-step mode must have exactly 2 pure quarter frames");
    assert_eq!(half_frames, 2, "4-step mode must have exactly 2 half frames (total 4 clocks)");
}

#[test]
fn test_frame_counter_5_step_mode() {
    let mut fc = FrameCounter::new();
    // 5-step mode ($4017 bit 7 = 1)
    let initial_action = fc.write_register(0x80);
    assert_eq!(initial_action, FrameClockAction::HalfFrame, "Writing mode 1 immediately triggers half frame");

    let mut quarter_frames = 0;
    let mut half_frames = 0;

    // 1シーケンス分 (37283 CPU サイクル) を進める
    for cycle in 1..=37283 {
        let action = fc.step();
        match action {
            FrameClockAction::QuarterFrame => {
                quarter_frames += 1;
                assert!(cycle == 7457 || cycle == 22371);
            }
            FrameClockAction::HalfFrame => {
                half_frames += 1;
                assert!(cycle == 14913 || cycle == 37281);
            }
            FrameClockAction::None => {}
        }
    }

    assert_eq!(quarter_frames, 2);
    assert_eq!(half_frames, 2);
    assert!(!fc.irq_flag, "5-step mode must NEVER generate an IRQ");
}

#[test]
fn test_frame_counter_irq_inhibit() {
    let mut fc = FrameCounter::new();
    // 4-step mode with interrupt inhibit ($4017 bit 6 = 1)
    fc.write_register(0x40);

    for _ in 0..30000 {
        fc.step();
    }

    assert!(!fc.irq_flag, "IRQ must be inhibited when bit 6 is set");
}
