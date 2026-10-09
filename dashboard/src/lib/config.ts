/** Mirrors dashboard src-tauri `DashboardConfig` (snake_case JSON). */
export type DashboardConfig = {
  activation: {
    dictate_hotkey: string;
    hold_tap_threshold_ms: number;
  };
  audio: {
    sounds_enabled: boolean;
    auto_stop_silence_secs: number;
    max_duration_secs: number;
  };
  general: {
    active_mode: string;
  };
  models: {
    asr_english: string;
  };
};

export const ACTIVE_MODES = ["plain", "email", "bullet", "code"] as const;

export type ActiveMode = (typeof ACTIVE_MODES)[number];

export function defaultConfig(): DashboardConfig {
  return {
    activation: {
      dictate_hotkey: "Ctrl+Alt+Space",
      hold_tap_threshold_ms: 250,
    },
    audio: {
      sounds_enabled: true,
      auto_stop_silence_secs: 2,
      max_duration_secs: 300,
    },
    general: {
      active_mode: "plain",
    },
    models: {
      asr_english: "parakeet-ctc-110m-int8",
    },
  };
}
