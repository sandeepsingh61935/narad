import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import type { AppProfile } from "./config";
import {
  POPULAR_APP_SUGGESTIONS,
  matchAppProfile,
  validateProfile,
} from "./profiles";

const here = dirname(fileURLToPath(import.meta.url));

test("popular app suggestions contains standard apps with correct defaults", () => {
  assert.equal(POPULAR_APP_SUGGESTIONS.length, 5);
  const code = POPULAR_APP_SUGGESTIONS.find((s) => s.app_id === "code");
  assert.ok(code);
  assert.equal(code.name, "VS Code");
  assert.equal(code.mode, "code");
  assert.equal(code.paste_with_shift, false);

  const terminal = POPULAR_APP_SUGGESTIONS.find((s) => s.app_id === "org.gnome.Terminal");
  assert.ok(terminal);
  assert.equal(terminal.name, "Terminal");
  assert.equal(terminal.mode, "plain");
  assert.equal(terminal.paste_with_shift, true);

  const slack = POPULAR_APP_SUGGESTIONS.find((s) => s.app_id === "slack");
  assert.ok(slack);
  assert.equal(slack.name, "Slack");
  assert.equal(slack.mode, "email");
  assert.equal(slack.paste_with_shift, false);

  const obsidian = POPULAR_APP_SUGGESTIONS.find((s) => s.app_id === "obsidian");
  assert.ok(obsidian);
  assert.equal(obsidian.mode, "bullet");

  const discord = POPULAR_APP_SUGGESTIONS.find((s) => s.app_id === "discord");
  assert.ok(discord);
  assert.equal(discord.mode, "email");
});

test("matchAppProfile matches exact and case-insensitive app IDs", () => {
  const profiles: AppProfile[] = [
    { app_id: "code", mode: "code" },
    { app_id: "slack", mode: "email" },
  ];

  assert.equal(matchAppProfile(profiles, "code")?.app_id, "code");
  assert.equal(matchAppProfile(profiles, "CODE")?.app_id, "code");
  assert.equal(matchAppProfile(profiles, "  Slack  ")?.app_id, "slack");
  assert.equal(matchAppProfile(profiles, "firefox"), null);
  assert.equal(matchAppProfile(profiles, ""), null);
  assert.equal(matchAppProfile(profiles, "   "), null);
});

test("matchAppProfile matches reverse and forward domain suffixes", () => {
  const profiles: AppProfile[] = [
    { app_id: "code", mode: "code" },
    { app_id: "org.gnome.Terminal", mode: "plain", paste_with_shift: true },
    { app_id: "slack", mode: "email" },
  ];

  // Target has domain prefix, profile is short name:
  assert.equal(matchAppProfile(profiles, "com.visualstudio.code")?.app_id, "code");
  assert.equal(matchAppProfile(profiles, "com.slack.Slack")?.app_id, "slack");
  assert.equal(matchAppProfile(profiles, "COM.SLACK.SLACK")?.app_id, "slack");
  assert.equal(matchAppProfile(profiles, "  com.slack.Slack  ")?.app_id, "slack");
  assert.equal(matchAppProfile(profiles, "org.mozilla.firefox")?.app_id, undefined);

  // Profile has domain prefix, target is short name:
  assert.equal(matchAppProfile(profiles, "terminal")?.app_id, "org.gnome.Terminal");
  assert.equal(matchAppProfile(profiles, "Terminal")?.app_id, "org.gnome.Terminal");
  assert.equal(matchAppProfile(profiles, "  terminal  ")?.app_id, "org.gnome.Terminal");
});

test("validateProfile checks for required and unique app ID", () => {
  const existing: AppProfile[] = [
    { app_id: "code", mode: "code" },
    { app_id: "slack", mode: "email" },
  ];

  // Empty app_id
  assert.equal(validateProfile({ app_id: "", mode: "code" }, existing).valid, false);
  assert.equal(validateProfile({ app_id: "   ", mode: "code" }, existing).valid, false);

  // Missing or empty mode
  assert.equal(validateProfile({ app_id: "discord", mode: "" }, existing).valid, false);
  assert.equal(validateProfile({ app_id: "discord", mode: "   " }, existing).valid, false);

  // Duplicate app_id
  const dup = validateProfile({ app_id: "code", mode: "code" }, existing);
  assert.equal(dup.valid, false);
  assert.ok(dup.error);

  const dupCase = validateProfile({ app_id: "  CODE  ", mode: "code" }, existing);
  assert.equal(dupCase.valid, false);

  const dupSlack = validateProfile({ app_id: "  Slack  ", mode: "email" }, existing);
  assert.equal(dupSlack.valid, false);

  // Editing existing profile (same app_id as currentAppId) is allowed
  const editSelf = validateProfile({ app_id: "code", mode: "plain" }, existing, "code");
  assert.equal(editSelf.valid, true);

  const editSelfCase = validateProfile({ app_id: "CODE", mode: "plain" }, existing, "code");
  assert.equal(editSelfCase.valid, true);

  // Valid new profile
  const valid = validateProfile({ app_id: "discord", mode: "email" }, existing);
  assert.equal(valid.valid, true);
  assert.equal(valid.error, undefined);
});

test("settings layout links to /settings/profiles", () => {
  const layout = readFileSync(join(here, "../routes/settings/+layout.svelte"), "utf8");
  assert.ok(layout.includes('{ label: "App profiles", icon: "tab", href: "/settings/profiles" }'));
});

test("app profiles page exists, wires auto-save, and follows sentence case copy", () => {
  const page = readFileSync(join(here, "../routes/settings/profiles/+page.svelte"), "utf8");

  // Required copy
  assert.ok(page.includes("App profiles"));
  assert.ok(page.includes("Configure default dictation mode and paste shortcut for specific applications."));
  assert.ok(
    page.includes(
      "No per-app profiles configured yet. Add an application profile below to automatically set the mode when focused."
    )
  );
  assert.ok(page.includes("Add application profile"));
  assert.ok(page.includes("Application ID"));
  assert.ok(page.includes("Default mode"));
  assert.ok(page.includes("Ctrl+Shift+V paste"));
  assert.ok(page.includes("Add profile"));
  assert.ok(page.includes("Popular applications:"));

  // Accessible labels and hit targets
  assert.ok(page.includes("Remove profile for"));
  assert.ok(page.includes("Default mode for"));
  assert.ok(page.includes("min-h-[36px]"));
  assert.ok(page.includes("focus-visible:ring-offset-surface"));
  assert.ok(page.includes('role="status"'));
  assert.ok(page.includes('aria-live="polite"'));

  // Auto-save integration
  assert.ok(page.includes("SAVED_NOTICE"));
  assert.ok(page.includes("saveDashboardConfig"));
  assert.ok(!page.includes("<button>Save</button>"));
  assert.ok(!page.includes("Save changes"));

  // Check forbidden words
  for (const forbidden of ["telemetry", "acoustic", "lexical", "CUDA", "TensorRT", "gsettings"]) {
    assert.equal(page.includes(forbidden), false, `Page contains forbidden word: ${forbidden}`);
  }
});
