# Native Desktop Design System ⚡

A hardware-accelerated, pure native desktop design system built with **Rust** and the **Slint UI engine**. Engineered for high performance, memory safety, and cross-platform consistency across **Windows, macOS, and Linux** with **zero web/Electron runtime overhead**.

![Native Desktop Design System](assets/app-icon.png)

---

## 🌟 Key Features

- **🚀 100% Pure Native & Compiled**: Compiles down to a standalone **~7.7 MB** native executable with Link-Time Optimization (`cargo build --release`). Zero Chromium or Node.js runtime bloat.
- **🎨 Reactive Design Tokens**: Unified design tokens for colors, typography, and spacing.
- **🌓 Automatic OS Dark/Light Detection**: Real-time operating system theme detection (queries Windows Registry, macOS Appearance, and Linux FreeDesktop portals).
- **🖥️ HiDPI & Retina-Ready**: Calibrated typography scaling and subpixel antialiasing using direct GPU vector rendering (DirectX / Metal / Vulkan).
- **🪟 Collapsible Master-Detail Navigation**: Responsive 250px full sidebar to 68px compact icon rail with GPU-smooth animations.
- **🧩 Reusable Library Architecture (Phase 6)**: Dual-target crate (both an interactive executable app and a reusable library for other desktop software).
- **🔔 Floating Toasts & Live Stream**: Real-time background telemetry feed and non-intrusive corner toast notifications.

---

## 📦 Component Library

```
ui/
├── index.slint            # Centralized library entrypoint (Phase 6 export barrel)
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
│   ├── alert.slint        # Status banners with vector icons and dismiss
│   └── tabs.slint         # Fluid segmented tabs control
├── organisms/
│   ├── modal.slint        # Centered dialog card with dimmed backdrop
│   ├── dropdown.slint     # Top-level PopupWindow with auto-dismiss
│   ├── feed.slint         # Live streaming telemetry event feed
│   ├── sidebar.slint      # Collapsible desktop master-detail navigation sidebar
│   └── toast.slint        # Floating auto-dismissing toast notification
└── app.slint              # Interactive component showcase window
```

---

## 🚀 Phase 6: How to Use in Another Desktop Project

Because this repository is structured as a dual **library and binary crate**, you can import this entire design system into any other native Rust desktop project with **a single line**!

### 1. Add to your project's `Cargo.toml`:
```toml
[dependencies]
# Import from local path or GitHub:
desktop-design-system = { path = "../path/to/desktop-design-system" }
# Or via Git:
# desktop-design-system = { git = "https://github.com/your-username/desktop-design-system" }

slint = "1.9"
```

### 2. Import into your own `.slint` file:
```slint
import { 
    Button, 
    TextInputField, 
    ToggleSwitch, 
    ModalDialog, 
    Dropdown, 
    AccordionItem, 
    ToastNotification, 
    Palette 
} from "desktop-design-system/ui/index.slint";

export component MyAppWindow inherits Window {
    title: "My Custom Desktop Software";
    background: Palette.bg-canvas;

    Button {
        text: "Click Me";
        variant: "primary";
    }
}
```

### 3. Initialize in your `main.rs`:
```rust
use desktop_design_system::is_system_dark_mode;

fn main() -> Result<(), slint::PlatformError> {
    let app = MyAppWindow::new()?;
    // Auto-detect OS dark mode and apply tokens:
    app.global::<Palette>().set_dark_mode(is_system_dark_mode());
    app.run()
}
```

---

## 🛠️ Development & Building

### Run the Interactive Component Showcase
```powershell
cargo run
```

### Production Release Build (Sub-8 MB)
```powershell
cargo build --release
```
The resulting single `.exe` file is located at:
```
target/release/desktop-design-system.exe
```

---

## 📄 License

MIT License. Free for personal and commercial use.
