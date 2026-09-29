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
  import * as jsyaml from "js-yaml";

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

  interface AssignmentRow {
    user: string;
    prefix: string;
    templates: string;
  }

  // User assignments input
  let assignments = $state<AssignmentRow[]>([]);
  let newAssignUser = $state("");
  let newAssignPrefix = $state("");
  let newAssignTemplates = $state("");

  let parsedPlans = $state<ProvisionPlan[]>([]);
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
  let previewTimer: ReturnType<typeof setTimeout> | null = null;

  $effect(() => {
    const inputText = getRawAssignmentsText();
    const _catalogSnapshot = templates.map((t) => `${t.key ?? ""}|${t.repo}|${t.deadline}`).join(";;");
    const currentOrg = orgName.trim();

    if (previewTimer) clearTimeout(previewTimer);
    previewTimer = setTimeout(() => {
      updatePreview(inputText, templates, currentOrg);
    }, 60);

    return () => {
      if (previewTimer) clearTimeout(previewTimer);
    };
  });

  onMount(async () => {
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
    if (previewTimer) clearTimeout(previewTimer);
    if (unlistenProgress) {
      unlistenProgress();
    }
  });

  function getRawAssignmentsText(): string {
    return assignments
      .filter((a) => a.user.trim())
      .map((a) => {
        const u = a.user.trim();
        const p = a.prefix.trim();
        const t = a.templates.trim();
        return `${u}|${p}|${t}`;
      })
      .join("\n");
  }

  function isTemplateInCatalog(query: string): boolean {
    const q = query.trim().toLowerCase();
    if (!q || q === "all" || q === "*") return true;
    return templates.some((t) => {
      const candRepo = t.repo.trim().toLowerCase();
      const candBase = (candRepo.split("/")[1] || candRepo).trim();
      const candClean = candBase.replace(/^(template[-_]|[-_]template)/, "");
      const candKey = (t.key || "").trim().toLowerCase();
      return q === candRepo || q === candBase || q === candClean || (candKey !== "" && q === candKey);
    });
  }

  function getInvalidTemplates(templateStr: string): string[] {
    if (!templateStr.trim()) return [];
    return templateStr
      .split(",")
      .map((s) => s.trim())
      .filter((s) => s && s.toLowerCase() !== "all" && s !== "*" && !isTemplateInCatalog(s));
  }

  async function updatePreview(inputText?: string, catalog?: TemplateEntry[], targetOrg?: string) {
    try {
      const text = inputText !== undefined ? inputText : getRawAssignmentsText();
      const cat = catalog !== undefined ? catalog : templates;
      const org = targetOrg !== undefined ? targetOrg : orgName;
      const plans: ProvisionPlan[] = await invoke("preview_repo_provisioning", {
        inputText: text,
        catalog: cat,
        orgName: org,
      });
      parsedPlans = plans;
    } catch (e: any) {
      console.error("Provisioning preview error:", e);
      onLog("error", `Provisioning preview error: ${e}`);
    }
  }

  function addAssignment() {
    if (!newAssignUser.trim()) return;

    if (templates.length === 0) {
      alert("Please add at least one template to the Template Repository Catalog before assigning templates to students.");
      return;
    }

    const invalid = getInvalidTemplates(newAssignTemplates);
    if (invalid.length > 0) {
      alert(
        `The following assigned template(s) are not in the Template Repository Catalog:\n- ${invalid.join("\n- ")}\n\nPlease choose from existing catalog templates or add them to the catalog first.`
      );
      return;
    }

    assignments = [
      ...assignments,
      {
        user: newAssignUser.trim(),
        prefix: newAssignPrefix.trim(),
        templates: newAssignTemplates.trim(),
      },
    ];
    newAssignUser = "";
    newAssignPrefix = "";
    newAssignTemplates = "";
  }

  function removeAssignment(index: number) {
    assignments = assignments.filter((_, i) => i !== index);
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

  function handleProvisioningYamlImport(event: Event) {
    const input = event.target as HTMLInputElement;
    if (input.files && input.files[0]) {
      const file = input.files[0];
      const reader = new FileReader();
      reader.onload = async (e) => {
        try {
          const content = (e.target?.result as string) || "";
          if (!content.trim()) return;

          const parsed = jsyaml.load(content) as any;
          if (!parsed || typeof parsed !== "object") {
            throw new Error("Invalid YAML structure: Expected a root YAML mapping/object.");
          }

          // 1. Organization Name
          const org = parsed.organization || parsed.organization_name || parsed.org || parsed.org_name;
          if (org) {
            orgName = String(org).trim();
          }

          // 2. List of Maintainers / Reviewers
          const rawMaintainers = parsed.maintainers || parsed.list_of_maintainers || parsed.reviewers || parsed.reviewers_list;
          if (Array.isArray(rawMaintainers)) {
            reviewersInput = rawMaintainers.map((m) => String(m).trim()).filter(Boolean).join(", ");
          } else if (typeof rawMaintainers === "string") {
            reviewersInput = rawMaintainers.trim();
          }

          // 3. Template Repository Catalog
          const rawCatalog =
            parsed.template_catalog ||
            parsed.template_repository_catalog ||
            parsed.templates ||
            parsed.catalog;
          if (Array.isArray(rawCatalog) && rawCatalog.length > 0) {
            templates = rawCatalog
              .map((item: any) => ({
                key: item.key ? String(item.key).trim() : null,
                repo: String(item.repo || item.repository || item.name || "").trim(),
                deadline: normalizeDeadline(item.deadline || item.due_date),
              }))
              .filter((t: TemplateEntry) => t.repo.length > 0);
          }

          // 4. User Assignments Inputs
          const rawAssignments =
            parsed.assignments ||
            parsed.user_assignments ||
            parsed.user_assignments_inputs;
          let assignmentRows: AssignmentRow[] = [];
          if (Array.isArray(rawAssignments)) {
            for (const item of rawAssignments) {
              if (typeof item === "string") {
                const parts = item.split(",").map((p) => p.trim());
                if (parts.length >= 3) {
                  assignmentRows.push({ user: parts[0], prefix: parts[1], templates: parts.slice(2).join(", ") });
                } else if (parts.length === 2) {
                  assignmentRows.push({ user: parts[0], prefix: "", templates: parts[1] });
                } else if (parts.length === 1 && parts[0]) {
                  assignmentRows.push({ user: parts[0], prefix: "", templates: "" });
                }
              } else if (item && typeof item === "object") {
                const user = item.user || item.username || item.student || "";
                const prefix = item.prefix || item.cohort || item.batch || "";
                const tmpls = item.templates || item.template_list || item.template || [];
                const tmplStr = Array.isArray(tmpls) ? tmpls.join(", ") : String(tmpls);
                if (user) {
                  assignmentRows.push({
                    user: String(user).trim(),
                    prefix: String(prefix).trim(),
                    templates: tmplStr.trim(),
                  });
                }
              }
            }
          } else if (typeof rawAssignments === "string") {
            const lines = rawAssignments.split(/\r?\n/).map((l) => l.trim()).filter(Boolean);
            for (const line of lines) {
              const parts = line.split(",").map((p) => p.trim());
              if (parts.length >= 3) {
                assignmentRows.push({ user: parts[0], prefix: parts[1], templates: parts.slice(2).join(", ") });
              } else if (parts.length === 2) {
                assignmentRows.push({ user: parts[0], prefix: "", templates: parts[1] });
              } else if (parts.length === 1 && parts[0]) {
                assignmentRows.push({ user: parts[0], prefix: "", templates: "" });
              }
            }
          }

          assignments = assignmentRows;
          await updatePreview();
          onLog(
            "success",
            `Imported YAML (${file.name}): Org: "${orgName || "N/A"}", ${templates.length} template(s), ${parsedPlans.length} plan(s).`
          );
        } catch (err: any) {
          console.error("Provisioning YAML Import Error:", err);
          onLog("error", `Failed to import YAML: ${err.message || err}`);
          alert(`Failed to import YAML: ${err.message || err}`);
        } finally {
          input.value = "";
        }
      };
      reader.readAsText(file);
    }
  }

  function exportProvisioningYamlFile() {
    const maintainersList = reviewersInput
      .split(",")
      .map((r) => r.trim())
      .filter(Boolean);

    const catalogList = templates.map((t) => ({
      key: t.key || null,
      repo: t.repo,
      deadline: t.deadline,
    }));

    const assignmentObjects = assignments
      .filter((a) => a.user.trim())
      .map((a) => {
        const tmpls = a.templates
          .split(",")
          .map((t) => t.trim())
          .filter(Boolean);
        const obj: any = { user: a.user.trim() };
        if (a.prefix.trim()) {
          obj.prefix = a.prefix.trim();
        }
        obj.templates = tmpls;
        return obj;
      });

    const doc: any = {
      organization: orgName.trim() || "sample-org",
      maintainers: maintainersList,
      template_catalog: catalogList,
      assignments: assignmentObjects,
    };

    const yamlStr = jsyaml.dump(doc, { indent: 2, lineWidth: -1 });
    const blob = new Blob([yamlStr], { type: "text/yaml" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    const cleanOrg = orgName.trim().replace(/[^a-zA-Z0-9_-]/g, "_") || "repo_provisioning";
    a.download = `repo_provisioning_${cleanOrg}.yaml`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
    onLog("info", `Exported repository provisioning configuration to ${a.download}`);
  }

  function clearAll() {
    orgName = "";
    reviewersInput = "";
    templates = [];
    assignments = [];
    parsedPlans = [];
    currentProgress = null;
    summary = null;
    executionResults = [];
    onLog("info", "Cleared all repository provisioning configurations, templates, and assignments.");
  }

  function clearAssignments() {
    assignments = [];
    parsedPlans = [];
    currentProgress = null;
    summary = null;
    executionResults = [];
    onLog("info", "Cleared all user assignments.");
  }

  async function runProvisioning() {
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
      <div class="card-headline-top">
        <div class="headline-left">
          <div class="workflow-tag">
            <Icon name="repo" size={14} color="#4f46e5" />
            <span>MODULE 002</span>
          </div>
          <h2 class="workflow-title">
            Assignment Repository Provisioning
          </h2>
        </div>
        <div class="headline-actions">
          <label class="action-btn file-btn" title="Import complete Repository Provisioning configuration from YAML file (.yaml, .yml)">
            <Icon name="upload" size={14} color="#059669" />
            <span>Import YAML</span>
            <input type="file" accept=".yaml,.yml,.txt" onchange={handleProvisioningYamlImport} />
          </label>
          <button
            type="button"
            class="action-btn"
            onclick={exportProvisioningYamlFile}
            disabled={!orgName && templates.length === 0 && assignments.length === 0}
            title="Export complete Repository Provisioning configuration as YAML file"
          >
            <Icon name="download" size={14} color="#4f46e5" />
            <span>Export YAML</span>
          </button>
          <button
            type="button"
            class="action-btn btn-danger-ghost"
            onclick={clearAll}
            title="Clear all configuration, templates, and assignments"
          >
            <Icon name="trash" size={14} color="#e11d48" />
            <span>Clear</span>
          </button>
        </div>
      </div>
      <p class="workflow-desc">
        Automated provisioning pipeline: Template generation &rarr; User write access &rarr; Reviewer maintain access &rarr; Deadline milestone & issue notification &rarr; Feedback Pull Request.
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
        <h3>1. Template Repository Catalog</h3>
        <span class="count-tag">{templates.length} templates</span>
      </div>
      <div class="catalog-actions">
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
          <p class="empty-text">No templates in catalog. Add a new template below or import a YAML configuration.</p>
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

  <!-- Stacked Workspace: Input on top, Matrix below -->
  <div class="split-workspace">
    <!-- Input Workspace -->
    <section class="pastel-card input-panel">
      <div class="panel-header">
        <div class="panel-title-group">
          <Icon name="file" size={16} color="#4f46e5" />
          <h3>2. User Assignments Input</h3>
          <span class="count-tag">{assignments.length} assignments</span>
        </div>
        <div class="panel-actions">
          <button type="button" class="action-btn btn-danger-ghost" onclick={clearAssignments} title="Clear all assignments">
            <Icon name="trash" size={14} color="#e11d48" />
            <span>Clear</span>
          </button>
        </div>
      </div>

      <!-- Assignments Table -->
      <div class="catalog-table-wrapper">
        {#if assignments.length === 0}
          <div class="empty-state-box catalog-empty">
            <Icon name="file" size={28} color="#94a3b8" />
            <p class="empty-text">No assignments in list. Add an assignment below or import a YAML configuration.</p>
          </div>
        {:else}
          <table class="pro-table catalog-table">
            <thead>
              <tr>
                <th class="th-index">#</th>
                <th style="width: 220px;">Student / Username</th>
                <th style="width: 180px;">Prefix / Cohort</th>
                <th>Assigned Templates (comma-separated keys)</th>
                <th style="width: 60px; text-align: center;">Action</th>
              </tr>
            </thead>
            <tbody>
              {#each assignments as item, i}
                <tr>
                  <td class="td-index">{i + 1}</td>
                  <td>
                    <input
                      type="text"
                      bind:value={item.user}
                      placeholder="e.g. student-alice"
                      class="table-cell-input user-cell-input"
                    />
                  </td>
                  <td>
                    <input
                      type="text"
                      bind:value={item.prefix}
                      placeholder="e.g. BATCH-01-DEV"
                      class="table-cell-input"
                    />
                  </td>
                  <td>
                    <input
                      type="text"
                      bind:value={item.templates}
                      placeholder="e.g. Web-Frontend, Backend-API"
                      class="table-cell-input"
                      list="catalog-templates-options"
                    />
                    {#if getInvalidTemplates(item.templates).length > 0}
                      <div class="cell-warning-tag" title="These templates do not exist in the Template Repository Catalog and are excluded from the pre-flight matrix">
                        <Icon name="alert" size={11} color="#b45309" />
                        <span>Not in catalog: {getInvalidTemplates(item.templates).join(", ")}</span>
                      </div>
                    {/if}
                  </td>
                  <td style="text-align: center;">
                    <button
                      type="button"
                      class="btn-icon-danger"
                      onclick={() => removeAssignment(i)}
                      title="Remove assignment"
                      aria-label="Remove assignment"
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

      <!-- Inline Add Assignment Form -->
      <div class="add-template-bar">
        <div class="add-bar-fields">
          <div class="add-field-group" style="flex: 1.2; min-width: 170px;">
            <label for="new-assign-user" class="add-field-label">Student / Username</label>
            <input
              id="new-assign-user"
              type="text"
              placeholder="e.g. student-alice"
              bind:value={newAssignUser}
              class="pastel-input"
            />
          </div>
          <div class="add-field-group" style="flex: 1; min-width: 140px;">
            <label for="new-assign-prefix" class="add-field-label">Prefix / Cohort (Optional)</label>
            <input
              id="new-assign-prefix"
              type="text"
              placeholder="e.g. BATCH-01-DEV"
              bind:value={newAssignPrefix}
              class="pastel-input"
            />
          </div>
          <div class="add-field-group" style="flex: 2; min-width: 230px;">
            <label for="new-assign-templates" class="add-field-label">Templates (comma-separated keys)</label>
            <input
              id="new-assign-templates"
              type="text"
              placeholder="e.g. Web-Frontend, Backend-API"
              bind:value={newAssignTemplates}
              class="pastel-input"
              list="catalog-templates-options"
            />
          </div>
        </div>
        <button
          type="button"
          class="btn-add-template"
          onclick={addAssignment}
          disabled={!newAssignUser.trim()}
        >
          <Icon name="plus" size={14} />
          <span>Add Assignment</span>
        </button>
      </div>

      <!-- Template options datalist from catalog -->
      <datalist id="catalog-templates-options">
        {#each templates as t}
          {#if t.key}
            <option value={t.key}>{t.repo} ({t.key})</option>
          {/if}
          <option value={t.repo}>{t.repo}</option>
        {/each}
      </datalist>

      <div class="panel-footer" style="margin-top: 0.85rem;">
        <div class="syntax-guide-wrap">
          <span class="syntax-guide">
            Specify student username, optional prefix, and template keys/names for each assignment, or import a Provisioning <code>.yaml</code> file.
          </span>
        </div>
      </div>
    </section>

    <!-- Pre-Flight Preview Plan -->
    <section class="pastel-card preview-panel">
      <div class="panel-header">
        <div class="panel-title-group">
          <Icon name="check-circle" size={16} color="#059669" />
          <h3>3. Pre-Flight Repository Matrix</h3>
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
            <p class="empty-text">No repository plans generated. Provide templates and user assignments above or import a YAML configuration.</p>
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
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .card-headline-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 0.75rem;
  }

  .headline-actions {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    flex-wrap: wrap;
  }

  .headline-left {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    flex-wrap: wrap;
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

  /* Stacked Workspace: Input on top, Matrix below */
  .split-workspace {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
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

  .cell-warning-tag {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    margin-top: 0.25rem;
    font-size: 0.675rem;
    font-weight: 600;
    color: #b45309;
    background: #fffbeb;
    border: 1px solid #fde68a;
    padding: 0.1rem 0.4rem;
    border-radius: 4px;
    width: fit-content;
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

</style>
