//! ターミナル・リアルタイム診断ダッシュボード描画

use crate::apu::NTSC_CPU_CLOCK;
use crate::source::format_note;
use crate::ui::SystemSnapshot;

#[cfg(not(target_os = "windows"))]
const CLEAR_SCREEN: &str = "\x1b[2J\x1b[H";
#[cfg(target_os = "windows")]
const CLEAR_SCREEN: &str = "";

/// 画面全体をクリアします
pub fn clear_screen() {
    println!("{}", CLEAR_SCREEN);
}

fn print_meter(val: u8, max: u8) -> String {
    let filled = val as usize;
    let empty = (max as usize).saturating_sub(filled);
    format!("[{}{}] {:2}", "█".repeat(filled), " ".repeat(empty), val)
}

fn print_meter_128(val: u8, max: u8) -> String {
    let scaled = ((val as usize) * 15) / (max as usize);
    let filled = scaled.min(15);
    let empty = 15 - filled;
    format!("[{}{}] {:3}", "█".repeat(filled), " ".repeat(empty), val)
}

fn print_bipolar_meter(val: f32) -> String {
    let clamped = val.clamp(-1.0, 1.0);
    let int_pos = ((clamped + 1.0) * 10.0).round() as usize; // 0..20
    let mut chars = vec![' '; 21];
    chars[10] = '|';
    if int_pos <= 10 {
        for i in int_pos..10 {
            chars[i] = '◄';
        }
    } else {
        for i in 11..=int_pos {
            chars[i] = '►';
        }
    }
    let bar: String = chars.into_iter().collect();
    format!("[{}] {:+5.2}", bar, val)
}

/// スナップショットをもとにターミナルへダッシュボードを 1 フレーム描画します
pub fn render_dashboard(snap: &SystemSnapshot) {
    let a = &snap.apu;
    let sim_time = a.total_cpu_cycles as f64 / NTSC_CPU_CLOCK as f64;

    print!("\x1b[H");
    println!("================================================================================");
    println!("🕹  NES APU Hardware Core - Phase 5 Complete System Dashboard");
    println!(
        "    実時間: {:06.2}s | シミュレーション時間: {:06.2}s | CPU サイクル: {:010}",
        snap.elapsed_real_secs, sim_time, a.total_cpu_cycles
    );
    println!("    動作モード: {}", snap.mode_title);
    println!("================================================================================");

    // --- 0. APU Frame Counter ($4017) ---
    let fc = &a.frame_counter;
    let fc_mode_str = if fc.mode_5_step {
        "5-Step モード (192Hz)"
    } else {
        "4-Step モード (240Hz)"
    };
    let fc_max_cyc = if fc.mode_5_step { 37282 } else { 29829 };
    let irq_str = if fc.irq_flag {
        "\x1b[1;31m[IRQ 発生中!]\x1b[0m"
    } else {
        "[IRQ なし]"
    };
    println!("┌─ [ APU Frame Counter ($4017) シーケンサ ] ────────────────────────────────────┐");
    println!(
        "│  動作モード: {:22} | 現在ステップ: Step {:1} (サイクル: {:5}/{:5}) │",
        fc_mode_str, fc.current_step, fc.cycle_counter, fc_max_cyc
    );
    println!(
        "│  IRQ 状態: {:35} | 割込抑制: {}                 │",
        irq_str,
        if fc.interrupt_inhibit {
            "有効 (抑制中)"
        } else {
            "無効"
        }
    );
    println!("└───────────────────────────────────────────────────────────────────────────────┘");

    // --- APU Status Register ($4015) ---
    let p1_act = if a.pulse1.length_counter.counter > 0 { "ON " } else { "off" };
    let p2_act = if a.pulse2.length_counter.counter > 0 { "ON " } else { "off" };
    let tri_act = if a.triangle.length_counter.counter > 0 { "ON " } else { "off" };
    let noi_act = if a.noise.length_counter.counter > 0 { "ON " } else { "off" };
    let dmc_act = if a.dmc.bytes_remaining > 0 { "ON " } else { "off" };
    let f_irq = if a.frame_counter.irq_flag { "IRQ!" } else { "--- " };
    let d_irq = if a.dmc.irq_flag { "IRQ!" } else { "--- " };
    let m_p1 = if a.channel_mute[0] { "\x1b[1;31mMUTE\x1b[0m" } else { "PLAY" };
    let m_p2 = if a.channel_mute[1] { "\x1b[1;31mMUTE\x1b[0m" } else { "PLAY" };
    let m_tr = if a.channel_mute[2] { "\x1b[1;31mMUTE\x1b[0m" } else { "PLAY" };
    let m_ns = if a.channel_mute[3] { "\x1b[1;31mMUTE\x1b[0m" } else { "PLAY" };
    let m_dm = if a.channel_mute[4] { "\x1b[1;31mMUTE\x1b[0m" } else { "PLAY" };
    println!("┌─ [ APU Status Register ($4015) 状態 ] ────────────────────────────────────────┐");
    println!(
        "│  Active: P1:[{}] P2:[{}] Tri:[{}] Noi:[{}] DMC:[{}] | IRQ: Frame:[{}] DMC:[{}] │",
        p1_act, p2_act, tri_act, noi_act, dmc_act, f_irq, d_irq
    );
    println!(
        "│  出力指定: P1:[{}] P2:[{}] Tri:[{}] Noi:[{}] DMC:[{}]                  │",
        m_p1, m_p2, m_tr, m_ns, m_dm
    );
    println!("└───────────────────────────────────────────────────────────────────────────────┘");

    // --- 1. Pulse 1 ---
    let p1 = &a.pulse1;
    let p1_freq = p1.calculate_frequency(NTSC_CPU_CLOCK);
    let p1_target = p1.sweep.calculate_target(p1.timer_period);
    let p1_target_freq = if p1_target >= 8 {
        NTSC_CPU_CLOCK / (16.0 * (p1_target as f32 + 1.0))
    } else {
        0.0
    };
    let p1_swp_mute = if p1.sweep.is_muting(p1.timer_period) {
        "MUTE (範囲外)"
    } else {
        "OK (正常)"
    };
    let p1_env_str = if p1.envelope.constant_volume {
        format!("固定音量: {:2}", p1.envelope.volume_reload)
    } else {
        format!(
            "ハード減衰: {:2} (V={}) Loop:{}",
            p1.envelope.decay_counter, p1.envelope.volume_reload, p1.envelope.loop_flag
        )
    };
    let p1_len_str = if p1.length_counter.is_silenced() {
        "消音 (0)"
    } else {
        "発音中"
    };
    let p1_header = if a.channel_mute[0] {
        "┌─ [ Pulse 1 (矩形波 1) ] \x1b[1;31m[MUTE]\x1b[0m ──────────────────────────────────────────────────┐"
    } else {
        "┌─ [ Pulse 1 (矩形波 1) ] ────────────────────────────────────────────────────────┐"
    };
    let p1_out_meter = if a.channel_mute[0] {
        format!(
            "{} \x1b[1;31m[消音 MUTE]\x1b[0m (内部生値: {:2})",
            print_meter(0, 15),
            p1.output()
        )
    } else {
        format!("{}                             ", print_meter(p1.output(), 15))
    };

    println!("{}", p1_header);
    println!(
        "│  タイマー: {:4} ({:5.1} Hz) -> カウンタ: {:4} / {:4} | シーケンサ: Step {:1}/7   │",
        p1.timer_period, p1_freq, p1.timer, p1.timer_period, p1.sequencer_step
    );
    println!(
        "│  Sweep [1の補数]: {:3} | 目標: {:4} ({:5.1} Hz) | 状態: {:14}  │",
        if p1.sweep.enabled { "ON" } else { "OFF" },
        p1_target,
        p1_target_freq,
        p1_swp_mute
    );
    println!(
        "│  Envelope: {:34} | Length: {:3} ({:6})   │",
        p1_env_str, p1.length_counter.counter, p1_len_str
    );
    println!("│  出力電圧: {}             │", p1_out_meter);
    println!("└───────────────────────────────────────────────────────────────────────────────┘");

    // --- 2. Pulse 2 ---
    let p2 = &a.pulse2;
    let p2_freq = p2.calculate_frequency(NTSC_CPU_CLOCK);
    let p2_target = p2.sweep.calculate_target(p2.timer_period);
    let p2_target_freq = if p2_target >= 8 {
        NTSC_CPU_CLOCK / (16.0 * (p2_target as f32 + 1.0))
    } else {
        0.0
    };
    let p2_swp_mute = if p2.sweep.is_muting(p2.timer_period) {
        "MUTE (範囲外)"
    } else {
        "OK (正常)"
    };
    let p2_env_str = if p2.envelope.constant_volume {
        format!("固定音量: {:2}", p2.envelope.volume_reload)
    } else {
        format!(
            "ハード減衰: {:2} (V={}) Loop:{}",
            p2.envelope.decay_counter, p2.envelope.volume_reload, p2.envelope.loop_flag
        )
    };
    let p2_len_str = if p2.length_counter.is_silenced() {
        "消音 (0)"
    } else {
        "発音中"
    };
    let p2_header = if a.channel_mute[1] {
        "┌─ [ Pulse 2 (矩形波 2) ] \x1b[1;31m[MUTE]\x1b[0m ──────────────────────────────────────────────────┐"
    } else {
        "┌─ [ Pulse 2 (矩形波 2) ] ────────────────────────────────────────────────────────┐"
    };
    let p2_out_meter = if a.channel_mute[1] {
        format!(
            "{} \x1b[1;31m[消音 MUTE]\x1b[0m (内部生値: {:2})",
            print_meter(0, 15),
            p2.output()
        )
    } else {
        format!("{}                             ", print_meter(p2.output(), 15))
    };

    println!("{}", p2_header);
    println!(
        "│  タイマー: {:4} ({:5.1} Hz) -> カウンタ: {:4} / {:4} | シーケンサ: Step {:1}/7   │",
        p2.timer_period, p2_freq, p2.timer, p2.timer_period, p2.sequencer_step
    );
    println!(
        "│  Sweep [2の補数]: {:3} | 目標: {:4} ({:5.1} Hz) | 状態: {:14}  │",
        if p2.sweep.enabled { "ON" } else { "OFF" },
        p2_target,
        p2_target_freq,
        p2_swp_mute
    );
    println!(
        "│  Envelope: {:34} | Length: {:3} ({:6})   │",
        p2_env_str, p2.length_counter.counter, p2_len_str
    );
    println!("│  出力電圧: {}             │", p2_out_meter);
    println!("└───────────────────────────────────────────────────────────────────────────────┘");

    // --- 3. Triangle ---
    let tri = &a.triangle;
    let tri_freq = tri.calculate_frequency(NTSC_CPU_CLOCK);
    let tri_len_str = if tri.length_counter.is_silenced() {
        "消音 (0)"
    } else {
        "発音中"
    };
    let tri_header = if a.channel_mute[2] {
        "┌─ [ Triangle (三角波) ] \x1b[1;31m[MUTE]\x1b[0m ──────────────────────────────────────────────────┐"
    } else {
        "┌─ [ Triangle (三角波) ] ────────────────────────────────────────────────────────┐"
    };
    let tri_out_meter = if a.channel_mute[2] {
        format!(
            "{} \x1b[1;31m[消音 MUTE]\x1b[0m (内部生値: {:2})",
            print_meter(0, 15),
            tri.output()
        )
    } else {
        format!("{}                             ", print_meter(tri.output(), 15))
    };

    println!("{}", tri_header);
    println!(
        "│  タイマー: {:4} ({:5.1} Hz) -> カウンタ: {:4} (CPU直結) | シーケンサ: Step {:2}/31 │",
        tri.timer_period, tri_freq, tri.timer, tri.sequencer_step
    );
    println!(
        "│  Linear Counter: {:3} / {:3} (Ctrl:{}) | Length: {:3} ({:6})              │",
        tri.linear_counter.counter,
        tri.linear_counter.reload_value,
        tri.linear_counter.control_flag,
        tri.length_counter.counter,
        tri_len_str
    );
    println!("│  出力電圧: {}             │", tri_out_meter);
    println!("└───────────────────────────────────────────────────────────────────────────────┘");

    // --- 4. Noise ---
    let noi = &a.noise;
    let noi_mode_str = if noi.short_mode {
        "短周期 93-step (金属音)"
    } else {
        "通常 32,767-step (砂嵐)"
    };
    let noi_env_str = if noi.envelope.constant_volume {
        format!("固定音量: {:2}", noi.envelope.volume_reload)
    } else {
        format!(
            "ハード減衰: {:2} (V={}) Loop:{}",
            noi.envelope.decay_counter, noi.envelope.volume_reload, noi.envelope.loop_flag
        )
    };
    let noi_len_str = if noi.length_counter.is_silenced() {
        "消音 (0)"
    } else {
        "発音中"
    };
    let noi_header = if a.channel_mute[3] {
        "┌─ [ Noise (ノイズ) ] \x1b[1;31m[MUTE]\x1b[0m ────────────────────────────────────────────────────┐"
    } else {
        "┌─ [ Noise (ノイズ) ] ──────────────────────────────────────────────────────────┐"
    };
    let noi_out_meter = if a.channel_mute[3] {
        format!(
            "{} \x1b[1;31m[消音 MUTE]\x1b[0m (内部生値: {:2})",
            print_meter(0, 15),
            noi.output()
        )
    } else {
        format!("{}                             ", print_meter(noi.output(), 15))
    };

    println!("{}", noi_header);
    println!(
        "│  LFSR: 0x{:04X} ({:015b}) | モード: {:22}   │",
        noi.shift_register, noi.shift_register, noi_mode_str
    );
    println!(
        "│  Envelope: {:34} | Length: {:3} ({:6})   │",
        noi_env_str, noi.length_counter.counter, noi_len_str
    );
    println!("│  出力電圧: {}             │", noi_out_meter);
    println!("└───────────────────────────────────────────────────────────────────────────────┘");

    // --- 5. DMC ---
    let dmc = &a.dmc;
    let dmc_status_str = if dmc.bytes_remaining > 0 { "再生中" } else { "停止" };
    let dmc_header = if a.channel_mute[4] {
        "┌─ [ DMC (デルタ変調 PCM) ] \x1b[1;31m[MUTE]\x1b[0m ────────────────────────────────────────────┐"
    } else {
        "┌─ [ DMC (デルタ変調 PCM) ] ──────────────────────────────────────────────────┐"
    };
    let dmc_out_meter = if a.channel_mute[4] {
        format!(
            "{} \x1b[1;31m[消音 MUTE]\x1b[0m (内部生値: {:3})",
            print_meter_128(0, 127),
            dmc.output()
        )
    } else {
        format!(
            "{}                              ",
            print_meter_128(dmc.output(), 127)
        )
    };

    println!("{}", dmc_header);
    println!(
        "│  Rate: Index {:2} (周期: {:3} cyc) | アドレス: ${:04X} | 残り: {:3} bytes ({:6}) │",
        dmc.rate_index, dmc.rate_period, dmc.current_address, dmc.bytes_remaining, dmc_status_str
    );
    println!(
        "│  Loop: {:5} | IRQ 有効: {:5} | IRQ フラグ: {:5} | 7-bit DAC: {:3} / 127     │",
        dmc.loop_flag,
        dmc.irq_enabled,
        dmc.irq_flag,
        dmc.output()
    );
    println!("│  出力電圧: {}             │", dmc_out_meter);
    println!("└───────────────────────────────────────────────────────────────────────────────┘");

    // --- 6. Analog Circuit & Nonlinear Mixer (Phase 4) ---
    let (p1_out, p2_out, tri_out, noi_out, dmc_out) = a.sample_raw_5ch();
    let p_mix = a.mixer.mix_pulse(p1_out, p2_out);
    let tnd_mix = a.mixer.mix_tnd(tri_out, noi_out, dmc_out);
    let raw_dac = a.filter_chain.last_raw;
    let s_hpf1 = a.filter_chain.last_hpf1;
    let s_hpf2 = a.filter_chain.last_hpf2;
    let s_out = a.filter_chain.last_output;
    let filter_status = if a.filter_chain.bypass {
        "\x1b[1;33m[バイパス中 (生デジタル)]\x1b[0m"
    } else {
        "\x1b[1;32m[実機 RC 3段フィルター稼働中]\x1b[0m"
    };
    println!("┌─ [ アナログ回路層: 非線形 DAC ミキサー ＆ 実機 RC フィルター ] ──────────────┐");
    println!(
        "│  状態: {:36} | Raw DAC: {:5.3} (P:{:4.2} TND:{:4.2}) │",
        filter_status, raw_dac, p_mix, tnd_mix
    );
    println!(
        "│  RC 経路: [Raw: {:4.2}] -> [HPF {:.0}Hz: {:+5.2}] -> [HPF {:.0}Hz: {:+5.2}]    │",
        raw_dac,
        a.filter_chain.hpf1.cutoff_hz,
        s_hpf1,
        a.filter_chain.hpf2.cutoff_hz,
        s_hpf2
    );
    println!(
        "│  最終出力 (LPF {:.0}Hz 通過後): {}                      │",
        a.filter_chain.lpf.cutoff_hz,
        print_bipolar_meter(s_out)
    );
    println!("└───────────────────────────────────────────────────────────────────────────────┘");

    // --- 7. Cycle-Accurate 6502 Sound Driver (Phase 5) ---
    if let Some(ref driver) = snap.driver {
        println!("┌─ [ 6502 サウンドドライバ (60Hz V-Blank サイクル完全同期) ] ────────────────────┐");
        println!(
            "│  曲名: {:26} | BPM: {:3} | フレーム: {:06} (V-Blank: {:5}/29780) │",
            driver.song_title, driver.bpm, driver.frame_counter, snap.vblank_cycle_counter
        );
        let p1_note = if driver.tracks[0].is_playing {
            format_note(driver.tracks[0].base_note)
        } else {
            "---".into()
        };
        let p2_note = if driver.tracks[1].is_playing {
            format_note(driver.tracks[1].base_note)
        } else {
            "---".into()
        };
        let tri_note = if driver.tracks[2].is_playing {
            format_note(driver.tracks[2].base_note)
        } else {
            "---".into()
        };
        let noi_note = if driver.tracks[3].is_playing {
            format_note(driver.tracks[3].base_note)
        } else {
            "---".into()
        };
        println!(
            "│  P1: {:4} (V:{:2} W:{:2}) | P2: {:4} (V:{:2} W:{:2}) | Tri: {:4} (W:{:2}) | Noi: {:4} (W:{:2}) │",
            p1_note,
            driver.tracks[0].last_vol,
            driver.tracks[0].wait_frames,
            p2_note,
            driver.tracks[1].last_vol,
            driver.tracks[1].wait_frames,
            tri_note,
            driver.tracks[2].wait_frames,
            noi_note,
            driver.tracks[3].wait_frames,
        );
        println!("└───────────────────────────────────────────────────────────────────────────────┘");
    }
}
