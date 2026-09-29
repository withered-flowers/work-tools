<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import { onMount, onDestroy } from "svelte";
  import Header from "$lib/components/Header.svelte";
  import TeamInvites from "$lib/components/TeamInvites.svelte";
  import RepoProvisioning from "$lib/components/RepoProvisioning.svelte";
  import ConsoleLogs from "$lib/components/ConsoleLogs.svelte";
  import Icon from "$lib/components/Icon.svelte";
  import type { GitHubUser, LogMessage } from "$lib/types";

  let activeTab = $state<"team-invites" | "repo-provision" | "logs">("team-invites");
  let token = $state("");
  let user = $state<GitHubUser | null>(null);
  let logs = $state<LogMessage[]>([]);

  let unlistenLogs: (() => void) | null = null;

  function addLog(level: "info" | "success" | "warn" | "error", message: string) {
    const now = new Date().toTimeString().split(" ")[0];
    logs = [...logs, { timestamp: now, level, message }];
  }

  function clearLogs() {
    logs = [];
  }

  onMount(async () => {
    addLog("info", "GitHub Automation Studio (Soft Pastel Light Theme) initialized.");

    unlistenLogs = await listen<LogMessage>("app-log", (event) => {
      logs = [...logs, event.payload];
    });
  });

  onDestroy(() => {
    if (unlistenLogs) {
      unlistenLogs();
    }
  });
</script>

<div class="app-shell">
  <Header bind:token bind:user onLog={addLog} />

  <!-- Navigation Tab Bar -->
  <nav class="tabs-nav-bar" aria-label="Main Workflows">
    <div class="tabs-inner-wrapper">
      <button
        type="button"
        role="tab"
        aria-selected={activeTab === "team-invites"}
        class="nav-tab-btn {activeTab === 'team-invites' ? 'active' : ''}"
        onclick={() => activeTab = "team-invites"}
      >
        <Icon name="users" size={16} />
        <span class="tab-title">Team Invitations</span>
        <span class="tab-id-pill">001</span>
      </button>

      <button
        type="button"
        role="tab"
        aria-selected={activeTab === "repo-provision"}
        class="nav-tab-btn {activeTab === 'repo-provision' ? 'active' : ''}"
        onclick={() => activeTab = "repo-provision"}
      >
        <Icon name="repo" size={16} />
        <span class="tab-title">Repository Provisioning</span>
        <span class="tab-id-pill">002</span>
      </button>

      <button
        type="button"
        role="tab"
        aria-selected={activeTab === "logs"}
        class="nav-tab-btn {activeTab === 'logs' ? 'active' : ''}"
        onclick={() => activeTab = "logs"}
      >
        <Icon name="terminal" size={16} />
        <span class="tab-title">Console & Activity</span>
        {#if logs.length > 0}
          <span class="log-indicator-badge">{logs.length}</span>
        {/if}
      </button>
    </div>
  </nav>

  <!-- Main View Area (Preserves tab state and progress when switching workflows) -->
  <main class="main-content-viewport">
    <div class="tab-pane" class:active-pane={activeTab === "team-invites"}>
      <TeamInvites {token} onLog={addLog} />
    </div>
    <div class="tab-pane" class:active-pane={activeTab === "repo-provision"}>
      <RepoProvisioning {token} onLog={addLog} />
    </div>
    <div class="tab-pane" class:active-pane={activeTab === "logs"}>
      <ConsoleLogs {logs} onClear={clearLogs} />
    </div>
  </main>
</div>

<style>
  :global(:root) {
    /* Soft Pastel Light Theme Tokens */
    --color-primary: #4f46e5;
    --color-primary-hover: #4338ca;
    --color-primary-soft: #eef2ff;
    --color-primary-border: #c7d2fe;

    --color-accent-mint: #059669;
    --color-accent-mint-soft: #ecfdf5;
    --color-accent-mint-border: #a7f3d0;

    --color-accent-peach: #b45309;
    --color-accent-peach-soft: #fffbeb;
    --color-accent-peach-border: #fde68a;

    --color-accent-rose: #e11d48;
    --color-accent-rose-soft: #fff1f2;
    --color-accent-rose-border: #fecdd3;

    --color-accent-sky: #0284c7;
    --color-accent-sky-soft: #f0f9ff;
    --color-accent-sky-border: #bae6fd;

    --color-background: #f8fafc;
    --color-surface-card: #ffffff;
    --color-surface-inset: #f1f5f9;
    --color-border: #e2e8f0;
    --color-border-subtle: #cbd5e1;

    --color-foreground: #0f172a;
    --color-foreground-secondary: #334155;
    --color-foreground-muted: #64748b;

    font-family: 'IBM Plex Sans', -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    color: var(--color-foreground);
    background-color: var(--color-background);
    line-height: 1.5;
    -webkit-font-smoothing: antialiased;
  }

  :global(body) {
    margin: 0;
    padding: 0;
    background-color: var(--color-background);
    color: var(--color-foreground);
    overflow-x: hidden;
  }

  :global(*),
  :global(*::before),
  :global(*::after) {
    box-sizing: border-box;
  }

  /* Custom soft scrollbar */
  :global(::-webkit-scrollbar) {
    width: 8px;
    height: 8px;
  }

  :global(::-webkit-scrollbar-track) {
    background: #f1f5f9;
  }

  :global(::-webkit-scrollbar-thumb) {
    background: #cbd5e1;
    border-radius: 4px;
    border: 2px solid #f1f5f9;
  }

  :global(::-webkit-scrollbar-thumb:hover) {
    background: #94a3b8;
  }

  .app-shell {
    display: flex;
    flex-direction: column;
    min-height: 100vh;
    background: #f8fafc;
  }

  .tabs-nav-bar {
    background: #ffffff;
    border-bottom: 1px solid #e2e8f0;
    padding: 0 1.75rem;
    position: sticky;
    top: 0;
    z-index: 9;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.03);
  }

  .tabs-inner-wrapper {
    display: flex;
    gap: 0.5rem;
    max-width: 1400px;
    margin: 0 auto;
  }

  .nav-tab-btn {
    display: flex;
    align-items: center;
    gap: 0.55rem;
    padding: 0.85rem 1.25rem;
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    color: #64748b;
    font-size: 0.875rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .nav-tab-btn:hover {
    color: #1e293b;
    background: rgba(99, 102, 241, 0.04);
  }

  .nav-tab-btn.active {
    color: #4f46e5;
    border-bottom-color: #4f46e5;
    background: rgba(99, 102, 241, 0.06);
  }

  .tab-title {
    letter-spacing: -0.01em;
  }

  .tab-id-pill {
    font-family: 'JetBrains Mono', monospace;
    font-size: 0.675rem;
    background: #f1f5f9;
    color: #64748b;
    padding: 0.1rem 0.4rem;
    border-radius: 4px;
    border: 1px solid #e2e8f0;
  }

  .nav-tab-btn.active .tab-id-pill {
    background: #eef2ff;
    color: #4f46e5;
    border-color: #c7d2fe;
  }

  .log-indicator-badge {
    font-family: 'JetBrains Mono', monospace;
    font-size: 0.675rem;
    background: #4f46e5;
    color: #ffffff;
    padding: 0.1rem 0.45rem;
    border-radius: 9999px;
    font-weight: 700;
  }

  .main-content-viewport {
    flex: 1;
    padding: 1.5rem 1.75rem 2.5rem 1.75rem;
    max-width: 1400px;
    width: 100%;
    margin: 0 auto;
  }

  .tab-pane {
    display: none;
    width: 100%;
  }

  .tab-pane.active-pane {
    display: block;
  }
</style>
