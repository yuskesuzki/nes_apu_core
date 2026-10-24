# NES APU Hardware Core (Complete Cycle-Accurate Core & 6502 Sound Driver)

ファミコン実機（Ricoh RP2A03 APU）の音響・挙動を完全再現するための低レベル・デジタル論理回路、アナログ回路、および 6502 サイクル同期演奏ドライバシステムです。

OS のタイマーや浮動小数点近似を一切使用せず、1.789773 MHz の CPU 整数クロック、APU ハードウェア変調論理、非線形 DAC 抵抗ラダー、実機基板の 3 段 RC フィルター、および 29,780 サイクル完全同期 V-Blank ドライバにより駆動されます。

---

## 実行方法（ターミナル・ハードウェア診断ダッシュボード）

### 1. NSF (NES Sound Format) 楽曲ファイルの演奏 (`-n`, `--nsf` 【必須】)
ファミコン実機の NSF ファイルを読み込み、60Hz V-Blank 完全同期で 6502 コードを実行してリアルタイム演奏・診断モニタリングします。
```bash
# 基本再生 (ターミナルダッシュボード表示)
cargo run -- --nsf path/to/music.nsf

# リアルタイム音声出力付きで再生
cargo run -- --audio --nsf path/to/music.nsf

# トラック番号を指定して再生 (1-based、例: トラック 3)
cargo run -- --audio --nsf path/to/music.nsf --track 3
```

### 2. 生デジタル DAC との比較 (`--filter-bypass`)
```bash
cargo run -- --audio --nsf path/to/music.nsf --filter-bypass
```

### 3. 演奏中のチャンネル消音 (Mute) 切り替えキー操作
演奏中にキーボードの数字キー `1` 〜 `5` を押すことで、各チャンネルの消音（Mute）／発音（Play）をリアルタイムにトグル切り替えできます。
ハードウェア内部シーケンサや 6502 の進行を止めることなく、非線形 DAC ミキサーへの出力レベルのみを安全にマスクするため、完全なクロック同期を維持したまま各トラックを個別確認できます。

- `1`: Pulse 1 (主旋律)
- `2`: Pulse 2 (ハーモニー・アルペジオ)
- `3`: Triangle (ベース)
- `4`: Noise (ドラム・パーカッション)
- `5`: DMC (サンプル再生)
- `q` または `Esc`: 終了

### 4. 温かい音色調整・フィルターカスタマイズ
- **マイルドで太いウォームトーン (デフォルト: 5.5kHz LPF)**:
  ```bash
  cargo run -- --audio --nsf path/to/music.nsf --profile warm
  ```
- **ブラウン管テレビ風のまろやかなトーン (4.2kHz LPF)**:
  ```bash
  cargo run -- --audio --nsf path/to/music.nsf --profile tv
  ```
- **さらにカットオフ周波数を任意指定 (例: 4.5kHz)**:
  ```bash
  cargo run -- --audio --nsf path/to/music.nsf --lpf 4500
  ```
- **デューティ比の上書き (0=12.5%, 1=25%, 2=50% 太い丸み, 3=75%)**:
  ```bash
  cargo run -- --audio --nsf path/to/music.nsf --duty 2
  ```

### 5. 各チャンネル波形の CUI モニターデモ (`waveform_demo`)
各チャンネルの出力電圧推移をターミナル上に波形として表示します。
```bash
# 1行スパークライン形式で表示
cargo run --example waveform_demo -- --nsf path/to/music.nsf

# 2D オシロスコープ形式で全チャンネル波形を表示
cargo run --example waveform_demo -- --nsf path/to/music.nsf --oscilloscope
```

### 6. ハードウェア検証・実行オプション
- **Frame Counter 5-Step モード (192Hz, IRQなし)**: `cargo run -- --nsf path/to/music.nsf --mode 5` (または `--5-step`)
- **1 サイクル刻みトレース**: `cargo run -- --nsf path/to/music.nsf --trace`
- **フレーム数指定**: `cargo run -- --nsf path/to/music.nsf --frames 1`

---

## テストの実行

```bash
cargo test
```

全 49 項目のハードウェア＆ドライバ検証テスト（Mute/Solo, Driver, Filter, DMC, Status $4015, Frame Counter, Envelope, Sweep, Length Counter, Linear Counter, Pulse, Triangle, Noise）がすべて PASS します。
