<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount, onDestroy } from "svelte";
  import type {
    TemplateEntry,
    ProvisionPlan,
    RepoProvisionConfig,
    RepoProvisionProgress,
    ProvisionSummary,
  } from "$lib/types";
  import Icon from "./Icon.svelte";
  import DateTimePicker from "./DateTimePicker.svelte";

  let {
    token = "",
    onLog = (level: "info" | "success" | "warn" | "error", message: string) => {},
  }: {
    token?: string;
    onLog?: (level: "info" | "success" | "warn" | "error", message: string) => void;
  } = $props();

  // Global Configuration
  let orgName = $state("");
  let reviewersInput = $state("");
  let dryRun = $state(true);

  function normalizeDeadline(val?: string | null): string {
    if (!val || !val.trim()) return "2026-12-31T23:59";
    const trimmed = val.trim();
    if (/^\d{4}-\d{2}-\d{2}\s+\d{2}:\d{2}/.test(trimmed)) {
      return trimmed.replace(/\s+/, "T").slice(0, 16);
    }
    if (/^\d{4}-\d{2}-\d{2}$/.test(trimmed)) {
      return `${trimmed}T23:59`;
    }
    return trimmed.slice(0, 16);
  }

  // Template Catalog
  let templates = $state<TemplateEntry[]>([]);

  let newTemplateRepo = $state("");
  let newTemplateKey = $state("");
  let newTemplateDeadline = $state("2026-12-31T23:59");

  // User assignments input
  let rawAssignmentsInput = $state("");

  let parsedPlans = $state<ProvisionPlan[]>([]);
  let lastParsedInput = $state("");
  let lastParsedTemplatesJson = $state("");
  let lastParsedOrg = $state("");

  let hasUnparsedChanges = $derived(
    rawAssignmentsInput.trim() !== lastParsedInput.trim() ||
    JSON.stringify(templates) !== lastParsedTemplatesJson ||
    orgName.trim() !== lastParsedOrg.trim()
  );

  let isParsing = $state(false);
  let isRunning = $state(false);

  let currentProgress = $state<RepoProvisionProgress | null>(null);
  let summary = $state<ProvisionSummary | null>(null);
  let executionResults = $state<Array<{
    index: number;
    username: string;
    targetRepo: string;
    templateRepo: string;
    status: string;
    error?: string | null;
  }>>([]);

  let unlistenProgress: (() => void) | null = null;

  onMount(async () => {
    await updatePreview();

    unlistenProgress = await listen<RepoProvisionProgress>(
      "repo-provision-progress",
      (event) => {
        currentProgress = event.payload;
        executionResults = [
          ...executionResults,
          {
            index: event.payload.current_index,
            username: event.payload.username,
            targetRepo: event.payload.target_repo,
            templateRepo: event.payload.template_repo,
            status: event.payload.status,
            error: event.payload.error,
          },
        ];
      }
    );
  });

  onDestroy(() => {
    if (unlistenProgress) {
      unlistenProgress();
    }
  });

  async function updatePreview() {
    isParsing = true;
    try {
      const plans: ProvisionPlan[] = await invoke("preview_repo_provisioning", {
        inputText: rawAssignmentsInput,
        catalog: templates,
        orgName,
      });
      parsedPlans = plans;
      lastParsedInput = rawAssignmentsInput;
      lastParsedTemplatesJson = JSON.stringify(templates);
      lastParsedOrg = orgName;
    } catch (e: any) {
      console.error("Provisioning preview error:", e);
      onLog("error", `Provisioning preview error: ${e}`);
    } finally {
      isParsing = false;
    }
  }

  function addTemplate() {
    if (!newTemplateRepo.trim()) return;
    const repoTrimmed = newTemplateRepo.trim();
    const keyTrimmed = newTemplateKey.trim();
    templates = [
      ...templates,
      {
        key: keyTrimmed.length > 0 ? keyTrimmed : null,
        repo: repoTrimmed,
        deadline: normalizeDeadline(newTemplateDeadline),
      },
    ];
    newTemplateRepo = "";
    newTemplateKey = "";
  }

  function removeTemplate(index: number) {
    templates = templates.filter((_, i) => i !== index);
  }

  function clearTemplates() {
    templates = [];
    onLog("info", "Cleared all template repository entries.");
  }

  function saveCatalogToFile() {
    if (templates.length === 0) {
      alert("No templates in catalog to save.");
      return;
    }
    const dataStr = JSON.stringify(templates, null, 2);
    const blob = new Blob([dataStr], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    const cleanOrg = orgName.trim().replace(/[^a-zA-Z0-9_-]/g, "_") || "catalog";
    a.download = `template_catalog_${cleanOrg}.json`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
    onLog("info", `Saved ${templates.length} templates to template_catalog_${cleanOrg}.json`);
  }

  function handleCatalogImport(event: Event) {
    const input = event.target as HTMLInputElement;
    if (input.files && input.files[0]) {
      const file = input.files[0];
      const reader = new FileReader();
      reader.onload = (e) => {
        try {
          const content = (e.target?.result as string) || "";
          if (!content.trim()) return;

          let imported: TemplateEntry[] = [];

          if (file.name.endsWith(".json") || content.trim().startsWith("[") || content.trim().startsWith("{")) {
            const data = JSON.parse(content);
            const list = Array.isArray(data) ? data : (data.templates || []);
            imported = list
              .map((item: any) => ({
                key: item.key ? String(item.key).trim() : null,
                repo: String(item.repo || item.name || "").trim(),
                deadline: normalizeDeadline(item.deadline),
              }))
              .filter((t: TemplateEntry) => t.repo.length > 0);
          } else {
            // CSV / text lines:
            // Format: key,repo,deadline or repo,deadline or repo
            const lines = content.split(/\r?\n/);
            for (const line of lines) {
              const trimmed = line.trim();
              if (!trimmed || trimmed.startsWith("#")) continue;
              if (trimmed.toLowerCase().startsWith("repo") || trimmed.toLowerCase().startsWith("key,repo")) continue;

              const parts = trimmed.split(",").map((p) => p.trim());
              if (parts.length === 1) {
                imported.push({ key: null, repo: parts[0], deadline: "2026-12-31T23:59" });
              } else if (parts.length === 2) {
                if (parts[1].includes("-") || parts[1].includes(":")) {
                  imported.push({ key: null, repo: parts[0], deadline: normalizeDeadline(parts[1]) });
                } else {
                  imported.push({ key: parts[0], repo: parts[1], deadline: "2026-12-31T23:59" });
                }
              } else if (parts.length >= 3) {
                imported.push({
                  key: parts[0] ? parts[0] : null,
                  repo: parts[1],
                  deadline: normalizeDeadline(parts[2]),
                });
              }
            }
          }

          if (imported.length > 0) {
            templates = imported;
            onLog("success", `Imported ${imported.length} templates from ${file.name}`);
          } else {
            alert("No valid templates found in the imported file.");
          }
        } catch (err: any) {
          onLog("error", `Failed to import catalog file: ${err}`);
          alert(`Error importing catalog file: ${err.message || err}`);
        }
      };
      reader.readAsText(file);
      input.value = "";
    }
  }

  function handleFileUpload(event: Event) {
    const input = event.target as HTMLInputElement;
    if (input.files && input.files[0]) {
      const file = input.files[0];
      const reader = new FileReader();
      reader.onload = async (e) => {
        rawAssignmentsInput = (e.target?.result as string) || "";
        await updatePreview();
        onLog("info", `Uploaded and loaded file: ${file.name}`);
      };
      reader.readAsText(file);
    }
  }

  function clearAssignments() {
    rawAssignmentsInput = "";
    lastParsedInput = "";
    parsedPlans = [];
    currentProgress = null;
    summary = null;
    executionResults = [];
  }

  async function runProvisioning() {
    if (hasUnparsedChanges) {
      await updatePreview();
    }
    if (!dryRun && !token.trim()) {
      alert("GitHub Token is required to execute real repository provisioning. Please provide and verify your token in the top header.");
      return;
    }

    if (!orgName.trim()) {
      alert("Please enter a valid GitHub Organization Name.");
      return;
    }

    if (parsedPlans.length === 0) {
      alert("No valid repository provisioning plans found. Please check your assignments input.");
      return;
    }

    isRunning = true;
    summary = null;
    executionResults = [];
    currentProgress = null;

    const reviewersList = reviewersInput
      .split(",")
      .map((r) => r.trim())
      .filter((r) => r.length > 0);

    const config: RepoProvisionConfig = {
      org_name: orgName.trim(),
      reviewers: reviewersList,
      dry_run: dryRun,
    };

    try {
      const result: ProvisionSummary = await invoke("run_repo_provisioning", {
        token: token.trim(),
        config,
        plans: parsedPlans,
      });
      summary = result;
      onLog(
        result.failed_count === 0 ? "success" : "warn",
        `Repository provisioning finished! Succeeded: ${result.success_count}, Failed: ${result.failed_count}`
      );
    } catch (err: any) {
      onLog("error", `Provisioning error: ${err}`);
      alert(`Error during provisioning: ${err}`);
    } finally {
      isRunning = false;
    }
  }

  let progressPercent = $derived(
    currentProgress && currentProgress.total > 0
      ? Math.round((currentProgress.current_index / currentProgress.total) * 100)
      : 0
  );
</script>

<div class="workflow-container">
  <!-- Top Global Configuration Card -->
  <section class="pastel-card">
    <div class="card-headline">
      <div class="headline-left">
        <div class="workflow-tag">
          <Icon name="repo" size={14} color="#4f46e5" />
          <span>MODULE 002</span>
        </div>
        <h2 class="workflow-title">Assignment Repository Provisioning</h2>
      </div>
      <p class="workflow-desc">
        Automated provisioning pipeline: Template generation &rarr; User write access &rarr; Reviewer maintain access &rarr; Deadline milestone & issue notification &rarr; Feedback Pull Request. Replicates <code>002_create_repos.sh</code>.
      </p>
    </div>

    <div class="config-grid">
      <div class="form-cell">
        <label for="repo-org-input" class="form-label">
          <span>Target Organization</span>
          <span class="required-mark">*</span>
        </label>
        <div class="input-container">
          <input
            id="repo-org-input"
            type="text"
            bind:value={orgName}
            placeholder="e.g. ORGANIZATION-NAME"
            class="pastel-input"
          />
        </div>
        <span class="form-hint">Organization where student assignment repos are created</span>
      </div>

      <div class="form-cell">
        <label for="repo-reviewers-input" class="form-label">
          <span>Reviewers (Maintainers)</span>
        </label>
        <div class="input-container">
          <input
            id="repo-reviewers-input"
            type="text"
            bind:value={reviewersInput}
            placeholder="reviewer1, reviewer2"
            class="pastel-input"
          />
        </div>
        <span class="form-hint">Assigned maintain access and added to Feedback PR</span>
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
              LIVE PROVISIONING
            </span>
          {/if}
        </div>

        <label class="toggle-control" for="repo-dry-run-toggle">
          <input
            id="repo-dry-run-toggle"
            type="checkbox"
            bind:checked={dryRun}
            class="toggle-input"
          />
          <span class="toggle-slider"></span>
          <span class="toggle-caption">
            {dryRun ? "Dry-Run Active (Simulates repo creation)" : "Live Mode Active (Creates repos & branches)"}
          </span>
        </label>
      </div>
    </div>
  </section>

  <!-- Template Catalog Card -->
  <section class="pastel-card template-catalog-card">
    <div class="panel-header">
      <div class="panel-title-group">
        <Icon name="repo" size={16} color="#4f46e5" />
        <h3>Template Repository Catalog</h3>
        <span class="count-tag">{templates.length} templates</span>
      </div>
      <div class="catalog-actions">
        <!-- File Importer -->
        <label class="action-btn file-btn" title="Import templates from JSON or CSV file">
          <Icon name="upload" size={13} color="#059669" />
          <span>Import File</span>
          <input type="file" accept=".json,.csv,.txt" onchange={handleCatalogImport} />
        </label>

        <!-- File Saver -->
        <button
          type="button"
          class="action-btn"
          onclick={saveCatalogToFile}
          disabled={templates.length === 0}
          title="Save catalog to JSON file"
        >
          <Icon name="save" size={13} color="#4f46e5" />
          <span>Save File</span>
        </button>

        <!-- Clear Catalog -->
        <button
          type="button"
          class="action-btn btn-danger-ghost"
          onclick={clearTemplates}
          title="Clear all catalog templates"
        >
          <Icon name="trash" size={13} color="#e11d48" />
          <span>Clear</span>
        </button>
      </div>
    </div>

    <!-- Catalog Table -->
    <div class="catalog-table-wrapper">
      {#if templates.length === 0}
        <div class="empty-state-box catalog-empty">
          <Icon name="repo" size={28} color="#94a3b8" />
          <p class="empty-text">No templates in catalog. Add a new template below or import a catalog file.</p>
        </div>
      {:else}
        <table class="pro-table catalog-table">
          <thead>
            <tr>
              <th class="th-index">#</th>
              <th>Template Repository (Org/Repo)</th>
              <th style="width: 170px;">Short Key / Slug</th>
              <th style="width: 270px;">Deadline (Date & Time)</th>
              <th style="width: 60px; text-align: center;">Action</th>
            </tr>
          </thead>
          <tbody>
            {#each templates as t, i}
              <tr>
                <td class="td-index">{i + 1}</td>
                <td>
                  <input
                    type="text"
                    bind:value={t.repo}
                    placeholder="e.g. ORGANIZATION-NAME/REPO-NAME"
                    class="table-cell-input repo-cell-input"
                  />
                </td>
                <td>
                  <input
                    type="text"
                    value={t.key ?? ""}
                    oninput={(e) => t.key = (e.target as HTMLInputElement).value || null}
                    placeholder="Auto-slug if blank"
                    class="table-cell-input key-cell-input"
                  />
                </td>
                <td>
                  <DateTimePicker
                    bind:value={t.deadline}
                    compact={true}
                  />
                </td>
                <td style="text-align: center;">
                  <button
                    type="button"
                    class="btn-icon-danger"
                    onclick={() => removeTemplate(i)}
                    title="Remove template from catalog"
                    aria-label="Remove template"
                  >
                    <Icon name="trash" size={13} color="#e11d48" />
                  </button>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    </div>

    <!-- Inline Add Template Form -->
    <div class="add-template-bar">
      <div class="add-bar-fields">
        <div class="add-field-group field-repo">
          <label for="new-template-repo" class="add-field-label">New Template Repository</label>
          <input
            id="new-template-repo"
            type="text"
            placeholder="e.g. ORGANIZATION-NAME/REPO-NAME"
            bind:value={newTemplateRepo}
            class="pastel-input"
          />
        </div>
        <div class="add-field-group field-key">
          <label for="new-template-key" class="add-field-label">Key / Short Slug (Optional)</label>
          <input
            id="new-template-key"
            type="text"
            placeholder="Auto-extracted if blank"
            bind:value={newTemplateKey}
            class="pastel-input"
          />
        </div>
        <div class="add-field-group field-deadline">
          <label for="new-template-deadline" class="add-field-label">Deadline (Date & Time)</label>
          <DateTimePicker
            id="new-template-deadline"
            bind:value={newTemplateDeadline}
          />
        </div>
      </div>
      <button
        type="button"
        class="btn-add-template"
        onclick={addTemplate}
        disabled={!newTemplateRepo.trim()}
      >
        <Icon name="plus" size={14} />
        <span>Add Template</span>
      </button>
    </div>
  </section>

  <!-- Dual Workspace: Assignments Input & Pre-Flight Plan -->
  <div class="split-workspace">
    <!-- Input Workspace -->
    <section class="pastel-card input-panel">
      <div class="panel-header">
        <div class="panel-title-group">
          <Icon name="file" size={16} color="#4f46e5" />
          <h3>1. User Assignments Input</h3>
        </div>
        <div class="panel-actions">
          <label class="action-btn file-btn" title="Upload local CSV or TXT file">
            <Icon name="upload" size={14} color="#059669" />
            <span>Upload File</span>
            <input type="file" accept=".csv,.txt" onchange={handleFileUpload} />
          </label>
          <button type="button" class="action-btn btn-danger-ghost" onclick={clearAssignments} title="Clear text input">
            <Icon name="trash" size={14} color="#e11d48" />
            <span>Clear</span>
          </button>
        </div>
      </div>

      <div class="editor-container">
        <label for="assignments-raw-editor" class="visually-hidden">Assignments Raw Input</label>
        <textarea
          id="assignments-raw-editor"
          bind:value={rawAssignmentsInput}
          placeholder={`# Format options:
# 1. username,prefix,template1,template2
# 2. username|prefix|template1,template2
# 3. username|prefix (assigns all templates)`}
          rows={11}
          class="pastel-textarea"
          spellcheck="false"
        ></textarea>
      </div>

      <div class="panel-footer">
        <div class="syntax-guide-wrap">
          <span class="syntax-guide">
            Format: <code>user,prefix,templates</code> or <code>user|prefix|templates</code>.
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
          title="Click or touch to parse assignments and refresh pre-flight repository matrix"
        >
          <Icon name="refresh" size={13} color={hasUnparsedChanges ? "#ffffff" : "#4f46e5"} />
          <span>{isParsing ? "Parsing..." : "Re-parse"}</span>
          {#if hasUnparsedChanges}
            <span class="pending-dot"></span>
          {/if}
        </button>
      </div>
    </section>

    <!-- Pre-Flight Preview Plan -->
    <section class="pastel-card preview-panel">
      <div class="panel-header">
        <div class="panel-title-group">
          <Icon name="check-circle" size={16} color="#059669" />
          <h3>2. Pre-Flight Repository Matrix</h3>
          {#if hasUnparsedChanges}
            <span class="preview-stale-tag" title="Click Re-parse to preview latest assignments and catalog">
              Unparsed Changes
            </span>
          {/if}
        </div>
        <div class="badge-cluster">
          <span class="kpi-pill kpi-accent">
            <span class="kpi-val">{parsedPlans.length}</span>
            <span class="kpi-lbl">Repositories Planned</span>
          </span>
        </div>
      </div>

      <div class="pro-table-wrapper">
        {#if parsedPlans.length === 0}
          <div class="empty-state-box">
            <Icon name="repo" size={32} color="#94a3b8" />
            <p class="empty-text">No repository plans generated. Provide templates and user assignments above.</p>
          </div>
        {:else}
          <table class="pro-table">
            <thead>
              <tr>
                <th class="th-index">#</th>
                <th>Target Repository</th>
                <th>Student User / Cohort</th>
                <th>Source Template</th>
                <th>Deadline</th>
              </tr>
            </thead>
            <tbody>
              {#each parsedPlans as plan, i}
                <tr>
                  <td class="td-index">{i + 1}</td>
                  <td class="td-repo-name">
                    <span class="repo-name-badge">{plan.target_repo_name}</span>
                  </td>
                  <td>
                    <div class="user-cohort-tag">
                      <span class="user-bold">@{plan.username}</span>
                      {#if plan.prefix}
                        <span class="cohort-badge">{plan.prefix}</span>
                      {/if}
                    </div>
                  </td>
                  <td class="td-dim">{plan.template_repo}</td>
                  <td class="td-deadline">
                    <span class="deadline-chip" title="Deadline: {plan.deadline_iso || plan.deadline}">
                      <Icon name="calendar" size={11} color="#6366f1" />
                      <span class="chip-text">{plan.deadline.replace('T', ' ')}</span>
                    </span>
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
          onclick={runProvisioning}
          disabled={isRunning || parsedPlans.length === 0}
        >
          {#if isRunning}
            <span class="btn-spinner"></span>
            <span>Provisioning Repositories...</span>
          {:else if dryRun}
            <Icon name="play" size={16} />
            <span>Run Provisioning Simulation (Dry-Run)</span>
          {:else}
            <Icon name="play" size={16} />
            <span>Execute Live Repository Provisioning</span>
          {/if}
        </button>
      </div>
    </section>
  </div>

  <!-- Real-Time Provisioning Progress Card -->
  {#if isRunning || summary || executionResults.length > 0}
    <section class="pastel-card execution-panel" aria-live="polite">
      <div class="panel-header">
        <div class="panel-title-group">
          <Icon name="terminal" size={18} color="#4f46e5" />
          <h3>3. Provisioning Progress & Created Repositories</h3>
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
              <span><strong>{currentProgress.target_repo}</strong></span>
              <span class="active-step-tag">({currentProgress.step})</span>
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
              <th>Target Repository</th>
              <th>Student User</th>
              <th>Source Template</th>
              <th>Execution Status</th>
            </tr>
          </thead>
          <tbody>
            {#each executionResults as res}
              <tr class={res.status.includes("FAILED") ? "tr-error" : ""}>
                <td class="td-index">{res.index}</td>
                <td>
                  <span class="repo-target-title">{res.targetRepo}</span>
                  {#if !dryRun && res.status.includes("SUCCESS")}
                    <a
                      href="https://github.com/{res.targetRepo}"
                      target="_blank"
                      rel="noopener noreferrer"
                      class="github-direct-link"
                    >
                      <span>View on GitHub</span>
                      <Icon name="external" size={12} color="#4f46e5" />
                    </a>
                  {/if}
                </td>
                <td class="td-user">@{res.username}</td>
                <td class="td-dim">{res.templateRepo}</td>
                <td>
                  {#if res.status.includes("SUCCESS")}
                    <span class="status-tag tag-success">
                      <Icon name="check" size={12} color="#059669" />
                      SUCCESSFULLY PROVISIONED
                    </span>
                  {:else if res.status.includes("DRY-RUN")}
                    <span class="status-tag tag-warning">
                      <Icon name="alert" size={12} color="#b45309" />
                      WOULD CREATE (DRY-RUN)
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
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 1.25rem;
    align-items: start;
  }

  @media (max-width: 960px) {
    .config-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }

  @media (max-width: 640px) {
    .config-grid {
      grid-template-columns: 1fr;
    }
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
    display: block;
    margin: 0;
  }

  .input-container {
    width: 100%;
    height: 42px;
    display: flex;
    align-items: stretch;
  }

  .pastel-input {
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

  .pastel-input:focus {
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
    box-sizing: border-box;
    min-height: 88px;
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

  /* Template Catalog Table & Toolbar */
  .template-catalog-card {
    background: #ffffff;
  }

  .count-tag {
    font-size: 0.7rem;
    font-family: 'JetBrains Mono', monospace;
    font-weight: 700;
    color: #4f46e5;
    background: #eef2ff;
    border: 1px solid #c7d2fe;
    padding: 0.15rem 0.5rem;
    border-radius: 9999px;
  }

  .catalog-actions {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    flex-wrap: wrap;
  }

  .catalog-table-wrapper {
    max-height: 290px;
    overflow-y: auto;
    border: 1px solid #e2e8f0;
    border-radius: 8px;
    background: #ffffff;
    margin-bottom: 0.85rem;
  }

  .catalog-table th {
    font-family: 'JetBrains Mono', monospace;
    font-size: 0.725rem;
    font-weight: 700;
    color: #475569;
    background: #f8fafc;
    padding: 0.55rem 0.75rem;
    border-bottom: 1px solid #e2e8f0;
  }

  .catalog-table td {
    padding: 0.4rem 0.6rem;
    border-bottom: 1px solid #f1f5f9;
    vertical-align: middle;
  }

  .catalog-table tr:hover {
    background: #fbfcfe;
  }

  .catalog-empty {
    padding: 2.2rem 1rem;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
  }

  .table-cell-input {
    width: 100%;
    box-sizing: border-box;
    background: #f8fafc;
    border: 1px solid #e2e8f0;
    border-radius: 6px;
    padding: 0.4rem 0.6rem;
    font-size: 0.8rem;
    color: #0f172a;
    font-family: 'JetBrains Mono', monospace;
    transition: all 0.15s ease;
  }

  .table-cell-input:hover {
    border-color: #cbd5e1;
    background: #ffffff;
  }

  .table-cell-input:focus {
    outline: none;
    background: #ffffff;
    border-color: #6366f1;
    box-shadow: 0 0 0 2px rgba(99, 102, 241, 0.15);
  }

  .repo-cell-input {
    font-weight: 600;
    color: #1e293b;
  }

  .key-cell-input {
    color: #4f46e5;
  }

  .td-deadline {
    white-space: nowrap;
  }

  .deadline-chip {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    background: #f8fafc;
    border: 1px solid #e2e8f0;
    border-radius: 6px;
    padding: 0.2rem 0.5rem;
    font-family: 'JetBrains Mono', monospace;
    font-size: 0.75rem;
    color: #334155;
    font-weight: 500;
  }

  .deadline-chip :global(.svg-icon) {
    flex-shrink: 0;
  }

  .btn-icon-danger {
    background: none;
    border: 1px solid transparent;
    cursor: pointer;
    padding: 5px;
    border-radius: 6px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    transition: all 0.15s ease;
  }

  .btn-icon-danger:hover {
    background: #fff1f2;
    border-color: #fecdd3;
  }

  /* Inline Add Template Bar */
  .add-template-bar {
    display: flex;
    align-items: flex-end;
    gap: 0.85rem;
    background: #f8fafc;
    padding: 0.85rem 1rem;
    border-radius: 8px;
    border: 1px solid #e2e8f0;
    flex-wrap: wrap;
  }

  .add-bar-fields {
    display: flex;
    gap: 0.75rem;
    flex: 1;
    min-width: 300px;
    flex-wrap: wrap;
  }

  .add-field-group {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }

  .field-repo {
    flex: 2;
    min-width: 220px;
  }

  .field-key {
    flex: 1;
    min-width: 140px;
  }

  .field-deadline {
    flex: 1.6;
    min-width: 260px;
  }

  .add-field-label {
    font-size: 0.725rem;
    font-weight: 600;
    color: #475569;
  }

  .btn-add-template {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.4rem;
    background: #4f46e5;
    border: 1px solid #4338ca;
    color: #ffffff;
    padding: 0 1rem;
    border-radius: 8px;
    font-size: 0.8rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
    box-shadow: 0 1px 3px rgba(79, 70, 229, 0.2);
    height: 42px;
    box-sizing: border-box;
  }

  .btn-add-template:hover:not(:disabled) {
    background: #4338ca;
    box-shadow: 0 2px 6px rgba(79, 70, 229, 0.35);
  }

  .btn-add-template:disabled {
    opacity: 0.5;
    cursor: not-allowed;
    background: #94a3b8;
    border-color: #cbd5e1;
    box-shadow: none;
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
    white-space: pre;
    overflow-x: auto;
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

  /* Pro Table */
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

  .td-repo-name {
    font-family: 'JetBrains Mono', monospace;
  }

  .repo-name-badge {
    color: #4f46e5;
    background: #eef2ff;
    padding: 0.2rem 0.5rem;
    border-radius: 4px;
    border: 1px solid #c7d2fe;
    font-weight: 600;
  }

  .user-cohort-tag {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }

  .user-bold {
    font-family: 'JetBrains Mono', monospace;
    font-weight: 600;
    color: #0f172a;
  }

  .cohort-badge {
    font-family: 'JetBrains Mono', monospace;
    font-size: 0.7rem;
    background: #f1f5f9;
    border: 1px solid #e2e8f0;
    padding: 0.1rem 0.35rem;
    border-radius: 4px;
    color: #64748b;
  }

  .td-dim {
    color: #64748b;
    font-family: 'JetBrains Mono', monospace;
    font-size: 0.775rem;
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

  .active-step-tag {
    font-family: 'JetBrains Mono', monospace;
    color: #059669;
    font-weight: 600;
    margin-left: 0.35rem;
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

  .repo-target-title {
    font-family: 'JetBrains Mono', monospace;
    font-weight: 600;
    color: #0f172a;
  }

  .github-direct-link {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    margin-left: 0.65rem;
    font-size: 0.75rem;
    color: #4f46e5;
    text-decoration: none;
    font-family: 'IBM Plex Sans', sans-serif;
    font-weight: 500;
  }

  .github-direct-link:hover {
    text-decoration: underline;
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
