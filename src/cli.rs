//! Clap によるコマンドライン引数定義と設定構造体

use std::path::PathBuf;
use clap::Parser;

use crate::emulator::ApuSystem;
use crate::source::{NsfSource, SoundSource};

#[derive(Parser, Debug, Clone)]
#[command(
    name = "nes_apu_core",
    about = "NES APU Hardware Core - Phase 5 Complete Diagnostic Dashboard & NSF Player"
)]
pub struct CliArgs {
    /// Enable CPAL realtime audio playback
    #[arg(short = 'a', long = "audio")]
    pub audio: bool,

    /// Play an NSF (NES Sound Format) file (.nsf)
    #[arg(short = 'n', long = "nsf", value_name = "FILE")]
    pub nsf: PathBuf,

    /// Select track / song number in NSF (1-based, default: NSF header starting song)
    #[arg(long = "track", value_name = "N")]
    pub track: Option<u8>,

    /// Mute comma-separated channels (e.g. --mute p2,noise,dmc)
    #[arg(long = "mute", value_name = "CHANNELS", value_delimiter = ',')]
    pub mute: Vec<String>,

    /// Solo comma-separated channels (e.g. --solo p1 or --solo p1,tri)
    #[arg(long = "solo", value_name = "CHANNELS", value_delimiter = ',')]
    pub solo: Vec<String>,

    /// Mute Pulse 1
    #[arg(long = "no-p1", visible_alias = "mute-p1")]
    pub no_p1: bool,
    /// Mute Pulse 2
    #[arg(long = "no-p2", visible_alias = "mute-p2")]
    pub no_p2: bool,
    /// Mute Triangle
    #[arg(long = "no-tri", visible_aliases = ["mute-tri", "no-triangle", "mute-triangle"])]
    pub no_tri: bool,
    /// Mute Noise
    #[arg(long = "no-noise", visible_aliases = ["mute-noise", "no-noi", "mute-noi"])]
    pub no_noise: bool,
    /// Mute DMC
    #[arg(long = "no-dmc", visible_aliases = ["mute-dmc", "no-dpcm", "mute-dpcm"])]
    pub no_dmc: bool,

    /// Solo Pulse 1
    #[arg(long = "only-p1", visible_alias = "solo-p1")]
    pub only_p1: bool,
    /// Solo Pulse 2
    #[arg(long = "only-p2", visible_alias = "solo-p2")]
    pub only_p2: bool,
    /// Solo Triangle
    #[arg(long = "only-tri", visible_aliases = ["solo-tri", "only-triangle", "solo-triangle"])]
    pub only_tri: bool,
    /// Solo Noise
    #[arg(long = "only-noise", visible_aliases = ["solo-noise", "only-noi", "solo-noi"])]
    pub only_noise: bool,
    /// Solo DMC
    #[arg(long = "only-dmc", visible_aliases = ["solo-dmc", "only-dpcm", "solo-dpcm"])]
    pub only_dmc: bool,

    /// Analog filter profile: 'warm' (default: 5.5kHz), 'tv' (4.2kHz CRT), 'studio' (14kHz)
    #[arg(long = "profile", value_name = "NAME")]
    pub profile: Option<String>,

    /// Custom LPF cutoff frequency in Hz (e.g. 4500 for very warm tone)
    #[arg(long = "lpf", value_name = "HZ")]
    pub lpf: Option<f32>,

    /// Override lead melody duty cycle (0=12.5%, 1=25%, 2=50% warm, 3=75%)
    #[arg(long = "duty", value_name = "0..3")]
    pub duty: Option<u8>,

    /// Bypass analog RC filter chain (compare with raw digital DAC)
    #[arg(long = "filter-bypass")]
    pub filter_bypass: bool,

    /// Print cycle-by-cycle hardware modulation trace and exit
    #[arg(short = 't', long = "trace")]
    pub trace: bool,

    /// APU Frame Counter mode: 4 (240Hz, IRQ) or 5 (192Hz, no IRQ)
    #[arg(long = "mode", visible_alias = "step", value_name = "4|5")]
    pub mode: Option<String>,

    /// Set APU Frame Counter to 5-step mode (192Hz, no IRQ)
    #[arg(long = "mode5", visible_aliases = ["5-step", "mode-5"])]
    pub mode5: bool,

    /// Set APU Frame Counter to 4-step mode (240Hz, IRQ)
    #[arg(long = "mode4", visible_aliases = ["4-step", "mode-4"])]
    pub mode4: bool,

    /// Run for N frames and exit (useful for headless testing)
    #[arg(long = "frames", value_name = "N")]
    pub frames: Option<usize>,
}

fn parse_channel_name(s: &str) -> Option<usize> {
    match s.trim().to_lowercase().as_str() {
        "p1" | "pulse1" | "1" | "pulse_1" | "sq1" => Some(0),
        "p2" | "pulse2" | "2" | "pulse_2" | "sq2" => Some(1),
        "tri" | "triangle" | "3" | "tr" => Some(2),
        "noise" | "noi" | "4" | "ns" => Some(3),
        "dmc" | "dpcm" | "5" | "pcm" => Some(4),
        _ => None,
    }
}

impl CliArgs {
    /// 5-Step モード ($4017 bit 7 = 1) が指定されているかを判定します
    pub fn is_mode5(&self) -> bool {
        if self.mode5 {
            return true;
        }
        if self.mode4 {
            return false;
        }
        if let Some(ref m) = self.mode {
            match m.as_str() {
                "5" | "5-step" | "5step" => return true,
                "4" | "4-step" | "4step" => return false,
                _ => {}
            }
        }
        false
    }

    /// `--mute` および `--solo` 系のオプションから 5 チャンネル分の消音マスクを計算します
    pub fn compute_channel_mutes(&self) -> [bool; 5] {
        let mut solo_set = Vec::new();
        let mut mute_set = Vec::new();

        for s in &self.solo {
            if let Some(ch) = parse_channel_name(s) {
                solo_set.push(ch);
            }
        }
        if self.only_p1 { solo_set.push(0); }
        if self.only_p2 { solo_set.push(1); }
        if self.only_tri { solo_set.push(2); }
        if self.only_noise { solo_set.push(3); }
        if self.only_dmc { solo_set.push(4); }

        for s in &self.mute {
            if let Some(ch) = parse_channel_name(s) {
                mute_set.push(ch);
            }
        }
        if self.no_p1 { mute_set.push(0); }
        if self.no_p2 { mute_set.push(1); }
        if self.no_tri { mute_set.push(2); }
        if self.no_noise { mute_set.push(3); }
        if self.no_dmc { mute_set.push(4); }

        let mut result = [false; 5];
        if !solo_set.is_empty() {
            result = [true; 5];
            for ch in solo_set {
                if ch < 5 {
                    result[ch] = false;
                }
            }
        }

        for ch in mute_set {
            if ch < 5 {
                result[ch] = true;
            }
        }

        result
    }

    /// CLI オプションに基づいて音源 (`SoundSource`) を生成します
    pub fn build_sound_source(&self) -> Result<Box<dyn SoundSource>, String> {
        let nsf = NsfSource::from_file(&self.nsf, self.track)
            .map_err(|err| format!("Error loading NSF {}: {}", self.nsf.display(), err))?;
        let mut source: Box<dyn SoundSource> = Box::new(nsf);

        // リード音色のデューティ比オーバーライド (ドライバ音源時)
        if let Some(d) = self.duty {
            if let Some(drv) = source.driver_state_mut() {
                if !drv.voices.is_empty() {
                    drv.voices[0].duty_macro = vec![d & 3];
                }
            }
        }

        Ok(source)
    }

    /// CLI オプションを反映した `ApuSystem` を構築して返します
    pub fn build_system(&self) -> Result<ApuSystem, String> {
        let source = self.build_sound_source()?;
        let mut sys = ApuSystem::new(source);

        sys.apu.channel_mute = self.compute_channel_mutes();
        sys.apu.filter_chain.bypass = self.filter_bypass;

        if let Some(ref prof) = self.profile {
            sys.apu.filter_chain.set_profile(prof);
        }
        if let Some(lpf_hz) = self.lpf {
            let h1 = sys.apu.filter_chain.hpf1.cutoff_hz;
            let h2 = sys.apu.filter_chain.hpf2.cutoff_hz;
            sys.apu.filter_chain.set_cutoffs(h1, h2, lpf_hz);
        }
        if self.is_mode5() {
            sys.apu.write_register(0x4017, 0x80); // 5-step mode
        }

        Ok(sys)
    }
}
