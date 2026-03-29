use crate::backend;
use crate::display_info::{DisplayConfig, DisplayMapping, DisplayProfile, DisplayTopology};

pub fn apply_profile(profile: &DisplayProfile) -> Result<(), String> {
    let normalized = normalize_profile(profile);
    backend::get_backend().apply_profile(&normalized)
}

pub fn apply_profile_with_mapping(
    profile: &DisplayProfile,
    mappings: &[DisplayMapping],
) -> Result<(), String> {
    backend::get_backend().apply_with_mapping(profile, mappings)
}

fn normalize_profile(profile: &DisplayProfile) -> DisplayProfile {
    if !matches!(profile.topology, DisplayTopology::Extend) {
        return profile.clone();
    }

    let enabled: Vec<(usize, &DisplayConfig)> = profile
        .displays
        .iter()
        .enumerate()
        .filter(|(_, display)| display.enabled)
        .collect();

    if enabled.len() < 2 || !has_overlap(&enabled) {
        return profile.clone();
    }

    let mut normalized = profile.clone();
    let mut order: Vec<(usize, i32, i32)> = enabled
        .iter()
        .map(|(index, display)| (*index, display.position_x, display.position_y))
        .collect();
    order.sort_by_key(|(index, x, y)| (*x, *y, *index));

    let mut next_x = 0;
    for (index, _, _) in order {
        let display = &mut normalized.displays[index];
        display.position_x = next_x;
        display.position_y = 0;
        next_x += display.width as i32;
    }

    normalized
}

fn has_overlap(enabled: &[(usize, &DisplayConfig)]) -> bool {
    for (i, (_, a)) in enabled.iter().enumerate() {
        for (_, b) in enabled.iter().skip(i + 1) {
            if rects_overlap(a, b) {
                return true;
            }
        }
    }
    false
}

fn rects_overlap(a: &DisplayConfig, b: &DisplayConfig) -> bool {
    let ax2 = a.position_x + a.width as i32;
    let ay2 = a.position_y + a.height as i32;
    let bx2 = b.position_x + b.width as i32;
    let by2 = b.position_y + b.height as i32;

    a.position_x < bx2 && ax2 > b.position_x && a.position_y < by2 && ay2 > b.position_y
}
