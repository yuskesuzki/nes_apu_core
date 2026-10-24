//! NES APU Cycle-Accurate 6502 Sound Driver & Music Sequencer
//!
//! OS の sleep タイマーによるジッターを完全に排除し、
//! CPU クロック (1.789773 MHz) から正確に 29,780 CPU サイクル (60Hz V-Blank) をカウントして
//! サウンドドライバをキックします。
//!
//! - 実機 6502 マシン語ドライバ仕様の音色マクロ (`voices::NesVoiceMacro`) による変調
//! - NTRK バイナリ楽曲データ (Super Mario Bros. 等) のロード＆リアルタイム演奏
//! - APU レジスタ ($4000〜$4017) への直接書き込み生成

use crate::apu::NTSC_CPU_CLOCK;
pub use crate::source::voices::NesVoiceMacro;

/// 1/60 秒 (NTSC 60Hz V-Blank) あたりの正確な CPU サイクル数 (1,789,773 / 60.0988 ≈ 29,780)
pub const NTSC_CYCLES_PER_FRAME: usize = 29780;

/// 72 半音分の NES 矩形波タイマーテーブル (C1〜B6, A4=440Hz)
pub fn note_to_nes_timer(note: usize) -> u16 {
    let midi = (note + 24) as f32; // C1 = MIDI 24
    let freq = 440.0 * 2.0_f32.powf((midi - 69.0) / 12.0);
    let timer = (NTSC_CPU_CLOCK / (16.0 * freq) - 1.0).round() as i32;
    timer.clamp(0, 2047) as u16
}

/// 72 半音分の NES 三角波タイマーテーブル (C1〜B6, A4=440Hz)
/// ※ 三角波は 32 ステップシーケンサのため周期式が f = f_CPU / (32 * (t + 1)) となる
pub fn note_to_triangle_timer(note: usize) -> u16 {
    let midi = (note + 24) as f32; // C1 = MIDI 24
    let freq = 440.0 * 2.0_f32.powf((midi - 69.0) / 12.0);
    let timer = (NTSC_CPU_CLOCK / (32.0 * freq) - 1.0).round() as i32;
    timer.clamp(0, 2047) as u16
}

/// ノート番号 (0..71) から音名表記文字列 ("C4", "F#5" など) を取得
pub fn note_name(note: u8) -> &'static str {
    const NAMES: [&str; 12] = [
        "C-", "C#", "D-", "D#", "E-", "F-", "F#", "G-", "G#", "A-", "A#", "B-",
    ];
    if note >= 72 {
        return "---";
    }
    NAMES[(note % 12) as usize]
}

/// 音階とオクターブのフォーマット文字列 (例: "C-4", "A-2")
pub fn format_note(note: u8) -> String {
    if note >= 72 {
        return "---".to_string();
    }
    let octave = (note / 12) + 1;
    format!("{}{}", note_name(note), octave)
}

/// 各チャンネルのシーケンサ演奏状態
#[derive(Debug, Clone)]
pub struct TrackState {
    pub pc: usize,
    pub wait_frames: u8,
    pub base_note: u8,
    pub is_playing: bool,
    pub voice_idx: usize,
    pub macro_frame: usize,
    pub last_vol: u8,
    pub last_timer: u16,
    pub last_duty: u8,
}

impl TrackState {
    pub fn new() -> Self {
        Self {
            pc: 0,
            wait_frames: 0,
            base_note: 0,
            is_playing: false,
            voice_idx: 0,
            macro_frame: 0,
            last_vol: 0,
            last_timer: 0,
            last_duty: 2,
        }
    }
}

/// サイクル完全同期型 6502 サウンドドライバ
#[derive(Debug, Clone)]
pub struct NesSoundDriver {
    pub song_title: String,
    pub bpm: u16,
    pub tracks: [TrackState; 5],
    pub bytecode: [Vec<u8>; 5],
    pub voices: Vec<NesVoiceMacro>,
    pub frame_counter: u64,
    pub is_playing: bool,
}

impl Default for NesSoundDriver {
    fn default() -> Self {
        Self {
            song_title: "Chiptune Demo".to_string(),
            bpm: 120,
            tracks: [
                TrackState::new(),
                TrackState::new(),
                TrackState::new(),
                TrackState::new(),
                TrackState::new(),
            ],
            bytecode: [Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new()],
            voices: NesVoiceMacro::default_bank(),
            frame_counter: 0,
            is_playing: true,
        }
    }
}

impl NesSoundDriver {
    pub fn new() -> Self {
        let mut driver = Self::default();
        driver.load_demo_song();
        driver
    }

    /// NTRK コンテナ形式バイナリ (4チャンネルまたは5チャンネル) をロード
    pub fn load_ntrk_bytes(&mut self, data: &[u8], title: &str) -> Result<(), String> {
        if data.len() < 8 || &data[0..4] != b"NTRK" {
            return Err("無効な NTRK バイナリフォーマットです".to_string());
        }

        let num_tracks = data[4] as usize;
        let bpm = u16::from_le_bytes([data[6], data[7]]);

        let mut offsets = Vec::new();
        for i in 0..num_tracks {
            let start = 8 + i * 2;
            if start + 2 > data.len() {
                return Err("NTRK ヘッダーが破損しています".to_string());
            }
            offsets.push(u16::from_le_bytes([data[start], data[start + 1]]) as usize);
        }
        offsets.push(data.len());

        self.reset();
        self.song_title = title.to_string();
        self.bpm = bpm;

        for i in 0..num_tracks.min(5) {
            let s = offsets[i];
            let e = offsets[i + 1].min(data.len());
            if s <= e {
                self.bytecode[i] = data[s..e].to_vec();
            }
        }

        self.is_playing = true;
        Ok(())
    }

    /// デモ曲 (ファミコン風ポップス) をロード
    pub fn load_demo_song(&mut self) {
        let c4 = 36u8;
        let g4 = 43u8;
        let a4 = 45u8;
        let c5 = 48u8;
        let d5 = 50u8;
        let e5 = 52u8;

        // Track 0: Pulse 1 (主旋律)
        let t0 = vec![
            0x80, // Voice 0 (デューティ変調リード)
            e5, 12, d5, 12, c5, 24, g4, 12, a4, 12, c5, 24, d5, 12, e5, 12, d5, 24, g4, 24, e5,
            12, d5, 12, c5, 24, g4, 12, a4, 12, c5, 24, d5, 12, c5, 12, c5, 36, 0x7F, 12, 0xFE,
            0x00, 0x00, // ループ
        ];

        // Track 1: Pulse 2 (疑似和音アルペジオ)
        let t1 = vec![
            0x81,
            c4,
            48,
            0x81,
            a4 - 12,
            48,
            0x81,
            g4 - 12,
            48,
            0x81,
            c4,
            48,
            0xFE,
            0x00,
            0x00,
        ];

        // Track 2: Triangle (階段波ベース)
        let c2 = 12u8;
        let a2 = 21u8;
        let g2 = 19u8;
        let t2 = vec![
            0x83, // Voice 3 (三角波ベース)
            c2, 12, 0x7F, 12, c2, 12, 0x7F, 12, a2, 12, 0x7F, 12, a2, 12, 0x7F, 12, g2, 12, 0x7F,
            12, g2, 12, 0x7F, 12, c2, 12, 0x7F, 12, c2, 24, 0xFE, 0x00, 0x00,
        ];

        // Track 3: Noise (スネア＆ハイハット)
        let t3 = vec![
            0x85, 4, 12, 0x84, 8, 12, 0x85, 4, 12, 0x84, 8, 12, 0x85, 4, 12, 0x84, 8, 12, 0x85, 4,
            12, 0x84, 8, 12, 0x85, 4, 12, 0x84, 8, 12, 0x85, 4, 12, 0x84, 8, 12, 0x85, 4, 12, 0x84,
            8, 12, 0x84, 8, 24, 0xFE, 0x00, 0x00,
        ];

        self.reset();
        self.song_title = "8-bit Pop Chiptune".to_string();
        self.bpm = 120;
        self.bytecode[0] = t0;
        self.bytecode[1] = t1;
        self.bytecode[2] = t2;
        self.bytecode[3] = t3;
        self.is_playing = true;
    }

    pub fn reset(&mut self) {
        self.frame_counter = 0;
        self.tracks = [
            TrackState::new(),
            TrackState::new(),
            TrackState::new(),
            TrackState::new(),
            TrackState::new(),
        ];
    }

    /// 60Hz V-Blank 割り込み時に実行される 1 フレーム処理
    /// APU レジスタへ即時適用すべき (addr, val) ペアのリストを返す
    pub fn tick_frame(&mut self) -> Vec<(u16, u8)> {
        if !self.is_playing {
            return Vec::new();
        }

        self.frame_counter += 1;
        let mut writes = Vec::with_capacity(16);

        // $4015: チャンネル有効化
        if self.frame_counter == 1 {
            writes.push((0x4015, 0x0F));
        }

        for ch in 0..4 {
            let code = &self.bytecode[ch];
            if code.is_empty() {
                continue;
            }

            let st = &mut self.tracks[ch];

            if st.wait_frames > 0 {
                st.wait_frames -= 1;
            }

            while st.wait_frames == 0 && st.pc < code.len() {
                let op = code[st.pc];
                st.pc += 1;

                if op <= 71 {
                    // 発音命令 (Note-on)
                    st.base_note = op;
                    if st.pc < code.len() {
                        st.wait_frames = code[st.pc];
                        st.pc += 1;
                    }
                    st.is_playing = true;
                    st.macro_frame = 0;
                    break;
                } else if op == 0x7E {
                    // Tie / Wait
                    if st.pc < code.len() {
                        st.wait_frames = code[st.pc];
                        st.pc += 1;
                    }
                    break;
                } else if op == 0x7F {
                    // Rest (休符・消音)
                    st.is_playing = false;
                    if st.pc < code.len() {
                        st.wait_frames = code[st.pc];
                        st.pc += 1;
                    }
                    break;
                } else if (0x80..=0x87).contains(&op) {
                    // 音色マクロ変更
                    st.voice_idx = (op - 0x80) as usize;
                } else if op == 0xFE {
                    // ループ命令: ジャンプアドレスの取得
                    if st.pc + 1 < code.len() {
                        let target_l = code[st.pc] as usize;
                        let target_h = code[st.pc + 1] as usize;
                        st.pc = target_l | (target_h << 8);
                    } else {
                        st.pc = 0;
                    }
                } else if op == 0xFF {
                    st.is_playing = false;
                    break;
                }
            }

            if !st.is_playing {
                // 消音レジスタ書き込み
                match ch {
                    0 => writes.push((0x4000, 0x30)),
                    1 => writes.push((0x4004, 0x30)),
                    2 => writes.push((0x4008, 0x00)),
                    3 => writes.push((0x400C, 0x30)),
                    _ => {}
                }
                st.last_vol = 0;
                continue;
            }

            let voice = &self.voices[st.voice_idx.min(self.voices.len() - 1)];

            // 音色マクロのサンプリング
            let vol = voice.sample_volume(st.macro_frame);
            let duty = voice.sample_duty(st.macro_frame);
            let semi = voice.sample_arpeggio(st.macro_frame);
            let detune = voice.sample_pitch(st.macro_frame);

            st.last_vol = vol;
            st.last_duty = duty;

            let final_note = (st.base_note as i32 + semi as i32).clamp(0, 71) as usize;
            let base_timer = if ch == 2 {
                note_to_triangle_timer(final_note)
            } else {
                note_to_nes_timer(final_note)
            };
            let final_timer = (base_timer as i32 + detune as i32).clamp(0, 2047) as u16;
            let prev_timer = st.last_timer;
            st.last_timer = final_timer;
            let is_note_start = st.macro_frame == 0;
            let high_byte_changed = (final_timer >> 8) != (prev_timer >> 8);

            // APU レジスタ更新
            match ch {
                0 => {
                    writes.push((0x4000, (duty << 6) | 0x30 | (vol & 0x0F)));
                    writes.push((0x4002, (final_timer & 0xFF) as u8));
                    // 実機仕様: $4003 への書き込みはシーケンサ位相を 0 にリセットするため、
                    // ノート開始時または上位ビット変化時のみ書き込む (60Hz 位相リセット歪みの防止)
                    if is_note_start || high_byte_changed {
                        writes.push((0x4003, ((final_timer >> 8) & 0x07) as u8));
                    }
                }
                1 => {
                    writes.push((0x4004, (duty << 6) | 0x30 | (vol & 0x0F)));
                    writes.push((0x4006, (final_timer & 0xFF) as u8));
                    if is_note_start || high_byte_changed {
                        writes.push((0x4007, ((final_timer >> 8) & 0x07) as u8));
                    }
                }
                2 => {
                    writes.push((0x4008, 0x80 | 0x7F)); // 常時発音
                    writes.push((0x400A, (final_timer & 0xFF) as u8));
                    if is_note_start || high_byte_changed {
                        writes.push((0x400B, ((final_timer >> 8) & 0x07) as u8));
                    }
                }
                3 => {
                    writes.push((0x400C, 0x30 | (vol & 0x0F)));
                    let short_mode = if st.voice_idx == 5 { 0x80 } else { 0x00 };
                    let rate = (st.base_note as u8).min(15);
                    writes.push((0x400E, short_mode | rate));
                    if is_note_start {
                        writes.push((0x400F, 0x08));
                    }
                }
                _ => {}
            }

            st.macro_frame += 1;
        }

        writes
    }
}
