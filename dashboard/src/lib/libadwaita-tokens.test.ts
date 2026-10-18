import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const css = readFileSync(join(here, "../app.css"), "utf8");

test("libadwaita dark tokens use oklch and hex fallbacks", () => {
  // Required token pairs: hex fallback and oklch definition
  const requiredTokens = [
    { name: "--desktop", hex: "#1e1d1c", oklch: "oklch(15.5% 0.012 65)" },
    { name: "--bg", hex: "#252423", oklch: "oklch(21% 0.01 72)" },
    { name: "--surface", hex: "#2d2c2b", oklch: "oklch(24.5% 0.01 72)" },
    { name: "--raised", hex: "#383635", oklch: "oklch(28.8% 0.01 72)" },
    { name: "--fg", hex: "#f0eeeb", oklch: "oklch(94% 0.014 85)" },
    { name: "--muted", hex: "#b4b1ad", oklch: "oklch(72% 0.016 78)" },
    { name: "--border", hex: "#4c4a48", oklch: "oklch(36% 0.01 72)" },
    { name: "--accent", hex: "#3584e4", oklch: "oklch(70% 0.125 250)" },
    { name: "--ok", hex: "#57ab5a", oklch: "oklch(72% 0.1 155)" },
    { name: "--warn", hex: "#e3b341", oklch: "oklch(80% 0.1 85)" },
    { name: "--danger", hex: "#e01b24", oklch: "oklch(66% 0.15 28)" },
  ];

  for (const token of requiredTokens) {
    assert.ok(css.includes(token.hex), `missing fallback hex ${token.hex} for ${token.name}`);
    assert.ok(css.includes(token.oklch), `missing oklch value ${token.oklch} for ${token.name}`);
  }

  assert.ok(css.includes("--radius: 8px;"), "missing --radius: 8px");
  assert.ok(css.includes("--window-radius: 12px;"), "missing --window-radius: 12px");
});

test("ibm plex fonts are configured with @font-face and font family variables", () => {
  // Font-face blocks with font-display: swap and /fonts/ URLs
  assert.ok(css.includes('font-family: "IBM Plex Sans"'), "missing IBM Plex Sans font-face");
  assert.ok(css.includes('font-family: "IBM Plex Mono"'), "missing IBM Plex Mono font-face");
  assert.ok(css.includes("font-display: swap;"), "missing font-display: swap");

  // Font family variables
  assert.ok(css.includes('--font-body: "IBM Plex Sans"'), "missing --font-body definition");
  assert.ok(css.includes('--font-mono: "IBM Plex Mono"'), "missing --font-mono definition");

  // Verify offline woff2 font files exist on disk
  const fontsDir = join(here, "../../static/fonts");
  const expectedFonts = [
    "ibm-plex-sans-400.woff2",
    "ibm-plex-sans-500.woff2",
    "ibm-plex-sans-600.woff2",
    "ibm-plex-mono-400.woff2",
    "ibm-plex-mono-500.woff2",
  ];

  for (const fontFile of expectedFonts) {
    const fullPath = join(fontsDir, fontFile);
    assert.ok(existsSync(fullPath), `missing font asset: ${fontFile}`);
  }
});
