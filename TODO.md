# プロジェクト TODO / 開発タスクリスト

## 優先タスク: Ratatui (https://ratatui.rs) による TUI ダッシュボード刷新

### 1. 概要・目的
現在 ANSI エスケープシーケンスおよび `crossterm` のプリミティブな出力で実装されているリアルタイム診断ダッシュボード（`src/ui/dashboard.rs`）を、Rust の高機能 TUI ライブラリ [Ratatui](https://ratatui.rs) を用いて再構築する。

### 2. 課題と Ratatui 移行によるメリット
- **描画アーティファクトの完全解消**:
  - 現在は `\x1b[H` と `\x1b[J` を用いて上書き描画しているが、Ratatui のダブルバッファリング / 差分更新により画面のチラつきや行伸縮時の残骸を完全に排除できる。
- **モーダルポップアップの堅牢化**:
  - `Shift+T`（BGM選択）や `Shift+Z/X`（SE選択）のダイアログを Ratatui の `Clear` ウィジェットと `Block` を用いた本格的なオーバーレイウィンドウとして実装可能。
- **レスポンシブな画面レイアウト**:
  - `Layout::default().direction(...).constraints(...)` により、ターミナルの縦横サイズに応じた見やすいグリッド配置（例: Pulse 1/2 を横並び、波形を右ペインに表示等）が可能。
- **標準ウィジェットの活用**:
  - 出力電圧やエンベロープメーター: `Gauge` ウィジェット
  - 各チャンネルの波形モニタリング: `Sparkline` ウィジェット
  - オシロスコープ機能のダッシュボード統合: `Chart` ウィジェット

### 3. 具体的な実装ステップ
1. **依存関係の追加 (`Cargo.toml`)**:
   - `ratatui = "0.29"` (バックエンドには既存の `crossterm` を指定)
2. **UI ウィジェットのモジュール化 (`src/ui/ratatui/`)**:
   - `status_bar.rs`: APU 全体ステータス ($4015, $4017, 実時間, CPUサイクル, 操作ガイド)
   - `channel_panel.rs`: Pulse 1/2, Triangle, Noise, DMC の個別状態パネル (`Block`, `Gauge`, `Paragraph`)
   - `analog_panel.rs`: 非線形ミキサーおよび実機 RC フィルター状態
   - `driver_panel.rs`: 6502 サウンドドライバの音符・音量・ウェイト情報
   - `popup_dialog.rs`: `Clear` ウィジェットを用いた BGM / SE 選択モーダルウィンドウ
3. **メインループとの統合 (`src/main.rs`)**:
   - `Terminal::draw(...)` を 60fps ループまたはイベントドリブンで呼び出し
   - crossterm イベントループと Ratatui のシームレスな連携
4. **オシロスコープ / スパークラインの統合 (`examples/waveform_demo.rs` の機能集約)**:
   - 各チャンネルのリアルタイム波形バッファを直接 Ratatui の `Sparkline` / `Chart` ウィジェットへバインド
