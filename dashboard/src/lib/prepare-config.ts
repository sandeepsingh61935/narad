import type { DashboardConfig } from "./config";

export type PreparedConfig =
  | { ok: true; config: DashboardConfig }
  | { ok: false; error: string };

const THRESHOLD_ERROR = "Hold/tap threshold must be a non-negative number.";

type ConfigDraft = Omit<DashboardConfig, "activation"> & {
  activation: Omit<DashboardConfig["activation"], "hold_tap_threshold_ms"> & {
    hold_tap_threshold_ms: unknown;
  };
};

/** Blank or missing means keep that kind forever. Non-negative numbers are whole days. */
function optionalDays(value: unknown): number | null {
  if (value == null) return null;
  if (typeof value === "string") {
    if (value.trim() === "") return null;
    return optionalDays(Number(value));
  }
  if (typeof value !== "number" || !Number.isFinite(value) || value < 0) {
    return null;
  }
  return Math.round(value);
}

function roundedThreshold(value: unknown): number | null {
  if (typeof value === "string") {
    if (value.trim() === "") return null;
    return roundedThreshold(Number(value));
  }
  if (typeof value !== "number" || !Number.isFinite(value) || value < 0) {
    return null;
  }
  return Math.round(value);
}

/** Config that should be passed to `save_config`, or a rejection. */
export function prepareConfigForSave(config: ConfigDraft): PreparedConfig {
  const hold_tap_threshold_ms = roundedThreshold(config.activation.hold_tap_threshold_ms);
  if (hold_tap_threshold_ms === null) {
    return { ok: false, error: THRESHOLD_ERROR };
  }
  return {
    ok: true,
    config: {
      activation: {
        dictate_hotkey: config.activation.dictate_hotkey.trim(),
        hold_tap_threshold_ms,
      },
      audio: {
        ...config.audio,
        input_target: config.audio.input_target.trim(),
      },
      general: {
        active_mode: config.general.active_mode.trim() || "plain",
      },
      models: {
        asr_english: config.models.asr_english.trim(),
      },
      history: {
        retain_audio: config.history.retain_audio,
        text_retention_days: optionalDays(config.history.text_retention_days),
        audio_retention_days: optionalDays(config.history.audio_retention_days),
      },
    },
  };
}
