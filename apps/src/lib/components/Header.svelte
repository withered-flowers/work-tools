<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import type { GitHubUser } from "$lib/types";
  import Icon from "./Icon.svelte";

  let {
    token = $bindable(""),
    user = $bindable(null as GitHubUser | null),
    onLog = (level: "info" | "success" | "warn" | "error", message: string) => {},
  }: {
    token?: string;
    user?: GitHubUser | null;
    onLog?: (level: "info" | "success" | "warn" | "error", message: string) => void;
  } = $props();

  let showPassword = $state(false);
  let isCheckingToken = $state(false);
  let statusMessage = $state("");

  onMount(async () => {
    const savedToken = localStorage.getItem("gh_pat_token");
    if (savedToken) {
      token = savedToken;
      await verifyToken(false);
    } else {
      await detectGhCliToken();
    }
  });

  async function detectGhCliToken() {
    try {
      const cliToken: string | null = await invoke("get_gh_cli_token");
      if (cliToken) {
        token = cliToken;
        localStorage.setItem("gh_pat_token", cliToken);
        statusMessage = "Token detected from GitHub CLI ('gh')";
        onLog("info", "GitHub token automatically detected from GitHub CLI ('gh')");
        await verifyToken(false);
      }
    } catch (e) {
      console.warn("Could not check gh CLI token:", e);
    }
  }

  export async function verifyToken(notifySuccess = true) {
    if (!token.trim()) {
      statusMessage = "Please enter a GitHub Personal Access Token";
      user = null;
      return;
    }

    isCheckingToken = true;
    statusMessage = "Verifying with GitHub REST API...";

    try {
      const u: GitHubUser = await invoke("verify_github_token", { token: token.trim() });
      user = u;
      localStorage.setItem("gh_pat_token", token.trim());
      statusMessage = `Authenticated as @${u.login}`;
      if (notifySuccess) {
        onLog("success", `Successfully authenticated as GitHub user: @${u.login}`);
      }
    } catch (err: any) {
      user = null;
      statusMessage = `Authentication failed: ${err}`;
      onLog("error", `Authentication verification failed: ${err}`);
    } finally {
      isCheckingToken = false;
    }
  }

  function handleTokenChange() {
    localStorage.setItem("gh_pat_token", token.trim());
    user = null;
    statusMessage = "Token updated. Click 'Verify' to test.";
  }
</script>

<header class="app-header">
  <div class="header-left">
    <div class="brand-container">
      <div class="brand-icon">
        <Icon name="github" size={24} color="#4f46e5" />
      </div>
      <div class="brand-meta">
        <div class="brand-title-row">
          <span class="brand-title">GitHub Automation Studio</span>
        </div>
        <span class="brand-subtitle">Team Invitations & Repository Provisioning Orchestrator</span>
      </div>
    </div>
  </div>

  <div class="header-right">
    <div class="auth-toolbar">
      <div class="token-field-wrapper">
        <span class="token-prefix-icon">
          <Icon name="shield" size={16} color="#6366f1" />
        </span>
        <label for="gh-token-input" class="visually-hidden">GitHub Personal Access Token</label>
        <input
          id="gh-token-input"
          type={showPassword ? "text" : "password"}
          placeholder="ghp_... (Personal Access Token)"
          bind:value={token}
          oninput={handleTokenChange}
          autocomplete="current-password"
          class="token-input"
        />
        <button
          type="button"
          class="icon-toggle-btn"
          aria-label={showPassword ? "Hide token" : "Show token"}
          onclick={() => showPassword = !showPassword}
          title={showPassword ? "Hide token" : "Show token"}
        >
          <Icon name={showPassword ? "eye-off" : "eye"} size={16} color="#64748b" />
        </button>
      </div>

      <button
        type="button"
        class="btn btn-primary"
        onclick={() => verifyToken(true)}
        disabled={isCheckingToken || !token.trim()}
      >
        {#if isCheckingToken}
          <span class="btn-spinner"></span>
          <span>Verifying...</span>
        {:else}
          <Icon name="check" size={15} />
          <span>Verify Token</span>
        {/if}
      </button>

      <button
        type="button"
        class="btn btn-soft-sky"
        onclick={detectGhCliToken}
        title="Detect and load token from local GitHub CLI (`gh auth token`)"
      >
        <Icon name="terminal" size={14} color="#0284c7" />
        <span>Load from <code>gh</code></span>
      </button>
    </div>

    <!-- User Profile Badge -->
    <div class="user-status-pill">
      {#if user}
        <div class="user-pill-connected" title="Authenticated User ID: {user.id}">
          {#if user.avatar_url}
            <img src={user.avatar_url} alt={user.login} class="user-avatar" />
          {:else}
            <div class="user-avatar-placeholder">
              <Icon name="github" size={15} color="#059669" />
            </div>
          {/if}
          <div class="user-meta">
            <span class="user-login">@{user.login}</span>
            <span class="user-state">
              <span class="live-dot pulse"></span>
              Active Session
            </span>
          </div>
        </div>
      {:else}
        <div class="user-pill-disconnected">
          <span class="disconnected-dot"></span>
          <span class="disconnected-text">{statusMessage || "Token Not Verified"}</span>
        </div>
      {/if}
    </div>
  </div>
</header>

<style>
  .app-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1.5rem;
    padding: 0.85rem 1.75rem;
    background: #ffffff;
    border-bottom: 1px solid #e2e8f0;
    flex-wrap: wrap;
    position: relative;
    z-index: 10;
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.03);
  }

  .header-left {
    display: flex;
    align-items: center;
  }

  .brand-container {
    display: flex;
    align-items: center;
    gap: 0.85rem;
  }

  .brand-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 42px;
    height: 42px;
    background: linear-gradient(135deg, #eef2ff 0%, #e0e7ff 100%);
    border: 1px solid #c7d2fe;
    border-radius: 10px;
    box-shadow: 0 2px 6px rgba(79, 70, 229, 0.12);
  }

  .brand-meta {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
  }

  .brand-title-row {
    display: flex;
    align-items: center;
    gap: 0.55rem;
  }

  .brand-title {
    font-family: 'JetBrains Mono', monospace;
    font-size: 1.05rem;
    font-weight: 700;
    color: #0f172a;
    letter-spacing: -0.02em;
  }


  .brand-subtitle {
    font-size: 0.775rem;
    color: #64748b;
  }

  .header-right {
    display: flex;
    align-items: center;
    gap: 1rem;
    flex-wrap: wrap;
  }

  .auth-toolbar {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .token-field-wrapper {
    position: relative;
    display: flex;
    align-items: center;
  }

  .token-prefix-icon {
    position: absolute;
    left: 10px;
    display: flex;
    align-items: center;
    pointer-events: none;
  }

  .token-input {
    background: #f8fafc;
    border: 1px solid #cbd5e1;
    color: #0f172a;
    border-radius: 8px;
    padding: 0.5rem 2.2rem 0.5rem 2.1rem;
    font-family: 'JetBrains Mono', monospace;
    font-size: 0.825rem;
    width: 230px;
    transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .token-input:focus {
    outline: none;
    border-color: #6366f1;
    background: #ffffff;
    box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.15);
  }

  .icon-toggle-btn {
    position: absolute;
    right: 6px;
    background: none;
    border: none;
    cursor: pointer;
    padding: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    transition: opacity 0.15s ease;
  }

  .icon-toggle-btn:hover {
    opacity: 0.7;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
    padding: 0.5rem 0.85rem;
    font-size: 0.825rem;
    font-weight: 600;
    border-radius: 8px;
    cursor: pointer;
    transition: all 0.18s cubic-bezier(0.16, 1, 0.3, 1);
    border: 1px solid transparent;
  }

  .btn:focus-visible {
    outline: 2px solid #6366f1;
    outline-offset: 2px;
  }

  .btn-primary {
    background: #4f46e5;
    color: #ffffff;
    box-shadow: 0 1px 3px rgba(79, 70, 229, 0.25);
  }

  .btn-primary:hover:not(:disabled) {
    background: #4338ca;
    box-shadow: 0 2px 6px rgba(79, 70, 229, 0.35);
  }

  .btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-soft-sky {
    background: #f0f9ff;
    color: #0369a1;
    border: 1px solid #bae6fd;
  }

  .btn-soft-sky:hover {
    background: #e0f2fe;
    border-color: #7dd3fc;
    color: #0284c7;
  }

  .btn-soft-sky code {
    font-family: 'JetBrains Mono', monospace;
    background: #ffffff;
    padding: 0.1rem 0.3rem;
    border-radius: 4px;
    font-size: 0.775rem;
    color: #0284c7;
    border: 1px solid #bae6fd;
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

  /* User Status Pill */
  .user-status-pill {
    display: flex;
    align-items: center;
  }

  .user-pill-connected {
    display: flex;
    align-items: center;
    gap: 0.65rem;
    padding: 0.35rem 0.75rem;
    background: #ecfdf5;
    border: 1px solid #a7f3d0;
    border-radius: 8px;
    box-shadow: 0 1px 2px rgba(5, 150, 105, 0.08);
  }

  .user-avatar {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    border: 1.5px solid #059669;
    object-fit: cover;
  }

  .user-avatar-placeholder {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    background: #ffffff;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1px solid #059669;
  }

  .user-meta {
    display: flex;
    flex-direction: column;
    line-height: 1.2;
  }

  .user-login {
    font-family: 'JetBrains Mono', monospace;
    font-size: 0.825rem;
    font-weight: 600;
    color: #065f46;
  }

  .user-state {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    font-size: 0.7rem;
    color: #059669;
    font-weight: 500;
  }

  .live-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: #059669;
  }

  .live-dot.pulse {
    box-shadow: 0 0 0 rgba(5, 150, 105, 0.5);
    animation: pulse-ring 2s infinite;
  }

  @keyframes pulse-ring {
    0% {
      box-shadow: 0 0 0 0 rgba(5, 150, 105, 0.5);
    }
    70% {
      box-shadow: 0 0 0 6px rgba(5, 150, 105, 0);
    }
    100% {
      box-shadow: 0 0 0 0 rgba(5, 150, 105, 0);
    }
  }

  .user-pill-disconnected {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.4rem 0.75rem;
    background: #f8fafc;
    border: 1px solid #e2e8f0;
    border-radius: 8px;
  }

  .disconnected-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #f59e0b;
  }

  .disconnected-text {
    font-size: 0.775rem;
    color: #64748b;
    max-width: 170px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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
