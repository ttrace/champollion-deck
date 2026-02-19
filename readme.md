# Champollion Deck (Tauri App)

Ollama を使ったローカル翻訳アプリです。Tauri の UI でストリーム出力を確認できます。
Automator から選択テキストを渡して自動翻訳する運用もできます。

---

## 特徴

- ✅ Ollama を使ったローカル翻訳
- ✅ ストリーム出力をリアルタイム表示
- ✅ Model を UI で切り替え可能
- ✅ Automator から選択テキストを引数で渡して自動翻訳

---

## 動作要件

- macOS
- Node.js (npm)
- Rust (cargo)
- Tauri CLI
- Ollama
  - `/usr/local/bin/ollama` または `/opt/homebrew/bin/ollama`

---

## 開発起動

```bash
npm install
npm run tauri dev
```

---

## ビルド

```bash
npm run tauri build
```

`.app` は `src-tauri/target/release/bundle/macos/` に生成されます。

---

## 使い方

- `Source` に翻訳したいテキストを入力
- `Translate` でストリーム出力開始
- `Stop` でキャンセル
- `Copy` で出力をクリップボードへ
- `Model` 欄で Ollama モデル名を切り替え可能（例: `llama3`）

---

## Automator から使う

「クイックアクション」経由で選択テキストを引数として渡します。
既にアプリが起動中の場合でも引数が確実に渡るように `open -na` を使います。

1. Automator を開き、「クイックアクション」を新規作成
2. 「ワークフローが受け取る現在の項目」を `テキスト` に設定
3. 「シェルスクリプトを実行」アクションを追加
4. シェル: `/bin/zsh`
5. 入力の引き渡し: `引数`
6. スクリプト欄に以下を記述

```bash
INPUT="$*"
open -na "/Applications/Champollion Deck.app" --args "$INPUT"
```

注意:
- `.app` のパスは実際の配置場所に合わせて変更してください
- 引数が渡された場合、アプリは自動で翻訳を開始します

---

## 補足

- デフォルトモデル: `translategemma:4b`
- Ollama のパスは以下を優先して探索します
  - `/usr/local/bin/ollama`
  - `/opt/homebrew/bin/ollama`
  - それ以外は `PATH` から `ollama`

---

## ディレクトリ構成

- `src-tauri/`
- `src/`
- `index.html`
- `vite.config.ts`
- `package.json`
- `main.sh`
- `readme.md`
- `Agent.md`

概要:
- `src-tauri/`: Tauri（Rust）側。Ollama 実行、ストリーム送信、引数受け取りを担当
- `src/`: フロントエンド（TS/CSS）。UI とイベント受信、表示制御
- `index.html`: UI の HTML
- `vite.config.ts`: Vite のビルド設定
- `package.json`: フロントエンド依存と scripts
- `main.sh`: 旧スクリプト版（参考）
- `readme.md`: 使い方と Automator 連携の説明
- `Agent.md`: 次回開発の手順メモ

---

## 旧スクリプト版（参考）

`main.sh` は従来の Automator 用シェルスクリプトです。現在はアプリ版がメインです。
必要であれば `main.sh` を使ったフローも引き続き利用できます。

---

## ライセンス / モデル利用について

- MIT
- 使用するモデルのライセンス・利用規約に従ってください
