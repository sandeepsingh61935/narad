<main class="flex-1 bg-surface w-full"><div class="flex flex-col w-full">
<!-- Main View Container: GNOME AdwPreferencesPage/AdwClamp styled container -->
<div class="px-space-xl py-space-lg flex flex-col gap-space-lg max-w-7xl mx-auto w-full">
<!-- View Header & Time Filter Bar -->
<div class="flex flex-col md:flex-row md:items-end justify-between gap-space-md pb-space-sm">
<div class="flex flex-col gap-0.5">
<div class="flex items-center gap-space-xs text-on-surface-variant font-label-code text-label-code">
<span class="material-symbols-outlined text-[15px] text-primary">analytics</span>
<span>Telemetry • Session Local SQLite</span>
</div>
<h1 class="font-headline-lg text-headline-lg text-on-surface tracking-tight">Performance &amp; Statistics</h1>
<p class="font-body-muted text-body-muted text-on-surface-variant">All telemetry computed strictly on-device without network telemetry or telemetry relays.</p>
</div>
<div class="flex items-center gap-space-sm self-start md:self-auto">
<!-- Segmented Pill Selector (AdwSegmentedButton style) -->
<div class="flex items-center bg-surface-container-low p-0.5 rounded-lg">
<button class="px-space-md py-1 rounded-md font-body-muted text-body-muted text-on-surface-variant hover:text-on-surface transition-colors" type="button">Day</button>
<button class="px-space-md py-1 rounded-md font-body-muted text-body-muted bg-surface-container-high text-on-surface font-medium transition-all" type="button">Week</button>
<button class="px-space-md py-1 rounded-md font-body-muted text-body-muted text-on-surface-variant hover:text-on-surface transition-colors" type="button">Month</button>
<button class="px-space-md py-1 rounded-md font-body-muted text-body-muted text-on-surface-variant hover:text-on-surface transition-colors" type="button">All</button>
</div>
<!-- Export Telemetry Action Button -->
<button class="flex items-center gap-1.5 px-space-md py-1.5 rounded-lg bg-surface-container-high hover:bg-surface-bright text-on-surface font-body-default text-body-default transition-colors" type="button">
<span class="material-symbols-outlined text-[16px]">file_download</span>
<span>Export JSON</span>
</button>
</div>
</div>
<!-- Quiet Consistency/Streak Banner -->
<div class="flex flex-wrap items-center justify-between gap-space-sm px-space-md py-2.5 bg-surface-container-low rounded-lg">
<div class="flex items-center gap-space-sm">
<div class="w-6 h-6 rounded flex items-center justify-center bg-primary-container/20 text-primary-container">
<span class="material-symbols-outlined text-[16px]">local_fire_department</span>
</div>
<span class="font-headline-md text-headline-md text-on-surface">12 days in a row</span>
<span class="text-on-surface-variant">•</span>
<span class="font-body-muted text-body-muted text-on-surface-variant">Current week: 7 / 7 active days</span>
</div>
<div class="flex items-center gap-space-md font-label-code text-label-code text-on-surface-variant">
<span>Longest recorded: <span class="text-on-surface font-semibold">28 days</span></span>
<span class="text-outline-variant">/</span>
<span>Acoustic engine state: <span class="text-emerald-400 font-semibold">Nominal</span></span>
</div>
</div>
<!-- Metric Cards (4 Adw grouped cards) -->
<div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-space-md">
<!-- Card 1: Words Dictated -->
<div class="bg-surface-container p-space-md rounded-xl flex flex-col justify-between min-h-[128px]">
<div class="flex items-start justify-between">
<span class="font-body-muted text-body-muted text-on-surface-variant">Words Dictated</span>
<span class="font-label-code text-label-code text-on-surface-variant">tokens: ~62.8k</span>
</div>
<div class="my-space-xs">
<div class="font-headline-lg text-headline-lg text-on-surface text-[26px] leading-8 font-semibold tracking-tight">48,210</div>
</div>
<div class="flex items-center gap-1 font-label-code text-label-code text-primary">
<span class="material-symbols-outlined text-[14px]">arrow_upward</span>
<span>+3,420 vs last week</span>
</div>
</div>
<!-- Card 2: Avg Speaking Speed -->
<div class="bg-surface-container p-space-md rounded-xl flex flex-col justify-between min-h-[128px]">
<div class="flex items-start justify-between">
<span class="font-body-muted text-body-muted text-on-surface-variant">Avg Speaking Speed</span>
<span class="material-symbols-outlined text-on-surface-variant text-[16px]">speed</span>
</div>
<div class="my-space-xs">
<div class="font-headline-lg text-headline-lg text-on-surface text-[26px] leading-8 font-semibold tracking-tight">142 <span class="text-body-muted font-normal text-on-surface-variant">WPM</span></div>
</div>
<div class="font-body-muted text-body-muted text-on-surface-variant truncate">
          Acoustic baseline: 130–160 WPM
        </div>
</div>
<!-- Card 3: Time Saved vs Typing -->
<div class="bg-surface-container p-space-md rounded-xl flex flex-col justify-between min-h-[128px]">
<div class="flex items-start justify-between">
<span class="font-body-muted text-body-muted text-on-surface-variant">Time Saved vs Typing</span>
<span class="material-symbols-outlined text-on-surface-variant text-[16px]">schedule</span>
</div>
<div class="my-space-xs">
<div class="font-headline-lg text-headline-lg text-on-surface text-[26px] leading-8 font-semibold tracking-tight">11h 20m</div>
</div>
<div class="font-body-muted text-body-muted text-on-surface-variant truncate">
          Benchmarked at 40 WPM manual input
        </div>
</div>
<!-- Card 4: Total Transcriptions -->
<div class="bg-surface-container p-space-md rounded-xl flex flex-col justify-between min-h-[128px]">
<div class="flex items-start justify-between">
<span class="font-body-muted text-body-muted text-on-surface-variant">Total Transcriptions</span>
<span class="material-symbols-outlined text-on-surface-variant text-[16px]">check_circle</span>
</div>
<div class="my-space-xs">
<div class="font-headline-lg text-headline-lg text-on-surface text-[26px] leading-8 font-semibold tracking-tight">1,936</div>
</div>
<div class="font-label-code text-label-code text-emerald-400">
          99.8% local inference success
        </div>
</div>
</div>
<!-- Center/Bottom Grid: Visual Analytics & System Breakdown -->
<div class="grid grid-cols-1 lg:grid-cols-12 gap-space-lg">
<!-- Left / Main section: Words per Day Bar Chart (7 cols) -->
<div class="lg:col-span-7 bg-surface-container p-space-lg rounded-xl flex flex-col justify-between gap-space-md">
<div class="flex items-center justify-between pb-space-xs">
<div class="flex flex-col">
<h2 class="font-headline-md text-headline-md text-on-surface">Daily Volume Distribution</h2>
<span class="font-body-muted text-body-muted text-on-surface-variant">Captured volume past 14 days (local SQLite journal)</span>
</div>
<!-- Today Callout Badge -->
<div class="flex items-center gap-1.5 px-space-sm py-1 bg-surface-container-high rounded text-on-surface font-label-code text-label-code">
<span class="w-1.5 h-1.5 rounded-full bg-primary-container"></span>
<span>Today: 3,420 words (42 captures)</span>
</div>
</div>
<!-- Inline SVG Histogram Chart without chart-junk -->
<div class="w-full h-56 flex flex-col justify-end pt-2">
<svg class="w-full h-44 overflow-visible" preserveAspectRatio="none" viewBox="0 0 560 170">
<!-- Subtle baseline & ceiling guides -->
<line stroke="#363433" stroke-dasharray="3 3" stroke-width="1" x1="0" x2="560" y1="16" y2="16"></line>
<line stroke="#363433" stroke-dasharray="3 3" stroke-width="1" x1="0" x2="560" y1="85" y2="85"></line>
<line stroke="#363433" stroke-width="1" x1="0" x2="560" y1="154" y2="154"></line>
<!-- Guide Labels -->
<text fill="#a18d7c" font-family="JetBrains Mono" font-size="10" text-anchor="end" x="560" y="14">4k</text>
<text fill="#a18d7c" font-family="JetBrains Mono" font-size="10" text-anchor="end" x="560" y="83">2k</text>
<!-- 14 Day Bars (Mon through Sun x 2) -->
<!-- 1: Oct 11 -->
<rect fill="#363433" height="80" rx="3" width="22" x="8" y="74"></rect>
<!-- 2: Oct 12 -->
<rect fill="#363433" height="94" rx="3" width="22" x="47" y="60"></rect>
<!-- 3: Oct 13 -->
<rect fill="#363433" height="64" rx="3" width="22" x="86" y="90"></rect>
<!-- 4: Oct 14 -->
<rect fill="#363433" height="112" rx="3" width="22" x="125" y="42"></rect>
<!-- 5: Oct 15 -->
<rect fill="#363433" height="99" rx="3" width="22" x="164" y="55"></rect>
<!-- 6: Oct 16 -->
<rect fill="#363433" height="44" rx="3" width="22" x="203" y="110"></rect>
<!-- 7: Oct 17 -->
<rect fill="#363433" height="29" rx="3" width="22" x="242" y="125"></rect>
<!-- 8: Oct 18 -->
<rect fill="#363433" height="104" rx="3" width="22" x="281" y="50"></rect>
<!-- 9: Oct 19 -->
<rect fill="#363433" height="122" rx="3" width="22" x="320" y="32"></rect>
<!-- 10: Oct 20 -->
<rect fill="#363433" height="106" rx="3" width="22" x="359" y="48"></rect>
<!-- 11: Oct 21 (Peak Thursday: 4,180) -->
<rect fill="#3a3938" height="136" rx="3" width="22" x="398" y="18"></rect>
<!-- 12: Oct 22 -->
<rect fill="#363433" height="90" rx="3" width="22" x="437" y="64"></rect>
<!-- 13: Oct 23 -->
<rect fill="#363433" height="56" rx="3" width="22" x="476" y="98"></rect>
<!-- 14: Oct 24 TODAY (Saffron Accent #e08a1e / primary-container) -->
<rect fill="#e08a1e" height="116" rx="3" width="22" x="515" y="38"></rect>
<!-- Peak Callout indicator -->
<circle cx="409" cy="18" fill="#e6e2df" r="3"></circle>
</svg>
<!-- X-Axis Labels (JetBrains Mono) -->
<div class="grid grid-cols-7 pt-2 font-label-code text-label-code text-on-surface-variant text-center">
<span>M</span>
<span>T</span>
<span>W</span>
<span>T</span>
<span>F</span>
<span>S</span>
<span class="text-primary font-semibold">Today</span>
</div>
</div>
<!-- Summary Footer under chart -->
<div class="flex flex-wrap items-center justify-between pt-space-xs font-body-muted text-body-muted text-on-surface-variant">
<div class="flex items-center gap-space-xs">
<span>Daily average:</span>
<span class="font-label-code text-label-code text-on-surface font-semibold">2,410 words</span>
</div>
<div class="flex items-center gap-space-xs">
<span>Peak Day:</span>
<span class="font-label-code text-label-code text-on-surface font-semibold">Thu, Oct 21 (4,180 words)</span>
</div>
<div class="flex items-center gap-space-xs">
<span>Variance:</span>
<span class="font-label-code text-label-code text-on-surface font-semibold">±14.2%</span>
</div>
</div>
</div>
<!-- Right Section: Breakdown & Diagnostics (5 cols) -->
<div class="lg:col-span-5 flex flex-col gap-space-md">
<!-- Breakdown by Mode -->
<div class="bg-surface-container p-space-md rounded-xl flex flex-col gap-space-sm">
<div class="flex items-center justify-between">
<h3 class="font-headline-md text-headline-md text-on-surface">Breakdown by Mode</h3>
<span class="font-label-code text-label-code text-on-surface-variant">100% Total</span>
</div>
<!-- Stacked Bar Representation -->
<div class="h-2 w-full bg-surface-container-high rounded-full overflow-hidden flex">
<div class="h-full bg-primary-container" style="width: 52%"></div>
<div class="h-full bg-tertiary-container" style="width: 22%"></div>
<div class="h-full bg-surface-bright" style="width: 14%"></div>
<div class="h-full bg-outline" style="width: 8%"></div>
<div class="h-full bg-outline-variant" style="width: 4%"></div>
</div>
<!-- Mode List Rows -->
<div class="flex flex-col gap-1.5 pt-1">
<div class="flex items-center justify-between font-body-muted text-body-muted">
<div class="flex items-center gap-2">
<span class="w-2 h-2 rounded-sm bg-primary-container"></span>
<span class="text-on-surface">Plain dictation</span>
</div>
<span class="font-label-code text-label-code text-on-surface">52% <span class="text-on-surface-variant">(25,069)</span></span>
</div>
<div class="flex items-center justify-between font-body-muted text-body-muted">
<div class="flex items-center gap-2">
<span class="w-2 h-2 rounded-sm bg-tertiary-container"></span>
<span class="text-on-surface">Code syntax</span>
</div>
<span class="font-label-code text-label-code text-on-surface">22% <span class="text-on-surface-variant">(10,606)</span></span>
</div>
<div class="flex items-center justify-between font-body-muted text-body-muted">
<div class="flex items-center gap-2">
<span class="w-2 h-2 rounded-sm bg-surface-bright"></span>
<span class="text-on-surface">Notes &amp; Markdown</span>
</div>
<span class="font-label-code text-label-code text-on-surface">14% <span class="text-on-surface-variant">(6,749)</span></span>
</div>
<div class="flex items-center justify-between font-body-muted text-body-muted">
<div class="flex items-center gap-2">
<span class="w-2 h-2 rounded-sm bg-outline"></span>
<span class="text-on-surface">Email composer</span>
</div>
<span class="font-label-code text-label-code text-on-surface">8% <span class="text-on-surface-variant">(3,856)</span></span>
</div>
<div class="flex items-center justify-between font-body-muted text-body-muted">
<div class="flex items-center gap-2">
<span class="w-2 h-2 rounded-sm bg-outline-variant"></span>
<span class="text-on-surface">HI Trans (Hindi)</span>
</div>
<span class="font-label-code text-label-code text-on-surface">4% <span class="text-on-surface-variant">(1,930)</span></span>
</div>
</div>
</div>
<!-- Breakdown by Application Target -->
<div class="bg-surface-container p-space-md rounded-xl flex flex-col gap-space-sm">
<div class="flex items-center justify-between">
<h3 class="font-headline-md text-headline-md text-on-surface">Active Ingestion Target</h3>
<span class="font-body-muted text-body-muted text-on-surface-variant">Focus Window</span>
</div>
<div class="space-y-2">
<!-- Slack -->
<div class="flex flex-col gap-1">
<div class="flex items-center justify-between font-body-muted text-body-muted">
<span class="text-on-surface">Slack</span>
<span class="font-label-code text-label-code text-on-surface-variant">38%</span>
</div>
<div class="h-1.5 w-full bg-surface-container-high rounded-full overflow-hidden">
<div class="h-full bg-primary-container" style="width: 38%"></div>
</div>
</div>
<!-- VS Code / Terminal -->
<div class="flex flex-col gap-1">
<div class="flex items-center justify-between font-body-muted text-body-muted">
<span class="text-on-surface">VS Code / Terminal</span>
<span class="font-label-code text-label-code text-on-surface-variant">29%</span>
</div>
<div class="h-1.5 w-full bg-surface-container-high rounded-full overflow-hidden">
<div class="h-full bg-surface-bright" style="width: 29%"></div>
</div>
</div>
<!-- Obsidian -->
<div class="flex flex-col gap-1">
<div class="flex items-center justify-between font-body-muted text-body-muted">
<span class="text-on-surface">Obsidian</span>
<span class="font-label-code text-label-code text-on-surface-variant">18%</span>
</div>
<div class="h-1.5 w-full bg-surface-container-high rounded-full overflow-hidden">
<div class="h-full bg-surface-bright" style="width: 18%"></div>
</div>
</div>
<!-- Thunderbird & Firefox in compact row -->
<div class="flex items-center justify-between pt-1 font-body-muted text-body-muted text-on-surface-variant">
<span>Thunderbird (9%)</span>
<span>•</span>
<span>Firefox (6%)</span>
</div>
</div>
</div>
<!-- Hardware & Local Processing Spec (AdwActionRow style group) -->
<div class="bg-surface-container p-space-md rounded-xl flex flex-col gap-space-xs">
<div class="flex items-center justify-between pb-1">
<span class="font-body-muted text-body-muted text-on-surface-variant font-medium">Inference Telemetry</span>
<span class="font-keycap text-keycap text-on-surface bg-surface-container-high px-1 rounded">Wayland / PipeWire</span>
</div>
<div class="grid grid-cols-3 gap-space-xs pt-1">
<div class="bg-surface-container-low p-2 rounded-lg flex flex-col gap-0.5">
<span class="font-body-muted text-[11px] text-on-surface-variant">Avg Latency</span>
<span class="font-label-code text-label-code text-on-surface font-semibold">184 ms</span>
</div>
<div class="bg-surface-container-low p-2 rounded-lg flex flex-col gap-0.5">
<span class="font-body-muted text-[11px] text-on-surface-variant">RSS Memory</span>
<span class="font-label-code text-label-code text-on-surface font-semibold">740 MB</span>
</div>
<div class="bg-surface-container-low p-2 rounded-lg flex flex-col gap-0.5">
<span class="font-body-muted text-[11px] text-on-surface-variant">Quantization</span>
<span class="font-label-code text-label-code text-primary font-semibold">INT8</span>
</div>
</div>
<div class="flex items-center justify-between pt-2 text-on-surface-variant font-label-code text-label-code">
<span class="flex items-center gap-1.5">
<span class="w-1.5 h-1.5 rounded-full bg-emerald-500"></span>
<span>Model: Parakeet CTC 0.6b</span>
</span>
<span>Zero cloud egress</span>
</div>
</div>
</div>
</div>
</div>
</div></main>
