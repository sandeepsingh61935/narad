import { deleteAllConfirmed } from "./screen-state";

export type ValidatedPrivacyDraft =
  | {
      ok: true;
      textRetentionDays: number | null;
      audioRetentionDays: number | null;
    }
  | {
      ok: false;
      error: string;
    };

export const RETENTION_ERROR =
  "Retention must be a whole number of days, or blank to keep.";

export const DELETE_ALL_CONFIRMATION_PHRASE = "delete all history";

/** Blank or missing means keep forever (null). Non-negative whole numbers return days. Negative/non-numeric returns undefined. */
export function parseRetentionDays(raw: unknown): number | null | undefined {
  if (raw == null) return null;
  if (typeof raw === "string") {
    const trimmed = raw.trim();
    if (trimmed === "") return null;
    if (!/^\d+$/.test(trimmed)) return undefined;
    return Number(trimmed);
  }
  if (typeof raw !== "number" || !Number.isFinite(raw) || raw < 0) {
    return undefined;
  }
  return Math.round(raw);
}

export function formatRetentionDays(value: number | null): string {
  return value == null ? "" : String(value);
}

export function validatePrivacyDraft(draft: {
  textDaysRaw: string;
  audioDaysRaw: string;
}): ValidatedPrivacyDraft {
  const textRetentionDays = parseRetentionDays(draft.textDaysRaw);
  const audioRetentionDays = parseRetentionDays(draft.audioDaysRaw);
  if (textRetentionDays === undefined || audioRetentionDays === undefined) {
    return {
      ok: false,
      error: RETENTION_ERROR,
    };
  }
  return {
    ok: true,
    textRetentionDays,
    audioRetentionDays,
  };
}

export function isDeleteAllConfirmed(phrase: string): boolean {
  return deleteAllConfirmed(phrase);
}
