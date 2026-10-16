export type ValidatedAudioConfig =
  | {
      ok: true;
      silenceSecs: number;
      capSecs: number;
      inputTarget: string;
    }
  | {
      ok: false;
      error: string;
    };

export function parseAudioSeconds(raw: unknown): number | undefined {
  if (raw == null) return 0;
  const str = String(raw).trim();
  if (str === "") return 0;
  if (!/^\d+$/.test(str)) return undefined;
  return Number(str);
}

export function validateAudioConfig(draft: {
  silenceRaw: string;
  capRaw: string;
  inputTarget: string;
}): ValidatedAudioConfig {
  const silenceSecs = parseAudioSeconds(draft.silenceRaw);
  const capSecs = parseAudioSeconds(draft.capRaw);
  if (silenceSecs === undefined || capSecs === undefined) {
    return {
      ok: false,
      error: "Silence stop and maximum duration must be whole seconds. 0 turns that limit off.",
    };
  }
  return {
    ok: true,
    silenceSecs,
    capSecs,
    inputTarget: draft.inputTarget.trim(),
  };
}
