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
    SystemSnapshot,
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

    loop {
        let t0 = Instant::now();

        // キーボード入力処理 (非ブロッキング: 1~5 キーで各チャンネルのミュート/発音切替)
        while event::poll(Duration::from_millis(0)).unwrap_or(false) {
            if let Ok(Event::Key(key)) = event::read() {
                if key.kind == KeyEventKind::Press {
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
                        KeyCode::Char('q') | KeyCode::Esc => {
                            return;
                        }
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
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
            if !args.audio {
                for _ in 0..735 {
                    sys.generate_audio_sample(44100.0);
                }
            }
            let elapsed_real = start_time.elapsed().as_secs_f64();
            SystemSnapshot::capture(&sys, elapsed_real)
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
