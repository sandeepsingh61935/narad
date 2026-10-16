<script lang="ts">
  import { page } from "$app/state";
  import "./stitch.css";

  let { children } = $props();

  const inactiveClass =
    "flex items-center gap-space-md px-space-md py-2.5 rounded-lg text-on-surface-variant hover:bg-surface-container hover:text-on-surface transition-colors";
  const activeClass =
    "flex items-center justify-between px-space-md py-2.5 rounded-lg bg-surface-container-high text-on-surface transition-colors relative font-medium group";

  const items: {
    label: string;
    icon: string;
    href: string | null;
    badge?: string;
  }[] = [
    { label: "General", icon: "tune", href: null },
    { label: "Hotkeys", icon: "keyboard", href: "/settings", badge: "Wayland" },
    { label: "Audio", icon: "mic", href: "/settings/audio" },
    { label: "Models", icon: "memory", href: "/settings/models" },
    { label: "Modes", icon: "auto_fix_high", href: "/settings/modes" },
    { label: "Dictionary", icon: "spellcheck", href: "/settings/dictionary" },
    { label: "App profiles", icon: "tab", href: null },
    { label: "Privacy", icon: "verified_user", href: "/settings/privacy" },
    { label: "About", icon: "info", href: null },
  ];

  function isActive(href: string | null): boolean {
    if (href == null) return false;
    if (href === "/settings") return page.url.pathname === "/settings";
    return page.url.pathname === href || page.url.pathname.startsWith(`${href}/`);
  }

  function stay(event: MouseEvent) {
    event.preventDefault();
  }
</script>

<svelte:head>
  <link
    href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600&family=JetBrains+Mono:wght@500;600&display=swap"
    rel="stylesheet"
  />
  <link
    href="https://fonts.googleapis.com/css2?family=Material+Symbols+Outlined:opsz,wght,FILL,GRAD@20..48,100..700,0..1,-50..200"
    rel="stylesheet"
  />
</svelte:head>

<div
  class="flex flex-1 w-full min-h-[calc(100vh-3.5rem)] bg-surface font-body-default text-body-default text-on-surface antialiased"
>
  <aside
    class="w-60 bg-surface-container-low border-r border-outline-variant/30 flex flex-col shrink-0 select-none"
  >
    <div class="px-space-md pt-space-lg pb-space-sm">
      <h2 class="font-headline-md text-headline-md text-on-surface">Settings</h2>
    </div>
    <nav class="flex flex-col px-space-sm gap-0.5" aria-label="Settings">
      {#each items as item}
        {@const active = isActive(item.href)}
        {#if active && item.href}
          <a class={activeClass} href={item.href} aria-current="page">
            <div class="flex items-center gap-space-md">
              <span class="w-1.5 h-4 rounded-full bg-primary-container absolute left-1"></span>
              <span
                class="material-symbols-outlined text-[18px] text-primary"
                style="font-variation-settings: 'FILL' 1;">{item.icon}</span
              >
              <span class="font-body-default text-body-default text-on-surface">{item.label}</span>
            </div>
            {#if item.badge}
              <span
                class="font-label-code text-label-code text-primary bg-primary/10 px-1.5 py-0.5 rounded text-[11px]"
                >{item.badge}</span
              >
            {/if}
          </a>
        {:else if item.href}
          <a class={inactiveClass} href={item.href}>
            <span class="material-symbols-outlined text-[18px]">{item.icon}</span>
            <span class="font-body-default text-body-default">{item.label}</span>
          </a>
        {:else}
          <a class={inactiveClass} href="#settings-{item.label}" onclick={stay}>
            <span class="material-symbols-outlined text-[18px]">{item.icon}</span>
            <span class="font-body-default text-body-default">{item.label}</span>
          </a>
        {/if}
      {/each}
    </nav>
    <div class="mt-auto p-space-md">
      <div
        class="bg-surface-container px-space-md py-space-sm rounded-lg border border-outline-variant/30 flex flex-col gap-1"
      >
        <div class="flex items-center justify-between">
          <span
            class="font-body-muted text-body-muted text-on-surface-variant text-[11px] uppercase tracking-wider"
            >Portal Session</span
          >
          <span class="w-1.5 h-1.5 rounded-full bg-emerald-400"></span>
        </div>
        <span class="font-label-code text-label-code text-on-surface text-[11px] truncate"
          >org.freedesktop.portal</span
        >
      </div>
    </div>
  </aside>
  {@render children()}
</div>
