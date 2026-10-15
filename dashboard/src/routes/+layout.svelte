<script lang="ts">
  import { page } from "$app/state";
  import "../app.css";

  let { children } = $props();

  const links = [
    { href: "/", label: "History", icon: "history" },
    { href: "/stats", label: "Stats", icon: "bar_chart" },
    { href: "/settings", label: "Settings", icon: "settings" },
  ] as const;

  const activeClass =
    "flex items-center gap-space-md px-space-md py-space-sm min-h-[44px] rounded-lg transition-colors bg-surface-container-high text-on-surface font-medium focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2 focus-visible:ring-offset-surface-container-low";
  const idleClass =
    "flex items-center gap-space-md px-space-md py-space-sm min-h-[44px] rounded-lg text-on-surface-variant hover:bg-surface-container hover:text-on-surface transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2 focus-visible:ring-offset-surface-container-low";

  function isActive(href: string): boolean {
    const path = page.url.pathname;
    if (href === "/") return path === "/";
    return path === href || path.startsWith(`${href}/`);
  }
</script>

<aside
  class="fixed left-0 top-0 h-full w-64 bg-surface-container-low border-r border-outline-variant flex flex-col z-30 select-none"
  aria-label="Sidebar"
>
  <div class="flex flex-col">
    <div
      class="h-14 px-space-md flex items-center gap-space-sm border-b border-outline-variant"
    >
      <span class="font-headline-md text-headline-md tracking-tight text-on-surface">Narad</span>
    </div>
    <nav
      class="flex flex-col p-space-sm gap-space-xs mt-space-xs"
      aria-label="Primary"
    >
      {#each links as link}
        <a
          href={link.href}
          class={isActive(link.href) ? activeClass : idleClass}
          aria-current={isActive(link.href) ? "page" : undefined}
        >
          <span class="material-symbols-outlined text-[18px]" aria-hidden="true">{link.icon}</span>
          <span class="font-body-default text-body-default">{link.label}</span>
        </a>
      {/each}
    </nav>
  </div>
</aside>
<main class="ml-64 h-screen overflow-auto">
  {@render children()}
</main>
