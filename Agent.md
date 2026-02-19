# Agent Notes

このリポジトリの開発をすぐ再開できるように、最低限の手順と構成をまとめています。

## 構成

- Tauri 本体: `src-tauri/`
- フロントエンド: `src/`, `index.html`, `vite.config.ts`
- Automator 連携: README のスクリプト参照
- 旧スクリプト: `main.sh`

## よく使うコマンド

開発起動:
```bash
npm run tauri dev
```

ビルド:
```bash
npm run tauri build
```

アイコン更新:
```bash
npx tauri icon src-tauri/icons/icon.png
```

## Automator 用スクリプト（引数渡し）

```bash
INPUT="$*"
open -na "/Applications/Champollion Deck.app" --args "$INPUT"
```

## 重要な実装ポイント

- 起動引数は `take_initial_input` でフロントが取得
- 既起動時の引数は `tauri-plugin-single-instance` で受け取り
- イベント名: `ollama://input`, `ollama://chunk`, `ollama://done`, `ollama://error`

## トラブルシュート

- 引数が届かない
  - `open -na` を使用
  - 既起動時は single-instance 経由で受け取り
- `ollama` が見つからない
  - `/usr/local/bin` と `/opt/homebrew/bin` を PATH に追加済み
- アイコン系のエラー
  - `npx tauri icon` で再生成
