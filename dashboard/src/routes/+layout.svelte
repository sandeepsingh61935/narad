<script lang="ts">
  import { page } from "$app/state";
  import "../app.css";

  let { children } = $props();

  const links = [
    { href: "/", label: "History" },
    { href: "/settings", label: "Settings" },
  ] as const;

  function isActive(href: string): boolean {
    return href === "/"
      ? page.url.pathname === "/"
      : page.url.pathname.startsWith(href);
  }
</script>

<div class="app">
  <nav class="nav" aria-label="Primary">
    {#each links as link}
      <a
        href={link.href}
        class="tab"
        class:active={isActive(link.href)}
        aria-current={isActive(link.href) ? "page" : undefined}
      >
        {link.label}
      </a>
    {/each}
  </nav>
  <div class="content">
    {@render children()}
  </div>
</div>

<style>
  .app {
    display: flex;
    flex-direction: column;
    min-height: 100vh;
  }

  .nav {
    display: flex;
    gap: var(--space-xs);
    padding: var(--space-sm) var(--space-xl);
    border-bottom: 1px solid var(--border);
    background: var(--surface-elevated);
  }

  .tab {
    padding: 0.4rem 0.75rem;
    border-radius: var(--radius);
    color: var(--text-secondary);
    text-decoration: none;
    font-size: 13px;
    font-weight: 500;
  }

  .tab:hover {
    color: var(--text);
    background: var(--surface);
  }

  .tab.active {
    color: var(--text);
    background: var(--surface);
    box-shadow: inset 0 -2px 0 var(--accent);
  }

  .content {
    flex: 1;
    min-height: 0;
  }
</style>
