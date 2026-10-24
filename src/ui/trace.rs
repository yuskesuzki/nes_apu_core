//! 1 CPU サイクル刻みのハードウェア変調トレース出力 (`--trace`)

use crate::emulator::ApuSystem;

/// 指定された CPU サイクル数だけ 1 サイクルずつ進めながら内部レジスタ・変調状態を一覧表示します
pub fn run_trace_mode(sys: &mut ApuSystem, max_cycles: usize) {
    println!("🔍 1 CPU サイクル刻みのハードウェア変調トレース (最初の {} サイクル):", max_cycles);
    println!("Cycle   | FC: Step Cyc IRQ | P1: Timer Target Swp Env Len Out | Tri: Timer Lin Len Out | Noise: LFSR Env Out | DMC: DAC Rate Rem");
    println!("-----------------------------------------------------------------------------------------------------------------------------");

    for c in 0..max_cycles {
        let apu = &sys.apu;
        let fc = &apu.frame_counter;
        let p1 = &apu.pulse1;
        let tri = &apu.triangle;
        let noi = &apu.noise;
        let dmc = &apu.dmc;

        let swp_status = if p1.sweep.enabled { "ON" } else { "--" };
        let target_p = p1.sweep.calculate_target(p1.timer_period);
        let env_val = p1.envelope.output_volume();
        let len_val = p1.length_counter.counter;

        println!(
            "{:07} | FC: St{:1} {:5}  {}  | P1: {:5}  {:5}   {}   {:2}  {:3}  {:2} | Tri: {:5} {:3} {:3}  {:2} | Noi: 0x{:04X}  {:2}  {:2} | DMC: {:3}  {:2}  {:3}",
            c,
            fc.current_step,
            fc.cycle_counter,
            if fc.irq_flag { "!" } else { " " },
            p1.timer,
            target_p,
            swp_status,
            env_val,
            len_val,
            p1.output(),
            tri.timer,
            tri.linear_counter.counter,
            tri.length_counter.counter,
            tri.output(),
            noi.shift_register,
            noi.envelope.output_volume(),
            noi.output(),
            dmc.output(),
            dmc.rate_index,
            dmc.bytes_remaining,
        );

        sys.step();
    }
}
