use crate::display_info::DisplayProfile;
use serde_json;
use std::fs;

struct ProfileInfo {
    pub name: String,
    pub path: String,
    pub profile: DisplayProfile,
}

pub fn save_profile(profile: &DisplayProfile, path: &str) {
    let header: &str = "DMaster_v0_1_1";
    let json = serde_json::to_string_pretty(profile).unwrap();
    fs::write(path, format!("{}\n{}", header, json)).unwrap();
}

pub fn load_profile(path: &str) -> DisplayProfile {
    let json = fs::read_to_string(path).unwrap();
    serde_json::from_str(&json).unwrap()
}

pub fn profile_detector() -> Vec<ProfileInfo> {
    let mut current_path = std::env::current_exe().unwrap();
    fs::read(path)
}
