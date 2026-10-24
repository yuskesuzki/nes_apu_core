//! NES APU Analog Signal Chain & Nonlinear DAC Mixer
//!
//! ファミコン実機（Ricoh RP2A03 基板）のアナログ回路を完全シミュレートします。
//!
//! 1. 非線形 DAC ミキサー (Nonlinear DAC Mixer):
//!    抵抗ラダーによる非線形電流合成。各チャンネルの音量に応じた相互変調・コンプレッションを再現。
//!    - pulse_out = 95.88 / ((8128.0 / (p1 + p2)) + 100.0)
//!    - tnd_out = 159.79 / ((1.0 / (tri / 8227.0 + noise / 12241.0 + dmc / 22638.0)) + 100.0)
//!
//! 2. 実機基板 3段 RC フィルター (Analog Filter Chain):
//!    - High-Pass Filter 1 (約 90 Hz): 実機出力コンデンサ（1µF）による AC 結合。直流成分カットと低域サグ。
//!    - High-Pass Filter 2 (約 440 Hz): アンプ結合コンデンサによる低域カット。
//!    - Low-Pass Filter (約 14 kHz): 高域 RC フィルターによる高周波エイリアシング除去と温かみのある太い音色。

use std::f32::consts::PI;

/// 1次 IIR ハイパスフィルター (High-Pass Filter)
#[derive(Debug, Clone)]
pub struct HighPassFilter {
    pub cutoff_hz: f32,
    pub sample_rate: f32,
    pub alpha: f32,
    prev_x: f32,
    prev_y: f32,
}

impl HighPassFilter {
    pub fn new(sample_rate: f32, cutoff_hz: f32) -> Self {
        let dt = 1.0 / sample_rate;
        let rc = 1.0 / (2.0 * PI * cutoff_hz);
        let alpha = rc / (rc + dt);
        Self {
            cutoff_hz,
            sample_rate,
            alpha,
            prev_x: 0.0,
            prev_y: 0.0,
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        let dt = 1.0 / sample_rate;
        let rc = 1.0 / (2.0 * PI * self.cutoff_hz);
        self.alpha = rc / (rc + dt);
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        let y = self.alpha * (self.prev_y + x - self.prev_x);
        self.prev_x = x;
        self.prev_y = y;
        y
    }

    pub fn reset(&mut self) {
        self.prev_x = 0.0;
        self.prev_y = 0.0;
    }
}

/// 1次 IIR ローパスフィルター (Low-Pass Filter)
#[derive(Debug, Clone)]
pub struct LowPassFilter {
    pub cutoff_hz: f32,
    pub sample_rate: f32,
    pub alpha: f32,
    prev_y: f32,
}

impl LowPassFilter {
    pub fn new(sample_rate: f32, cutoff_hz: f32) -> Self {
        let dt = 1.0 / sample_rate;
        let rc = 1.0 / (2.0 * PI * cutoff_hz);
        let alpha = dt / (rc + dt);
        Self {
            cutoff_hz,
            sample_rate,
            alpha,
            prev_y: 0.0,
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        let dt = 1.0 / sample_rate;
        let rc = 1.0 / (2.0 * PI * self.cutoff_hz);
        self.alpha = dt / (rc + dt);
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        let y = self.prev_y + self.alpha * (x - self.prev_y);
        self.prev_y = y;
        y
    }

    pub fn reset(&mut self) {
        self.prev_y = 0.0;
    }
}

/// 非線形 DAC ミキサー (RP2A03 抵抗ラダー回路)
#[derive(Debug, Clone)]
pub struct NonlinearMixer {
    pulse_table: [f32; 31],
}

impl NonlinearMixer {
    pub fn new() -> Self {
        let mut pulse_table = [0.0f32; 31];
        for i in 1..31 {
            pulse_table[i] = 95.88 / ((8128.0 / (i as f32)) + 100.0);
        }
        Self { pulse_table }
    }

    /// Pulse 1 と Pulse 2 の非線形合成出力 (0.0..~0.312)
    #[inline]
    pub fn mix_pulse(&self, p1: u8, p2: u8) -> f32 {
        let sum = (p1 as usize) + (p2 as usize);
        if sum < 31 {
            self.pulse_table[sum]
        } else {
            self.pulse_table[30]
        }
    }

    /// Triangle, Noise, DMC の非線形合成出力 (0.0..~0.688)
    #[inline]
    pub fn mix_tnd(&self, tri: u8, noise: u8, dmc: u8) -> f32 {
        if tri == 0 && noise == 0 && dmc == 0 {
            return 0.0;
        }

        let denom = (tri as f32) / 8227.0 + (noise as f32) / 12241.0 + (dmc as f32) / 22638.0;
        159.79 / ((1.0 / denom) + 100.0)
    }

    /// 全 5 チャンネルの非線形合成出力 (0.0..~1.0)
    #[inline]
    pub fn mix(&self, p1: u8, p2: u8, tri: u8, noise: u8, dmc: u8) -> f32 {
        self.mix_pulse(p1, p2) + self.mix_tnd(tri, noise, dmc)
    }
}

/// 実機基板 3段 アナログ RC フィルターチェーン
#[derive(Debug, Clone)]
pub struct AnalogFilterChain {
    pub hpf1: HighPassFilter, // 90 Hz
    pub hpf2: HighPassFilter, // 440 Hz
    pub lpf: LowPassFilter,   // 14,000 Hz
    pub bypass: bool,

    // 視覚監視用の中間出力バッファ
    pub last_raw: f32,
    pub last_hpf1: f32,
    pub last_hpf2: f32,
    pub last_output: f32,
}

impl AnalogFilterChain {
    pub fn new(sample_rate: f32) -> Self {
        // デフォルト: 実機ファミコン（RF/AVアンプおよびCRT TV）特有の太く温かいアナログ音響特性
        // HPF 1: 37 Hz (AC結合コンデンサ、可聴域低音を損なわない直流カット)
        // HPF 2: 90 Hz (アンプ段カップリング)
        // LPF: 5,500 Hz (実機基板RCおよびTVスピーカーのロールオフ。金属的な耳障りな超高域を除去)
        Self {
            hpf1: HighPassFilter::new(sample_rate, 37.0),
            hpf2: HighPassFilter::new(sample_rate, 90.0),
            lpf: LowPassFilter::new(sample_rate, 5_500.0),
            bypass: false,
            last_raw: 0.0,
            last_hpf1: 0.0,
            last_hpf2: 0.0,
            last_output: 0.0,
        }
    }

    /// フィルターのカットオフ周波数を手動設定
    pub fn set_cutoffs(&mut self, hpf1_hz: f32, hpf2_hz: f32, lpf_hz: f32) {
        self.hpf1 = HighPassFilter::new(self.hpf1.sample_rate, hpf1_hz);
        self.hpf2 = HighPassFilter::new(self.hpf2.sample_rate, hpf2_hz);
        self.lpf = LowPassFilter::new(self.lpf.sample_rate, lpf_hz);
    }

    /// 音響プロファイルの切り替え
    /// - "tv": 1980年代ブラウン管テレビ (LPF 4,200Hz, 極めて太く丸い音)
    /// - "warm" (標準): 実機AVファミコン / アナログアンプ (LPF 5,500Hz, バランスの良い温かみ)
    /// - "studio": 高帯域スタジオライン録音 (LPF 14,000Hz, エッジの効いたクリア音)
    pub fn set_profile(&mut self, profile: &str) {
        match profile {
            "tv" => self.set_cutoffs(35.0, 80.0, 4_200.0),
            "studio" => self.set_cutoffs(90.0, 440.0, 14_000.0),
            _ => self.set_cutoffs(37.0, 90.0, 5_500.0), // "warm"
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.hpf1.set_sample_rate(sample_rate);
        self.hpf2.set_sample_rate(sample_rate);
        self.lpf.set_sample_rate(sample_rate);
    }

    /// 1 サンプルを 3 段フィルターへ通す
    pub fn process(&mut self, raw: f32) -> f32 {
        self.last_raw = raw;
        if self.bypass {
            // バイパス時は単純に直流オフセット（0.5）を引いて -1.0〜+1.0 に正規化
            self.last_output = (raw - 0.5) * 2.0;
            return self.last_output;
        }

        // 1. HPF 1 (90 Hz)
        let s1 = self.hpf1.process(raw);
        self.last_hpf1 = s1;

        // 2. HPF 2 (440 Hz)
        let s2 = self.hpf2.process(s1);
        self.last_hpf2 = s2;

        // 3. LPF (14 kHz)
        let s3 = self.lpf.process(s2);
        // 実機回路通過後のピークをオーディオレンジ (-1.0..+1.0) に合わせるゲイン
        let out = (s3 * 2.5).clamp(-1.0, 1.0);
        self.last_output = out;
        out
    }

    pub fn reset(&mut self) {
        self.hpf1.reset();
        self.hpf2.reset();
        self.lpf.reset();
        self.last_raw = 0.0;
        self.last_hpf1 = 0.0;
        self.last_hpf2 = 0.0;
        self.last_output = 0.0;
    }
}
