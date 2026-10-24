//! 1行スパークライン形式の波形プロッター
//!
//! Unicode のブロック文字（` ▂▃▄▅▆▇█`）を用いて、
//! 各チャンネルの直近の出力値推移を 1 行のコンパクトな波形バーとして描画します。

const BLOCK_LEVELS: [char; 8] = [' ', ' ', '▂', '▃', '▄', '▅', '▆', '▇'];

/// 指定されたサンプルデータ列を 1 行の Unicode スパークライン波形文字列に変換します
///
/// - `data`: 各チャンネルのサンプル履歴配列 (0..=max_val)
/// - `max_val`: 出力値の最大スケール (Pulse/Tri/Noise は 15, DMC は 127)
/// - `width`: 描画する文字数（横幅）
pub fn render_sparkline(data: &[u8], max_val: u8, width: usize) -> String {
    let mut s = String::with_capacity(width);
    let start = data.len().saturating_sub(width);
    let slice = &data[start..];

    // 幅に満たない場合は左側を空白でパディング
    let padding = width.saturating_sub(slice.len());
    for _ in 0..padding {
        s.push(' ');
    }

    let max_f = max_val.max(1) as f32;
    for &val in slice {
        let normalized = (val as f32 / max_f).clamp(0.0, 1.0);
        let idx = (normalized * 7.0).round() as usize;
        s.push(BLOCK_LEVELS[idx.min(7)]);
    }

    s
}

/// チャンネル名や現在の値を含めた整形済みスパークライン表示行を返します
pub fn format_channel_sparkline(
    ch_name: &str,
    data: &[u8],
    current_val: u8,
    max_val: u8,
    width: usize,
) -> String {
    let sparkline = render_sparkline(data, max_val, width);
    format!("{:<5} [{}] {:2}", ch_name, sparkline, current_val)
}
