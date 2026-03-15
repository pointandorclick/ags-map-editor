# <img src="src-tauri/icons/icon.png" alt="AGS Map Editor Logo" width="128" height="128"> AGS Map Editor

A desktop application for visually designing room maps for [Adventure Game Studio](https://www.adventuregamestudio.co.uk/) (AGS) projects.

## Download

Download the latest installer for your platform from [GitHub Releases](https://github.com/pointandorclick/ags-map-editor/releases):

| Platform | File |
|----------|------|
| Windows | `.exe` (NSIS installer) |
| macOS | `.dmg` |
| Linux | `.AppImage` or `.deb` |

## Overview

AGS Map Editor lets you lay out your game's rooms on a spatial grid, assign AGS room numbers, attach concept art, and add notes — all saved directly into your AGS project folder. It gives you a bird's-eye view of how your game world fits together.

## Getting Started

1. Launch the app and click **Open AGS Project...** (or select a recent project).
2. Point it at a directory that contains a `Game.agf` file.
3. Start building your map by clicking the **+** buttons to add rooms to the grid.

## Core Concepts

### Maps

A map is a named collection of rooms arranged on a 2D grid. A single project can have multiple maps — useful for organizing separate regions, floors, or acts of your game. Use the toolbar dropdown to switch between maps, and the **New / Edit / Delete** buttons to manage them.

### Rooms

Each cell on the grid represents a room. In **Edit mode**, **+** buttons appear on empty adjacent cells so you can expand the map in any direction. Rooms can be moved around the grid by dragging, and deleted from the edit panel.

Click a room to open its edit panel where you can set:

- **Room ID** — the AGS room number this cell corresponds to (e.g. Room 1, Room 305)
- **Title** — a short label displayed on the cell
- **Notes** — free-text notes for design documentation
- **Image** — drag-and-drop concept art or a screenshot onto a room cell
- **Complete** — a checkbox flag meaning "skip AGS code generation"

### Templates

Right-click a room that has an image to mark it as a **template**. Templates are reusable room designs that can be assigned to other rooms, reducing repetitive work when many rooms share a similar layout.

### Display Mode

Toggle to **Display mode** for a compact, read-only view of the map with zoom controls (25%–200%). Click any room to view its details in a modal.

## The `.agm` Project File

When you save, the editor writes a **`<ProjectName>.agm`** file into your AGS project directory (alongside `Game.agf`). This is a JSON file containing all maps, rooms, templates, and editor state. It is the single source of truth for your map data.

```
MyGame/
  Game.agf          ← AGS project file
  MyGame.agm        ← Map editor data (auto-generated)
  ...
```

The `.agm` filename matches the project directory name. Changes are auto-saved as you edit.

## How AGS Rooms Are Mapped

Each room cell on the grid can be assigned an **AGS Room ID** — the number that corresponds to a `roomXX.crm` / `roomXX.asc` file in your AGS project. The editor provides a dropdown of available room IDs (1–18 and 300–308) and prevents you from assigning the same room ID to multiple cells within a map.

This mapping is purely organizational: it connects a spatial position on your map to an AGS room number, helping you visualize which rooms connect to each other and plan your game's world layout.

## Development

### Prerequisites

- [Node.js](https://nodejs.org/) v18+
- [Rust](https://www.rust-lang.org/tools/install) (latest stable)
- Tauri CLI v2 (installed via npm)

### Setup

```bash
npm install
```

### Run (development)

```bash
npm run dev
```

### Build (production)

```bash
npm run build
```

Produces platform-specific bundles (`.app` on macOS, `.msi` on Windows, `.deb` on Linux).

### Releasing

Releases are built automatically by GitHub Actions when a version tag is pushed. Use the release script:

```bash
./release.sh 0.2.0
```

This bumps the version in `tauri.conf.json`, `Cargo.toml`, and `package.json`, commits, tags, and pushes. The workflow builds for all platforms and creates a **draft release** on GitHub. Go to [Releases](https://github.com/pointandorclick/ags-map-editor/releases), review the draft, and publish it.

## License

GNU General Public License v3.0
