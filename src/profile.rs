use crate::display_info::DisplayProfile;
use serde_json;
use std::fs;

const PROFILE_HEADER: &str = "DMaster_v0_1_1";

pub struct ProfileInfo {
    pub name: String,
    pub path: String,
    pub profile: DisplayProfile,
}

pub fn save_profile(profile: &DisplayProfile, path: &str) {
    let json = serde_json::to_string_pretty(profile).unwrap();
    fs::write(path, format!("{}\n{}", PROFILE_HEADER, json)).unwrap();
}

pub fn profile_detector() -> Vec<ProfileInfo> {
    let mut profiles = Vec::new();

    let mut exe_path = std::env::current_exe().unwrap();
    exe_path.pop(); // Remove the executable name to get the directory

    let entries = match fs::read_dir(&exe_path) {
        Ok(entries) => entries,
        Err(e) => {
            eprintln!("Failed to read directory {}: {}", exe_path.display(), e);
            return vec![]; // 또는 panic!() 또는 continue 등
        }
    };

    for entry_result in entries {
        if let Ok(entry) = entry_result {
            let path = entry.path();

            if path.is_file() {
                let content = fs::read_to_string(&path).unwrap_or_default();
                let mut lines = content.lines();

                if lines.next() != Some(PROFILE_HEADER) {
                    continue;
                }

                let json_data = lines.collect::<Vec<&str>>().join("\n");
                match serde_json::from_str::<DisplayProfile>(&json_data) {
                    Ok(profile) => {
                        profiles.push(ProfileInfo {
                            name: path
                                .file_stem()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string(),
                            path: path.to_string_lossy().to_string(),
                            profile,
                        });
                    }
                    Err(e) => {
                        eprintln!("프로필 파싱 실패 ({}): {}", path.display(), e);
                        continue;
                    }
                }
            }
        }
    }

    profiles
}
