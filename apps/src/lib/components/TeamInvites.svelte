<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount, onDestroy } from "svelte";
  import type { TeamInvitationEntry, TeamInviteProgress, InvitationSummary } from "$lib/types";
  import Icon from "./Icon.svelte";

  let {
    token = "",
    onLog = (level: "info" | "success" | "warn" | "error", message: string) => {},
  }: {
    token?: string;
    onLog?: (level: "info" | "success" | "warn" | "error", message: string) => void;
  } = $props();

  let orgName = $state("ORGANIZATION-NAME");
  let role = $state<"member" | "maintainer">("member");
  let dryRun = $state(true);
  let rawInput = $state(`octocat,Phase 1 - Set 1
johndoe,Phase 1 - Set 1,Phase 2 - Set 1
janedoe,phase-1-set-1,phase-2-set-1
alexsmith,Phase 3 - Set 2`);

  let parsedEntries = $state<TeamInvitationEntry[]>([]);
  let lastParsedInput = $state("");
  let hasUnparsedChanges = $derived(rawInput.trim() !== lastParsedInput.trim());
  let isParsing = $state(false);
  let isRunning = $state(false);

  let currentProgress = $state<TeamInviteProgress | null>(null);
  let summary = $state<InvitationSummary | null>(null);
  let executionResults = $state<Array<{
    index: number;
    username: string;
    team: string;
    slug: string;
    status: string;
    error?: string | null;
  }>>([]);

  let unlistenProgress: (() => void) | null = null;

  onMount(async () => {
    await updatePreview();

    unlistenProgress = await listen<TeamInviteProgress>("team-invite-progress", (event) => {
      currentProgress = event.payload;
      executionResults = [
        ...executionResults,
        {
          index: event.payload.current_index,
          username: event.payload.username,
          team: event.payload.team_name,
          slug: event.payload.team_slug,
          status: event.payload.status,
          error: event.payload.error,
        },
      ];
    });
  });

  onDestroy(() => {
    if (unlistenProgress) {
      unlistenProgress();
    }
  });

  async function updatePreview() {
    isParsing = true;
    try {
      const res: TeamInvitationEntry[] = await invoke("preview_team_invitations", {
        inputText: rawInput,
      });
      parsedEntries = res;
      lastParsedInput = rawInput;
    } catch (e: any) {
      console.error("Preview error:", e);
      onLog("error", `Parsing error: ${e}`);
    } finally {
      isParsing = false;
    }
  }

  async function loadSample() {
    try {
      const sample: string = await invoke("load_sample_file", {
        sampleType: "team_invitations",
      });
      rawInput = sample;
      await updatePreview();
      onLog("info", "Loaded sample team invitations (001_xdummy_users.csv)");
    } catch (e: any) {
      onLog("error", `Failed to load sample: ${e}`);
    }
  }

  function handleFileUpload(event: Event) {
    const input = event.target as HTMLInputElement;
    if (input.files && input.files[0]) {
      const file = input.files[0];
      const reader = new FileReader();
      reader.onload = async (e) => {
        rawInput = (e.target?.result as string) || "";
        await updatePreview();
        onLog("info", `Uploaded and loaded file: ${file.name}`);
      };
      reader.readAsText(file);
    }
  }

  function clearAll() {
    rawInput = "";
    lastParsedInput = "";
    parsedEntries = [];
    currentProgress = null;
    summary = null;
    executionResults = [];
  }

  async function runInvitations() {
    if (hasUnparsedChanges) {
      await updatePreview();
    }
    if (!dryRun && !token.trim()) {
      alert("GitHub Token is required to execute real invitations. Please provide and verify your token in the top header.");
      return;
    }

    if (!orgName.trim()) {
      alert("Please enter a valid GitHub Organization Name.");
      return;
    }

    if (parsedEntries.length === 0) {
      alert("No valid invitation entries found. Please provide user and team data.");
      return;
    }

    isRunning = true;
    summary = null;
    executionResults = [];
    currentProgress = null;

    try {
      const result: InvitationSummary = await invoke("run_team_invitations", {
        token: token.trim(),
        orgName: orgName.trim(),
        role,
        dryRun,
        entries: parsedEntries,
      });
      summary = result;
      onLog(
        result.failed_count === 0 ? "success" : "warn",
        `Team invitations complete! Succeeded: ${result.success_count}, Failed: ${result.failed_count}`
      );
    } catch (err: any) {
      onLog("error", `Execution halted with error: ${err}`);
      alert(`Error during team invitation process: ${err}`);
    } finally {
      isRunning = false;
    }
  }

  let totalPlannedInvitations = $derived(
    parsedEntries.reduce((acc, curr) => acc + curr.teams.length, 0)
  );

  let progressPercent = $derived(
    currentProgress && currentProgress.total > 0
      ? Math.round((currentProgress.current_index / currentProgress.total) * 100)
      : 0
  );
</script>

<div class="workflow-container">
  <!-- Top Configuration Header Card -->
  <section class="pastel-card">
    <div class="card-headline">
      <div class="headline-left">
        <div class="workflow-tag">
          <Icon name="users" size={14} color="#4f46e5" />
          <span>MODULE 001</span>
        </div>
        <h2 class="workflow-title">GitHub Organization Team Invitations</h2>
      </div>
      <p class="workflow-desc">
        Batch invite and synchronize GitHub users into organization teams with role enforcement. Replicates <code>001_invite_teams.sh</code> logic natively without shell calls.
      </p>
    </div>

    <div class="config-grid">
      <div class="form-cell">
        <label for="org-name-input" class="form-label">
          <span>Target Organization</span>
          <span class="required-mark">*</span>
        </label>
        <div class="input-container">
          <input
            id="org-name-input"
            type="text"
            bind:value={orgName}
            placeholder="e.g. My-Organization"
            class="pastel-input"
          />
        </div>
        <span class="form-hint">Target GitHub organization where teams are managed</span>
      </div>

      <div class="form-cell">
        <label for="team-role-select" class="form-label">
          <span>Team Membership Role</span>
        </label>
        <div class="select-container">
          <select id="team-role-select" bind:value={role} class="pastel-select">
            <option value="member">member (Default membership)</option>
            <option value="maintainer">maintainer (Team administrative access)</option>
          </select>
        </div>
        <span class="form-hint">Assigned permission level for all invited users</span>
      </div>

      <!-- Execution Mode Switcher Card -->
      <div class="mode-switch-card {dryRun ? 'mode-dry-run' : 'mode-live'}">
        <div class="mode-header">
          <span class="mode-label">Execution Mode</span>
          {#if dryRun}
            <span class="mode-badge badge-warning">
              <Icon name="alert" size={12} color="#b45309" />
              SIMULATION ONLY
            </span>
          {:else}
            <span class="mode-badge badge-danger">
              <Icon name="shield" size={12} color="#be123c" />
              LIVE API MUTATIONS
            </span>
          {/if}
        </div>

        <label class="toggle-control" for="team-dry-run-toggle">
          <input
            id="team-dry-run-toggle"
            type="checkbox"
            bind:checked={dryRun}
            class="toggle-input"
          />
          <span class="toggle-slider"></span>
          <span class="toggle-caption">
            {dryRun ? "Dry-Run Active (No writes to GitHub)" : "Live Mode Active (Sends PUT requests)"}
          </span>
        </label>
      </div>
    </div>
  </section>

  <!-- Dual Workspace: Input & Preview -->
  <div class="split-workspace">
    <!-- Input Workspace -->
    <section class="pastel-card input-panel">
      <div class="panel-header">
        <div class="panel-title-group">
          <Icon name="file" size={16} color="#4f46e5" />
          <h3>1. Invitation List Input</h3>
        </div>
        <div class="panel-actions">
          <button type="button" class="action-btn" onclick={loadSample} title="Load sample users CSV">
            <Icon name="file" size={14} color="#4f46e5" />
            <span>Load Sample</span>
          </button>
          <label class="action-btn file-btn" title="Upload local CSV or TXT file">
            <Icon name="upload" size={14} color="#059669" />
            <span>Upload File</span>
            <input type="file" accept=".csv,.txt" onchange={handleFileUpload} />
          </label>
          <button type="button" class="action-btn btn-danger-ghost" onclick={clearAll} title="Clear text input">
            <Icon name="trash" size={14} color="#e11d48" />
            <span>Clear</span>
          </button>
        </div>
      </div>

      <div class="editor-container">
        <label for="invitations-raw-editor" class="visually-hidden">Invitations Raw Input</label>
        <textarea
          id="invitations-raw-editor"
          bind:value={rawInput}
          placeholder={`# Format: username,team1,team2,... or username|team1,team2
octocat,Phase 1 - Set 1
johndoe,Phase 1 - Set 1,Phase 2 - Set 1
janedoe,phase-1-set-1,phase-2-set-1`}
          rows={11}
          class="pastel-textarea"
          spellcheck="false"
        ></textarea>
      </div>

      <div class="panel-footer">
        <div class="syntax-guide-wrap">
          <span class="syntax-guide">
            Syntax: <code>username,team1,team2</code> or <code>username|team1,team2</code>.
          </span>
          {#if hasUnparsedChanges}
            <span class="unparsed-indicator">
              <Icon name="alert" size={12} color="#b45309" />
              Unparsed changes pending
            </span>
          {/if}
        </div>
        <button
          type="button"
          class="btn-reparse {hasUnparsedChanges ? 'btn-reparse-pending' : ''}"
          onclick={updatePreview}
          disabled={isParsing}
          title="Click or touch to parse invitation list and refresh pre-flight preview"
        >
          <Icon name="refresh" size={13} color={hasUnparsedChanges ? "#ffffff" : "#4f46e5"} />
          <span>{isParsing ? "Parsing..." : "Re-parse"}</span>
          {#if hasUnparsedChanges}
            <span class="pending-dot"></span>
          {/if}
        </button>
      </div>
    </section>

    <!-- Preview Workspace -->
    <section class="pastel-card preview-panel">
      <div class="panel-header">
        <div class="panel-title-group">
          <Icon name="check-circle" size={16} color="#059669" />
          <h3>2. Pre-Flight Preview</h3>
          {#if hasUnparsedChanges}
            <span class="preview-stale-tag" title="Click Re-parse to preview latest text input">
              Unparsed Changes
            </span>
          {/if}
        </div>
        <div class="badge-cluster">
          <span class="kpi-pill">
            <span class="kpi-val">{parsedEntries.length}</span>
            <span class="kpi-lbl">Users</span>
          </span>
          <span class="kpi-pill kpi-accent">
            <span class="kpi-val">{totalPlannedInvitations}</span>
            <span class="kpi-lbl">Invitations</span>
          </span>
        </div>
      </div>

      <div class="pro-table-wrapper">
        {#if parsedEntries.length === 0}
          <div class="empty-state-box">
            <Icon name="file" size={32} color="#94a3b8" />
            <p class="empty-text">No invitation entries detected.</p>
            <button type="button" class="btn-subtle" onclick={loadSample}>
              Load Sample Data
            </button>
          </div>
        {:else}
          <table class="pro-table">
            <thead>
              <tr>
                <th class="th-index">#</th>
                <th>GitHub User</th>
                <th>Target Teams & Normalized Slugs</th>
              </tr>
            </thead>
            <tbody>
              {#each parsedEntries as entry, i}
                <tr>
                  <td class="td-index">{i + 1}</td>
                  <td class="td-user">
                    <span class="user-handle">@{entry.username}</span>
                  </td>
                  <td>
                    <div class="team-capsules">
                      {#each entry.teams as team}
                        <div class="team-capsule" title="Slug: {team.slug}">
                          <span class="team-label">{team.display_name}</span>
                          <span class="team-slug-badge">{team.slug}</span>
                        </div>
                      {/each}
                    </div>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        {/if}
      </div>

      <div class="panel-cta-bar">
        <button
          type="button"
          class="btn-cta {dryRun ? 'btn-cta-simulate' : 'btn-cta-execute'}"
          onclick={runInvitations}
          disabled={isRunning || parsedEntries.length === 0}
        >
          {#if isRunning}
            <span class="btn-spinner"></span>
            <span>Executing Invitations...</span>
          {:else if dryRun}
            <Icon name="play" size={16} />
            <span>Run Simulation (Dry-Run)</span>
          {:else}
            <Icon name="play" size={16} />
            <span>Execute Live Team Invitations</span>
          {/if}
        </button>
      </div>
    </section>
  </div>

  <!-- Real-Time Execution Card -->
  {#if isRunning || summary || executionResults.length > 0}
    <section class="pastel-card execution-panel" aria-live="polite">
      <div class="panel-header">
        <div class="panel-title-group">
          <Icon name="terminal" size={18} color="#4f46e5" />
          <h3>3. Execution Progress & Audit Log</h3>
        </div>
        {#if summary}
          <div class="summary-kpis">
            <span class="summary-kpi kpi-success">
              <Icon name="check-circle" size={14} color="#059669" />
              {summary.success_count} Succeeded
            </span>
            {#if summary.failed_count > 0}
              <span class="summary-kpi kpi-danger">
                <Icon name="alert" size={14} color="#e11d48" />
                {summary.failed_count} Failed
              </span>
            {/if}
            <span class="summary-kpi kpi-mode">
              {summary.is_dry_run ? "DRY-RUN MODE" : "LIVE MODE"}
            </span>
          </div>
        {/if}
      </div>

      {#if isRunning && currentProgress}
        <div class="progress-container">
          <div class="progress-meta">
            <div class="progress-status-desc">
              <span class="progress-step-pill">[{currentProgress.current_index}/{currentProgress.total}]</span>
              <span>Inviting <strong>@{currentProgress.username}</strong> into <code>{currentProgress.team_slug}</code></span>
            </div>
            <span class="progress-percent-val">{progressPercent}%</span>
          </div>
          <div class="progress-bar-track">
            <div class="progress-bar-fill" style="width: {progressPercent}%"></div>
          </div>
        </div>
      {/if}

      <div class="results-table-scroll">
        <table class="pro-table">
          <thead>
            <tr>
              <th class="th-index">#</th>
              <th>Username</th>
              <th>Team Display</th>
              <th>Team Slug</th>
              <th>Execution Status</th>
            </tr>
          </thead>
          <tbody>
            {#each executionResults as res}
              <tr class={res.status.includes("FAILED") ? "tr-error" : ""}>
                <td class="td-index">{res.index}</td>
                <td class="td-user"><strong>@{res.username}</strong></td>
                <td>{res.team}</td>
                <td><code class="slug-code">{res.slug}</code></td>
                <td>
                  {#if res.status.includes("ACTIVE")}
                    <span class="status-tag tag-success">
                      <Icon name="check" size={12} color="#059669" />
                      ACTIVE MEMBER
                    </span>
                  {:else if res.status.includes("INVITATION SENT")}
                    <span class="status-tag tag-info">
                      <Icon name="check" size={12} color="#0284c7" />
                      INVITATION SENT
                    </span>
                  {:else if res.status.includes("DRY-RUN")}
                    <span class="status-tag tag-warning">
                      <Icon name="alert" size={12} color="#b45309" />
                      WOULD INVITE (DRY-RUN)
                    </span>
                  {:else}
                    <span class="status-tag tag-danger">
                      <Icon name="cross" size={12} color="#e11d48" />
                      FAILED
                    </span>
                    {#if res.error}
                      <span class="error-msg-text" title={res.error}>{res.error}</span>
                    {/if}
                  {/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </section>
  {/if}
</div>

<style>
  .workflow-container {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }

  .pastel-card {
    background: #ffffff;
    border: 1px solid #e2e8f0;
    border-radius: 12px;
    padding: 1.25rem 1.5rem;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.03), 0 1px 3px rgba(0, 0, 0, 0.02);
  }

  .card-headline {
    margin-bottom: 1.25rem;
  }

  .headline-left {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 0.35rem;
  }

  .workflow-tag {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.2rem 0.55rem;
    background: #eef2ff;
    border: 1px solid #c7d2fe;
    border-radius: 6px;
    font-family: 'JetBrains Mono', monospace;
    font-size: 0.7rem;
    font-weight: 700;
    color: #4f46e5;
    letter-spacing: 0.04em;
  }

  .workflow-title {
    font-family: 'JetBrains Mono', monospace;
    font-size: 1.15rem;
    font-weight: 700;
    color: #0f172a;
    margin: 0;
  }

  .workflow-desc {
    color: #475569;
    font-size: 0.825rem;
    margin: 0;
  }

  .workflow-desc code {
    font-family: 'JetBrains Mono', monospace;
    color: #1e293b;
    background: #f1f5f9;
    padding: 0.1rem 0.35rem;
    border-radius: 4px;
    border: 1px solid #e2e8f0;
  }

  .config-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
    gap: 1.25rem;
    align-items: start;
  }

  .form-cell {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .form-label {
    font-size: 0.825rem;
    font-weight: 600;
    color: #334155;
    display: flex;
    align-items: center;
    gap: 0.25rem;
    min-height: 20px;
    margin: 0;
  }

  .required-mark {
    color: #e11d48;
    line-height: 1;
  }

  .form-hint {
    font-size: 0.75rem;
    color: #64748b;
    min-height: 18px;
    line-height: 1.3;
    margin: 0;
  }

  .input-container,
  .select-container {
    width: 100%;
    height: 42px;
    display: flex;
    align-items: stretch;
    position: relative;
  }

  .pastel-input,
  .pastel-select {
    width: 100%;
    height: 42px;
    min-height: 42px;
    max-height: 42px;
    box-sizing: border-box;
    background: #f8fafc;
    border: 1px solid #cbd5e1;
    color: #0f172a;
    border-radius: 8px;
    padding: 0 0.85rem;
    font-size: 0.85rem;
    line-height: 40px;
    font-family: inherit;
    margin: 0;
    vertical-align: middle;
    transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .pastel-select {
    appearance: none;
    -webkit-appearance: none;
    -moz-appearance: none;
    background-color: #f8fafc;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='16' height='16' viewBox='0 0 24 24' fill='none' stroke='%2364748b' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpath d='m6 9 6 6 6-6'/%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 0.85rem center;
    background-size: 14px;
    padding-right: 2.2rem;
    cursor: pointer;
  }

  .pastel-input:focus,
  .pastel-select:focus {
    outline: none;
    border-color: #6366f1;
    background: #ffffff;
    box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.15);
  }

  /* Mode Switch Card */
  .mode-switch-card {
    border-radius: 10px;
    padding: 0.85rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    transition: all 0.2s ease;
  }

  .mode-switch-card.mode-dry-run {
    border: 1px solid #fde68a;
    background: #fffbeb;
  }

  .mode-switch-card.mode-live {
    border: 1px solid #fecdd3;
    background: #fff1f2;
  }

  .mode-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .mode-label {
    font-size: 0.8rem;
    font-weight: 700;
    color: #1e293b;
  }

  .mode-badge {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    font-family: 'JetBrains Mono', monospace;
    font-size: 0.675rem;
    font-weight: 700;
    padding: 0.15rem 0.45rem;
    border-radius: 4px;
  }

  .badge-warning {
    background: #fef3c7;
    color: #92400e;
    border: 1px solid #fde68a;
  }

  .badge-danger {
    background: #ffe4e6;
    color: #9f1239;
    border: 1px solid #fecdd3;
  }

  .toggle-control {
    display: flex;
    align-items: center;
    gap: 0.65rem;
    cursor: pointer;
    user-select: none;
  }

  .toggle-input {
    width: 18px;
    height: 18px;
    accent-color: #d97706;
    cursor: pointer;
  }

  .toggle-caption {
    font-size: 0.8rem;
    color: #334155;
    font-weight: 500;
  }

  /* Split Workspace */
  .split-workspace {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1.5rem;
  }

  @media (max-width: 960px) {
    .split-workspace {
      grid-template-columns: 1fr;
    }
  }

  .panel-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.85rem;
    flex-wrap: wrap;
    gap: 0.5rem;
  }

  .panel-title-group {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .panel-title-group h3 {
    margin: 0;
    font-family: 'JetBrains Mono', monospace;
    font-size: 0.95rem;
    font-weight: 600;
    color: #0f172a;
  }

  .panel-actions {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }

  .action-btn {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.35rem 0.65rem;
    background: #f8fafc;
    border: 1px solid #e2e8f0;
    color: #334155;
    border-radius: 6px;
    font-size: 0.775rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .action-btn:hover {
    background: #f1f5f9;
    color: #0f172a;
    border-color: #cbd5e1;
  }

  .file-btn {
    position: relative;
    overflow: hidden;
  }

  .file-btn input[type="file"] {
    position: absolute;
    top: 0;
    left: 0;
    opacity: 0;
    width: 100%;
    height: 100%;
    cursor: pointer;
  }

  .btn-danger-ghost:hover {
    background: #fff1f2;
    color: #e11d48;
    border-color: #fecdd3;
  }

  .editor-container {
    margin-bottom: 0.65rem;
  }

  .pastel-textarea {
    width: 100%;
    box-sizing: border-box;
    background: #f8fafc;
    border: 1px solid #cbd5e1;
    color: #0f172a;
    border-radius: 8px;
    padding: 0.75rem;
    font-family: 'JetBrains Mono', monospace;
    font-size: 0.825rem;
    line-height: 1.5;
    resize: vertical;
    transition: all 0.2s ease;
  }

  .pastel-textarea:focus {
    outline: none;
    border-color: #6366f1;
    background: #ffffff;
    box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.15);
  }

  .panel-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.75rem;
    flex-wrap: wrap;
  }

  .syntax-guide-wrap {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }

  .syntax-guide {
    font-size: 0.75rem;
    color: #64748b;
  }

  .syntax-guide code {
    font-family: 'JetBrains Mono', monospace;
    background: #f1f5f9;
    padding: 0.1rem 0.3rem;
    border-radius: 4px;
    color: #4f46e5;
    border: 1px solid #e2e8f0;
  }

  .unparsed-indicator {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    font-size: 0.725rem;
    font-weight: 600;
    color: #b45309;
    background: #fffbeb;
    border: 1px solid #fde68a;
    padding: 0.15rem 0.5rem;
    border-radius: 4px;
    width: fit-content;
  }

  .btn-reparse {
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
    background: #f8fafc;
    border: 1px solid #cbd5e1;
    color: #334155;
    padding: 0.42rem 0.85rem;
    border-radius: 7px;
    font-size: 0.775rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .btn-reparse:hover:not(:disabled) {
    background: #eef2ff;
    color: #4f46e5;
    border-color: #c7d2fe;
    box-shadow: 0 1px 3px rgba(79, 70, 229, 0.12);
  }

  .btn-reparse-pending {
    background: #4f46e5 !important;
    border-color: #4338ca !important;
    color: #ffffff !important;
    box-shadow: 0 2px 8px rgba(79, 70, 229, 0.3);
  }

  .btn-reparse-pending:hover:not(:disabled) {
    background: #4338ca !important;
    box-shadow: 0 4px 12px rgba(79, 70, 229, 0.4);
  }

  .pending-dot {
    width: 6px;
    height: 6px;
    background: #ffffff;
    border-radius: 50%;
    animation: pulse-dot 1.5s infinite;
  }

  @keyframes pulse-dot {
    0%, 100% { opacity: 1; transform: scale(1); }
    50% { opacity: 0.4; transform: scale(0.8); }
  }

  .preview-stale-tag {
    display: inline-flex;
    align-items: center;
    font-size: 0.675rem;
    font-weight: 600;
    color: #b45309;
    background: #fffbeb;
    border: 1px solid #fde68a;
    padding: 0.15rem 0.5rem;
    border-radius: 9999px;
  }

  .btn-subtle {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    background: #f8fafc;
    border: 1px solid #cbd5e1;
    color: #334155;
    padding: 0.35rem 0.65rem;
    border-radius: 6px;
    font-size: 0.775rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .btn-subtle:hover:not(:disabled) {
    background: #eef2ff;
    color: #4f46e5;
    border-color: #c7d2fe;
  }

  /* Badge Cluster */
  .badge-cluster {
    display: flex;
    gap: 0.4rem;
  }

  .kpi-pill {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.2rem 0.6rem;
    background: #f1f5f9;
    border: 1px solid #e2e8f0;
    border-radius: 9999px;
  }

  .kpi-val {
    font-family: 'JetBrains Mono', monospace;
    font-weight: 700;
    color: #0f172a;
    font-size: 0.8rem;
  }

  .kpi-lbl {
    font-size: 0.75rem;
    color: #64748b;
  }

  .kpi-accent {
    border-color: #c7d2fe;
    background: #eef2ff;
  }

  .kpi-accent .kpi-val {
    color: #4f46e5;
  }

  /* Table */
  .pro-table-wrapper,
  .results-table-scroll {
    max-height: 320px;
    overflow-y: auto;
    border: 1px solid #e2e8f0;
    border-radius: 8px;
    background: #ffffff;
  }

  .pro-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.825rem;
    text-align: left;
  }

  .pro-table th {
    background: #f8fafc;
    color: #475569;
    font-family: 'JetBrains Mono', monospace;
    font-size: 0.75rem;
    font-weight: 600;
    padding: 0.65rem 0.85rem;
    position: sticky;
    top: 0;
    z-index: 2;
    border-bottom: 1px solid #e2e8f0;
  }

  .pro-table td {
    padding: 0.65rem 0.85rem;
    border-bottom: 1px solid #f1f5f9;
    color: #1e293b;
  }

  .th-index,
  .td-index {
    width: 35px;
    text-align: center;
    color: #94a3b8;
    font-family: 'JetBrains Mono', monospace;
  }

  .td-user {
    font-family: 'JetBrains Mono', monospace;
    color: #4f46e5;
    font-weight: 600;
  }

  .team-capsules {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
  }

  .team-capsule {
    display: inline-flex;
    flex-direction: column;
    background: #f8fafc;
    border: 1px solid #e2e8f0;
    padding: 0.25rem 0.55rem;
    border-radius: 6px;
    gap: 0.1rem;
  }

  .team-label {
    font-size: 0.775rem;
    font-weight: 600;
    color: #0f172a;
  }

  .team-slug-badge {
    font-family: 'JetBrains Mono', monospace;
    font-size: 0.7rem;
    color: #6366f1;
  }

  .empty-state-box {
    padding: 3rem 1.5rem;
    text-align: center;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.65rem;
  }

  .empty-text {
    font-size: 0.85rem;
    color: #64748b;
    margin: 0;
  }

  .panel-cta-bar {
    margin-top: 1rem;
    display: flex;
    justify-content: flex-end;
  }

  .btn-cta {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.75rem 1.5rem;
    font-size: 0.9rem;
    font-weight: 600;
    border-radius: 8px;
    cursor: pointer;
    border: 1px solid transparent;
    transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .btn-cta-simulate {
    background: #d97706;
    color: #ffffff;
    box-shadow: 0 2px 6px rgba(217, 119, 6, 0.25);
  }

  .btn-cta-simulate:hover:not(:disabled) {
    background: #b45309;
    box-shadow: 0 4px 10px rgba(217, 119, 6, 0.35);
  }

  .btn-cta-execute {
    background: #4f46e5;
    color: #ffffff;
    box-shadow: 0 2px 6px rgba(79, 70, 229, 0.25);
  }

  .btn-cta-execute:hover:not(:disabled) {
    background: #4338ca;
    box-shadow: 0 4px 10px rgba(79, 70, 229, 0.35);
  }

  .btn-cta:disabled {
    opacity: 0.5;
    cursor: not-allowed;
    box-shadow: none;
  }

  /* Execution Progress Panel */
  .execution-panel {
    border-color: #cbd5e1;
  }

  .summary-kpis {
    display: flex;
    gap: 0.5rem;
  }

  .summary-kpi {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    font-family: 'JetBrains Mono', monospace;
    font-size: 0.75rem;
    font-weight: 600;
    padding: 0.25rem 0.65rem;
    border-radius: 6px;
  }

  .kpi-success {
    background: #ecfdf5;
    color: #065f46;
    border: 1px solid #a7f3d0;
  }

  .kpi-danger {
    background: #fff1f2;
    color: #9f1239;
    border: 1px solid #fecdd3;
  }

  .kpi-mode {
    background: #fffbeb;
    color: #92400e;
    border: 1px solid #fde68a;
  }

  .progress-container {
    background: #f8fafc;
    border: 1px solid #e2e8f0;
    padding: 0.85rem 1rem;
    border-radius: 8px;
    margin-bottom: 1rem;
  }

  .progress-meta {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 0.8rem;
    margin-bottom: 0.5rem;
    color: #1e293b;
  }

  .progress-step-pill {
    font-family: 'JetBrains Mono', monospace;
    color: #4f46e5;
    font-weight: 700;
  }

  .progress-percent-val {
    font-family: 'JetBrains Mono', monospace;
    font-weight: 700;
    color: #059669;
  }

  .progress-bar-track {
    height: 7px;
    background: #e2e8f0;
    border-radius: 4px;
    overflow: hidden;
  }

  .progress-bar-fill {
    height: 100%;
    background: linear-gradient(90deg, #6366f1, #10b981);
    border-radius: 4px;
    transition: width 0.3s ease;
  }

  .slug-code {
    font-family: 'JetBrains Mono', monospace;
    color: #475569;
    background: #f1f5f9;
    padding: 0.1rem 0.3rem;
    border-radius: 4px;
  }

  .status-tag {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    font-family: 'JetBrains Mono', monospace;
    font-size: 0.7rem;
    font-weight: 700;
    padding: 0.2rem 0.55rem;
    border-radius: 4px;
  }

  .tag-success {
    background: #ecfdf5;
    color: #065f46;
    border: 1px solid #a7f3d0;
  }

  .tag-info {
    background: #f0f9ff;
    color: #0369a1;
    border: 1px solid #bae6fd;
  }

  .tag-warning {
    background: #fffbeb;
    color: #92400e;
    border: 1px solid #fde68a;
  }

  .tag-danger {
    background: #fff1f2;
    color: #9f1239;
    border: 1px solid #fecdd3;
  }

  .error-msg-text {
    display: block;
    font-size: 0.725rem;
    color: #e11d48;
    margin-top: 0.2rem;
    max-width: 250px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tr-error {
    background: #fff1f2;
  }

  .btn-spinner {
    width: 14px;
    height: 14px;
    border: 2px solid rgba(255, 255, 255, 0.3);
    border-radius: 50%;
    border-top-color: #ffffff;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .visually-hidden {
    position: absolute;
    clip-path: inset(50%);
    overflow: hidden;
    width: 1px;
    height: 1px;
    margin: -1px;
    padding: 0;
    border: 0;
    white-space: nowrap;
  }
</style>
