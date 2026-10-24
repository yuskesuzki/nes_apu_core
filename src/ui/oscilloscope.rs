//! 複数行オシロスコープ形式の波形プロッター
//!
//! 2D ASCII/Unicode グリッドを用いて、オシロスコープのように
//! 矩形波のデューティ比、三角波のステップ、ノイズ・DMC の波形変化を可視化します。

/// 単一チャンネルの 2D 波形グリッドを行単位の文字列のベクタとして生成します
///
/// - `data`: サンプル履歴配列
/// - `max_val`: 出力値の最大スケール (Pulse/Tri/Noise は 15, DMC は 127)
/// - `width`: 描画幅（列数）
/// - `height`: 描画高さ（行数, 推奨 4..=8）
/// - `symbol`: 波形描画に使用する文字（例: `'█'`, `'•'`, `'*'`）
pub fn render_waveform_grid(
    data: &[u8],
    max_val: u8,
    width: usize,
    height: usize,
    symbol: char,
) -> Vec<String> {
    if height == 0 || width == 0 {
        return Vec::new();
    }

    let mut grid = vec![vec![' '; width]; height];
    let start = data.len().saturating_sub(width);
    let slice = &data[start..];
    let offset_x = width.saturating_sub(slice.len());

    let max_f = max_val.max(1) as f32;
    for (i, &val) in slice.iter().enumerate() {
        let x = offset_x + i;
        if x >= width {
            break;
        }
        let ratio = (val as f32 / max_f).clamp(0.0, 1.0);
        // 上端が最大値、下端が 0
        let y = ((1.0 - ratio) * (height - 1) as f32).round() as usize;
        grid[y.min(height - 1)][x] = symbol;
    }

    grid.into_iter()
        .map(|row| row.into_iter().collect())
        .collect()
}

/// 枠線および Y 軸ラベル付きのオシロスコープ表示ブロックを生成します
pub fn format_oscilloscope_panel(
    title: &str,
    data: &[u8],
    max_val: u8,
    width: usize,
    height: usize,
) -> Vec<String> {
    let mut lines = Vec::with_capacity(height + 2);
    let grid_lines = render_waveform_grid(data, max_val, width, height, '█');

    // ヘッダー行
    let header_border = "─".repeat(width.saturating_sub(title.chars().count() + 3));
    lines.push(format!("┌─[ {} ]{}┐", title, header_border));

    // 各波形行（左右枠線付き）
    for (row_idx, row_str) in grid_lines.into_iter().enumerate() {
        let label = if row_idx == 0 {
            format!("{:2}", max_val)
        } else if row_idx == height - 1 {
            " 0".to_string()
        } else {
            "  ".to_string()
        };
        lines.push(format!("│{}│ {}", row_str, label));
    }

    // フッター行
    lines.push(format!("└{}┘", "─".repeat(width)));

    lines
}
