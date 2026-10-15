import type { Replacement } from "./config";

export type PairValidation = {
  valid: boolean;
  error?: string;
};

export function cleanDictionaryPairs(pairs: readonly Replacement[]): Replacement[] {
  return pairs
    .map((pair) => ({
      spoken: pair.spoken.trim(),
      written: pair.written.trim(),
    }))
    .filter((pair) => pair.spoken.length > 0);
}

export function findDuplicateSpoken(pairs: readonly Replacement[]): Set<string> {
  const seen = new Set<string>();
  const duplicates = new Set<string>();
  for (const pair of pairs) {
    const key = pair.spoken.trim().toLowerCase();
    if (key.length === 0) continue;
    if (seen.has(key)) {
      duplicates.add(key);
    } else {
      seen.add(key);
    }
  }
  return duplicates;
}

export function validateDictionaryPair(pair: Replacement): PairValidation {
  const spoken = pair.spoken.trim();
  if (spoken.length === 0) {
    return { valid: false, error: "Spoken phrase cannot be empty." };
  }
  return { valid: true };
}
