//! 各チャンネルの波形履歴を保持するリングバッファ
//!
//! Pulse 1, Pulse 2, Triangle, Noise, DMC のデジタル出力値 (output) を時系列で保持し、
//! スパークラインやオシロスコープ描画モジュールに渡すデータソースとなります。

#[derive(Debug, Clone)]
pub struct ChannelWaveformBuffer {
    pub p1: Vec<u8>,
    pub p2: Vec<u8>,
    pub tri: Vec<u8>,
    pub noise: Vec<u8>,
    pub dmc: Vec<u8>,
    capacity: usize,
}

impl Default for ChannelWaveformBuffer {
    fn default() -> Self {
        Self::new(128)
    }
}

impl ChannelWaveformBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            p1: Vec::with_capacity(capacity),
            p2: Vec::with_capacity(capacity),
            tri: Vec::with_capacity(capacity),
            noise: Vec::with_capacity(capacity),
            dmc: Vec::with_capacity(capacity),
            capacity,
        }
    }

    /// 各チャンネルの現在の出力値 (0..15, DMC は 0..127) をリングバッファに追加します
    pub fn push(&mut self, p1: u8, p2: u8, tri: u8, noise: u8, dmc: u8) {
        if self.p1.len() >= self.capacity {
            self.p1.remove(0);
            self.p2.remove(0);
            self.tri.remove(0);
            self.noise.remove(0);
            self.dmc.remove(0);
        }
        self.p1.push(p1);
        self.p2.push(p2);
        self.tri.push(tri);
        self.noise.push(noise);
        self.dmc.push(dmc);
    }

    /// バッファをクリアします
    pub fn clear(&mut self) {
        self.p1.clear();
        self.p2.clear();
        self.tri.clear();
        self.noise.clear();
        self.dmc.clear();
    }
}
