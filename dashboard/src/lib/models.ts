export type ModelType = "asr" | "mt" | "llm" | "kws";

export type ModelPack = {
  id: string;
  name: string;
  description: string;
  type: ModelType;
  languages: string[];
  sizeMb: number;
  sha256: string;
  isDefault?: boolean;
};

export const DEFAULT_SPEECH_MODEL_ID = "parakeet-ctc-110m-int8";

export const MODEL_CATALOG: ModelPack[] = [
  {
    id: "parakeet-ctc-110m-int8",
    name: "Parakeet CTC 110M int8",
    description: "Fast, low-latency English speech recognition. Lowest CPU usage.",
    type: "asr",
    languages: ["en"],
    sizeMb: 126,
    sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    isDefault: true,
  },
  {
    id: "parakeet-tdt-0.6b-int8",
    name: "Parakeet TDT 0.6B int8",
    description: "High-accuracy English speech recognition with transducer decoding.",
    type: "asr",
    languages: ["en"],
    sizeMb: 640,
    sha256: "d544062145da5776dcfd7589d38c642eef72535084920fcad4e39ec0ef6e36d4",
  },
  {
    id: "moonshine-base-en-int8",
    name: "Moonshine Base en int8",
    description: "Encoder-decoder speech model optimized for variable-length speech.",
    type: "asr",
    languages: ["en"],
    sizeMb: 180,
    sha256: "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9",
  },
  {
    id: "whisper-small-en",
    name: "Whisper Small en",
    description: "Robust OpenAI Whisper architecture for English transcription.",
    type: "asr",
    languages: ["en"],
    sizeMb: 480,
    sha256: "722f4625be25633a65096ab6cfd3fc6b13b41bf3735165971a8fb18037ca0f80",
  },
];

export function findModelById(id: string): ModelPack | undefined {
  return MODEL_CATALOG.find((m) => m.id === id);
}

export function isModelActive(
  modelId: string,
  config: { models?: { asr_english?: string } } | null | undefined
): boolean {
  if (!modelId || !config?.models?.asr_english) return false;
  return modelId === config.models.asr_english;
}
