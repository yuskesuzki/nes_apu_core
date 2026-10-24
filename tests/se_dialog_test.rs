//! 効果音（SE）選択ダイアログおよびキー割り当てのテスト

use nes_apu_core::source::NesSoundDriver;
use nes_apu_core::emulator::ApuSystem;
use nes_apu_core::ui::{SeDialogState, SeSelectionTarget};
use std::path::PathBuf;

#[test]
fn test_apu_system_default_se_tracks() {
    let driver = Box::new(NesSoundDriver::new());
    let sys = ApuSystem::new_with_se(
        driver,
        PathBuf::from("dummy.nsf"),
        16, // total_tracks
        Some(1),
        Some(2),
    );

    assert_eq!(sys.total_tracks, 16);
    assert_eq!(sys.se_track_z, Some(1));
    assert_eq!(sys.se_track_x, Some(2));
}

#[test]
fn test_se_dialog_track_cycling() {
    let mut dialog = SeDialogState {
        target: SeSelectionTarget::Z,
        selected_track: 1,
        total_tracks: 5,
    };

    // 左キーで 1 から末尾 (5) へループ
    dialog.selected_track = if dialog.selected_track <= 1 {
        dialog.total_tracks
    } else {
        dialog.selected_track - 1
    };
    assert_eq!(dialog.selected_track, 5);

    // さらに左キーで 4 へ
    dialog.selected_track = if dialog.selected_track <= 1 {
        dialog.total_tracks
    } else {
        dialog.selected_track - 1
    };
    assert_eq!(dialog.selected_track, 4);

    // 右キーで 5 へ
    dialog.selected_track = if dialog.selected_track >= dialog.total_tracks {
        1
    } else {
        dialog.selected_track + 1
    };
    assert_eq!(dialog.selected_track, 5);

    // さらに右キーで 5 から先頭 (1) へループ
    dialog.selected_track = if dialog.selected_track >= dialog.total_tracks {
        1
    } else {
        dialog.selected_track + 1
    };
    assert_eq!(dialog.selected_track, 1);
}

#[test]
fn test_se_dialog_target_apply() {
    let mut sys = ApuSystem::new_with_se(
        Box::new(NesSoundDriver::new()),
        PathBuf::from("dummy.nsf"),
        10,
        Some(1),
        Some(2),
    );

    let dialog_z = SeDialogState {
        target: SeSelectionTarget::Z,
        selected_track: 7,
        total_tracks: 10,
    };
    match dialog_z.target {
        SeSelectionTarget::Z => sys.se_track_z = Some(dialog_z.selected_track),
        SeSelectionTarget::X => sys.se_track_x = Some(dialog_z.selected_track),
    }
    assert_eq!(sys.se_track_z, Some(7));
    assert_eq!(sys.se_track_x, Some(2)); // X は変化なし

    let dialog_x = SeDialogState {
        target: SeSelectionTarget::X,
        selected_track: 9,
        total_tracks: 10,
    };
    match dialog_x.target {
        SeSelectionTarget::Z => sys.se_track_z = Some(dialog_x.selected_track),
        SeSelectionTarget::X => sys.se_track_x = Some(dialog_x.selected_track),
    }
    assert_eq!(sys.se_track_z, Some(7)); // Z は維持
    assert_eq!(sys.se_track_x, Some(9)); // X が 9 に更新
}

#[test]
fn test_render_dashboard_with_dialog() {
    let sys = ApuSystem::new_with_se(
        Box::new(NesSoundDriver::new()),
        PathBuf::from("dummy.nsf"),
        10,
        Some(1),
        Some(2),
    );

    let dialog = Some(SeDialogState {
        target: SeSelectionTarget::Z,
        selected_track: 3,
        total_tracks: 10,
    });

    let snap = nes_apu_core::ui::SystemSnapshot::capture(&sys, 0.0, dialog);
    nes_apu_core::ui::dashboard::render_dashboard(&snap);
}
