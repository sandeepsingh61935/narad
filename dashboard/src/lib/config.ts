/** Mirrors dashboard src-tauri `DashboardConfig` (snake_case JSON). */
export type Replacement = {
  spoken: string;
  written: string;
};

export type AppProfile = {
  app_id: string;
  mode: string;
  paste_with_shift?: boolean;
};

export type DashboardConfig = {
  activation: {
    dictate_hotkey: string;
    paste_last_hotkey: string;
    undo_last_hotkey: string;
    selection_hotkey: string;
    mode_hotkeys_enabled: boolean;
    hold_tap_threshold_ms: number;
    wake_word_enabled: boolean;
    wake_word_phrase: string;
  };
  audio: {
    sounds_enabled: boolean;
    auto_stop_silence_secs: number;
    max_duration_secs: number;
    /** PipeWire source id. Empty follows the system default (FR-AUD-4). */
    input_target: string;
    /** RNNoise before VAD/ASR (FR-AUD-5). Off until the user turns it on. */
    noise_suppression: boolean;
  };
  general: {
    active_mode: string;
  };
  models: {
    asr_english: string;
  };
  history: {
    /** Keep utterance audio. Off until the user turns it on. */
    retain_audio: boolean;
    /** Days before text rows are deleted. Null means keep text forever. */
    text_retention_days: number | null;
    /** Days before audio files are removed. Null means keep audio forever. */
    audio_retention_days: number | null;
  };
  /** Spoken → written phrases (FR-TXT-3). Empty until the user adds one. */
  replacements: Replacement[];
  /** Names and jargon applied after replacements (FR-TXT-2). */
  vocabulary: Replacement[];
  /** Per-app dictation profiles (FR-CTX-2). */
  profiles: AppProfile[];
};

export const ACTIVE_MODES = ["plain", "email", "bullet", "code"] as const;

export type ActiveMode = (typeof ACTIVE_MODES)[number];

export function defaultConfig(): DashboardConfig {
  return {
    activation: {
      dictate_hotkey: "Ctrl+Alt+Space",
      paste_last_hotkey: "Ctrl+Alt+V",
      undo_last_hotkey: "Ctrl+Alt+Z",
      selection_hotkey: "Ctrl+Alt+R",
      mode_hotkeys_enabled: true,
      hold_tap_threshold_ms: 250,
      wake_word_enabled: false,
      wake_word_phrase: "hey narad",
    },
    audio: {
      sounds_enabled: true,
      auto_stop_silence_secs: 2,
      max_duration_secs: 300,
      input_target: "",
      noise_suppression: false,
    },
    general: {
      active_mode: "plain",
    },
    models: {
      asr_english: "parakeet-ctc-110m-int8",
    },
    history: {
      retain_audio: false,
      text_retention_days: null,
      audio_retention_days: null,
    },
    replacements: [],
    vocabulary: [],
    profiles: [],
  };
}
