use crate::backend::DisplayBackend;
use crate::display_info::{DisplayConfig, DisplayMapping, DisplayProfile, DisplayTopology};
use std::process::Command;

pub struct LinuxDisplayBackend;

impl DisplayBackend for LinuxDisplayBackend {
    fn get_display_profile(&self) -> Result<DisplayProfile, String> {
        let output = run_xrandr(&["--query"])?;
        parse_xrandr_query(&output)
    }

    fn apply_profile(&self, profile: &DisplayProfile) -> Result<(), String> {
        apply_linux_profile(profile)
    }

    fn apply_with_mapping(
        &self,
        profile: &DisplayProfile,
        mappings: &[DisplayMapping],
    ) -> Result<(), String> {
        let current_profile = self.get_display_profile()?;
        let mut mapped_displays = Vec::new();

        for mapping in mappings {
            let current_display = current_profile
                .displays
                .iter()
                .find(|display| display.device_name == mapping.current_display_name)
                .ok_or_else(|| {
                    format!(
                        "current display '{}' was not found while applying mapped profile",
                        mapping.current_display_name
                    )
                })?;

            let source_display = profile
                .displays
                .get(mapping.profile_display_index)
                .ok_or_else(|| {
                    format!(
                        "profile display index {} is out of range",
                        mapping.profile_display_index
                    )
                })?;

            mapped_displays.push(DisplayConfig {
                label: source_display
                    .label
                    .clone()
                    .or_else(|| current_display.label.clone()),
                device_name: current_display.device_name.clone(),
                device_id: current_display.device_id.clone(),
                device_key: current_display.device_key.clone(),
                width: source_display.width,
                height: source_display.height,
                position_x: source_display.position_x,
                position_y: source_display.position_y,
                orientation: source_display.orientation,
                enabled: source_display.enabled,
            });
        }

        apply_linux_profile(&DisplayProfile {
            topology: profile.topology.clone(),
            displays: mapped_displays,
        })
    }
}

fn apply_linux_profile(profile: &DisplayProfile) -> Result<(), String> {
    if profile.displays.is_empty() {
        return Err(String::from(
            "profile does not contain any displays to apply",
        ));
    }

    if !profile.displays.iter().any(|d| d.enabled) {
        return Err(String::from("no enabled displays in profile"));
    }

    let mut args: Vec<String> = Vec::new();
    for display in &profile.displays {
        args.push(String::from("--output"));
        args.push(display.device_name.clone());
        if display.enabled {
            args.push(String::from("--mode"));
            args.push(format!("{}x{}", display.width, display.height));
            args.push(String::from("--pos"));
            args.push(format!("{}x{}", display.position_x, display.position_y));
            args.push(String::from("--rotate"));
            args.push(rotation_to_xrandr(display.orientation).to_string());
        } else {
            args.push(String::from("--off"));
        }
    }

    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    run_xrandr(&arg_refs).map(|_| ())
}

fn rotation_to_xrandr(orientation: u32) -> &'static str {
    match orientation {
        1 => "left",
        2 => "inverted",
        3 => "right",
        _ => "normal",
    }
}

fn run_xrandr(args: &[&str]) -> Result<String, String> {
    if std::env::var("DISPLAY").is_err() {
        return Err(String::from(
            "DISPLAY is not set; X11/xrandr is unavailable",
        ));
    }

    let output = Command::new("xrandr")
        .args(args)
        .output()
        .map_err(|error| format!("failed to execute xrandr: {error}"))?;

    if !output.status.success() {
        return Err(format!(
            "xrandr failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn parse_xrandr_query(output: &str) -> Result<DisplayProfile, String> {
    let mut displays = Vec::new();

    for line in output.lines() {
        if !line.contains(" connected") {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }

        let device_name = parts[0].to_string();
        let geometry_token = parts
            .iter()
            .find(|token| token.contains('x') && token.contains('+'))
            .copied();

        let (width, height, position_x, position_y) = geometry_token
            .map(parse_geometry)
            .transpose()?
            .unwrap_or((0, 0, 0, 0));

        displays.push(DisplayConfig {
            label: Some(device_name.clone()),
            device_name,
            device_id: String::new(),
            device_key: String::new(),
            width,
            height,
            position_x,
            position_y,
            orientation: 0,
            enabled: true,
        });
    }

    if displays.is_empty() {
        return Err(String::from("xrandr returned no connected displays"));
    }

    let topology = if displays.len() > 1 {
        DisplayTopology::Extend
    } else if displays[0].device_name.starts_with("eDP")
        || displays[0].device_name.starts_with("LVDS")
    {
        DisplayTopology::Internal
    } else {
        DisplayTopology::External
    };

    Ok(DisplayProfile { topology, displays })
}

fn parse_geometry(token: &str) -> Result<(u32, u32, i32, i32), String> {
    let (size, position) = token
        .split_once('+')
        .ok_or_else(|| format!("failed to parse xrandr geometry token '{token}'"))?;
    let mut position_parts = position.split('+');
    let x = position_parts
        .next()
        .ok_or_else(|| format!("missing x position in '{token}'"))?
        .parse::<i32>()
        .map_err(|error| format!("invalid x position in '{token}': {error}"))?;
    let y = position_parts
        .next()
        .ok_or_else(|| format!("missing y position in '{token}'"))?
        .parse::<i32>()
        .map_err(|error| format!("invalid y position in '{token}': {error}"))?;
    let (width, height) = size
        .split_once('x')
        .ok_or_else(|| format!("missing size in '{token}'"))?;

    Ok((
        width
            .parse::<u32>()
            .map_err(|error| format!("invalid width in '{token}': {error}"))?,
        height
            .parse::<u32>()
            .map_err(|error| format!("invalid height in '{token}': {error}"))?,
        x,
        y,
    ))
}
