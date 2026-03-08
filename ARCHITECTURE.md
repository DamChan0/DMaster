# DMaster Architecture

## Overview

DMaster is a Rust workspace for saving and applying monitor profiles across Linux and Windows.

- `dmaster_core`: cross-platform model, profile storage, and platform backends
- `dmaster_cli`: interactive CLI for save/list/apply/delete/manual-mapping flows
- `dmaster_gui`: Slint 1.15 native GUI over `dmaster_core`

## Workspace Graph

```text
+--------------------+
| Cargo workspace    |
| DMaster            |
+---------+----------+
          |
          +-------------------+-------------------+
          |                   |                   |
          v                   v                   v
+----------------+  +----------------+  +----------------+
| dmaster_core   |  | dmaster_cli    |  | dmaster_gui    |
| library crate  |  | binary crate   |  | binary crate   |
+--------+-------+  +--------+-------+  +--------+-------+
         ^                   |                   |
         +-------------------+-------------------+
                             |
                    consumes public API
```

## Core Layer Graph

```text
+----------------------------- dmaster_core -----------------------------+
|                                                                       |
|  display_info.rs                                                      |
|  - DisplayConfig, DisplayProfile, MonitorProfile                      |
|  - DisplayMapping, DisplayTopology                                    |
|                                                                       |
|  profile.rs                                                           |
|  - save_profile(), load_profiles()                                    |
|  - load_profile_by_name(), delete_profile()                           |
|                                                                       |
|  query.rs / apply.rs                                                  |
|  - thin facade over backend::get_backend()                            |
|                                                                       |
|  backend/                                                             |
|  +-------------------+ +-------------------+ +----------------------+ |
|  | windows.rs        | | linux.rs          | | gnome_wayland.rs     | |
|  | Win32 APIs        | | xrandr (X11)      | | Mutter D-Bus API     | |
|  +-------------------+ +-------------------+ +----------------------+ |
|  +-------------------+                                                |
|  | unsupported.rs    |                                                |
|  | fallback          |                                                |
|  +-------------------+                                                |
|                                                                       |
+-----------------------------------------------------------------------+
```

## Backend Routing

```text
get_backend()
    |
    +-- target_os = "windows"  --> WindowsDisplayBackend (Win32 API)
    |
    +-- target_os = "linux"
    |       |
    |       +-- WAYLAND_DISPLAY set && XDG_CURRENT_DESKTOP contains "gnome"
    |       |       --> GnomeWaylandBackend (Mutter D-Bus via python3-dbus)
    |       |
    |       +-- otherwise
    |               --> LinuxDisplayBackend (xrandr subprocess)
    |
    +-- other OS --> UnsupportedDisplayBackend
```

## GUI Architecture (Slint 1.15)

```text
dmaster_gui/
├── build.rs              slint-build compiles .slint -> Rust
├── ui/
│   ├── app.slint         Main window: profile list + detail panel + status bar
│   ├── save_dialog.slint Save profile overlay (name + description)
│   └── mapping_dialog.slint  Manual display mapping overlay (ComboBox per monitor)
└── src/
    └── main.rs           Slint bridge: VecModel bindings, callbacks, mapping logic
```

Key Slint data structures:

- `ProfileEntry`: name, description, display-count, created-at, topology
- `DisplayEntry`: name, resolution, position, orientation, is-primary
- `MappingRow`: current-name, current-resolution, profile-options, selected-index

Apply flow in GUI:

```text
User clicks "Apply Profile"
        |
        v
request-apply callback (Rust)
        |
        +-- connectors match? --> apply_profile() directly
        |
        +-- mismatch --> populate MappingDialog with current monitors + profile options
                              |
                              v
                         User assigns mappings via ComboBox
                              |
                              v
                         mapping-confirmed callback
                              |
                              v
                         apply_profile_with_mapping()
```

## Profile Storage Graph

```text
Current monitor state
        |
        v
get_display_profile()
        |
        v
DisplayProfile
        |
        v
save_profile(name, description, profile)
        |
        v
MonitorProfile JSON
        |
        v
~/.dmaster/profiles/<name>.json
```

## Apply Flow Graph

### Direct apply

```text
load_profile_by_name(name)
        |
        v
MonitorProfile
        |
        v
to_display_profile()
        |
        v
apply_profile()
        |
        v
platform backend
  - Windows: SetDisplayConfig + ChangeDisplaySettingsExW
  - Linux/X11: xrandr subprocess
  - Linux/GNOME Wayland: Mutter ApplyMonitorsConfig via D-Bus
```

### Manual mapping apply

```text
Saved profile displays      Current connected displays
        |                            |
        +------------ user chooses mapping -----------+
                                                      |
                                                      v
                                      DisplayMapping[current -> saved]
                                                      |
                                                      v
                                       apply_profile_with_mapping()
                                                      |
                                                      v
                                         backend resolves target output
                                                      |
                                                      v
                                              platform apply
```

## GNOME Wayland Backend

Uses `python3-dbus` subprocess to communicate with Mutter's `org.gnome.Mutter.DisplayConfig` D-Bus interface.

- `GetCurrentState()`: returns serial, monitors (with all available modes), logical monitors, properties
- `ApplyMonitorsConfig(serial, method=2, logical_monitors, props)`: applies persistent configuration
- Mode ID matching: searches available modes list for exact resolution match, preferring preferred modes, then highest refresh rate

## Current JSON Schema

```json
{
  "schema_version": "0.2.0",
  "name": "work-setup",
  "description": "Office desk layout",
  "created_at": "1772900816",
  "topology": "Extend",
  "displays": [
    {
      "label": "Main monitor",
      "device_name": "eDP-1",
      "device_id": "",
      "device_key": "",
      "width": 1920,
      "height": 1200,
      "position_x": 0,
      "position_y": 0,
      "orientation": 0
    }
  ]
}
```

## Platform Notes

### Windows

- Query/apply via Win32 APIs (`EnumDisplayDevices`, `EnumDisplaySettings`, `SetDisplayConfig`, `ChangeDisplaySettingsExW`)
- Cross-compiles from Linux with `x86_64-pc-windows-gnu` target + MinGW linker

### Linux — X11

- Query/apply via `xrandr` subprocess
- Save/list/apply all functional on X11 sessions

### Linux — GNOME Wayland

- Query/apply via Mutter D-Bus API (`python3-dbus` required)
- Auto-detected when `WAYLAND_DISPLAY` is set and `XDG_CURRENT_DESKTOP` contains "gnome"
- Available modes list used for accurate mode_id resolution matching

## Legacy Compatibility

- Old header-based files starting with `DMaster_v0_1_1` are still accepted
- Legacy payload is imported into the new `MonitorProfile` structure at load time

## Known Gaps

- `created_at` uses epoch seconds, not ISO-8601
- Non-GNOME Wayland compositors (KDE, wlroots) not yet supported
- Windows runtime validation requires a Windows machine or CI runner
