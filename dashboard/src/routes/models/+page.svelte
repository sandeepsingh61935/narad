<main class="flex-1 bg-surface w-full"><div class="flex flex-col w-full">
<div class="w-full max-w-6xl mx-auto px-space-md py-space-lg flex flex-col gap-space-lg">
<!-- Header Section -->
<div class="flex flex-col md:flex-row md:items-center justify-between gap-space-md pb-space-xs">
<div class="flex flex-col gap-1">
<div class="flex items-center gap-space-sm flex-wrap">
<h1 class="font-headline-lg text-headline-lg text-on-surface tracking-tight">Models &amp; Engines</h1>
<span class="px-space-xs py-0.5 rounded bg-surface-container-high font-keycap text-keycap text-primary border border-outline-variant">
            Local Neural Weights · Hardware Accelerated (CUDA/Vulkan)
          </span>
</div>
<p class="font-body-muted text-body-muted text-on-surface-variant flex items-center gap-1.5">
<span class="material-symbols-outlined text-[15px] text-primary" style="font-variation-settings: 'FILL' 1;">lock</span>
          Downloads are the only time Narad uses the internet. All inference runs strictly offline.
        </p>
</div>
<div class="flex items-center gap-space-xs self-start md:self-auto">
<span class="font-label-code text-label-code text-on-surface-variant bg-surface-container px-2 py-1 rounded-lg flex items-center gap-1.5">
<span class="w-2 h-2 rounded-full bg-emerald-500"></span>
          ONNX Runtime 1.18 · CUDA 12.2
        </span>
</div>
</div>
<!-- Benchmark Banner AdwCard -->
<div class="bg-surface-container-low rounded-xl p-space-lg flex flex-col gap-space-md">
<div class="flex flex-col lg:flex-row lg:items-center justify-between gap-space-md">
<div class="flex items-start gap-space-md">
<div class="w-10 h-10 rounded-lg bg-surface-container-high flex items-center justify-center shrink-0 text-primary">
<span class="material-symbols-outlined text-[22px]">speed</span>
</div>
<div class="flex flex-col gap-0.5">
<div class="flex items-center gap-2 flex-wrap">
<span class="font-headline-md text-headline-md text-on-surface">Hardware Acoustic Benchmark</span>
<span class="font-label-code text-label-code text-on-surface-variant bg-surface-container px-1.5 py-0.5 rounded">
                AMD Ryzen 9 7940HS · 8 cores · TensorRT-LLM / ONNX Runtime
              </span>
</div>
<p class="font-body-default text-body-default text-on-surface-variant">
              Benchmark your computer — recommends the fastest accurate model for your hardware.
            </p>
</div>
</div>
<div class="flex items-center gap-space-sm shrink-0">
<button class="flex items-center gap-space-sm bg-primary-container hover:bg-primary text-on-primary-container px-space-md py-2 rounded-lg font-headline-md text-headline-md transition-colors shadow-sm" id="runBenchmarkBtn">
<span class="material-symbols-outlined text-[18px]">play_arrow</span>
<span>Run benchmark</span>
<div class="flex items-center gap-0.5 ml-1">
<kbd class="px-1 py-0.2 rounded bg-surface-container-lowest text-on-surface font-keycap text-keycap">Ctrl</kbd>
<kbd class="px-1 py-0.2 rounded bg-surface-container-lowest text-on-surface font-keycap text-keycap">B</kbd>
</div>
</button>
</div>
</div>
<!-- Benchmark Metrics Mini-strip -->
<div class="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-6 gap-space-sm bg-surface-container rounded-lg p-space-md">
<div class="flex flex-col">
<span class="font-body-muted text-body-muted text-on-surface-variant">Recommended</span>
<span class="font-label-code text-label-code text-primary font-medium truncate">Parakeet CTC 0.6B</span>
</div>
<div class="flex flex-col">
<span class="font-body-muted text-body-muted text-on-surface-variant">Latency</span>
<span class="font-label-code text-label-code text-on-surface">0.18s / 5s audio</span>
</div>
<div class="flex flex-col">
<span class="font-body-muted text-body-muted text-on-surface-variant">Real-Time Factor (RTF)</span>
<span class="font-label-code text-label-code text-emerald-400">0.036x</span>
</div>
<div class="flex flex-col">
<span class="font-body-muted text-body-muted text-on-surface-variant">Allocated VRAM</span>
<span class="font-label-code text-label-code text-on-surface">1.1 GB</span>
</div>
<div class="flex flex-col">
<span class="font-body-muted text-body-muted text-on-surface-variant">WER Accuracy</span>
<span class="font-label-code text-label-code text-emerald-400">98.4%</span>
</div>
<div class="flex flex-col">
<span class="font-body-muted text-body-muted text-on-surface-variant">Storage (~/.local)</span>
<span class="font-label-code text-label-code text-on-surface truncate">3.8 GB / 512 GB</span>
</div>
</div>
</div>
<!-- Navigation Tabs / Libadwaita Segmented Buttons -->
<div class="flex flex-col sm:flex-row sm:items-center justify-between gap-space-sm">
<div class="flex items-center bg-surface-container-low p-1 rounded-lg gap-1 select-none overflow-x-auto">
<button class="flex items-center gap-space-xs px-space-md py-1.5 rounded-md bg-surface-container-high text-on-surface font-headline-md text-headline-md transition-colors" type="button">
<span>Speech (ASR)</span>
<span class="px-1.5 py-0.2 rounded-full bg-primary-container text-on-primary-container font-keycap text-keycap">5</span>
</button>
<button class="flex items-center gap-space-xs px-space-md py-1.5 rounded-md text-on-surface-variant hover:text-on-surface hover:bg-surface-container transition-colors font-body-default text-body-default" type="button">
<span>Translation (NMT)</span>
<span class="w-2 h-2 rounded-full bg-surface-bright"></span>
</button>
<button class="flex items-center gap-space-xs px-space-md py-1.5 rounded-md text-on-surface-variant hover:text-on-surface hover:bg-surface-container transition-colors font-body-default text-body-default" type="button">
<span>Cleanup AI (SLM)</span>
</button>
<button class="flex items-center gap-space-xs px-space-md py-1.5 rounded-md text-on-surface-variant hover:text-on-surface hover:bg-surface-container transition-colors font-body-default text-body-default" type="button">
<span>Wake Word (KWS)</span>
</button>
</div>
<div class="flex items-center gap-space-sm font-label-code text-label-code text-on-surface-variant px-space-xs">
<span>Engine:</span>
<span class="text-on-surface bg-surface-container-low px-2 py-1 rounded">FastConformer · NeMo</span>
</div>
</div>
<!-- Model Cards Section -->
<div class="flex flex-col gap-space-md" id="modelCardList">
<!-- Card 1: Active Default -->
<div class="bg-surface-container-low rounded-xl p-space-lg flex flex-col md:flex-row md:items-center justify-between gap-space-md transition-all hover:bg-surface-container">
<div class="flex items-start gap-space-md min-w-0 flex-1">
<div class="mt-1 cursor-pointer">
<div class="w-5 h-5 rounded-full bg-surface-container-highest flex items-center justify-center">
<span class="w-2.5 h-2.5 rounded-full bg-primary-container"></span>
</div>
</div>
<div class="flex flex-col gap-1.5 min-w-0 flex-1">
<div class="flex items-center gap-space-sm flex-wrap">
<span class="font-headline-md text-headline-md text-on-surface">NVIDIA Parakeet CTC 0.6B</span>
<span class="font-label-code text-label-code text-on-surface-variant bg-surface-container px-2 py-0.5 rounded">
                parakeet-ctc-0.6b-int8
              </span>
<span class="px-2 py-0.5 rounded bg-primary-container/20 text-primary font-keycap text-keycap font-semibold flex items-center gap-1">
<span class="w-1.5 h-1.5 rounded-full bg-primary-container"></span> ACTIVE DEFAULT
              </span>
</div>
<p class="font-body-muted text-body-muted text-on-surface-variant">
              State-of-the-art conversational English · 98.6% WER rating
            </p>
<div class="flex items-center gap-space-md flex-wrap pt-1 font-label-code text-label-code text-on-surface-variant">
<div class="flex items-center gap-1">
<span class="text-on-surface font-medium">620 MB</span> disk
              </div>
<span class="text-outline-variant">·</span>
<div class="flex items-center gap-1 text-primary">
<span class="material-symbols-outlined text-[15px]">bolt</span>
<span>0.19s for 5s audio (RTF 0.038x)</span>
</div>
<span class="text-outline-variant">·</span>
<span>TensorRT-LLM / CUDA Int8</span>
</div>
<div class="flex items-center gap-1.5 pt-1">
<span class="px-1.5 py-0.5 rounded bg-surface-container font-keycap text-keycap text-on-surface">en-US</span>
<span class="px-1.5 py-0.5 rounded bg-surface-container font-keycap text-keycap text-on-surface">en-GB</span>
<span class="px-1.5 py-0.5 rounded bg-surface-container font-keycap text-keycap text-on-surface">en-IN</span>
</div>
</div>
</div>
<div class="flex items-center gap-space-sm shrink-0 self-end md:self-center">
<button class="px-space-md py-1.5 rounded-lg bg-surface-container hover:bg-surface-container-high text-on-surface font-body-default text-body-default flex items-center gap-1.5 transition-colors">
<span class="material-symbols-outlined text-[16px]">rotate_right</span>
<span>Re-test</span>
</button>
<button class="w-9 h-9 rounded-lg bg-surface-container hover:bg-surface-container-high text-on-surface flex items-center justify-center transition-colors" title="Engine tuning parameters">
<span class="material-symbols-outlined text-[18px]">tune</span>
</button>
</div>
</div>
<!-- Card 2: Installed Whisper.cpp -->
<div class="bg-surface-container-low rounded-xl p-space-lg flex flex-col md:flex-row md:items-center justify-between gap-space-md transition-all hover:bg-surface-container">
<div class="flex items-start gap-space-md min-w-0 flex-1">
<div class="mt-1 cursor-pointer">
<div class="w-5 h-5 rounded-full bg-surface-container-highest flex items-center justify-center hover:bg-surface-bright transition-colors">
<span class="w-2 h-2 rounded-full bg-transparent"></span>
</div>
</div>
<div class="flex flex-col gap-1.5 min-w-0 flex-1">
<div class="flex items-center gap-space-sm flex-wrap">
<span class="font-headline-md text-headline-md text-on-surface">Whisper.cpp Small (Distil-Whisper)</span>
<span class="font-label-code text-label-code text-on-surface-variant bg-surface-container px-2 py-0.5 rounded">
                distil-whisper-small.en-q5_1
              </span>
<span class="px-2 py-0.5 rounded bg-surface-container font-keycap text-keycap text-on-surface-variant">
                INSTALLED
              </span>
</div>
<p class="font-body-muted text-body-muted text-on-surface-variant">
              High punctuation fidelity · Whisper architecture transformer engine
            </p>
<div class="flex items-center gap-space-md flex-wrap pt-1 font-label-code text-label-code text-on-surface-variant">
<span><strong class="text-on-surface font-medium">480 MB</strong> disk</span>
<span class="text-outline-variant">·</span>
<span class="text-on-surface">0.34s for 5s audio</span>
<span class="text-outline-variant">·</span>
<span>GGML / Q5_1 Quantized</span>
</div>
<div class="flex items-center gap-1.5 pt-1">
<span class="px-1.5 py-0.5 rounded bg-surface-container font-keycap text-keycap text-on-surface">en-US</span>
</div>
</div>
</div>
<div class="flex items-center gap-space-sm shrink-0 self-end md:self-center">
<button class="px-space-md py-1.5 rounded-lg bg-surface-container hover:bg-surface-container-high text-on-surface font-body-default text-body-default transition-colors">
            Activate
          </button>
<button class="w-9 h-9 rounded-lg bg-surface-container hover:bg-error-container hover:text-on-error-container text-on-surface-variant flex items-center justify-center transition-colors" title="Delete weights from disk">
<span class="material-symbols-outlined text-[18px]">delete</span>
</button>
</div>
</div>
<!-- Card 3: IndicWav2Vec Bilingual -->
<div class="bg-surface-container-low rounded-xl p-space-lg flex flex-col md:flex-row md:items-center justify-between gap-space-md transition-all hover:bg-surface-container">
<div class="flex items-start gap-space-md min-w-0 flex-1">
<div class="mt-1 cursor-pointer">
<div class="w-5 h-5 rounded-full bg-surface-container-highest flex items-center justify-center hover:bg-surface-bright transition-colors">
<span class="w-2 h-2 rounded-full bg-transparent"></span>
</div>
</div>
<div class="flex flex-col gap-1.5 min-w-0 flex-1">
<div class="flex items-center gap-space-sm flex-wrap">
<span class="font-headline-md text-headline-md text-on-surface">IndicWav2Vec Bilingual</span>
<span class="font-label-code text-label-code text-on-surface-variant bg-surface-container px-2 py-0.5 rounded">
                indicwav2vec-v1-fp16
              </span>
<span class="px-2 py-0.5 rounded bg-surface-container font-keycap text-keycap text-on-surface-variant">
                INSTALLED
              </span>
</div>
<p class="font-body-muted text-body-muted text-on-surface-variant">
              Optimized for Hindi code-switching &amp; Indian English accent patterns
            </p>
<div class="flex items-center gap-space-md flex-wrap pt-1 font-label-code text-label-code text-on-surface-variant">
<span><strong class="text-on-surface font-medium">1.2 GB</strong> disk</span>
<span class="text-outline-variant">·</span>
<span class="text-on-surface">0.42s for 5s audio</span>
<span class="text-outline-variant">·</span>
<span>PyTorch C++ LibTorch / FP16</span>
</div>
<div class="flex items-center gap-1.5 pt-1">
<span class="px-1.5 py-0.5 rounded bg-surface-container font-keycap text-keycap text-on-surface">hi-IN</span>
<span class="px-1.5 py-0.5 rounded bg-surface-container font-keycap text-keycap text-on-surface">en-IN</span>
<span class="px-1.5 py-0.5 rounded bg-surface-container font-keycap text-keycap text-on-surface">mr-IN</span>
</div>
</div>
</div>
<div class="flex items-center gap-space-sm shrink-0 self-end md:self-center">
<button class="px-space-md py-1.5 rounded-lg bg-surface-container hover:bg-surface-container-high text-on-surface font-body-default text-body-default transition-colors">
            Activate
          </button>
<button class="w-9 h-9 rounded-lg bg-surface-container hover:bg-error-container hover:text-on-error-container text-on-surface-variant flex items-center justify-center transition-colors" title="Delete weights">
<span class="material-symbols-outlined text-[18px]">delete</span>
</button>
</div>
</div>
<!-- Card 4: Downloading in Progress Model -->
<div class="bg-surface-container-low rounded-xl p-space-lg flex flex-col gap-space-md relative overflow-hidden">
<div class="flex flex-col md:flex-row md:items-center justify-between gap-space-md">
<div class="flex items-start gap-space-md min-w-0 flex-1">
<div class="mt-1">
<span class="material-symbols-outlined text-primary text-[20px] animate-spin">sync</span>
</div>
<div class="flex flex-col gap-1.5 min-w-0 flex-1">
<div class="flex items-center gap-space-sm flex-wrap">
<span class="font-headline-md text-headline-md text-on-surface">Whisper Medium (OpenAI v3)</span>
<span class="font-label-code text-label-code text-on-surface-variant bg-surface-container px-2 py-0.5 rounded">
                  whisper-medium-v3-q8_0
                </span>
<span class="px-2 py-0.5 rounded bg-primary-container/20 text-primary font-keycap text-keycap">
                  DOWNLOADING 56%
                </span>
</div>
<p class="font-body-muted text-body-muted text-on-surface-variant">
                Highest accuracy for noisy audio environments and 99+ international languages
              </p>
<div class="flex items-center gap-space-md flex-wrap pt-1 font-label-code text-label-code text-on-surface-variant">
<span class="text-primary font-medium">840 MB / 1.5 GB</span>
<span class="text-outline-variant">·</span>
<span>14.2 MB/s</span>
<span class="text-outline-variant">·</span>
<span>~45s remaining</span>
</div>
<div class="flex items-center gap-1.5 pt-1">
<span class="px-1.5 py-0.5 rounded bg-surface-container font-keycap text-keycap text-on-surface">99 Languages · Multilingual</span>
</div>
</div>
</div>
<div class="flex items-center gap-space-sm shrink-0 self-end md:self-center">
<button class="px-space-md py-1.5 rounded-lg bg-surface-container hover:bg-surface-container-high text-on-surface font-body-default text-body-default flex items-center gap-1.5 transition-colors">
<span class="material-symbols-outlined text-[16px]">pause</span>
<span>Pause</span>
</button>
<button class="w-9 h-9 rounded-lg bg-surface-container hover:bg-error-container hover:text-on-error-container text-on-surface flex items-center justify-center transition-colors" title="Cancel download">
<span class="material-symbols-outlined text-[18px]">close</span>
</button>
</div>
</div>
<!-- 4px Clean Saffron Progress Bar -->
<div class="w-full bg-surface-container rounded-full h-1 overflow-hidden mt-1">
<div class="bg-primary-container h-full rounded-full transition-all duration-300" style="width: 56%;"></div>
</div>
</div>
<!-- Card 5: Available for Download -->
<div class="bg-surface-container-low rounded-xl p-space-lg flex flex-col md:flex-row md:items-center justify-between gap-space-md transition-all hover:bg-surface-container">
<div class="flex items-start gap-space-md min-w-0 flex-1">
<div class="mt-1">
<span class="material-symbols-outlined text-on-surface-variant text-[20px]">cloud</span>
</div>
<div class="flex flex-col gap-1.5 min-w-0 flex-1">
<div class="flex items-center gap-space-sm flex-wrap">
<span class="font-headline-md text-headline-md text-on-surface">Conformer-CTC Large Hindi</span>
<span class="font-label-code text-label-code text-on-surface-variant bg-surface-container px-2 py-0.5 rounded">
                conformer-ctc-hi-v2
              </span>
<span class="px-2 py-0.5 rounded bg-surface-container font-keycap text-keycap text-on-surface-variant">
                CATALOG
              </span>
</div>
<p class="font-body-muted text-body-muted text-on-surface-variant">
              Dedicated pure Hindi conversational model with natural colloquial phrasing
            </p>
<div class="flex items-center gap-space-md flex-wrap pt-1 font-label-code text-label-code text-on-surface-variant">
<span><strong class="text-on-surface font-medium">890 MB</strong> archive</span>
<span class="text-outline-variant">·</span>
<span>Est. ~0.28s for 5s audio</span>
<span class="text-outline-variant">·</span>
<span>ONNX INT8 Optimized</span>
</div>
<div class="flex items-center gap-1.5 pt-1">
<span class="px-1.5 py-0.5 rounded bg-surface-container font-keycap text-keycap text-on-surface">hi-IN</span>
</div>
</div>
</div>
<div class="flex items-center gap-space-sm shrink-0 self-end md:self-center">
<button class="px-space-md py-1.5 rounded-lg bg-surface-container hover:bg-primary-container hover:text-on-primary-container text-on-surface font-body-default text-body-default flex items-center gap-1.5 transition-colors">
<span class="material-symbols-outlined text-[16px]">download</span>
<span>Download (890 MB)</span>
</button>
</div>
</div>
</div>
<!-- Sub-panel / Translation Preview Drawer -->
<div class="bg-surface-container-low rounded-xl p-space-lg flex flex-col gap-space-md">
<div class="flex items-center justify-between">
<div class="flex items-center gap-space-sm">
<span class="material-symbols-outlined text-[20px] text-primary">translate</span>
<span class="font-headline-md text-headline-md text-on-surface">Configured Neural Translation Pairs (NMT)</span>
</div>
<span class="font-keycap text-keycap text-on-surface-variant bg-surface-container px-2 py-1 rounded">2 Pairs Ready</span>
</div>
<div class="grid grid-cols-1 md:grid-cols-2 gap-space-sm">
<div class="bg-surface-container p-space-md rounded-lg flex items-center justify-between">
<div class="flex flex-col gap-0.5">
<div class="flex items-center gap-2">
<span class="font-headline-md text-headline-md text-on-surface">English (en) → Hindi (hi)</span>
<span class="w-1.5 h-1.5 rounded-full bg-emerald-500"></span>
</div>
<span class="font-label-code text-label-code text-on-surface-variant">nllb-200-distilled-600m · Active</span>
</div>
<span class="font-keycap text-keycap text-on-surface-variant bg-surface-container-high px-2 py-1 rounded">600M params</span>
</div>
<div class="bg-surface-container p-space-md rounded-lg flex items-center justify-between">
<div class="flex flex-col gap-0.5">
<div class="flex items-center gap-2">
<span class="font-headline-md text-headline-md text-on-surface">Hindi (hi) → English (en)</span>
<span class="w-1.5 h-1.5 rounded-full bg-emerald-500"></span>
</div>
<span class="font-label-code text-label-code text-on-surface-variant">indictrans2-en-hi · Active</span>
</div>
<span class="font-keycap text-keycap text-on-surface-variant bg-surface-container-high px-2 py-1 rounded">420M params</span>
</div>
</div>
<!-- Trust & Integrity Callout -->
<div class="flex items-center gap-space-sm bg-surface-container-high/40 p-space-sm rounded-lg text-on-surface-variant">
<span class="material-symbols-outlined text-[18px] text-primary shrink-0">verified_user</span>
<span class="font-body-muted text-body-muted">
          All model weights are verified with cryptographic SHA-256 signatures before being written to disk. Downloads are the only time Narad uses the internet.
        </span>
</div>
</div>
</div>
</div></main>
