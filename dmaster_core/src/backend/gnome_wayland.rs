use crate::backend::DisplayBackend;
use crate::display_info::{DisplayConfig, DisplayMapping, DisplayProfile, DisplayTopology};
use std::process::Command;

pub struct GnomeWaylandBackend;

const QUERY_SCRIPT: &str = r#"
import json, dbus

bus = dbus.SessionBus()
proxy = bus.get_object(
    'org.gnome.Mutter.DisplayConfig',
    '/org/gnome/Mutter/DisplayConfig',
)
iface = dbus.Interface(proxy, 'org.gnome.Mutter.DisplayConfig')
serial, monitors, logical_monitors, props = iface.GetCurrentState()

lm_map = {}
for lm in logical_monitors:
    x, y, scale, transform, primary, monitor_refs, _ = lm
    for mref in monitor_refs:
        lm_map[str(mref[0])] = {'x': int(x), 'y': int(y), 'scale': float(scale), 'transform': int(transform), 'primary': bool(primary)}

displays = []
for mon in monitors:
    mon_id, modes, mon_props = mon
    connector = str(mon_id[0])
    vendor = str(mon_id[1])
    product = str(mon_id[2])
    serial_num = str(mon_id[3])
    display_name = str(mon_props.get('display-name', connector))
    current_mode_id = None
    current_w = 0
    current_h = 0
    available_modes = []
    for mode in modes:
        mode_id, w, h, refresh, _, _, mode_props = mode
        available_modes.append({
            'mode_id': str(mode_id),
            'width': int(w),
            'height': int(h),
            'refresh': float(refresh),
            'is_preferred': bool(mode_props.get('is-preferred', False)),
        })
        if mode_props.get('is-current', False):
            current_mode_id = str(mode_id)
            current_w = int(w)
            current_h = int(h)
    lm_info = lm_map.get(connector, {})
    if current_mode_id is None and available_modes:
        pref = next((m for m in available_modes if m['is_preferred']), available_modes[0])
        current_mode_id = pref['mode_id']
        current_w = pref['width']
        current_h = pref['height']
    if current_mode_id is None:
        continue
    displays.append({
        'connector': connector,
        'vendor': vendor,
        'product': product,
        'serial': serial_num,
        'display_name': display_name,
        'mode_id': current_mode_id,
        'width': current_w,
        'height': current_h,
        'x': lm_info.get('x', 0),
        'y': lm_info.get('y', 0),
        'transform': lm_info.get('transform', 0),
        'scale': lm_info.get('scale', 1.0),
        'primary': lm_info.get('primary', False),
        'available_modes': available_modes,
    })

print(json.dumps({'serial': int(serial), 'displays': displays}))
"#;

fn build_apply_script(serial: u32, displays: &[ApplyEntry]) -> String {
    let entries_json = serde_json::to_string(displays).unwrap_or_default();
    format!(
        r#"
import json, dbus

serial = {serial}
entries = json.loads('{entries_json}')

bus = dbus.SessionBus()
proxy = bus.get_object(
    'org.gnome.Mutter.DisplayConfig',
    '/org/gnome/Mutter/DisplayConfig',
)
iface = dbus.Interface(proxy, 'org.gnome.Mutter.DisplayConfig')

logical_monitors = []
for e in entries:
    logical_monitors.append(
        dbus.Struct([
            dbus.Int32(e['x']),
            dbus.Int32(e['y']),
            dbus.Double(e.get('scale', 1.0)),
            dbus.UInt32(e.get('transform', 0)),
            dbus.Boolean(e.get('primary', False)),
            dbus.Array([
                dbus.Struct([
                    dbus.String(e['connector']),
                    dbus.String(e['mode_id']),
                    dbus.Dictionary({{}}, signature='sv'),
                ], signature='ssa{{sv}}'),
            ], signature='(ssa{{sv}})'),
        ], signature='iiduba(ssa{{sv}})'),
    )

iface.ApplyMonitorsConfig(
    dbus.UInt32(serial),
    dbus.UInt32(2),
    dbus.Array(logical_monitors, signature='(iiduba(ssa{{sv}}))'),
    dbus.Dictionary({{}}, signature='sv'),
)
print('ok')
"#
    )
}

#[derive(serde::Serialize)]
struct ApplyEntry {
    connector: String,
    mode_id: String,
    x: i32,
    y: i32,
    transform: u32,
    scale: f64,
    primary: bool,
}

#[derive(serde::Deserialize)]
struct QueryResult {
    serial: u32,
    displays: Vec<QueryDisplay>,
}

#[derive(serde::Deserialize)]
struct QueryDisplay {
    connector: String,
    display_name: String,
    vendor: String,
    product: String,
    serial: String,
    mode_id: String,
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    transform: u32,
    scale: f64,
    primary: bool,
    #[serde(default)]
    available_modes: Vec<AvailableMode>,
}

#[derive(serde::Deserialize)]
struct AvailableMode {
    mode_id: String,
    width: u32,
    height: u32,
    refresh: f64,
    #[serde(default)]
    is_preferred: bool,
}

fn run_python(script: &str) -> Result<String, String> {
    let output = Command::new("python3")
        .arg("-c")
        .arg(script)
        .output()
        .map_err(|e| format!("failed to run python3: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format_python_error(stderr.trim()));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn format_python_error(stderr: &str) -> String {
    if let Some(line) = stderr.lines().rev().find(|line| !line.trim().is_empty()) {
        let message = line.split(": ").skip(1).collect::<Vec<_>>().join(": ");

        if !message.is_empty() {
            if message.contains("Refusing to activate a closed laptop panel") {
                return String::from(
                    "cannot enable the built-in display because the laptop panel is closed",
                );
            }
            return format!("Mutter D-Bus apply failed: {message}");
        }
    }

    format!("Mutter D-Bus apply failed: {stderr}")
}

fn query_current_state() -> Result<QueryResult, String> {
    let json_str = run_python(QUERY_SCRIPT)?;
    serde_json::from_str(&json_str).map_err(|e| format!("failed to parse Mutter response: {e}"))
}

fn transform_to_orientation(transform: u32) -> u32 {
    match transform {
        1 => 1,
        2 => 2,
        3 => 3,
        _ => 0,
    }
}

fn orientation_to_transform(orientation: u32) -> u32 {
    match orientation {
        1 => 1,
        2 => 2,
        3 => 3,
        _ => 0,
    }
}

impl DisplayBackend for GnomeWaylandBackend {
    fn get_display_profile(&self) -> Result<DisplayProfile, String> {
        let state = query_current_state()?;

        let displays: Vec<DisplayConfig> = state
            .displays
            .iter()
            .map(|d| DisplayConfig {
                label: Some(d.display_name.clone()),
                device_name: d.connector.clone(),
                device_id: format!("{}:{}:{}", d.vendor, d.product, d.serial),
                device_key: d.mode_id.clone(),
                width: d.width,
                height: d.height,
                position_x: d.x,
                position_y: d.y,
                orientation: transform_to_orientation(d.transform),
                enabled: true,
            })
            .collect();

        let topology = if displays.len() > 1 {
            DisplayTopology::Extend
        } else if displays
            .first()
            .map_or(false, |d| d.device_name.starts_with("eDP"))
        {
            DisplayTopology::Internal
        } else {
            DisplayTopology::External
        };

        Ok(DisplayProfile { topology, displays })
    }

    fn apply_profile(&self, profile: &DisplayProfile) -> Result<(), String> {
        let state = query_current_state()?;

        let enabled_displays: Vec<&DisplayConfig> =
            profile.displays.iter().filter(|d| d.enabled).collect();

        if enabled_displays.is_empty() {
            return Err(String::from("no enabled displays in profile"));
        }

        let entries: Vec<ApplyEntry> = enabled_displays
            .iter()
            .enumerate()
            .map(|(i, d)| {
                let mode_id = state
                    .displays
                    .iter()
                    .find(|sd| sd.connector == d.device_name)
                    .map(|sd| find_mode_id_for_resolution(sd, d.width, d.height))
                    .unwrap_or_else(|| format!("{}x{}@60.000", d.width, d.height));

                ApplyEntry {
                    connector: d.device_name.clone(),
                    mode_id,
                    x: d.position_x,
                    y: d.position_y,
                    transform: orientation_to_transform(d.orientation),
                    scale: state
                        .displays
                        .iter()
                        .find(|sd| sd.connector == d.device_name)
                        .map(|sd| sd.scale)
                        .unwrap_or(1.0),
                    primary: i == 0,
                }
            })
            .collect();

        let script = build_apply_script(state.serial, &entries);
        let output = run_python(&script)?;
        if output.trim() == "ok" {
            Ok(())
        } else {
            Err(format!("unexpected apply result: {output}"))
        }
    }

    fn apply_with_mapping(
        &self,
        profile: &DisplayProfile,
        mappings: &[DisplayMapping],
    ) -> Result<(), String> {
        let state = query_current_state()?;
        let mut mapped_displays = Vec::new();

        for mapping in mappings {
            let current = state
                .displays
                .iter()
                .find(|d| d.connector == mapping.current_display_name)
                .ok_or_else(|| {
                    format!(
                        "current display '{}' not found",
                        mapping.current_display_name
                    )
                })?;

            let source = profile
                .displays
                .get(mapping.profile_display_index)
                .ok_or_else(|| {
                    format!(
                        "profile display index {} out of range",
                        mapping.profile_display_index
                    )
                })?;

            mapped_displays.push(DisplayConfig {
                label: source.label.clone().or(Some(current.display_name.clone())),
                device_name: current.connector.clone(),
                device_id: format!("{}:{}:{}", current.vendor, current.product, current.serial),
                device_key: current.mode_id.clone(),
                width: source.width,
                height: source.height,
                position_x: source.position_x,
                position_y: source.position_y,
                orientation: source.orientation,
                enabled: source.enabled,
            });
        }

        self.apply_profile(&DisplayProfile {
            topology: profile.topology.clone(),
            displays: mapped_displays,
        })
    }
}

fn find_mode_id_for_resolution(display: &QueryDisplay, width: u32, height: u32) -> String {
    if display.width == width && display.height == height {
        return display.mode_id.clone();
    }

    let matching: Vec<&AvailableMode> = display
        .available_modes
        .iter()
        .filter(|m| m.width == width && m.height == height)
        .collect();

    if matching.is_empty() {
        return format!("{}x{}@60.000", width, height);
    }

    if let Some(preferred) = matching.iter().find(|m| m.is_preferred) {
        return preferred.mode_id.clone();
    }

    matching
        .iter()
        .max_by(|a, b| {
            a.refresh
                .partial_cmp(&b.refresh)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|m| m.mode_id.clone())
        .unwrap_or_else(|| format!("{}x{}@60.000", width, height))
}
