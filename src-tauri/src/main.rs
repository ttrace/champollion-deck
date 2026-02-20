#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::Serialize;
use std::path::Path;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State, Window};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;
use tokio::sync::watch;

const DEFAULT_MODEL: &str = "translategemma:4b";
const DEFAULT_TARGET_LANGUAGE: &str = "Japanese";
const LOG_EVENT: &str = "ollama://log";
const INPUT_EVENT: &str = "ollama://input";

#[derive(Default)]
struct AppState {
  cancel_tx: Mutex<Option<watch::Sender<bool>>>,
  pending_input: Mutex<Option<String>>,
}

impl AppState {
  fn set_cancel(&self, tx: watch::Sender<bool>) {
    let mut guard = self.cancel_tx.lock().expect("cancel lock poisoned");
    *guard = Some(tx);
  }

  fn is_running(&self) -> bool {
    let guard = self.cancel_tx.lock().expect("cancel lock poisoned");
    guard.is_some()
  }

  fn clear_cancel(&self) {
    let mut guard = self.cancel_tx.lock().expect("cancel lock poisoned");
    *guard = None;
  }

  fn cancel(&self) {
    let guard = self.cancel_tx.lock().expect("cancel lock poisoned");
    if let Some(tx) = guard.as_ref() {
      let _ = tx.send(true);
    }
  }

  fn set_input(&self, input: String) {
    let mut guard = self.pending_input.lock().expect("input lock poisoned");
    *guard = Some(input);
  }

  fn take_input(&self) -> Option<String> {
    let mut guard = self.pending_input.lock().expect("input lock poisoned");
    guard.take()
  }
}

#[derive(Serialize, Clone)]
struct DonePayload {
  ok: bool,
  code: Option<i32>,
}

#[derive(Serialize, Clone)]
struct ErrorPayload {
  message: String,
}

fn build_prompt(text: &str, target_language: &str) -> String {
  let mut prompt = format!(
    "You are a professional translator.\n- Translate the following text into {}.\n- Output must be {} only.\n- Keep tone and nuance.\n- Do not add explanations.\n---\n{}",
    target_language, target_language, text
  );
  prompt.push('\n');
  prompt
}

fn resolve_ollama_path() -> String {
  let candidates = ["/usr/local/bin/ollama", "/opt/homebrew/bin/ollama"];
  for candidate in candidates {
    if Path::new(candidate).exists() {
      return candidate.to_string();
    }
  }
  "ollama".to_string()
}

fn emit_log(window: &Window, message: impl Into<String>) {
  let _ = window.emit(LOG_EVENT, message.into());
}

fn strip_ansi(input: &str) -> String {
  let mut out = String::with_capacity(input.len());
  let mut chars = input.chars().peekable();
  while let Some(ch) = chars.next() {
    if ch == '\u{1b}' {
      if matches!(chars.peek(), Some('[')) {
        let _ = chars.next();
        while let Some(next) = chars.next() {
          if ('A'..='Z').contains(&next) || ('a'..='z').contains(&next) {
            break;
          }
        }
      }
      continue;
    }
    out.push(ch);
  }
  out
}

#[tauri::command]
async fn translate_stream(
  window: Window,
  app: AppHandle,
  state: State<'_, AppState>,
  text: String,
  model: Option<String>,
  target_language: Option<String>,
) -> Result<(), String> {
  if text.trim().is_empty() {
    return Err("source text is empty".into());
  }

  if state.is_running() {
    return Err("translation already running".into());
  }

  let model = model.unwrap_or_else(|| DEFAULT_MODEL.to_string());
  let target_language = target_language
    .map(|value| value.trim().to_string())
    .filter(|value| !value.is_empty())
    .unwrap_or_else(|| DEFAULT_TARGET_LANGUAGE.to_string());
  let prompt = build_prompt(&text, &target_language);
  let cmd_path = resolve_ollama_path();
  emit_log(
    &window,
    format!(
      "start: cmd={} model={} target_language={} prompt_bytes={}",
      cmd_path,
      model,
      target_language,
      prompt.len()
    ),
  );

  let (tx, rx) = watch::channel(false);
  state.set_cancel(tx);

  tauri::async_runtime::spawn(async move {
    let result = run_translation(window, prompt, model, cmd_path, rx).await;
    if let Err(message) = result {
      let _ = app.emit("ollama://error", ErrorPayload { message });
    }
    app.state::<AppState>().clear_cancel();
  });

  Ok(())
}

#[tauri::command]
async fn cancel_translation(state: State<'_, AppState>) -> Result<(), String> {
  state.cancel();
  Ok(())
}

#[tauri::command]
fn take_initial_input(state: State<'_, AppState>) -> Option<String> {
  state.take_input()
}

async fn run_translation(
  window: Window,
  prompt: String,
  model: String,
  cmd_path: String,
  mut cancel_rx: watch::Receiver<bool>,
) -> Result<(), String> {
  let mut path_env = std::env::var("PATH").unwrap_or_default();
  for extra in ["/usr/local/bin", "/opt/homebrew/bin"] {
    if !path_env.split(':').any(|entry| entry == extra) {
      path_env.push(':');
      path_env.push_str(extra);
    }
  }

  let mut child = Command::new(&cmd_path)
    .arg("run")
    .arg(model)
    .env("TERM", "dumb")
    .env("OLLAMA_NO_COLOR", "1")
    .env("PATH", path_env)
    .stdin(std::process::Stdio::piped())
    .stdout(std::process::Stdio::piped())
    .stderr(std::process::Stdio::piped())
    .spawn()
    .map_err(|err| format!("failed to start ollama: {err}"))?;

  if let Some(pid) = child.id() {
    emit_log(&window, format!("spawned: pid={pid}"));
  }

  if let Some(mut stdin) = child.stdin.take() {
    stdin
      .write_all(prompt.as_bytes())
      .await
      .map_err(|err| format!("failed to write prompt: {err}"))?;
    stdin
      .shutdown()
      .await
      .map_err(|err| format!("failed to close stdin: {err}"))?;
  }

  let mut stdout = child
    .stdout
    .take()
    .ok_or_else(|| "failed to capture stdout".to_string())?;
  let stderr = child
    .stderr
    .take()
    .ok_or_else(|| "failed to capture stderr".to_string())?;

  let stderr_window = window.clone();
  let stderr_task = tauri::async_runtime::spawn(async move {
    let mut buffer = Vec::new();
    let mut stderr = stderr;
    let mut chunk = [0u8; 1024];
    loop {
      match stderr.read(&mut chunk).await {
        Ok(0) => break,
        Ok(n) => {
          buffer.extend_from_slice(&chunk[..n]);
          let text = String::from_utf8_lossy(&chunk[..n]);
          let cleaned = strip_ansi(&text);
          let trimmed = cleaned.trim();
          if !trimmed.is_empty() {
            emit_log(&stderr_window, format!("stderr: {trimmed}"));
          }
        }
        Err(_) => break,
      }
    }
    String::from_utf8_lossy(&buffer).to_string()
  });

  let mut buf = vec![0u8; 1024];
  let mut received_any = false;
  loop {
    tokio::select! {
      read = stdout.read(&mut buf) => {
        let n = read.map_err(|err| format!("failed to read stdout: {err}"))?;
        if n == 0 {
          emit_log(&window, "stdout closed");
          break;
        }
        received_any = true;
        let chunk = String::from_utf8_lossy(&buf[..n]).to_string();
        let _ = window.emit("ollama://chunk", chunk);
      }
      changed = cancel_rx.changed() => {
        if changed.is_ok() && *cancel_rx.borrow() {
          emit_log(&window, "cancel signal received");
          let _ = child.kill().await;
          break;
        }
      }
    }
  }

  let status = child
    .wait()
    .await
    .map_err(|err| format!("failed to wait for process: {err}"))?;
  let stderr_output = stderr_task.await.unwrap_or_default();
  emit_log(
    &window,
    format!("exit: success={} code={:?}", status.success(), status.code()),
  );

  if !status.success() {
    let message = if stderr_output.trim().is_empty() {
      "ollama exited with an error".to_string()
    } else {
      stderr_output
    };
    let _ = window.emit("ollama://error", ErrorPayload { message });
  } else if !received_any {
    let _ = window.emit(
      "ollama://error",
      ErrorPayload {
        message: "ollama returned no output".to_string(),
      },
    );
  }

  let _ = window.emit(
    "ollama://done",
    DonePayload {
      ok: status.success(),
      code: status.code(),
    },
  );

  Ok(())
}

fn main() {
  let initial_input = std::env::args().skip(1).collect::<Vec<String>>().join(" ");
  tauri::Builder::default()
    .manage(AppState::default())
    .invoke_handler(tauri::generate_handler![
      translate_stream,
      cancel_translation,
      take_initial_input
    ])
    .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
      let input = argv.iter().skip(1).cloned().collect::<Vec<String>>().join(" ");
      if input.trim().is_empty() {
        return;
      }
      if let Some(state) = app.try_state::<AppState>() {
        state.set_input(input.clone());
      }
      if let Some(window) = app.get_webview_window("main") {
        let _ = window.emit(INPUT_EVENT, input);
      }
    }))
    .setup(move |app| {
      if !initial_input.trim().is_empty() {
        if let Some(state) = app.try_state::<AppState>() {
          state.set_input(initial_input.clone());
        }
        if let Some(window) = app.get_webview_window("main") {
          let _ = window.emit(INPUT_EVENT, initial_input.clone());
        }
      }
      Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
