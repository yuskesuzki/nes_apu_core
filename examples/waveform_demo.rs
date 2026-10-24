//! 各チャンネルの波形（オシロスコープ・スパークライン）表示サンプル
//!
//! 実行方法:
//!   cargo run --example waveform_demo -- --nsf <FILE>
//!   cargo run --example waveform_demo -- --nsf <FILE> --oscilloscope

use std::thread;
use std::time::Duration;

use nes_apu_core::cli::CliArgs;
use nes_apu_core::ui::oscilloscope::format_oscilloscope_panel;
use nes_apu_core::ui::sparkline::format_channel_sparkline;
use nes_apu_core::ui::ChannelWaveformBuffer;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let is_oscilloscope = args.iter().any(|a| a == "--oscilloscope" || a == "-o");
    let nsf_path = args.windows(2).find_map(|w| {
        if w[0] == "--nsf" || w[0] == "-n" {
            Some(std::path::PathBuf::from(&w[1]))
        } else {
            None
        }
    });

    let nsf_path = match nsf_path {
        Some(path) => path,
        None => {
            eprintln!("使用方法: cargo run --example waveform_demo -- --nsf <FILE.nsf> [--oscilloscope]");
            std::process::exit(1);
        }
    };

    let cli_args = CliArgs {
        audio: false,
        nsf: nsf_path,
        track: None,
        profile: None,
        lpf: None,
        duty: None,
        filter_bypass: false,
        trace: false,
        mode: None,
        mode5: false,
        mode4: false,
        frames: None,
        se_z: None,
        se_x: None,
    };

    let mut sys = match cli_args.build_system() {
        Ok(s) => s,
        Err(err) => {
            eprintln!("{}", err);
            std::process::exit(1);
        }
    };
    let mut buffer = ChannelWaveformBuffer::new(60);

    println!("\x1b[2J\x1b[H");
    println!("=== NES APU Waveform CUI Monitor Demo ===");
    println!("Mode: {}", if is_oscilloscope { "2D Oscilloscope" } else { "1-Line Sparkline" });
    println!("(Press Ctrl+C to stop)\n");

    loop {
        // 1/60 秒 (約 29,780 CPU サイクル) 分進めつつ、波形をサンプリング
        for _ in 0..60 {
            sys.clock_cycles(29780 / 60);
            buffer.push(
                sys.apu.pulse1.output(),
                sys.apu.pulse2.output(),
                sys.apu.triangle.output(),
                sys.apu.noise.output(),
                sys.apu.dmc.output(),
            );
        }

        print!("\x1b[H");
        println!("=== NES APU Waveform CUI Monitor Demo ===");
        println!("Mode: {} (Switch with --oscilloscope / --sparkline)", if is_oscilloscope { "2D Oscilloscope" } else { "1-Line Sparkline" });
        println!("Source: {}", sys.source.title());
        println!("--------------------------------------------------");

        if is_oscilloscope {
            // 2D オシロスコープ表示 (高さ 4行 × 幅 48文字)
            let p1_panel = format_oscilloscope_panel("Pulse 1", &buffer.p1, 15, 48, 4);
            let p2_panel = format_oscilloscope_panel("Pulse 2", &buffer.p2, 15, 48, 4);
            let tri_panel = format_oscilloscope_panel("Triangle", &buffer.tri, 15, 48, 4);
            let noi_panel = format_oscilloscope_panel("Noise", &buffer.noise, 15, 48, 4);
            let dmc_panel = format_oscilloscope_panel("DMC", &buffer.dmc, 127, 48, 4);

            for line in p1_panel { println!("{}", line); }
            for line in p2_panel { println!("{}", line); }
            for line in tri_panel { println!("{}", line); }
            for line in noi_panel { println!("{}", line); }
            for line in dmc_panel { println!("{}", line); }
        } else {
            // 1行スパークライン表示
            println!("{}", format_channel_sparkline("Pulse1", &buffer.p1, sys.apu.pulse1.output(), 15, 40));
            println!("{}", format_channel_sparkline("Pulse2", &buffer.p2, sys.apu.pulse2.output(), 15, 40));
            println!("{}", format_channel_sparkline("Tri   ", &buffer.tri, sys.apu.triangle.output(), 15, 40));
            println!("{}", format_channel_sparkline("Noise ", &buffer.noise, sys.apu.noise.output(), 15, 40));
            println!("{}", format_channel_sparkline("DMC   ", &buffer.dmc, sys.apu.dmc.output(), 127, 40));
        }

        println!("--------------------------------------------------");
        thread::sleep(Duration::from_millis(50));
    }
}
