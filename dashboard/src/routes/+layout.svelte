<script lang="ts">
  import { onMount } from "svelte";
  import { isTauri, invoke } from "@tauri-apps/api/core";
  import { page } from "$app/state";
  import ToastHost from "$lib/ToastHost.svelte";
  import SetupModal from "$lib/SetupModal.svelte";
  import "../app.css";

  let { children } = $props();

  let showSetupModal = $state(false);
  let isTauriApp = $state(typeof window !== "undefined" && isTauri());
  let daemonConnected = $state<boolean | null>(null);

  async function checkDaemonConnection() {
    if (typeof window === "undefined") return;
    if (!isTauri()) {
      daemonConnected = true; // web preview mock mode
      return;
    }
    try {
      await invoke("session_status");
      daemonConnected = true;
    } catch {
      daemonConnected = false;
    }
  }

  onMount(() => {
    isTauriApp = isTauri();
    checkDaemonConnection();
    const interval = setInterval(checkDaemonConnection, 4000);

    function onOpenSetup() {
      showSetupModal = true;
    }
    window.addEventListener("open-setup-wizard", onOpenSetup);
    return () => {
      clearInterval(interval);
      window.removeEventListener("open-setup-wizard", onOpenSetup);
    };
  });

  const links = [
    { href: "/hud", label: "Floating HUD", icon: "hud", odId: "nav-hud", svg: `<path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z"/><path d="M19 10v2a7 7 0 0 1-14 0v-2"/><path d="M12 19v3"/>` },
    { href: "/", label: "History", icon: "history", odId: "nav-history", svg: `<circle cx="12" cy="12" r="9"/><path d="M12 7v6l4 2"/>` },
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
    if (pathname.startsWith("/stats")) return "Stats";
    if (pathname.startsWith("/models")) return "Speech models";
    if (pathname.startsWith("/words") || pathname.startsWith("/dictionary")) return "Custom words";
    if (pathname.startsWith("/settings")) return "Settings";
    return "History";
  }

  function getSidebarFooter(pathname: string): string {
    if (pathname.startsWith("/words") || pathname.startsWith("/dictionary")) return "Applied on the next transcription";
    if (pathname.startsWith("/settings")) return "Changes apply immediately";
    if (pathname.startsWith("/models")) return "Weights never leave this machine";
    if (pathname.startsWith("/stats")) return "Counts stay on this machine";
    return "small · local";
  }

  function getStatusShortcuts(pathname: string): { kbd: string; label: string }[] {
    if (pathname.startsWith("/words") || pathname.startsWith("/dictionary")) {
      return [
        { kbd: "N", label: "add word" },
        { kbd: "Esc", label: "close dialog" },
      ];
    }
    if (pathname.startsWith("/settings")) {
      return [];
    }
    if (pathname.startsWith("/models")) {
      return [
        { kbd: "J", label: "" },
        { kbd: "K", label: "move" },
        { kbd: "Enter", label: "download or set active" },
      ];
    }
    if (pathname.startsWith("/stats")) {
      return [
        { kbd: "1", label: "today" },
        { kbd: "2", label: "7 days" },
        { kbd: "3", label: "30 days" },
        { kbd: "4", label: "all" },
      ];
    }
    if (pathname.startsWith("/hud")) {
      return [];
    }
    return [
      { kbd: "J", label: "" },
      { kbd: "K", label: "move" },
      { kbd: "Enter", label: "insert" },
      { kbd: "C", label: "copy" },
      { kbd: "Del", label: "delete" },
      { kbd: "/", label: "search" },
    ];
  }

  async function minimize() {
    if (typeof window === "undefined") return;
    try {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      await getCurrentWindow().minimize();
    } catch (e) {
      console.warn("Could not minimize window:", e);
    }
  }

  async function toggleMaximize() {
    if (typeof window === "undefined") return;
    try {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      await getCurrentWindow().toggleMaximize();
    } catch (e) {
      console.warn("Could not toggle maximize window:", e);
    }
  }

  async function closeWindow() {
    if (typeof window === "undefined") return;
    try {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      await getCurrentWindow().close();
    } catch (e) {
      console.warn("Could not close window:", e);
    }
  }

  async function startDragging(e: MouseEvent) {
    if (e.button !== 0) return;
    const target = e.target as HTMLElement | null;
    if (target?.closest("button, a, input, select, textarea, .wc")) return;
    if (typeof window === "undefined") return;
    try {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      await getCurrentWindow().startDragging();
    } catch {
      // Non-Tauri fallback
    }
  }
</script>

<div
  class="stage min-h-screen w-full flex items-center justify-center {isTauriApp ? 'p-0 bg-bg h-screen overflow-hidden' : 'p-4 bg-desktop'}"
  class:is-tauri-stage={isTauriApp}
>
  <div
    class="window flex-shrink-0 flex flex-col bg-bg select-none relative {isTauriApp ? 'w-full h-full max-w-full max-h-full rounded-none border-0 shadow-none overflow-hidden' : 'w-[1100px] max-w-[calc(100vw-32px)] h-[760px] max-h-[calc(100vh-32px)] border border-border rounded-[var(--window-radius)] overflow-hidden shadow-2xl'}"
    class:is-tauri-win={isTauriApp}
  >
      <header
        class="headerbar"
        data-od-id="headerbar"
        onmousedown={startDragging}
        ondblclick={toggleMaximize}
      >
        <div class="header-title" data-tauri-drag-region>
          <svg class="icon" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z" />
            <path d="M19 10v2a7 7 0 0 1-14 0v-2" />
            <path d="M12 19v3" />
          </svg>
          <span>Narad</span>
          <span class="header-sep" aria-hidden="true">/</span>
          <span class="header-view">{getActiveTitle(page.url.pathname)}</span>
          <span class="stamp {daemonConnected === false ? 'is-disconnected' : ''}" data-od-id="offline-stamp">
            <i class={daemonConnected === false ? 'dot-warn' : 'dot-ok'}></i>
            {daemonConnected === false ? 'Daemon offline' : 'Ready · local'}
          </span>
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
          <div class="sidebar-foot" data-model-stamp>{getSidebarFooter(page.url.pathname)}</div>
        </nav>
        <main class="content flex-1 min-w-0 min-h-0 flex flex-col overflow-auto">
          {@render children()}
        </main>
      </div>
      <footer class="statusbar" data-od-id="statusbar">
        {#each getStatusShortcuts(page.url.pathname) as item}
          <span><span class="kbd">{item.kbd}</span> {item.label}</span>
        {/each}
        <span class="spacer mono">
          {#if page.url.pathname.startsWith('/words') || page.url.pathname.startsWith('/dictionary')}
            Saved on this machine
          {:else if page.url.pathname.startsWith('/stats')}
            All metrics computed locally
          {:else if page.url.pathname.startsWith('/settings')}
            Changes apply immediately
          {:else}
            small · local
          {/if}
        </span>
      </footer>
      <ToastHost />
      <SetupModal bind:open={showSetupModal} />
    </div>
  </div>

