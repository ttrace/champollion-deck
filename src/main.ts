import "./style.css";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getVersion } from "@tauri-apps/api/app";

const sourceEl = document.querySelector<HTMLTextAreaElement>("#source")!;
const resultEl = document.querySelector<HTMLPreElement>("#result")!;
const statusEl = document.querySelector<HTMLDivElement>("#status")!;
const modelEl = document.querySelector<HTMLInputElement>("#model")!;
const targetLanguageEl = document.querySelector<HTMLInputElement>("#target-language")!;
const modelPopoverEl = document.querySelector<HTMLDivElement>("#model-popover")!;
const modelToggleBtn = document.querySelector<HTMLButtonElement>("#model-toggle")!;
const modelPanelEl = document.querySelector<HTMLDivElement>("#model-panel")!;
const appVersionEl = document.querySelector<HTMLSpanElement>("#app-version")!;
const translateBtn = document.querySelector<HTMLButtonElement>("#translate")!;
const stopBtn = document.querySelector<HTMLButtonElement>("#stop")!;
const copyBtn = document.querySelector<HTMLButtonElement>("#copy")!;
const pasteBtn = document.querySelector<HTMLButtonElement>("#paste")!;
const clearBtn = document.querySelector<HTMLButtonElement>("#clear")!;

const MODEL_KEY = "ollama-translator-model";
const DEFAULT_MODEL = "translategemma:4b";
const TARGET_LANGUAGE_KEY = "ollama-translator-target-language";
const DEFAULT_TARGET_LANGUAGE = "Japanese";

let running = false;
let unsubscribeChunk: (() => void) | null = null;
let unsubscribeDone: (() => void) | null = null;
let unsubscribeError: (() => void) | null = null;
let unsubscribeInput: (() => void) | null = null;
let pendingInput: string | null = null;

function setStatus(message: string) {
  statusEl.textContent = message;
}

function isTauriAvailable() {
  return Boolean((window as any).__TAURI__ || (window as any).__TAURI_INTERNALS__);
}

function setRunning(value: boolean) {
  running = value;
  translateBtn.disabled = value;
  stopBtn.disabled = !value;
}

function handleIncomingInput(text: string) {
  const trimmed = text.trim();
  if (!trimmed) {
    return;
  }
  sourceEl.value = trimmed;
  if (running) {
    pendingInput = trimmed;
    setStatus("Restarting...");
    void stop();
    return;
  }
  setStatus("Input received.");
  void translate();
}

async function setupListeners() {
  try {
    unsubscribeChunk = await listen<string>("ollama://chunk", (event) => {
      resultEl.textContent += event.payload;
    });

    unsubscribeDone = await listen<{ ok: boolean; code: number | null }>(
      "ollama://done",
      (event) => {
        setRunning(false);
        const suffix = event.payload.ok ? "Completed" : "Exited";
        setStatus(`${suffix} (code: ${event.payload.code ?? "?"}).`);
        if (pendingInput) {
          const next = pendingInput;
          pendingInput = null;
          handleIncomingInput(next);
        }
      }
    );

    unsubscribeError = await listen<{ message: string }>("ollama://error", (event) => {
      setRunning(false);
      setStatus(`Error: ${event.payload.message}`);
    });

    unsubscribeInput = await listen<string>("ollama://input", (event) => {
      handleIncomingInput(event.payload ?? "");
    });
  } catch (error) {
    throw error;
  }
}

function cleanupListeners() {
  unsubscribeChunk?.();
  unsubscribeDone?.();
  unsubscribeError?.();
  unsubscribeInput?.();
  unsubscribeChunk = null;
  unsubscribeDone = null;
  unsubscribeError = null;
  unsubscribeInput = null;
}

function getModel() {
  const saved = localStorage.getItem(MODEL_KEY);
  return saved?.trim() || DEFAULT_MODEL;
}

function setModel(value: string) {
  localStorage.setItem(MODEL_KEY, value.trim());
}

function getTargetLanguage() {
  const saved = localStorage.getItem(TARGET_LANGUAGE_KEY);
  return saved?.trim() || DEFAULT_TARGET_LANGUAGE;
}

function setTargetLanguage(value: string) {
  localStorage.setItem(TARGET_LANGUAGE_KEY, value.trim());
}

function setModelPanelOpen(open: boolean) {
  modelPanelEl.hidden = !open;
  modelToggleBtn.setAttribute("aria-expanded", String(open));
}

function toggleModelPanel() {
  setModelPanelOpen(modelPanelEl.hidden);
}

async function translate() {
  const text = sourceEl.value.trim();
  if (!text) {
    setStatus("Source text is empty.");
    return;
  }

  if (!running) {
    resultEl.textContent = "";
  }

  setRunning(true);
  setStatus("Streaming...");
  if (!isTauriAvailable()) {
    setStatus("Tauri API unavailable.");
    setRunning(false);
    return;
  }

  const model = modelEl.value.trim() || DEFAULT_MODEL;
  const targetLanguage = targetLanguageEl.value.trim() || DEFAULT_TARGET_LANGUAGE;
  setModel(model);
  setTargetLanguage(targetLanguage);

  cleanupListeners();
  await setupListeners();

  try {
    await invoke("translate_stream", { text, model, targetLanguage });
  } catch (error) {
    setRunning(false);
    setStatus(`Error: ${String(error)}`);
  }
}

async function stop() {
  await invoke("cancel_translation");
  setStatus("Stopping...");
}

async function copyResult() {
  const text = resultEl.textContent || "";
  if (!text) {
    setStatus("No output to copy.");
    return;
  }
  await navigator.clipboard.writeText(text);
  setStatus("Copied to clipboard.");
}

async function pasteSource() {
  try {
    const text = await navigator.clipboard.readText();
    sourceEl.value = text;
    setStatus("Pasted from clipboard.");
  } catch {
    setStatus("Clipboard permission denied.");
  }
}

function clearSource() {
  sourceEl.value = "";
  setStatus("Cleared.");
}

async function setAppVersion() {
  try {
    appVersionEl.textContent = await getVersion();
  } catch {
    appVersionEl.textContent = "dev";
  }
}


modelEl.value = getModel();
targetLanguageEl.value = getTargetLanguage();
setModelPanelOpen(false);
setRunning(false);
setStatus("Ready.");
void setAppVersion();

void setupListeners().then(async () => {
  try {
    const text = (await invoke<string | null>("take_initial_input"))?.trim() || "";
    if (!text) {
      return;
    }
    handleIncomingInput(text);
  } catch (error) {
    setStatus(`Error: ${String(error)}`);
  }
});

translateBtn.addEventListener("click", () => {
  if (!running) void translate();
});

stopBtn.addEventListener("click", () => {
  if (running) void stop();
});

copyBtn.addEventListener("click", () => void copyResult());

pasteBtn.addEventListener("click", () => void pasteSource());

clearBtn.addEventListener("click", clearSource);

modelToggleBtn.addEventListener("click", (event) => {
  event.stopPropagation();
  toggleModelPanel();
});

document.addEventListener("click", (event) => {
  if (modelPanelEl.hidden) {
    return;
  }
  const target = event.target as Node | null;
  if (!target || modelPopoverEl.contains(target)) {
    return;
  }
  setModelPanelOpen(false);
});

document.addEventListener("keydown", (event) => {
  if (event.key === "Escape") {
    setModelPanelOpen(false);
  }
});

window.addEventListener("beforeunload", cleanupListeners);
