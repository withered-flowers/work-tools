# GitHub Automation Studio

Aplikasi desktop modern lintas platform (*cross-platform*) yang dibangun menggunakan **Tauri v2**, **SvelteKit** (dengan **Svelte 5 Runes** dan **Bun** sebagai runtime/package manager), serta **Rust** pada sisi backend. 

Aplikasi ini mengintegrasikan dan memodernisasi alur kerja otomatisasi GitHub untuk manajemen tim organisasi dan *provisioning* repositori tugas siswa/mahasiswa secara native tanpa ketergantungan pada script shell bash eksternal.

---

## Daftar Isi

- [Fitur Utama](#fitur-utama)
  - [1. Otentikasi Terpadu](#1-otentikasi-terpadu)
  - [2. Modul 001: Undangan Tim Organisasi GitHub](#2-modul-001-undangan-tim-organisasi-github)
  - [3. Modul 002: Provisioning Repositori Tugas](#3-modul-002-provisioning-repositori-tugas)
  - [4. Console & Streaming Log Realtime](#4-console--streaming-log-realtime)
- [Struktur Proyek](#struktur-proyek)
- [Prasyarat Sistem](#prasyarat-sistem)
- [Panduan Pengembangan & Menjalankan Aplikasi](#panduan-pengembangan--menjalankan-aplikasi)
  - [1. Instalasi Dependensi Frontend](#1-instalasi-dependensi-frontend)
  - [2. Pemeriksaan Tipe & Pengujian Unit](#2-pemeriksaan-tipe--pengujian-unit)
  - [3. Menjalankan Mode Development](#3-menjalankan-mode-development)
  - [4. Build Aplikasi Desktop](#4-build-aplikasi-desktop)
- [Format Konfigurasi YAML](#format-konfigurasi-yaml)
  - [Contoh YAML Undangan Tim (Module 001)](#contoh-yaml-undangan-tim-module-001)
  - [Contoh YAML Provisioning Repositori (Module 002)](#contoh-yaml-provisioning-repositori-module-002)
- [CI/CD & Otomatisasi Rilis (GitHub Actions)](#cicd--otomatisasi-rilis-github-actions)
- [Lisensi](#lisensi)

---

## Fitur Utama

### 1. Otentikasi Terpadu
- **GitHub Personal Access Token (PAT)** dengan opsi tampil/sembunyikan token.
- **Deteksi Otomatis Satu Klik**: Mendeteksi token aktif langsung dari GitHub CLI (`gh auth token`) jika terpasang di sistem.
- **Verifikasi Realtime**: Memvalidasi token ke GitHub API secara langsung dan menampilkan identitas pengguna terotentikasi (nama, username, ID, dan avatar).
- **Penyimpanan Lokal Persisten**: Token disimpan dengan aman pada local storage agar tidak perlu dimasukkan ulang setiap sesi.

### 2. Modul 001: Undangan Tim Organisasi GitHub
- **Konfigurasi Organisasi**: Menentukan nama target organisasi GitHub dan memilih peran keanggotaan (`member` atau `maintainer`).
- **Tabel Daftar Undangan Interaktif**: Input data pengguna dan tim target langsung melalui tabel, bar penambahan baris inline, serta tombol hapus per baris.
- **Realtime Pre-Flight Preview**: Tabel pratinjau verifikasi diperbarui secara instan dan otomatis saat ada perubahan pada tabel tanpa perlu menekan tombol parse ulang.
- **Normalisasi Slug Tim Otomatis**: Nama tim dikonversi menjadi slug standar GitHub (contoh: `"Phase 1 - Set 1"` &rarr; `phase-1-set-1`).
- **Mode Uji Coba Aman (Dry-Run)**: Mensimulasikan seluruh proses keanggotaan dan undangan tanpa melakukan perubahan nyata pada GitHub.
- **Import & Export YAML**: Mendukung ekspor konfigurasi ke file `.yaml` dan impor kembali dengan sekali klik.
- **Tombol Clear Khusus**: Tombol Clear di header untuk mereset seluruh formulir dan konfigurasi, serta tombol Clear di tabel untuk menghapus daftar undangan.

### 3. Modul 002: Provisioning Repositori Tugas
- **Konfigurasi Organisasi & Reviewer**: Pengaturan target organisasi dan daftar akun maintainer/reviewer.
- **Template Repository Catalog**:
  - Tabel katalog template interaktif (Key, Nama Repositori, dan Batas Waktu / Deadline).
  - Input batas waktu dilengkapi pemilih **Kalender** dan **Jam** terintegrasi.
- **User Assignments Input**:
  - Penugasan repositori per siswa dengan prefix cohort opsional.
  - **Validasi Ketat Katalog**: Memastikan template yang ditugaskan benar-benar ada di *Template Repository Catalog*. Dilengkapi dengan dropdown autocomplete `<datalist>` dan penanda peringatan jika ada template yang tidak terdaftar.
- **Realtime Pre-Flight Repository Matrix**:
  - Matriks pratinjau diperbarui secara langsung saat data katalog atau penugasan diubah.
  - Hanya template valid dari katalog yang akan dimasukkan ke dalam matriks rilis.
- **Pipeline Provisioning Native Multi-Langkah**:
  1. Membuat private repository dari template (`POST /repos/{template_owner}/{template_repo}/generate`).
  2. Menunggu hingga repositori berhasil diinisialisasi oleh GitHub.
  3. Menambahkan hak akses kontributor (`push`/write) kepada siswa.
  4. Menambahkan hak akses `maintain` kepada seluruh reviewer yang ditentukan.
  5. Memperbarui deskripsi repositori dengan tenggat waktu penugasan.
  6. Membuat milestone `"Assignment Deadline"` dengan tanggal jatuh tempo standar ISO 8601.
  7. Membuat issue notifikasi pertama yang terhubung langsung ke milestone.
  8. Membuat branch `feedback`, menambahkan file `.github/FEEDBACK_HINT.md`, dan membuka **Feedback Pull Request** untuk proses peninjauan kode.
- **Import & Export YAML**: Dukungan penuh impor dan ekspor konfigurasi lengkap dalam format YAML.

### 4. Console & Streaming Log Realtime
- Output log proses dikirimkan secara langsung dari backend Rust melalui event channel Tauri.
- Filter tingkat log: `All`, `Info`, `Success`, `Warn`, dan `Error`.
- Fitur pencarian teks log, auto-scroll realtime, pembersihan log, dan penyalinan seluruh log ke clipboard (*Copy Logs*).

---

## Struktur Proyek

```
global-tools/
├── .github/
│   └── workflows/
│       └── release.yml          # GitHub Actions workflow untuk rilis multi-platform (Windows & macOS)
├── apps/                        # Aplikasi Desktop Tauri + SvelteKit
│   ├── package.json             # Konfigurasi dependensi frontend (Bun)
│   ├── svelte.config.js
│   ├── vite.config.js
│   ├── src/                     # Frontend SvelteKit 2 + Svelte 5 Runes
│   │   ├── app.html
│   │   ├── lib/
│   │   │   ├── types.ts         # Definisi tipe TypeScript
│   │   │   └── components/
│   │   │       ├── Header.svelte            # Header otentikasi PAT & status token
│   │   │       ├── TeamInvites.svelte       # Komponen Modul 001 (Undangan Tim)
│   │   │       ├── RepoProvisioning.svelte  # Komponen Modul 002 (Provisioning Repositori)
│   │   │       ├── ConsoleLogs.svelte       # Terminal streaming log aktivitas
│   │   │       └── Icon.svelte              # Komponen ikon SVG serbaguna
│   │   └── routes/
│   │       ├── +layout.ts
│   │       └── +page.svelte                 # Halaman utama & navigasi tab
│   └── src-tauri/               # Backend Native Rust
│       ├── Cargo.toml           # Dependensi Rust (tauri, reqwest, tokio, serde, chrono, regex)
│       ├── tauri.conf.json      # Konfigurasi Tauri v2 (window, bundling, izin)
│       ├── capabilities/        # Izin security Tauri v2
│       └── src/
│           ├── lib.rs           # Entry point aplikasi Tauri
│           ├── main.rs
│           ├── commands.rs      # Command Tauri & handler pemanggilan frontend
│           ├── github_client.rs # Klien HTTP GitHub REST API berbasis Rust
│           └── parser.rs        # Parser data tim, assignment, catalog & unit tests
├── dummy-yamls/                 # File contoh YAML siap pakai untuk pengujian
│   ├── team_invitations_sample.yaml
│   └── repo_provisioning_sample.yaml
├── LICENSE
└── README.md
```

---

## Prasyarat Sistem

Sebelum menjalankan atau membangun aplikasi, pastikan perangkat Anda telah terpasang:

1. **Bun** (v1.0 atau lebih baru)  
   Instalasi via terminal:
   ```bash
   curl -fsSL https://bun.sh/install | bash
   ```
2. **Rust & Cargo** (Edisi 2021 / stable)  
   Instalasi via rustup:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```
3. **Build Tools Sesuai Sistem Operasi**:
   - **macOS**: Xcode Command Line Tools (`xcode-select --install`).
   - **Windows**: Microsoft Visual Studio C++ Build Tools (dengan komponen Desktop development with C++).
4. **GitHub CLI (`gh`)** *(Opsional)*: Untuk mendeteksi token GitHub secara otomatis.

---

## Panduan Pengembangan & Menjalankan Aplikasi

Jalankan seluruh perintah dari root direktori proyek:

### 1. Instalasi Dependensi Frontend
```bash
bun install --cwd apps
```

### 2. Pemeriksaan Tipe & Pengujian Unit
```bash
# Pemeriksaan tipe Svelte & TypeScript
bun run --cwd apps check

# Pengujian unit parser backend Rust
cargo test --manifest-path apps/src-tauri/Cargo.toml
```

### 3. Menjalankan Mode Development
Menjalankan frontend SvelteKit dengan hot-reload dan jendela aplikasi desktop Tauri secara simultan:
```bash
bun run --cwd apps tauri dev
```

### 4. Build Aplikasi Desktop
Menghasilkan installer native produksi:
```bash
bun run --cwd apps tauri build
```
Hasil kompilasi dan paket installer (*installer bundle*) akan tersedia di:
- **macOS**: `apps/src-tauri/target/release/bundle/dmg/` (file `.dmg`) dan `.app`
- **Windows**: `apps/src-tauri/target/release/bundle/nsis/` (file `.exe`) dan `msi/` (file `.msi`)

---

## Format Konfigurasi YAML

Aplikasi mendukung impor dan ekspor konfigurasi menggunakan format file YAML. Contoh file YAML siap pakai tersedia di direktori [`dummy-yamls/`](file:///Users/standard/Workspaces/Works/Works-AIEN/global-tools/dummy-yamls).

### Contoh YAML Undangan Tim (Module 001)
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

### Contoh YAML Provisioning Repositori (Module 002)
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

## CI/CD & Otomatisasi Rilis (GitHub Actions)

Alur kerja GitHub Actions telah dikonfigurasi di file [`.github/workflows/release.yml`](file:///Users/standard/Workspaces/Works/Works-AIEN/global-tools/.github/workflows/release.yml) untuk membuat rilis biner resmi secara otomatis.

### Cara Menjalankan Rilis Manual:
1. Pastikan seluruh commit telah di-push ke GitHub:
   ```bash
   git push origin main
   ```
2. Buka repositori Anda di GitHub melalui browser.
3. Klik tab **Actions**.
4. Di panel sebelah kiri, pilih workflow **Release**.
5. Klik tombol menu dropdown **Run workflow** di sisi kanan atas:
   - Masukkan tag rilis (contoh: `v0.1.0`) atau biarkan kosong untuk menggunakan versi dari `tauri.conf.json`.
   - Masukkan judul rilis atau biarkan kosong untuk menggunakan judul default.
   - Atur opsi rilis draft (*Draft*) atau pra-rilis (*Pre-release*) jika diperlukan.
6. Klik tombol **Run workflow**.

GitHub Actions akan secara otomatis menjalankan proses kompilasi paralel untuk:
- **Windows (`windows-latest`)**: Menghasilkan installer `.exe` (NSIS) dan `.msi` (WiX).
- **macOS (`macos-latest`)**: Menghasilkan installer `.dmg` dan file `.app` untuk arsitektur **Apple Silicon** (`aarch64-apple-darwin`) dan **Intel** (`x86_64-apple-darwin`).

Seluruh paket installer akan otomatis diunggah langsung ke halaman **Releases** repositori GitHub Anda.

---

## Lisensi

Proyek ini dilisensikan di bawah lisensi [MIT](file:///Users/standard/Workspaces/Works/Works-AIEN/global-tools/LICENSE).