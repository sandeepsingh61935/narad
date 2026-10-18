import { DEFAULT_SPEECH_MODEL_ID } from './models';

export type ModelInstallStatus = 'available' | 'downloading' | 'verifying' | 'installed' | 'error';

/**
 * Determine if a model can be deleted from disk.
 * Rules (DESIGN §6):
 * 1. Default model ("parakeet-ctc-110m-int8") can NEVER be deleted.
 * 2. Currently active model can NEVER be deleted.
 * 3. Only installed, non-default, non-active models can be deleted.
 */
export function canDeleteModel(modelId: string, isDefault: boolean, isActive: boolean): boolean {
  if (!modelId) return false;
  if (isDefault || modelId === DEFAULT_SPEECH_MODEL_ID) return false;
  if (isActive) return false;
  return true;
}

const HEX_64_REGEX = /^[0-9a-f]{64}$/;

/**
 * Compare actual hash against expected pinned SHA-256 checksum (NFR-S-5).
 */
export function verifySha256(expectedHash: string, actualHash: string): boolean {
  if (!expectedHash || !actualHash) return false;
  const exp = expectedHash.trim().toLowerCase();
  const act = actualHash.trim().toLowerCase();
  if (!HEX_64_REGEX.test(exp) || !HEX_64_REGEX.test(act)) return false;
  return exp === act;
}

/**
 * Format model disk or download size into sentence-case string.
 * Example: 126 -> "126 MB"
 */
export function formatModelSize(sizeMb: number): string {
  if (!Number.isFinite(sizeMb) || sizeMb <= 0) {
    return "";
  }
  return `${Math.round(sizeMb)} MB`;
}
