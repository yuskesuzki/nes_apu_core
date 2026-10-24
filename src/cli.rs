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
        let nsf = NsfSource::from_file(&self.nsf, self.track)
            .map_err(|err| format!("Error loading NSF {}: {}", self.nsf.display(), err))?;
        let total_songs = nsf.header.total_songs;
        let starting_song = nsf.header.starting_song;

        let mut source: Box<dyn SoundSource> = Box::new(nsf);

        // リード音色のデューティ比オーバーライド (ドライバ音源時)
        if let Some(d) = self.duty {
            if let Some(drv) = source.driver_state_mut() {
                if !drv.voices.is_empty() {
                    drv.voices[0].duty_macro = vec![d & 3];
                }
            }
        }

        // 初期 BGM トラック番号
        let current_bgm_track = self.track.unwrap_or(starting_song).clamp(1, total_songs);

        // SE キーの初期デフォルト割り当て (Z: トラック 1, X: トラック 2 (存在する場合))
        let default_se_z = Some(1);
        let default_se_x = Some(if total_songs >= 2 { 2 } else { 1 });

        let mut sys = ApuSystem::new_with_se(
            source,
            self.nsf.clone(),
            total_songs,
            current_bgm_track,
            default_se_z,
            default_se_x,
        );

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
