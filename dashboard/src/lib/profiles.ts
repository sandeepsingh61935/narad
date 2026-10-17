import type { AppProfile } from "./config";

export const POPULAR_APP_SUGGESTIONS = [
  { app_id: "code", name: "VS Code", mode: "code", paste_with_shift: false },
  { app_id: "slack", name: "Slack", mode: "email", paste_with_shift: false },
  { app_id: "org.gnome.Terminal", name: "Terminal", mode: "plain", paste_with_shift: true },
  { app_id: "obsidian", name: "Obsidian", mode: "bullet", paste_with_shift: false },
  { app_id: "discord", name: "Discord", mode: "email", paste_with_shift: false },
] as const;

export type PopularAppSuggestion = (typeof POPULAR_APP_SUGGESTIONS)[number];

export type ProfileValidation = {
  valid: boolean;
  error?: string;
};

/**
 * Match a profile by application identifier.
 * Matches exact case-insensitive, or reverse/domain suffix (e.g. com.slack.Slack matches slack).
 */
export function matchAppProfile(
  profiles: readonly AppProfile[],
  focusedApp: string,
): AppProfile | null {
  const normTarget = focusedApp.trim().toLowerCase();
  if (!normTarget) return null;
  const targetSuffix = normTarget.split(".").pop();

  for (const profile of profiles) {
    const normProf = profile.app_id.trim().toLowerCase();
    if (!normProf) continue;
    const profSuffix = normProf.split(".").pop();

    if (
      normProf === normTarget ||
      (targetSuffix && targetSuffix === normProf) ||
      (profSuffix && profSuffix === normTarget)
    ) {
      return profile;
    }
  }
  return null;
}

/**
 * Validates an app profile draft against required fields and uniqueness.
 */
export function validateProfile(
  profile: Partial<AppProfile>,
  existingProfiles: readonly AppProfile[],
  currentAppId?: string,
): ProfileValidation {
  const appId = profile.app_id?.trim() ?? "";
  if (appId.length === 0) {
    return { valid: false, error: "Application ID cannot be empty." };
  }

  const mode = profile.mode?.trim() ?? "";
  if (mode.length === 0) {
    return { valid: false, error: "Mode cannot be empty." };
  }

  const normAppId = appId.toLowerCase();
  const currentNorm = currentAppId?.trim().toLowerCase();

  const isDuplicate = existingProfiles.some((p) => {
    const existingNorm = p.app_id.trim().toLowerCase();
    if (currentNorm && existingNorm === currentNorm) {
      return false;
    }
    return existingNorm === normAppId;
  });

  if (isDuplicate) {
    return {
      valid: false,
      error: `A profile for "${appId}" already exists.`,
    };
  }

  return { valid: true };
}
