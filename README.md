# DM Control Panel

Linux desktop app (Tauri 2) with a React + TypeScript UI, Rust backend, SQLite storage, and a Discord music-bot module scaffold.

UI kit lives in the git submodule [`src/ui`](src/ui) (`pet-ui`).

## Prerequisites

- Rust (rustc / cargo)
- Node.js **20.12.0** (see `engines` in `package.json`)
- Yarn **4.10.3** via Corepack
- Linux packages for Tauri (Arch examples): `webkit2gtk-4.1`, `base-devel`, `sqlite`, `alsa-lib`; for system tray also `libayatana-appindicator`

```bash
corepack enable
corepack prepare yarn@4.10.3 --activate
```

## Setup

```bash
git submodule update --init --recursive
yarn
```

Yarn uses `nodeLinker: node-modules` (required for Tauri/Vite; Plug'n'Play breaks `yarn tauri dev` with `EBADF`).

## Develop / build

```bash
yarn tauri dev
yarn tauri build
```

Frontend-only Vite server:

```bash
yarn dev
```

Lint / format:

```bash
yarn lint
yarn format
```

## Notes

- Discord integration is module stubs and crate dependencies only (`serenity`, `songbird`). No client, token, or voice loop yet.
- SQLite is initialized under the app data directory on startup.
- Host `package.json` provides `pet-ui` peer dependencies (React 18, Tailwind 4, forms, floating-ui, etc.) plus ESLint, Prettier, Husky, and lint-staged aligned with the submodule.
