# PLI PAT Schema Builder

A desktop application built with [Tauri v2](https://tauri.app/), [SvelteKit](https://svelte.dev/docs/kit), and [Tailwind CSS v4](https://tailwindcss.com/).

Upload an Excel file and the app will apply transformations to produce **two output `.xlsx` files**.

---

## Requirements

- [Node.js](https://nodejs.org/) 18+
- [Rust](https://www.rust-lang.org/tools/install) (stable toolchain)
- Tauri system dependencies – see the [Tauri prerequisites guide](https://tauri.app/start/prerequisites/) for your OS

## Getting Started

Install JavaScript dependencies:

```sh
npm install
```

### Development

Run the full desktop app in development mode (hot-reload):

```sh
npm run tauri dev
```

### Build

Compile a production release bundle:

```sh
npm run tauri build
```

---

## Project Structure

```
├── src/                     # SvelteKit frontend (UI)
│   ├── app.css              # Tailwind CSS entry point
│   ├── routes/
│   │   ├── +layout.svelte   # Root layout
│   │   ├── +layout.ts       # Static adapter config (SSR disabled)
│   │   └── +page.svelte     # Main upload / processing page
└── src-tauri/               # Tauri / Rust backend
    ├── src/
    │   ├── controller/      # Tauri command handlers
    │   ├── service/         # Business logic & transformations
    │   └── repository/      # Excel I/O (calamine + rust_xlsxwriter)
    ├── capabilities/        # Tauri permission scopes
    └── tauri.conf.json      # App configuration
```

## Adding Transformations

Business logic lives in `src-tauri/src/service/mod.rs`.  
Extend `transform_output1` and `transform_output2` with the required row/cell processing.
