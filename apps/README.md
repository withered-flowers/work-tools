# GitHub Automation Studio (Tauri + SvelteKit + Bun + Rust)

A native cross-platform desktop application built with **Tauri v2**, **SvelteKit** (using **Bun** as runtime and package manager), and **Rust** that replicates and unifies the automation workflows from:
1. `001_invite_teams.sh`: GitHub Organization Team Invitations
2. `002_create_repos.sh`: Repository Provisioning from Templates with Permissions, Milestones, Issues, and Feedback Pull Requests

> [!NOTE]
> This application does **not** invoke the `.sh` scripts. All GitHub API operations, parsing rules, team slug normalization, milestone ISO 8601 formatting, and template repo cloning are implemented natively in Rust (`reqwest` + `tokio`) and rendered in an accessible SvelteKit frontend.

---

## Features

### 1. Unified Authentication
- GitHub Personal Access Token (PAT) input with visibility toggle.
- **One-click detection** from local GitHub CLI (`gh auth token`) if available.
- Real-time token verification with live display of authenticated username, user ID, and avatar.
- Persistent local storage of token across sessions.

### 2. Team Invitations (`001_invite_teams.sh`)
- Configure target GitHub Organization name.
- Role selection (`member` or `maintainer`).
- Multiple input formats supported:
  - CSV format: `username,team1,team2,...`
  - Pipe format: `username|team1,team2,...`
- Automated team slug generation (e.g., `"Phase 1 - Set 1"` &rarr; `phase-1-set-1`), strictly matching bash sed logic.
- "Load Sample CSV" button for instant testing with `001_xdummy_users.csv`.
- CSV file upload & drag-and-drop support.
- Pre-flight preview table displaying parsed teams and slugs before execution.
- Safe **Dry-Run mode** toggle with visual badge indicators.
- Real-time progress bar, live event logging, and status badges (`[ACTIVE MEMBER]`, `[INVITATION SENT]`, `[DRY-RUN]`, `[FAILED]`).

### 3. Repository Provisioning (`002_create_repos.sh`)
- Organization name, fallback cohort prefix (convention: `FTDS-XXX-HCK|RMT`), default deadline, and reviewer usernames.
- Optional closed team creation and org membership invitation synchronization.
- Dynamic Template Catalog manager (add, remove, and configure individual template repositories & deadlines).
- "Load Default Templates" preset button (`P0-LC1-Set-1`, `P0-LC2-Set-1`, `P0-LC3-Set-1`).
- Assignment input parsing supporting:
  - `username,prefix,template1,template2,...`
  - `username|prefix|template1,template2,...`
  - `username|prefix`
  - Template-centric: `template|deadline|user1,user2`
- "Load Sample CSV" button for `002_xdummy_assignments.csv`.
- Pre-flight matrix preview table detailing exact target repo names (e.g. `P0-LC1-Set-1-FTDS-045-HCK-user1`) and ISO 8601 deadline timestamps.
- Native multi-step repository pipeline:
  1. Generate private repository from template (`POST /repos/{template}/generate`)
  2. Wait for repository initialization
  3. Assign write access (`push`) to student user
  4. Assign maintainer access (`maintain`) to reviewers
  5. Update description with assignment deadline
  6. Create milestone `"Assignment Deadline"` with ISO 8601 due date
  7. Create milestone-linked issue with notification message
  8. Create `feedback` branch, commit `.github/FEEDBACK_HINT.md`, and open Feedback PR requesting reviewers

### 4. Live Console & Logs
- Streaming activity logs emitted in real-time from the Rust backend via Tauri event channels.
- Filter by level (`All`, `Info`, `Success`, `Warnings`, `Errors`).
- Search filter, auto-scroll, and "Copy Logs" functionality.

---

## Prerequisites

- **Bun**: `bun` installed on system (v1.0+)
- **Rust & Cargo**: Rust toolchain (2021 edition)
- **GitHub CLI (`gh`)**: (Optional, for auto-token detection)

---

## Development & Usage

All commands can be run from the repository root:

### Install Frontend Dependencies
```bash
bun install --cwd apps
```

### Run Type-check and Tests
```bash
# Frontend svelte-check
bun run --cwd apps check

# Backend Rust unit tests
cargo test --manifest-path apps/src-tauri/Cargo.toml
```

### Run in Development Mode
```bash
bun run --cwd apps tauri dev
```

### Build Desktop Application
```bash
bun run --cwd apps tauri build
```
The compiled macOS binary will be placed at `apps/src-tauri/target/release/apps` (or `apps/src-tauri/target/debug/apps` for debug builds).

---

## Architecture Overview

```
apps/
├── package.json               # SvelteKit + Vite + Tauri frontend dependencies (managed with bun)
├── svelte.config.js
├── vite.config.js
├── src/
│   ├── app.html
│   ├── lib/
│   │   ├── types.ts           # Shared TypeScript interfaces
│   │   └── components/
│   │       ├── Header.svelte            # Auth & token management
│   │       ├── TeamInvites.svelte       # Module 1 (001_invite_teams)
│   │       ├── RepoProvisioning.svelte  # Module 2 (002_create_repos)
│   │       └── ConsoleLogs.svelte       # Real-time streaming log terminal
│   └── routes/
│       ├── +layout.ts
│       └── +page.svelte       # Main dashboard & tab container
└── src-tauri/
    ├── Cargo.toml             # Rust dependencies (tauri, reqwest, tokio, regex, chrono, serde)
    ├── tauri.conf.json        # Tauri v2 configuration (window size, permissions, dev commands)
    ├── capabilities/
    │   └── default.json
    └── src/
        ├── lib.rs             # Tauri app runner & invoke handler registration
        ├── main.rs
        ├── commands.rs         # Tauri commands & event emission
        ├── github_client.rs    # Pure Rust GitHub REST API client
        └── parser.rs          # Parser replicating shell script normalizations + unit tests
```
