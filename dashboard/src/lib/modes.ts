export interface ModeDefinition {
  id: string;
  name: string;
  description: string;
  cleanup: boolean;
  prompt: string;
  hotkey: string;
  spoken_prefix: string;
  is_builtin: boolean;
}

export const BUILTIN_MODES: ModeDefinition[] = [
  {
    id: "plain",
    name: "Plain",
    description: "Fastest dictation without AI cleanup. Voice commands and dictionary only.",
    cleanup: false,
    prompt: "",
    hotkey: "Ctrl+Alt+1",
    spoken_prefix: "plain:",
    is_builtin: true,
  },
  {
    id: "clean",
    name: "Clean",
    description: "Remove filler words and false starts while preserving the speaker's original wording.",
    cleanup: true,
    prompt: "Remove filler words, repetitions, and hesitations. Fix basic punctuation and capitalization without changing wording.",
    hotkey: "Ctrl+Alt+2",
    spoken_prefix: "clean:",
    is_builtin: true,
  },
  {
    id: "email",
    name: "Email",
    description: "Format as a clear, polite email with greetings and structured paragraphs.",
    cleanup: true,
    prompt: "Format as a clear, polite email with appropriate paragraphing and polite tone.",
    hotkey: "Ctrl+Alt+3",
    spoken_prefix: "email:",
    is_builtin: true,
  },
  {
    id: "bullet",
    name: "Bullet",
    description: "Structure dictation into clear, concise bullet points.",
    cleanup: true,
    prompt: "Format as concise bullet points.",
    hotkey: "Ctrl+Alt+4",
    spoken_prefix: "bullet list:",
    is_builtin: true,
  },
  {
    id: "code",
    name: "Code",
    description: "Code dictation formatting rules without automatic capitalization or punctuation.",
    cleanup: false,
    prompt: "Format as programming identifiers using code dictation rules.",
    hotkey: "Ctrl+Alt+5",
    spoken_prefix: "code:",
    is_builtin: true,
  },
];

/**
 * Duplicate a mode definition into a user-defined preset.
 */
export function duplicateMode(source: ModeDefinition, newId: string, newName: string): ModeDefinition {
  return {
    ...source,
    id: newId.trim().toLowerCase(),
    name: newName.trim(),
    is_builtin: false,
  };
}

/**
 * Validate mode definition fields.
 */
export function validateMode(mode: ModeDefinition): { valid: boolean; error?: string } {
  if (!mode.id || !mode.id.trim()) {
    return { valid: false, error: "Mode ID cannot be empty." };
  }
  const slug = mode.id.trim();
  if (!/^[a-z0-9_-]+$/.test(slug)) {
    return { valid: false, error: "Mode ID must contain only lowercase letters, numbers, hyphens, and underscores." };
  }
  if (!mode.name || !mode.name.trim()) {
    return { valid: false, error: "Mode name cannot be empty." };
  }
  return { valid: true };
}

/**
 * Deterministically transform a sample phrase based on the mode configuration.
 */
export function previewSampleTransform(rawSample: string, mode: ModeDefinition): { before: string; after: string } {
  const before = rawSample.trim();
  if (!mode.cleanup) {
    return { before, after: before };
  }
  if (mode.id === "clean") {
    // Remove typical filler words
    const cleaned = before
      .replace(/\b(um|uh|you know|basically)\b\s*/gi, "")
      .replace(/\s+/g, " ")
      .trim();
    return { before, after: cleaned };
  }
  if (mode.id === "bullet") {
    const lines = before.split(/\.\s+/).filter(Boolean);
    const bullets = lines.map(line => `• ${line.replace(/\.$/, "")}`).join("\n");
    return { before, after: bullets };
  }
  if (mode.id === "email") {
    return {
      before,
      after: `Hi,\n\n${before}\n\nBest,\nSender`,
    };
  }
  return { before, after: before };
}
