#!/usr/bin/env bash
# Download and extract Phase 0 ASR model packs into spikes/asr-bench/models/.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MODELS_DIR="${MODELS_DIR:-$ROOT/models}"
BASE_URL="https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models"

mkdir -p "$MODELS_DIR"

download_one() {
  local archive="$1"
  local dest="$MODELS_DIR/$archive"
  if [[ -f "$dest" ]]; then
    echo "already have archive $dest"
  else
    echo "downloading $archive ..."
    curl -fL --retry 3 -o "$dest" "$BASE_URL/$archive"
  fi
  local extract_marker="${dest}.extracted"
  if [[ -f "$extract_marker" ]]; then
    echo "already extracted $archive"
    return 0
  fi
  echo "extracting $archive -> $MODELS_DIR"
  tar xf "$dest" -C "$MODELS_DIR"
  touch "$extract_marker"
}

# Args: model ids from models.toml, or "all"
ids=("$@")
if [[ ${#ids[@]} -eq 0 ]] || [[ "${ids[0]}" == "all" ]]; then
  ids=(
    parakeet-tdt-0.6b-v2-int8
    parakeet-ctc-110m-int8
    moonshine-base-en-int8
    whisper-small-en
  )
fi

declare -A ARCHIVES=(
  [parakeet-tdt-0.6b-v2-int8]="sherpa-onnx-nemo-parakeet-tdt-0.6b-v2-int8.tar.bz2"
  [parakeet-ctc-110m-int8]="sherpa-onnx-nemo-parakeet_tdt_ctc_110m-en-36000-int8.tar.bz2"
  [moonshine-base-en-int8]="sherpa-onnx-moonshine-base-en-int8.tar.bz2"
  [whisper-small-en]="sherpa-onnx-whisper-small.en.tar.bz2"
)

for id in "${ids[@]}"; do
  archive="${ARCHIVES[$id]:-}"
  if [[ -z "$archive" ]]; then
    echo "unknown model id: $id" >&2
    exit 1
  fi
  download_one "$archive"
done

echo "Done. Model roots under: $MODELS_DIR"
