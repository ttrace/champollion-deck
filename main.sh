#!/bin/zsh
set -euo pipefail

LCMODEL="${LCMODEL:-translategemma:4b}"   # 環境変数でモデル切替OK (例: LCMODEL=llama3)
# LCMODEL="hoangquan456/qwen3-nothink:8b" # 環境変数を使わない場合はこの行を有効化して任意のモデルを指定

# Enable debug when run from a terminal (TTY). If stdout is a TTY then enable DEBUG=1.
if [ -t 1 ]; then
  DEBUG=1
else
  DEBUG=${DEBUG:-0}
fi

if [ "${DEBUG:-0}" -eq 1 ]; then
  LOGFILE="${LOGFILE:-/tmp/ollama_main_debug.log}"
  # Send stdout/stderr to both the terminal and the log file
  exec > >(tee -a "$LOGFILE") 2>&1
  set -x  # enable command tracing
  echo "DEBUG mode enabled. Logging to $LOGFILE"
fi


# Read input text. When run under Automator stdin contains the selected text.
# When run interactively from a terminal (stdin is a TTY), fall back to first argument or the clipboard for easy debugging.
if [ -t 0 ]; then
  if [ "$#" -gt 0 ]; then
    SRC_TEXT="$*"   # arguments as the source text
    printf '%s\n' "[DEBUG] read SRC_TEXT from args" >&2
  else
    SRC_TEXT="$(pbpaste)"  # use clipboard when no args provided
    printf '%s\n' "[DEBUG] read SRC_TEXT from clipboard (pbpaste)" >&2
  fi
else
  SRC_TEXT="$(cat)"           # Automator からの選択テキスト
fi

if [[ -z "$SRC_TEXT" ]]; then
  osascript -e 'display notification "選択テキストが空です" with title "Ollama Translator"'
  exit 1
fi

PROMPT=$'You are a professional translator.\n- Translate the following text into Japanese.\n- Keep tone and nuance.\nDo not add explanations.\n---\n'"$SRC_TEXT"

# ※ ollama run はプレーンテキストで逐次出力されるので、そのまま受け取れる
RESULT="$(printf "%s" "$PROMPT" | /usr/local/bin/ollama run "$LCMODEL" 2>/dev/null || true)"
# Homebrew 以外の人用に PATH も一応探す
if [[ -z "$RESULT" && -x /opt/homebrew/bin/ollama ]]; then
  RESULT="$(printf "%s" "$PROMPT" | /opt/homebrew/bin/ollama run "$LCMODEL" 2>/dev/null || true)"
fi

if [[ -z "$RESULT" ]]; then
  osascript -e 'display notification "Ollama 実行に失敗しました" with title "Ollama Translator"'
  exit 2
fi

# ここで "\n" → 実際の改行 に変換する
RESULT="${RESULT//\\n/$'\n'}"

# クリップボードへ
printf '%s' "$RESULT" | pbcopy

# 翻訳結果を一時ファイルに保存して TextEdit で表示
# macOS では -t オプションでプレフィックスを指定するのが簡単
tmpfile="$(mktemp -t ollama-translate)"
printf '%s\n' "$RESULT" > "$tmpfile"
open -a TextEdit "$tmpfile"

# Automator の出力としても返す（アプリによってはそのまま差し替えできる）
printf '%s' "$RESULT"
