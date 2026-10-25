//! NES APU Core - Terminal Clock Monitor & Diagnostic Dashboard
//!
//! エントリーポイント:
//! - `cli`: Clap によるコマンドライン引数の解析
//! - `source`: NSF バイナリ楽曲やハードウェアデモのロード
//! - `apu` & `emulator`: RP2A03 APU のサイクル進行と V-Blank 同期
//! - `ui`: ターミナルダッシュボードおよびトレース表の描画

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use nes_apu_core::audio::start_audio_stream;
use nes_apu_core::cli::CliArgs;
use nes_apu_core::ui::{
    dashboard::{clear_screen, render_dashboard},
    trace::run_trace_mode,
    SeDialogState, TrackDialogTarget, SystemSnapshot,
};

/// ターミナル raw モードの安全な復元用 RAII ガード
struct RawModeGuard;

impl RawModeGuard {
    fn new() -> Option<Self> {
        if enable_raw_mode().is_ok() {
            Some(Self)
        } else {
            None
        }
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
    }
}

fn main() {
    let args = CliArgs::parse();
    let system = match args.build_system() {
        Ok(sys) => sys,
        Err(err) => {
            eprintln!("{}", err);
            std::process::exit(1);
        }
    };

    // 1 CPU サイクル刻みのトレースモード (--trace)
    if args.trace {
        let mut sys = system;
        run_trace_mode(&mut sys, 32);
        return;
    }

    let shared_system = Arc::new(Mutex::new(system));

    let _stream = if args.audio {
        println!("🔊 CPAL オーディオ出力デバイスを起動中...");
        start_audio_stream(Arc::clone(&shared_system))
    } else {
        None
    };

    clear_screen();

    let _raw_guard = if args.frames.is_none() {
        RawModeGuard::new()
    } else {
        None
    };

    let frame_interval = Duration::from_nanos(16_666_667); // 60fps
    let start_time = Instant::now();
    let mut frame_count = 0;
    let mut dialog_state: Option<SeDialogState> = None;

    loop {
        let t0 = Instant::now();

        // キーボード入力処理 (非ブロッキング: 1~5 キーで各チャンネルのミュート/発音切替, Shift+Z/X で SE 設定ダイアログ)
        while event::poll(Duration::from_millis(0)).unwrap_or(false) {
            if let Ok(Event::Key(key)) = event::read() {
                if key.kind == KeyEventKind::Press {
                    // Ctrl+C はいつでも終了
                    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                        return;
                    }

                    // ダイアログ表示中のキー操作
                    if let Some(ref mut d) = dialog_state {
                        let is_shift_t = key.code == KeyCode::Char('T')
                            || ((key.code == KeyCode::Char('t') || key.code == KeyCode::Char('T'))
                                && key.modifiers.contains(KeyModifiers::SHIFT));
                        let is_shift_z = key.code == KeyCode::Char('Z')
                            || ((key.code == KeyCode::Char('z') || key.code == KeyCode::Char('Z'))
                                && key.modifiers.contains(KeyModifiers::SHIFT));
                        let is_shift_x = key.code == KeyCode::Char('X')
                            || ((key.code == KeyCode::Char('x') || key.code == KeyCode::Char('X'))
                                && key.modifiers.contains(KeyModifiers::SHIFT));

                        if is_shift_t {
                            let sys = shared_system.lock().unwrap();
                            d.target = TrackDialogTarget::Bgm;
                            d.selected_track = sys.current_bgm_track;
                            d.original_track = sys.current_bgm_track;
                            continue;
                        }
                        if is_shift_z {
                            let sys = shared_system.lock().unwrap();
                            d.target = TrackDialogTarget::Z;
                            d.selected_track = sys.se_track_z.unwrap_or(1);
                            d.original_track = d.selected_track;
                            continue;
                        }
                        if is_shift_x {
                            let sys = shared_system.lock().unwrap();
                            d.target = TrackDialogTarget::X;
                            d.selected_track = sys.se_track_x.unwrap_or(if d.total_tracks >= 2 { 2 } else { 1 });
                            d.original_track = d.selected_track;
                            continue;
                        }

                        match key.code {
                            KeyCode::Left => {
                                d.selected_track = if d.selected_track <= 1 {
                                    d.total_tracks
                                } else {
                                    d.selected_track - 1
                                };
                            }
                            KeyCode::Right => {
                                d.selected_track = if d.selected_track >= d.total_tracks {
                                    1
                                } else {
                                    d.selected_track + 1
                                };
                            }
                            KeyCode::Char(' ') => {
                                let mut sys = shared_system.lock().unwrap();
                                match d.target {
                                    TrackDialogTarget::Bgm => {
                                        // 選択中のトラックに即座に切り替えて試聴！
                                        sys.switch_bgm_track(d.selected_track);
                                    }
                                    _ => {
                                        // SE を試聴プレビュー再生
                                        sys.trigger_se(Some(d.selected_track));
                                    }
                                }
                            }
                            KeyCode::Enter => {
                                // 確定
                                let mut sys = shared_system.lock().unwrap();
                                match d.target {
                                    TrackDialogTarget::Bgm => {
                                        sys.switch_bgm_track(d.selected_track);
                                    }
                                    TrackDialogTarget::Z => sys.se_track_z = Some(d.selected_track),
                                    TrackDialogTarget::X => sys.se_track_x = Some(d.selected_track),
                                }
                                dialog_state = None;
                                clear_screen();
                            }
                            KeyCode::Esc | KeyCode::Char('q') => {
                                // 取消（ダイアログを閉じる）
                                if d.target == TrackDialogTarget::Bgm && d.selected_track != d.original_track {
                                    // Space 試聴等で変わっていた場合は元のトラックに戻す
                                    let mut sys = shared_system.lock().unwrap();
                                    sys.switch_bgm_track(d.original_track);
                                }
                                dialog_state = None;
                                clear_screen();
                            }
                            _ => {}
                        }
                        continue;
                    }

                    // 通常演奏時のキー操作
                    let is_shift_t = key.code == KeyCode::Char('T')
                        || ((key.code == KeyCode::Char('t') || key.code == KeyCode::Char('T'))
                            && key.modifiers.contains(KeyModifiers::SHIFT));
                    let is_shift_z = key.code == KeyCode::Char('Z')
                        || ((key.code == KeyCode::Char('z') || key.code == KeyCode::Char('Z'))
                            && key.modifiers.contains(KeyModifiers::SHIFT));
                    let is_shift_x = key.code == KeyCode::Char('X')
                        || ((key.code == KeyCode::Char('x') || key.code == KeyCode::Char('X'))
                            && key.modifiers.contains(KeyModifiers::SHIFT));

                    if is_shift_t {
                        let sys = shared_system.lock().unwrap();
                        let total = sys.total_tracks;
                        let cur = sys.current_bgm_track;
                        dialog_state = Some(SeDialogState {
                            target: TrackDialogTarget::Bgm,
                            selected_track: cur,
                            total_tracks: total,
                            original_track: cur,
                        });
                        clear_screen();
                        continue;
                    }

                    if is_shift_z {
                        let sys = shared_system.lock().unwrap();
                        let total = sys.total_tracks;
                        let cur = sys.se_track_z.unwrap_or(1);
                        dialog_state = Some(SeDialogState {
                            target: TrackDialogTarget::Z,
                            selected_track: cur,
                            total_tracks: total,
                            original_track: cur,
                        });
                        clear_screen();
                        continue;
                    }

                    if is_shift_x {
                        let sys = shared_system.lock().unwrap();
                        let total = sys.total_tracks;
                        let cur = sys.se_track_x.unwrap_or(if total >= 2 { 2 } else { 1 });
                        dialog_state = Some(SeDialogState {
                            target: TrackDialogTarget::X,
                            selected_track: cur,
                            total_tracks: total,
                            original_track: cur,
                        });
                        clear_screen();
                        continue;
                    }

                    match key.code {
                        KeyCode::Char('1') => {
                            let mut sys = shared_system.lock().unwrap();
                            sys.apu.toggle_mute_channel(0);
                        }
                        KeyCode::Char('2') => {
                            let mut sys = shared_system.lock().unwrap();
                            sys.apu.toggle_mute_channel(1);
                        }
                        KeyCode::Char('3') => {
                            let mut sys = shared_system.lock().unwrap();
                            sys.apu.toggle_mute_channel(2);
                        }
                        KeyCode::Char('4') => {
                            let mut sys = shared_system.lock().unwrap();
                            sys.apu.toggle_mute_channel(3);
                        }
                        KeyCode::Char('5') => {
                            let mut sys = shared_system.lock().unwrap();
                            sys.apu.toggle_mute_channel(4);
                        }
                        KeyCode::Char(' ') => {
                            let mut sys = shared_system.lock().unwrap();
                            sys.toggle_pause();
                        }
                        KeyCode::Char('z') => {
                            let mut sys = shared_system.lock().unwrap();
                            let trk = sys.se_track_z;
                            sys.trigger_se(trk);
                        }
                        KeyCode::Char('x') => {
                            let mut sys = shared_system.lock().unwrap();
                            let trk = sys.se_track_x;
                            sys.trigger_se(trk);
                        }
                        KeyCode::Char('c') => {
                            let mut sys = shared_system.lock().unwrap();
                            sys.stop_se();
                        }
                        KeyCode::Char('q') | KeyCode::Esc => {
                            return;
                        }
                        _ => {}
                    }
                }
            }
        }

        // オーディオ非再生時はメインスレッド側で 1/60 秒分 (44.1kHz * 1/60 = 735 サンプル) のクロックを進める
        let snapshot = {
            let mut sys = shared_system.lock().unwrap();
            if !args.audio && !sys.paused {
                for _ in 0..735 {
                    sys.generate_audio_sample(44100.0);
                }
            }
            let elapsed_real = start_time.elapsed().as_secs_f64();
            SystemSnapshot::capture(&sys, elapsed_real, dialog_state)
        };

        // ロックを解放した状態で画面を描画
        render_dashboard(&snapshot);

        frame_count += 1;
        if let Some(max) = args.frames {
            if frame_count >= max {
                break;
            }
        }

        let elapsed = t0.elapsed();
        if elapsed < frame_interval {
            thread::sleep(frame_interval - elapsed);
        }
    }
}
