# GitHub Automation Studio

A modern, cross-platform desktop application built with **Tauri v2**, **SvelteKit** (featuring **Svelte 5 Runes** and **Bun** as runtime/package manager), and a high-performance **Rust** backend.

This application integrates and modernizes GitHub automation workflows for organization team management and student assignment repository provisioning natively—eliminating external shell script dependencies while providing an intuitive, reactive user interface.

---

## Table of Contents

- [Key Features](#key-features)
  - [1. Unified Authentication](#1-unified-authentication)
  - [2. Module 001: GitHub Organization Team Invitations](#2-module-001-github-organization-team-invitations)
  - [3. Module 002: Assignment Repository Provisioning](#3-module-002-assignment-repository-provisioning)
  - [4. Real-time Console & Log Streaming](#4-real-time-console--log-streaming)
- [Project Architecture](#project-architecture)
- [System Prerequisites](#system-prerequisites)
- [Getting Started & Development](#getting-started--development)
  - [1. Install Frontend Dependencies](#1-install-frontend-dependencies)
  - [2. Type Checking & Unit Tests](#2-type-checking--unit-tests)
  - [3. Run in Development Mode](#3-run-in-development-mode)
  - [4. Build Desktop Application Installers](#4-build-desktop-application-installers)
- [YAML Configuration Specification](#yaml-configuration-specification)
  - [Team Invitations Sample (Module 001)](#team-invitations-sample-module-001)
  - [Repository Provisioning Sample (Module 002)](#repository-provisioning-sample-module-002)
- [CI/CD & Automated Releases (GitHub Actions)](#cicd--automated-releases-github-actions)
- [License](#license)

---

## Key Features

### 1. Unified Authentication
- **GitHub Personal Access Token (PAT)** management with secure visibility toggle.
- **One-Click Auto Detection**: Automatically reads active authentication credentials from the local GitHub CLI (`gh auth token`) when available.
- **Live Token Verification**: Instant validation against GitHub's API displaying user identity (display name, username, user ID, and avatar).
- **Persistent Local Storage**: Securely caches credentials locally across app restarts.

### 2. Module 001: GitHub Organization Team Invitations
- **Organization & Role Configuration**: Define the target GitHub Organization and enforce membership role (`member` or `maintainer`).
- **Interactive Invitation Table**: Add, edit, and remove student usernames and comma-separated target teams directly in a clean table interface.
- **Real-time Pre-Flight Preview**: The preview table updates instantly and automatically as you modify table rows—no manual "re-parse" button required.
- **Automated Slug Normalization**: Converts raw team names into GitHub-compatible slugs (e.g., `"Phase 1 - Set 1"` &rarr; `phase-1-set-1`).
- **Safe Dry-Run Mode**: Test and verify invitation rosters without modifying organization memberships or sending actual invites.
- **YAML Import & Export**: One-click export to `.yaml` and instant import of existing roster files.
- **Dedicated Clear Action**: Header "Clear" button resets configuration and table data back to empty state.

### 3. Module 002: Assignment Repository Provisioning
- **Organization & Maintainer Setup**: Configure target organization and maintainer/reviewer usernames.
- **Template Repository Catalog**:
  - Interactive catalog table managing template keys, source repository URLs, and submission deadlines.
  - Interactive **Calendar** date picker and **Clock** time picker for deadline selection.
- **User Assignments Input**:
  - Assign student repositories with optional cohort/batch prefixes (e.g. `BATCH-01-DEV`).
  - **Strict Catalog Validation**: Ensures assigned templates exist in the *Template Repository Catalog*. Includes native `<datalist>` autocomplete suggestions and visible warning badges for unrecognized templates.
- **Real-time Pre-Flight Repository Matrix**:
  - Instant live computation of target repository names and formatted ISO 8601 deadlines.
  - Strictly filters and excludes non-catalog templates to prevent accidental phantom repositories.
- **Native 8-Step Provisioning Pipeline**:
  1. Generate private repository from template (`POST /repos/{template_owner}/{template_repo}/generate`).
  2. Poll and await repository initialization on GitHub.
  3. Grant collaborator write access (`push`) to the assigned student.
  4. Grant maintainer access (`maintain`) to designated reviewers.
  5. Update repository description with assignment submission deadline.
  6. Create milestone `"Assignment Deadline"` with ISO 8601 due date.
  7. Create initial notification issue linked to the milestone.
  8. Create `feedback` branch, commit `.github/FEEDBACK_HINT.md`, and open a **Feedback Pull Request** requesting reviewer reviews.
- **YAML Import & Export**: Comprehensive import and export for full provisioning configuration and assignment lists.

### 4. Real-time Console & Log Streaming
- Live activity logs streamed directly from the Rust backend via Tauri event channels.
- Filter by level: `All`, `Info`, `Success`, `Warn`, and `Error`.
- Live search filtering, autoscroll toggle, log clearing, and one-click clipboard copy (*Copy Logs*).

---

## Project Architecture

```
global-tools/
├── .github/
│   └── workflows/
│       └── release.yml          # GitHub Actions workflow for multi-platform releases (Windows & macOS)
├── apps/                        # Tauri + SvelteKit Desktop App
│   ├── package.json             # Frontend dependencies managed with Bun
│   ├── svelte.config.js
│   ├── vite.config.js
│   ├── src/                     # SvelteKit 2 + Svelte 5 Runes frontend
│   │   ├── app.html
│   │   ├── lib/
│   │   │   ├── types.ts         # Shared TypeScript interfaces
│   │   │   └── components/
│   │   │       ├── Header.svelte            # PAT authentication & user status
│   │   │       ├── TeamInvites.svelte       # Module 001 (Team Invitations)
│   │   │       ├── RepoProvisioning.svelte  # Module 002 (Repo Provisioning)
│   │   │       ├── ConsoleLogs.svelte       # Real-time streaming log terminal
│   │   │       └── Icon.svelte              # Multi-purpose SVG icon library
│   │   └── routes/
│   │       ├── +layout.ts
│   │       └── +page.svelte                 # Dashboard container & tab navigation
│   └── src-tauri/               # Native Rust Backend
│       ├── Cargo.toml           # Rust dependencies (tauri, reqwest, tokio, serde, chrono, regex)
│       ├── tauri.conf.json      # Tauri v2 configuration (window dimensions, bundling, permissions)
│       ├── capabilities/        # Tauri v2 security capabilities
│       └── src/
│           ├── lib.rs           # Tauri app runner & invoke handler registration
│           ├── main.rs
│           ├── commands.rs      # Tauri commands & event emission
│           ├── github_client.rs # Pure Rust GitHub REST API client
│           └── parser.rs        # Data normalization, catalog resolution & unit tests
├── dummy-yamls/                 # Sample YAML configuration files for testing
│   ├── team_invitations_sample.yaml
│   └── repo_provisioning_sample.yaml
├── LICENSE
└── README.md
```

---

## System Prerequisites

Ensure the following tools are installed on your machine before running or building the application:

1. **Bun** (v1.0 or newer)  
   Install via terminal:
   ```bash
   curl -fsSL https://bun.sh/install | bash
   ```
2. **Rust & Cargo** (2021 Edition / stable)  
   Install via rustup:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```
3. **OS-Specific Build Tools**:
   - **macOS**: Xcode Command Line Tools (`xcode-select --install`).
   - **Windows**: Microsoft Visual Studio C++ Build Tools (with "Desktop development with C++").
4. **GitHub CLI (`gh`)** *(Optional)*: For automatic token detection.

---

## Getting Started & Development

All commands should be executed from the repository root:

### 1. Install Frontend Dependencies
```bash
bun install --cwd apps
```

### 2. Type Checking & Unit Tests
```bash
# Frontend SvelteKit and TypeScript verification
bun run --cwd apps check

# Backend Rust parser and normalization unit tests
cargo test --manifest-path apps/src-tauri/Cargo.toml
```

### 3. Run in Development Mode
Starts the Vite dev server with hot-module reloading alongside the native Tauri desktop window:
```bash
bun run --cwd apps tauri dev
```

### 4. Build Desktop Application Installers
Compile optimized release binaries and native installer packages:
```bash
bun run --cwd apps tauri build
```
Compiled artifacts will be located in:
- **macOS**: `apps/src-tauri/target/release/bundle/dmg/` (`.dmg` installer) and `.app` bundle.
- **Windows**: `apps/src-tauri/target/release/bundle/nsis/` (`.exe` setup) and `msi/` (`.msi` installer).

---

## YAML Configuration Specification

Configurations can be imported and exported as `.yaml` files. Ready-to-use sample files are located in the [`dummy-yamls/`](file:///Users/standard/Workspaces/Works/Works-AIEN/global-tools/dummy-yamls) directory.

### Team Invitations Sample (Module 001)
File: `team_invitations_sample.yaml`
```yaml
organization: my-org-name
role: member
invitations:
  - user: student-alice
    teams:
      - Phase 1 - Set 1
      - General Students
  - user: student-bob
    teams:
      - Phase 1 - Set 1
```

### Repository Provisioning Sample (Module 002)
File: `repo_provisioning_sample.yaml`
```yaml
organization: my-org-name
maintainers:
  - instructor-jane
  - mentor-john
template_catalog:
  - key: Web-Frontend
    repo: my-org-name/template-web-frontend
    deadline: '2026-10-15 17:00'
  - key: Backend-API
    repo: my-org-name/template-backend-api
    deadline: '2026-10-20 23:59'
assignments:
  - user: student-alice
    prefix: BATCH-01-DEV
    templates:
      - Web-Frontend
      - Backend-API
  - user: student-bob
    prefix: BATCH-01-DEV
    templates:
      - Web-Frontend
```

---

## CI/CD & Automated Releases (GitHub Actions)

A GitHub Actions workflow is configured in [`.github/workflows/release.yml`](file:///Users/standard/Workspaces/Works/Works-AIEN/global-tools/.github/workflows/release.yml) to build and publish official release binaries.

### Triggering a Manual Release:
1. Push your latest commits to GitHub:
   ```bash
   git push origin main
   ```
2. Navigate to your repository on GitHub in your web browser.
3. Select the **Actions** tab.
4. In the left sidebar, click the **Release** workflow.
5. Click the **Run workflow** dropdown on the right:
   - Provide a release tag (e.g., `v0.1.0`) or leave blank to automatically use the version defined in `tauri.conf.json`.
   - Provide a release title or leave blank to use the default title.
   - Configure **Draft** or **Pre-release** flags if desired.
6. Click **Run workflow**.

GitHub Actions will execute concurrent matrix builds for:
- **Windows (`windows-latest`)**: Generates `.exe` (NSIS setup) and `.msi` (WiX installer).
- **macOS (`macos-latest`)**: Generates `.dmg` and `.app` bundles for both **Apple Silicon** (`aarch64-apple-darwin`) and **Intel** (`x86_64-apple-darwin`) architectures.

All installer bundles will be automatically attached and published directly to your repository's **Releases** page.

---

## License

This project is licensed under the [MIT License](file:///Users/standard/Workspaces/Works/Works-AIEN/global-tools/LICENSE).
