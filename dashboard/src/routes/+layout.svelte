<script lang="ts">
  import { page } from "$app/state";
  import "../app.css";

  let { children } = $props();

  const links = [
    { href: "/", label: "History", icon: "history" },
    { href: "/stats", label: "Stats", icon: "bar_chart" },
    { href: "/models", label: "Models", icon: "memory" },
    { href: "/settings", label: "Settings", icon: "settings" },
  ] as const;

  const activeClass =
    "flex items-center gap-space-md px-space-md py-space-sm rounded-lg transition-colors bg-surface-container-high text-on-surface font-medium";
  const idleClass =
    "flex items-center gap-space-md px-space-md py-space-sm rounded-lg text-on-surface-variant hover:bg-surface-container hover:text-on-surface transition-colors";

  const bare = $derived(page.url.pathname.startsWith("/setup"));

  function isActive(href: string): boolean {
    const path = page.url.pathname;
    if (href === "/") return path === "/";
    return path === href || path.startsWith(`${href}/`);
  }
</script>

{#if bare}
  {@render children()}
{:else}
  <aside
    class="fixed left-0 top-0 h-full w-64 bg-surface-container-low border-r border-outline-variant flex flex-col justify-between z-30 select-none"
  >
    <div class="flex flex-col">
      <div
        class="h-14 px-space-md flex items-center gap-space-sm border-b border-outline-variant"
      >
        <img
          alt="Narad Icon"
          class="h-8 w-auto object-contain"
          src="https://lh3.googleusercontent.com/aida/AEtjO1VaTHW5iF3eItDNdx3CAXBVrU_rWCpVyVSps2uDlDolWgR-uLG9aOfbxwi38I3SzALsa4_Yjns3urY3wg4HELs6PxYO7Z17iGLmhxLEPIl5SZhYxJhHeFlAtF_9bYY1eydHSlxVuF_ezVU6nyzLL7VXg-YGiIV6g82ckKD4qX6DnC3qu7Wii43-SpaCrYLpirKawJs5vbmV0RD7YrMjqdgOeh-TEjgHMDO7DI6CZnw9rNr3XN0b-XO-t1E"
        />
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
            <span class="material-symbols-outlined text-[18px]">{link.icon}</span>
            <span class="font-body-default text-body-default">{link.label}</span>
          </a>
        {/each}
      </nav>
    </div>
    <div
      class="p-space-sm m-space-sm bg-surface-container border border-outline-variant rounded-lg flex flex-col gap-space-xs"
    >
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-space-xs">
          <span class="w-2 h-2 rounded-full bg-emerald-500 shrink-0"></span>
          <span class="font-label-code text-label-code text-on-surface">Ready · Parakeet EN</span>
        </div>
        <span
          class="px-space-xs py-0.5 rounded bg-surface-container-high border border-outline-variant font-keycap text-keycap text-on-surface-variant"
          >Plain</span
        >
      </div>
      <div
        class="flex items-center justify-between text-on-surface-variant font-body-muted text-body-muted"
      >
        <span>Buffer RAM</span>
        <span class="font-label-code text-label-code">124 MB</span>
      </div>
    </div>
  </aside>
  <header
    class="fixed top-0 left-64 right-0 h-14 bg-surface-container-low border-b border-outline-variant z-20 flex items-center justify-between px-space-md select-none"
  >
    <div class="flex items-center gap-space-md">
      <div
        class="flex items-center bg-surface-container px-space-sm py-1 rounded-lg border border-outline-variant gap-space-sm w-64"
      >
        <span class="material-symbols-outlined text-on-surface-variant text-[18px]">search</span>
        <input
          class="bg-transparent border-none outline-none font-body-muted text-body-muted text-on-surface placeholder:text-on-surface-variant w-full"
          placeholder="Search transcripts..."
          type="text"
          aria-label="Search transcripts"
        />
        <span
          class="font-keycap text-keycap text-on-surface-variant bg-surface-container-high px-1 rounded border border-outline-variant"
          >/</span
        >
      </div>
      <div class="hidden lg:flex items-center gap-1">
        <span class="font-body-muted text-body-muted text-on-surface-variant mr-1">Global</span>
        <kbd
          class="px-1.5 py-0.5 bg-surface-container-high border border-outline-variant rounded font-keycap text-keycap text-on-surface-variant"
          >Ctrl</kbd
        >
        <kbd
          class="px-1.5 py-0.5 bg-surface-container-high border border-outline-variant rounded font-keycap text-keycap text-on-surface-variant"
          >Alt</kbd
        >
        <kbd
          class="px-1.5 py-0.5 bg-surface-container-high border border-outline-variant rounded font-keycap text-keycap text-on-surface-variant"
          >Space</kbd
        >
      </div>
    </div>
    <div class="flex items-center gap-space-md">
      <button
        class="flex items-center gap-space-xs bg-primary-container hover:bg-primary text-on-primary-container px-space-md py-1.5 rounded-lg transition-colors font-headline-md text-headline-md"
        type="button"
      >
        <span class="material-symbols-outlined text-[18px]">mic</span>
        <span>Record</span>
      </button>
      <div class="h-4 w-px bg-outline-variant mx-space-xs"></div>
      <div class="w-8 h-8 rounded-full bg-primary flex items-center justify-center">
        <span class="material-symbols-outlined text-on-primary text-[18px]">person</span>
      </div>
      <div class="flex items-center gap-1.5 ml-space-xs">
        <button
          aria-label="Minimize"
          class="w-3.5 h-3.5 rounded-full bg-surface-container-highest hover:bg-surface-bright flex items-center justify-center transition-colors"
          type="button"
        ></button>
        <button
          aria-label="Maximize"
          class="w-3.5 h-3.5 rounded-full bg-surface-container-highest hover:bg-surface-bright flex items-center justify-center transition-colors"
          type="button"
        ></button>
        <button
          aria-label="Close"
          class="w-3.5 h-3.5 rounded-full bg-error-container hover:bg-error flex items-center justify-center transition-colors"
          type="button"
        ></button>
      </div>
    </div>
  </header>
  <main class="ml-64 pt-14 h-screen overflow-auto">
    {@render children()}
  </main>
{/if}
