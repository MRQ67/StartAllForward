# StartAllForward — Project Scope & Research Foundation

## 1. Executive Summary

**StartAllForward** is an open-source Windows shell customization application designed to deliver classic, refined, and user-centric taskbar and desktop navigation on modern Windows 11 systems, inspired by the capabilities of StartAllBack.

While StartAllBack is a commercial, closed-source utility relying heavily on in-process shell injection into `explorer.exe`, StartAllForward aims to deliver comparable or superior usability through an independently developed, modular, and upgrade-resilient architecture that adheres strictly to clean-room reverse engineering standards and Windows public APIs.

---

## 2. Project Goals & Non-Goals

### Goals
1. **User Control & Usability**: Restore essential productivity features: taskbar positioning (top/bottom/left/right), ungrouped window buttons with readable labels, fine-grained icon sizing, classic start menu navigation, and clean system tray integration.
2. **Safety & Zero-Breakage Guarantee**: Design an architecture that cannot brick Windows, corrupt Explorer, or cause black-screen boot loops upon Windows cumulative updates.
3. **Clean-Room Independent Implementation**: Develop 100% original code, assets, and branding without extracting or copying proprietary binaries, icons, or disassembly from StartAllBack.
4. **Performance & Lightweight Footprint**: Maintain instant responsiveness, sub-second startup, low CPU utilization, and modest memory footprint.
5. **Modern Extensibility**: Provide an open plugin/theming ecosystem (modern XAML/vector themes, SVG orbs, customizable keyboard shortcuts).

### Non-Goals
1. **No Explorer Binary Patching**: StartAllForward will not patch memory bytes of system files or modify system DLLs on disk.
2. **No Proprietary DRM or Licensing Bypass**: The project has zero interest in circumvention, keygens, or cracking reference software.
3. **No Unsafe In-Process Explorer Injection in Core MVP**: The core architecture avoids brittle in-process hooks that break on minor OS builds.
4. **No System Configuration Corruption**: StartAllForward will not alter system-wide registry hives or require destructive OS modifications.

---

## 3. Legal & Clean-Room Boundaries

To guarantee complete independence and intellectual property compliance:

| Requirement | Implementation Policy |
| :--- | :--- |
| **Clean-Room Analysis** | Reverse engineering of StartAllBack is strictly functional and architectural (black-box / clean-room interoperability research). We observe public behavior, interfaces, window hierarchies, and registry contracts. |
| **No Asset Reuse** | All visual assets (start button orbs, taskbar skins, icons, ribbon glyphs) will be created from scratch or sourced from permissive open-source licenses (MIT/Apache 2.0/CC0). |
| **Independent Codebase** | No decompiled proprietary C++ code from StartAllBack is copied. All implementations are written independently based on documented Windows Win32, Shell, and COM specifications. |
| **Trademark Separation** | The project is named *StartAllForward*, distinctly distinguished from *StartAllBack* and *StartIsBack*. |

---

## 4. Methodology & Research Process

The research process is organized into five structured milestones:
1. **Phase 0 — Capability Verification**: Audit REA MCP tools and local analysis facilities.
2. **Phase 1 — Reference Target Identification**: Locate and inventory installed files, binaries, architectures, and registry footprints.
3. **Phase 2 — Functional Investigation**: Deconstruct Taskbar, Start Menu, File Explorer, and System Tray implementations.
4. **Phase 3 — Technical Feasibility**: Evaluate Win32/Shell APIs, COM contracts, and benchmark technology stacks (.NET/WPF, .NET/WinUI 3, C++/Win32).
5. **Phase 4 & 5 — Architecture & MVP Strategy**: Document findings and formulate an actionable, low-risk implementation roadmap.
