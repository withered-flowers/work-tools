<script lang="ts">
  import type { LogMessage } from "$lib/types";
  import Icon from "./Icon.svelte";

  let {
    logs = [] as LogMessage[],
    onClear = () => {},
  }: {
    logs?: LogMessage[];
    onClear?: () => void;
  } = $props();

  let filterLevel = $state<"all" | "info" | "success" | "warn" | "error">("all");
  let searchQuery = $state("");
  let autoScroll = $state(true);
  let logContainer: HTMLDivElement | null = $state(null);
  let copiedToast = $state(false);

  let filteredLogs = $derived(
    logs.filter((log) => {
      const matchLevel = filterLevel === "all" || log.level === filterLevel;
      const matchQuery =
        !searchQuery.trim() ||
        log.message.toLowerCase().includes(searchQuery.toLowerCase());
      return matchLevel && matchQuery;
    })
  );

  $effect(() => {
    if (autoScroll && logContainer && logs.length > 0) {
      logContainer.scrollTop = logContainer.scrollHeight;
    }
  });

  async function copyLogs() {
    const text = logs
      .map((l) => `[${l.timestamp}] [${l.level.toUpperCase()}] ${l.message}`)
      .join("\n");
    await navigator.clipboard.writeText(text);
    copiedToast = true;
    setTimeout(() => {
      copiedToast = false;
    }, 2000);
  }
</script>

<div class="terminal-card">
  <!-- Terminal Header Toolbar -->
  <div class="terminal-toolbar">
    <div class="filter-cluster">
      <button
        type="button"
        class="filter-pill {filterLevel === 'all' ? 'active' : ''}"
        onclick={() => filterLevel = "all"}
      >
        <span>All</span>
        <span class="count-tag">{logs.length}</span>
      </button>
      <button
        type="button"
        class="filter-pill pill-info {filterLevel === 'info' ? 'active' : ''}"
        onclick={() => filterLevel = "info"}
      >
        <Icon name="info" size={12} />
        <span>Info</span>
        <span class="count-tag">{logs.filter((l) => l.level === "info").length}</span>
      </button>
      <button
        type="button"
        class="filter-pill pill-success {filterLevel === 'success' ? 'active' : ''}"
        onclick={() => filterLevel = "success"}
      >
        <Icon name="check-circle" size={12} />
        <span>Success</span>
        <span class="count-tag">{logs.filter((l) => l.level === "success").length}</span>
      </button>
      <button
        type="button"
        class="filter-pill pill-warn {filterLevel === 'warn' ? 'active' : ''}"
        onclick={() => filterLevel = "warn"}
      >
        <Icon name="alert" size={12} />
        <span>Warnings</span>
        <span class="count-tag">{logs.filter((l) => l.level === "warn").length}</span>
      </button>
      <button
        type="button"
        class="filter-pill pill-error {filterLevel === 'error' ? 'active' : ''}"
        onclick={() => filterLevel = "error"}
      >
        <Icon name="cross-circle" size={12} />
        <span>Errors</span>
        <span class="count-tag">{logs.filter((l) => l.level === "error").length}</span>
      </button>
    </div>

    <div class="actions-cluster">
      <div class="search-box">
        <Icon name="search" size={13} color="#64748b" />
        <input
          type="text"
          placeholder="Filter logs..."
          bind:value={searchQuery}
          class="terminal-search-input"
        />
      </div>

      <label class="auto-scroll-toggle" for="console-auto-scroll">
        <input
          id="console-auto-scroll"
          type="checkbox"
          bind:checked={autoScroll}
          class="scroll-checkbox"
        />
        <span>Auto-scroll</span>
      </label>

      <button type="button" class="btn-action" onclick={copyLogs} title="Copy all logs to clipboard">
        <Icon name="copy" size={13} />
        <span>{copiedToast ? "Copied!" : "Copy"}</span>
      </button>

      <button type="button" class="btn-action btn-danger-action" onclick={onClear} title="Clear terminal logs">
        <Icon name="trash" size={13} />
        <span>Clear</span>
      </button>
    </div>
  </div>

  <!-- Terminal Body -->
  <div class="terminal-body" bind:this={logContainer}>
    {#if filteredLogs.length === 0}
      <div class="terminal-empty-state">
        <Icon name="terminal" size={32} color="#94a3b8" />
        <p>No activity log entries recorded.</p>
      </div>
    {:else}
      {#each filteredLogs as log, i}
        <div class="log-entry level-{log.level}">
          <span class="entry-index">{i + 1}</span>
          <span class="entry-time">{log.timestamp}</span>
          <span class="entry-badge">[{log.level.toUpperCase()}]</span>
          <span class="entry-text">{log.message}</span>
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .terminal-card {
    background: #ffffff;
    border: 1px solid var(--color-border, #e2e8f0);
    border-radius: 14px;
    padding: 1.1rem 1.25rem;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.04), 0 1px 2px rgba(0, 0, 0, 0.02);
    display: flex;
    flex-direction: column;
    gap: 0.9rem;
  }

  .terminal-toolbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1rem;
    flex-wrap: wrap;
  }

  .filter-cluster {
    display: flex;
    gap: 0.4rem;
    flex-wrap: wrap;
  }

  .filter-pill {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    background: #f8fafc;
    border: 1px solid #e2e8f0;
    color: #475569;
    font-family: 'JetBrains Mono', monospace;
    font-size: 0.725rem;
    font-weight: 600;
    padding: 0.32rem 0.65rem;
    border-radius: 8px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .filter-pill:hover {
    color: #0f172a;
    background: #f1f5f9;
    border-color: #cbd5e1;
  }

  .filter-pill.active {
    background: #eef2ff;
    color: #4338ca;
    border-color: #c7d2fe;
    box-shadow: 0 1px 2px rgba(79, 70, 229, 0.08);
  }

  .filter-pill.pill-info.active {
    border-color: #bae6fd;
    color: #0369a1;
    background: #f0f9ff;
    box-shadow: 0 1px 2px rgba(2, 132, 199, 0.08);
  }

  .filter-pill.pill-success.active {
    border-color: #a7f3d0;
    color: #065f46;
    background: #ecfdf5;
    box-shadow: 0 1px 2px rgba(5, 150, 105, 0.08);
  }

  .filter-pill.pill-warn.active {
    border-color: #fde68a;
    color: #92400e;
    background: #fffbeb;
    box-shadow: 0 1px 2px rgba(180, 83, 9, 0.08);
  }

  .filter-pill.pill-error.active {
    border-color: #fecdd3;
    color: #be123c;
    background: #fff1f2;
    box-shadow: 0 1px 2px rgba(225, 29, 72, 0.08);
  }

  .count-tag {
    font-size: 0.675rem;
    background: #ffffff;
    padding: 0.1rem 0.35rem;
    border-radius: 4px;
    color: #64748b;
    border: 1px solid rgba(0, 0, 0, 0.06);
  }

  .filter-pill.active .count-tag {
    background: rgba(255, 255, 255, 0.85);
    color: inherit;
  }

  .actions-cluster {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    flex-wrap: wrap;
  }

  .search-box {
    position: relative;
    display: flex;
    align-items: center;
  }

  .search-box :global(.svg-icon) {
    position: absolute;
    left: 8px;
    pointer-events: none;
  }

  .terminal-search-input {
    background: #ffffff;
    border: 1px solid #cbd5e1;
    color: #0f172a;
    border-radius: 7px;
    padding: 0.35rem 0.65rem 0.35rem 1.8rem;
    font-family: 'JetBrains Mono', monospace;
    font-size: 0.775rem;
    width: 160px;
    transition: all 0.2s ease;
  }

  .terminal-search-input::placeholder {
    color: #94a3b8;
  }

  .terminal-search-input:focus {
    outline: none;
    border-color: #6366f1;
    box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.12);
    width: 200px;
  }

  .auto-scroll-toggle {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.775rem;
    color: #475569;
    cursor: pointer;
    user-select: none;
  }

  .scroll-checkbox {
    width: 14px;
    height: 14px;
    accent-color: #4f46e5;
    cursor: pointer;
  }

  .btn-action {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    background: #ffffff;
    border: 1px solid #cbd5e1;
    color: #334155;
    font-size: 0.775rem;
    font-weight: 500;
    padding: 0.35rem 0.7rem;
    border-radius: 7px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .btn-action:hover {
    background: #eef2ff;
    color: #4338ca;
    border-color: #c7d2fe;
  }

  .btn-danger-action {
    color: #64748b;
  }

  .btn-danger-action:hover {
    background: #fff1f2;
    color: #be123c;
    border-color: #fecdd3;
  }

  /* Terminal Body */
  .terminal-body {
    background: #f8fafc;
    border: 1px solid #e2e8f0;
    border-radius: 10px;
    padding: 0.85rem 1rem;
    font-family: 'JetBrains Mono', ui-monospace, SFMono-Regular, monospace;
    font-size: 0.8rem;
    line-height: 1.6;
    height: 500px;
    overflow-y: auto;
    color: #1e293b;
  }

  .terminal-empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: #64748b;
    gap: 0.5rem;
  }

  .terminal-empty-state p {
    font-size: 0.85rem;
    margin: 0;
  }

  .log-entry {
    display: flex;
    align-items: flex-start;
    gap: 0.65rem;
    padding: 0.25rem 0.4rem;
    border-radius: 5px;
    word-break: break-all;
    border-bottom: 1px solid #f1f5f9;
    transition: background 0.1s ease;
  }

  .log-entry:hover {
    background: #ffffff;
  }

  .entry-index {
    color: #94a3b8;
    width: 25px;
    text-align: right;
    user-select: none;
    flex-shrink: 0;
    font-size: 0.75rem;
  }

  .entry-time {
    color: #64748b;
    flex-shrink: 0;
    font-size: 0.75rem;
  }

  .entry-badge {
    font-weight: 700;
    flex-shrink: 0;
    padding: 0.05rem 0.35rem;
    border-radius: 4px;
    font-size: 0.72rem;
    text-align: center;
    min-width: 65px;
  }

  .level-info .entry-badge {
    color: #0369a1;
    background: #e0f2fe;
    border: 1px solid #bae6fd;
  }

  .level-info .entry-text {
    color: #0f172a;
  }

  .level-success .entry-badge {
    color: #065f46;
    background: #d1fae5;
    border: 1px solid #a7f3d0;
  }

  .level-success .entry-text {
    color: #064e3b;
    font-weight: 500;
  }

  .level-warn .entry-badge {
    color: #92400e;
    background: #fef3c7;
    border: 1px solid #fde68a;
  }

  .level-warn .entry-text {
    color: #78350f;
  }

  .level-error .entry-badge {
    color: #9f1239;
    background: #ffe4e6;
    border: 1px solid #fecdd3;
  }

  .level-error .entry-text {
    color: #881337;
    font-weight: 500;
  }
</style>
