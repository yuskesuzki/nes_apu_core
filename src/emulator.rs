//! NES APU System Coordinator (Emulator Layer)
//!
//! 純粋なハードウェア層である [`NesApuCore`] と、音源・演奏制御層である [`SoundSource`] を束ね、
//! 1.789773 MHz の CPU クロック進行と 29,780 サイクルごとの 60Hz V-Blank 割り込み同期を管理します。
//!
//! 実機ファミコンのサウンドドライバ同様、BGM 演奏中に効果音（SE）がチャンネルを
//! 一時的に乗っ取り（ハイジャック）、SE 終了後に自動復帰（レストア）する調停機構を備えます。

use std::path::PathBuf;
use crate::apu::{NesApuCore, RegisterWriteSource, NTSC_CPU_CLOCK};
use crate::source::{NsfSource, SoundSource, NTSC_CYCLES_PER_FRAME};

pub struct ApuSystem {
    /// RP2A03 APU ハードウェアコア
    pub apu: NesApuCore,
    /// 現在アタッチされているメイン音源（BGM）
    pub source: Box<dyn SoundSource>,
    /// 効果音（SE）音源インスタンス（再生中のみ有効）
    pub se_source: Option<Box<dyn SoundSource>>,
    /// V-Blank (60Hz) サイクルアキュムレータ (0..29,780)
    pub vblank_cycle_counter: usize,
    /// 再生一時停止フラグ
    pub paused: bool,
    /// NSF ファイルパス（SE 再生時のインスタンス生成用）
    pub nsf_path: Option<PathBuf>,
    /// NSF の総トラック数 (デフォルト: 1)
    pub total_tracks: u8,
    /// 'z' キーに割り当てられた SE トラック番号 (1-based)
    pub se_track_z: Option<u8>,
    /// 'x' キーに割り当てられた SE トラック番号 (1-based)
    pub se_track_x: Option<u8>,
    /// 現在再生中の SE トラック番号
    pub se_current_track: Option<u8>,
    /// 各チャンネルの SE 消音・無書き込み連続フレーム数
    pub se_silence_frames: [usize; 5],
    /// 現在の SE 再生経過フレーム数
    pub se_frame_count: usize,
}

impl ApuSystem {
    /// 指定された音源 (`SoundSource`) をアタッチしてシステムを初期化します
    pub fn new(mut source: Box<dyn SoundSource>) -> Self {
        let mut apu = NesApuCore::new();
        source.init(&mut apu);
        Self {
            apu,
            source,
            se_source: None,
            vblank_cycle_counter: 0,
            paused: false,
            nsf_path: None,
            total_tracks: 1,
            se_track_z: None,
            se_track_x: None,
            se_current_track: None,
            se_silence_frames: [0; 5],
            se_frame_count: 0,
        }
    }

    /// SE 再生用の NSF パスおよびキー割り当てを設定して初期化します
    pub fn new_with_se(
        mut source: Box<dyn SoundSource>,
        nsf_path: PathBuf,
        total_tracks: u8,
        se_track_z: Option<u8>,
        se_track_x: Option<u8>,
    ) -> Self {
        let mut apu = NesApuCore::new();
        source.init(&mut apu);
        Self {
            apu,
            source,
            se_source: None,
            vblank_cycle_counter: 0,
            paused: false,
            nsf_path: Some(nsf_path),
            total_tracks: total_tracks.max(1),
            se_track_z,
            se_track_x,
            se_current_track: None,
            se_silence_frames: [0; 5],
            se_frame_count: 0,
        }
    }

    /// 演奏の一時停止状態を反転します (一時停止中なら true を返す)
    pub fn toggle_pause(&mut self) -> bool {
        self.paused = !self.paused;
        self.paused
    }

    /// 新しい音源に差し替えます
    pub fn set_source(&mut self, mut source: Box<dyn SoundSource>) {
        source.init(&mut self.apu);
        self.source = source;
        self.vblank_cycle_counter = 0;
    }

    /// 指定したトラック番号の効果音（SE）をトリガー（再生開始）します
    pub fn trigger_se(&mut self, track_opt: Option<u8>) {
        let track = match track_opt {
            Some(t) => t,
            None => return,
        };
        let path = match self.nsf_path.as_ref() {
            Some(p) => p,
            None => return,
        };

        // 新しい SE 用の NsfSource を生成
        if let Ok(mut se_nsf) = NsfSource::from_file(path, Some(track)) {
            // SE 用のレジスタ書き込みモードに切り替えて INIT 実行
            self.apu.write_source = RegisterWriteSource::SoundEffect;
            se_nsf.init(&mut self.apu);
            self.apu.write_source = RegisterWriteSource::Bgm;

            self.se_source = Some(Box::new(se_nsf));
            self.se_current_track = Some(track);
            self.se_frame_count = 0;
            self.se_silence_frames = [0; 5];
        }
    }

    /// 現在鳴っている効果音（SE）を即座に停止し、全チャンネルを BGM に復元します
    pub fn stop_se(&mut self) {
        self.se_source = None;
        self.se_current_track = None;
        self.se_frame_count = 0;
        self.se_silence_frames = [0; 5];
        self.apu.restore_all_channels();
        self.apu.write_source = RegisterWriteSource::Bgm;
    }

    /// 60Hz V-Blank フレーム更新処理（BGM 進行、SE 進行、およびハイジャック/復元調停）
    fn on_vblank(&mut self) {
        // 1. 今フレームの SE 書き込み検知フラグをリセット
        self.apu.se_channel_touched = [false; 5];

        // 2. BGM のフレーム実行 (write_source = Bgm)
        self.apu.write_source = RegisterWriteSource::Bgm;
        self.source.on_vblank(&mut self.apu);

        // 3. SE のフレーム実行 (再生中の場合)
        if let Some(ref mut se) = self.se_source {
            self.apu.write_source = RegisterWriteSource::SoundEffect;
            se.on_vblank(&mut self.apu);
            self.apu.write_source = RegisterWriteSource::Bgm;
            self.se_frame_count += 1;

            // 各チャンネルの SE 終了判定
            for ch in 0..5 {
                if self.apu.channel_hijacked[ch] {
                    let touched = self.apu.se_channel_touched[ch];
                    // チャンネルが消音されているか（音量 0 または無効）
                    let is_silent = match ch {
                        0 => !self.apu.pulse1.enabled || (self.apu.pulse1.envelope.decay_counter == 0 && self.apu.pulse1.envelope.volume_reload == 0),
                        1 => !self.apu.pulse2.enabled || (self.apu.pulse2.envelope.decay_counter == 0 && self.apu.pulse2.envelope.volume_reload == 0),
                        2 => !self.apu.triangle.enabled || self.apu.triangle.linear_counter.counter == 0,
                        3 => !self.apu.noise.enabled || (self.apu.noise.envelope.decay_counter == 0 && self.apu.noise.envelope.volume_reload == 0),
                        4 => !self.apu.dmc.enabled || self.apu.dmc.bytes_remaining == 0,
                        _ => true,
                    };

                    if !touched || is_silent {
                        self.se_silence_frames[ch] += 1;
                    } else {
                        self.se_silence_frames[ch] = 0;
                    }

                    // 4 フレーム連続で書き込みなし/消音、または最大 180 フレーム (約3秒) 経過で自動復帰
                    if self.se_silence_frames[ch] >= 4 || self.se_frame_count >= 180 {
                        self.apu.restore_channel_from_bgm(ch);
                    }
                }
            }

            // 全チャンネルのハイジャックが解除されたら SE 自体を終了
            if !self.apu.channel_hijacked.iter().any(|&h| h) {
                self.se_source = None;
                self.se_current_track = None;
            }
        }
    }

    /// 1 CPU サイクル (約 558.7 ナノ秒) を進め、29,780 サイクルごとに V-Blank 更新を発火します
    pub fn step(&mut self) {
        self.apu.step();

        self.vblank_cycle_counter += 1;
        if self.vblank_cycle_counter >= NTSC_CYCLES_PER_FRAME {
            self.vblank_cycle_counter -= NTSC_CYCLES_PER_FRAME;
            self.on_vblank();
        }
    }

    /// 指定した CPU サイクル数だけ一括進行します
    pub fn clock_cycles(&mut self, cycles: usize) {
        for _ in 0..cycles {
            self.step();
        }
    }

    /// オーディオ出力サンプル周期 (例: 44.1kHz / 48kHz) 分の CPU サイクルを進め、
    /// V-Blank 同期を維持しながら非線形 DAC + 3 段 RC フィルター出力を生成します
    pub fn generate_audio_sample(&mut self, sample_rate: f32) -> f32 {
        if (self.apu.filter_chain.hpf1.sample_rate - sample_rate).abs() > 1.0 {
            self.apu.filter_chain.set_sample_rate(sample_rate);
        }

        let cycles_per_sample = (NTSC_CPU_CLOCK / sample_rate).round() as usize;
        let mut accum: f32 = 0.0;

        for _ in 0..cycles_per_sample {
            self.step();
            accum += self.apu.sample_nonlinear();
        }

        let raw_dac = if cycles_per_sample > 0 {
            accum / (cycles_per_sample as f32)
        } else {
            0.0
        };

        self.apu.filter_chain.process(raw_dac)
    }
}
