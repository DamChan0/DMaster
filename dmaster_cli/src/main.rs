use dmaster_core::{
    apply_profile, apply_profile_with_mapping, delete_profile, get_display_profile, load_profiles,
    profiles_dir, save_profile, DisplayMapping,
};
use std::io::{self, Write};

fn main() {
    loop {
        println!("\n==== DMaster Display Profile Menu ====");
        println!("Profile directory: {}", profile_dir_label());
        println!("[1] Save current display profile");
        println!("[2] List saved profiles");
        println!("[3] Apply saved profile directly");
        println!("[4] Apply saved profile with manual mapping");
        println!("[5] Delete saved profile");
        println!("[6] Exit");

        match prompt("Select an option: ").trim() {
            "1" => save_current_profile_flow(),
            "2" => list_profiles_flow(),
            "3" => apply_profile_flow(false),
            "4" => apply_profile_flow(true),
            "5" => delete_profile_flow(),
            "6" => break,
            _ => println!("Invalid option."),
        }
    }

    println!("Goodbye.");
}

fn save_current_profile_flow() {
    let profile = match get_display_profile() {
        Ok(profile) => profile,
        Err(error) => {
            println!("Failed to read current display profile: {error}");
            return;
        }
    };

    let name = prompt("Profile name: ");
    if name.trim().is_empty() {
        println!("Profile name cannot be empty.");
        return;
    }

    let description = prompt("Description (optional): ");
    match save_profile(trimmed(&name), optional_trimmed(&description), &profile) {
        Ok(path) => println!("Saved profile to {}", path.display()),
        Err(error) => println!("Failed to save profile: {error}"),
    }
}

fn list_profiles_flow() {
    match load_profiles() {
        Ok(profiles) if profiles.is_empty() => println!("No saved profiles found."),
        Ok(profiles) => {
            println!("\n=== Saved Profiles ===");
            for (index, profile) in profiles.iter().enumerate() {
                println!(
                    "[{}] {} | displays={} | created_at={} | description={}",
                    index,
                    profile.name,
                    profile.profile.displays.len(),
                    profile.profile.created_at,
                    profile.profile.description.as_deref().unwrap_or("-")
                );
            }
        }
        Err(error) => println!("Failed to load profiles: {error}"),
    }
}

fn apply_profile_flow(use_mapping: bool) {
    let profiles = match load_profiles() {
        Ok(profiles) if profiles.is_empty() => {
            println!("No saved profiles found.");
            return;
        }
        Ok(profiles) => profiles,
        Err(error) => {
            println!("Failed to load profiles: {error}");
            return;
        }
    };

    let selected_profile = match select_profile(&profiles) {
        Some(profile) => profile,
        None => return,
    };

    let result = if use_mapping {
        apply_with_manual_mapping(selected_profile)
    } else {
        apply_profile(&selected_profile.profile.to_display_profile())
    };

    match result {
        Ok(()) => println!("Profile '{}' applied successfully.", selected_profile.name),
        Err(error) => println!(
            "Failed to apply profile '{}': {error}",
            selected_profile.name
        ),
    }
}

fn apply_with_manual_mapping(selected_profile: &dmaster_core::ProfileInfo) -> Result<(), String> {
    let current_profile = get_display_profile()?;
    if current_profile.displays.is_empty() {
        return Err(String::from("no current displays were detected"));
    }

    println!("\n=== Current Displays ===");
    for (index, display) in current_profile.displays.iter().enumerate() {
        println!(
            "[{}] {} {}x{} @ {}x{}",
            index,
            display.device_name,
            display.width,
            display.height,
            display.position_x,
            display.position_y
        );
    }

    println!("\n=== Profile Displays ===");
    for (index, display) in selected_profile.profile.displays.iter().enumerate() {
        println!(
            "[{}] {} {}x{} @ {}x{}",
            index,
            display.label.as_deref().unwrap_or(&display.device_name),
            display.width,
            display.height,
            display.position_x,
            display.position_y
        );
    }

    let mut mappings = Vec::new();
    let mut disabled_names = Vec::new();

    for current_display in &current_profile.displays {
        let answer = prompt(&format!(
            "Map current display '{}' (index, 'off' to disable, blank to skip): ",
            current_display.device_name
        ));
        let trimmed_answer = answer.trim();
        if trimmed_answer.is_empty() {
            continue;
        }

        if trimmed_answer.eq_ignore_ascii_case("off") {
            disabled_names.push(current_display.device_name.clone());
            continue;
        }

        let profile_index = trimmed_answer.parse::<usize>().map_err(|error| {
            format!("invalid profile display index '{trimmed_answer}': {error}")
        })?;
        mappings.push(DisplayMapping {
            current_display_name: current_display.device_name.clone(),
            profile_display_index: profile_index,
        });
    }

    if mappings.is_empty() && disabled_names.is_empty() {
        return Err(String::from("no display mappings were selected"));
    }

    if !mappings.is_empty() {
        apply_profile_with_mapping(&selected_profile.profile.to_display_profile(), &mappings)?;
    }

    if !disabled_names.is_empty() {
        let disabled_profile = dmaster_core::DisplayProfile {
            topology: selected_profile.profile.topology.clone(),
            displays: disabled_names
                .iter()
                .map(|name| dmaster_core::DisplayConfig {
                    label: None,
                    device_name: name.clone(),
                    device_id: String::new(),
                    device_key: String::new(),
                    width: 0,
                    height: 0,
                    position_x: 0,
                    position_y: 0,
                    orientation: 0,
                    enabled: false,
                })
                .collect(),
        };
        apply_profile(&disabled_profile)?;
    }

    Ok(())
}

fn delete_profile_flow() {
    let profiles = match load_profiles() {
        Ok(profiles) if profiles.is_empty() => {
            println!("No saved profiles found.");
            return;
        }
        Ok(profiles) => profiles,
        Err(error) => {
            println!("Failed to load profiles: {error}");
            return;
        }
    };

    let selected_profile = match select_profile(&profiles) {
        Some(profile) => profile,
        None => return,
    };

    match delete_profile(&selected_profile.name) {
        Ok(()) => println!("Deleted profile '{}'.", selected_profile.name),
        Err(error) => println!(
            "Failed to delete profile '{}': {error}",
            selected_profile.name
        ),
    }
}

fn select_profile<'a>(
    profiles: &'a [dmaster_core::ProfileInfo],
) -> Option<&'a dmaster_core::ProfileInfo> {
    println!("\n=== Saved Profiles ===");
    for (index, profile) in profiles.iter().enumerate() {
        println!(
            "[{}] {} ({} displays)",
            index,
            profile.name,
            profile.profile.displays.len()
        );
    }

    let answer = prompt("Choose a profile index: ");
    let index = match answer.trim().parse::<usize>() {
        Ok(index) => index,
        Err(error) => {
            println!("Invalid profile index: {error}");
            return None;
        }
    };

    profiles.get(index).or_else(|| {
        println!("Profile index {} is out of range.", index);
        None
    })
}

fn profile_dir_label() -> String {
    match profiles_dir() {
        Ok(path) => path.display().to_string(),
        Err(error) => format!("<unavailable: {error}>"),
    }
}

fn prompt(message: &str) -> String {
    print!("{message}");
    io::stdout().flush().expect("stdout flush failed");

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("stdin read failed");
    input.trim_end().to_string()
}

fn trimmed(value: &str) -> &str {
    value.trim()
}

fn optional_trimmed(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
