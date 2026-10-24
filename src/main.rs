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
use nes_apu_core::audio::start_audio_stream;
use nes_apu_core::cli::CliArgs;
use nes_apu_core::ui::{
    dashboard::{clear_screen, render_dashboard},
    trace::run_trace_mode,
    SystemSnapshot,
};

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

    let frame_interval = Duration::from_nanos(16_666_667); // 60fps
    let start_time = Instant::now();
    let mut frame_count = 0;

    loop {
        let t0 = Instant::now();

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
