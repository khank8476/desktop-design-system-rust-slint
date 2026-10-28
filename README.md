# Native Desktop Design System ⚡

A hardware-accelerated, pure native desktop design system built with **Rust** and the **Slint UI engine**. Engineered for high performance, memory safety, and cross-platform consistency across **Windows, macOS, and Linux** with **zero web/Electron runtime overhead**.

![Native Desktop Design System](assets/app-icon.png)

---

## 🌟 Key Features

- **🚀 100% Pure Native & Compiled**: Compiles down to a standalone **~7.7 MB** native executable with Link-Time Optimization (`cargo build --release`). Zero Chromium or Node.js runtime bloat.
- **🎨 Reactive Design Tokens**: Unified design tokens for colors, typography, and spacing.
- **🌓 Automatic OS Dark/Light Detection**: Real-time operating system theme detection (queries Windows Registry, macOS Appearance, and Linux FreeDesktop portals).
- **🖥️ HiDPI & Retina-Ready**: Calibrated typography scaling and subpixel antialiasing using direct GPU vector rendering (DirectX / Metal / Vulkan).
- **🪟 Smart Desktop Window Management**: Dynamic multi-monitor work-area awareness respecting system taskbars.
- **🧩 Atomic Design Architecture**: Modular and reusable component hierarchy (Atoms, Molecules, Organisms).

---

## 📦 Component Library

```
ui/
├── tokens/
│   ├── colors.slint       # Reactive Light & Dark mode palette
│   ├── typography.slint   # Desktop-density font scales & weights
│   └── spacing.slint      # 4px/8px grid system & corner radii
├── atoms/
│   ├── button.slint       # Primary, Secondary, Ghost, Danger states
│   ├── input.slint        # Focus rings, placeholder, password masking
│   └── toggle.slint       # Smooth animated slider switch
├── molecules/
│   ├── accordion.slint    # GPU-clipped collapsible height animation
│   └── alert.slint        # Status banners (info, success, warning, danger)
├── organisms/
│   ├── modal.slint        # Centered dialog card with dimmed backdrop
│   └── dropdown.slint     # Top-level PopupWindow with auto-dismiss
└── app.slint              # Interactive component showcase window
```

---

## 🚀 Getting Started

### Prerequisites
- Install **Rust** (Cargo & Rustc 1.80+) from [https://rustup.rs/](https://rustup.rs/)

### Development
To launch the application in development mode:
```powershell
cargo run
```

### Production Release Build
To generate an optimized, standalone distribution executable:
```powershell
cargo build --release
```
The resulting single `.exe` file is located at:
```
target/release/desktop-design-system.exe
```
This binary can be shared and run directly on any Windows machine without requiring Rust, Node, Python, or external installers.

### Live Preview in VS Code
Install the official **Slint** extension in VS Code. Open any `.slint` file and click the **Slint Preview icon** in the top-right corner for instant, sub-millisecond Hot Reloading without recompiling.

---

## 📐 Atomic Hierarchy Overview

| Level | Component | Description |
| :--- | :--- | :--- |
| **Tokens** | `Palette`, `Typography`, `Spacing` | Centralized styles that cascade automatically on theme change. |
| **Atom** | `Button` | 4 variants (`primary`, `secondary`, `ghost`, `danger`) with animated hover/press transitions. |
| **Atom** | `TextInputField` | Single-line text & password input with animated focus ring. |
| **Atom** | `ToggleSwitch` | Fluid toggle switch wired to application states. |
| **Molecule** | `AccordionItem` | Collapsible panels with rotating chevrons and smooth GPU clipping. |
| **Molecule** | `Alert` | Contextual alert banners with dismiss button and status badges. |
| **Organism** | `ModalDialog` | High-elevation dialog window with dimmed backdrop and click-outside dismiss. |
| **Organism** | `Dropdown` | Native `PopupWindow` selector that floats above all layout stacks. |

---

## 📄 License

MIT License. Free for personal and commercial use.

