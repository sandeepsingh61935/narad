<script lang="ts">
  import { page } from "$app/state";
  import "../app.css";

  let { children } = $props();

  const links = [
    { href: "/hud", label: "Floating HUD", icon: "hud", odId: "nav-hud", svg: `<path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z"/><path d="M19 10v2a7 7 0 0 1-14 0v-2"/><path d="M12 19v3"/>` },
    { href: "/", label: "History", icon: "history", odId: "nav-history", svg: `<circle cx="12" cy="12" r="9"/><path d="M12 7v6l4 2"/>` },
    { href: "/meetings", label: "Meetings", icon: "meetings", odId: "nav-meetings", svg: `<path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M22 21v-2a4 4 0 0 0-3-3.87"/><path d="M16 3.13a4 4 0 0 1 0 7.75"/>` },
    { href: "/stats", label: "Stats", icon: "stats", odId: "nav-stats", svg: `<path d="M4 19V9M10 19V5M16 19v-6M22 19H2"/>` },
    { href: "/models", label: "Speech models", icon: "models", odId: "nav-models", svg: `<rect x="6" y="6" width="12" height="12" rx="2"/><path d="M9 2v4M15 2v4M9 18v4M15 18v4M2 9h4M18 9h4"/>` },
    { href: "/words", label: "Custom words", icon: "words", odId: "nav-words", svg: `<path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20"/><path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2Z"/>` },
    { href: "/settings", label: "Settings", icon: "settings", odId: "nav-settings", svg: `<path d="M4 21v-7M4 10V3M12 21v-9M12 8V3M20 21v-5M20 12V3M2 14h4M10 8h4M18 16h4"/>` },
  ];

  function isActive(href: string): boolean {
    const path = page.url.pathname;
    if (href === "/") return path === "/";
    return path === href || path.startsWith(`${href}/`);
  }

  function getActiveTitle(pathname: string): string {
    if (pathname.startsWith("/hud")) return "Floating HUD";
    if (pathname.startsWith("/meetings")) return "Meetings";
    if (pathname.startsWith("/stats")) return "Stats";
    if (pathname.startsWith("/models")) return "Speech models";
    if (pathname.startsWith("/words") || pathname.startsWith("/dictionary")) return "Custom words";
    if (pathname.startsWith("/settings")) return "Settings";
    return "History";
  }

  async function minimize() {
    if (typeof window === "undefined") return;
    try {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      await getCurrentWindow().minimize();
    } catch {
      // Non-Tauri fallback
    }
  }

  async function toggleMaximize() {
    if (typeof window === "undefined") return;
    try {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      await getCurrentWindow().toggleMaximize();
    } catch {
      // Non-Tauri fallback
    }
  }

  async function closeWindow() {
    if (typeof window === "undefined") return;
    try {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      await getCurrentWindow().close();
    } catch {
      // Non-Tauri fallback
    }
  }
</script>

<div class="window h-screen w-screen flex flex-col bg-bg border border-border rounded-[var(--window-radius)] overflow-hidden select-none">
  <header class="headerbar" data-tauri-drag-region data-od-id="headerbar">
    <div class="header-title" data-tauri-drag-region>
      <svg class="icon" viewBox="0 0 24 24" aria-hidden="true">
        <path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z" />
        <path d="M19 10v2a7 7 0 0 1-14 0v-2" />
        <path d="M12 19v3" />
      </svg>
      <span>Narad</span>
      <span class="header-sep" aria-hidden="true">/</span>
      <span class="header-view">{getActiveTitle(page.url.pathname)}</span>
      <span class="stamp" data-od-id="offline-stamp"><i></i>Offline</span>
    </div>
    <div class="window-controls">
      <button
        type="button"
        class="wc"
        data-win-min
        aria-label="Minimize"
        data-od-id="win-minimize"
        onclick={minimize}
      >
        <svg class="icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M5 12h14" /></svg>
      </button>
      <button
        type="button"
        class="wc"
        data-win-max
        aria-label="Maximize"
        data-od-id="win-maximize"
        onclick={toggleMaximize}
      >
        <svg class="icon" viewBox="0 0 24 24" aria-hidden="true"><rect x="7" y="7" width="10" height="10" rx="1" /></svg>
      </button>
      <button
        type="button"
        class="wc wc-close"
        aria-label="Close"
        data-od-id="win-close"
        onclick={closeWindow}
      >
        <svg class="icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M7 7l10 10M17 7 7 17" /></svg>
      </button>
    </div>
  </header>
  <div class="app-body flex-1 min-h-0 flex">
    <nav class="sidebar" aria-label="Narad" data-od-id="sidebar">
      {#each links as link}
        <a
          href={link.href}
          class="nav-item {isActive(link.href) ? 'is-active' : ''}"
          aria-current={isActive(link.href) ? "page" : undefined}
          data-od-id={link.odId}
        >
          <svg class="icon" viewBox="0 0 24 24" aria-hidden="true">
            {@html link.svg}
          </svg>
          <span>{link.label}</span>
        </a>
      {/each}
      <div class="sidebar-foot" data-model-stamp>small · local</div>
    </nav>
    <main class="content flex-1 min-w-0 min-h-0 flex flex-col overflow-auto">
      {@render children()}
    </main>
  </div>
</div>
