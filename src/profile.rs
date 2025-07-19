use crate::display_info::DisplayProfile;
use serde_json;
use std::fs;

pub fn save_profile(profile: &DisplayProfile, path: &str) {
    let json = serde_json::to_string_pretty(profile).unwrap();
    fs::write(path, json).unwrap();
}

pub fn load_profile(path: &str) -> DisplayProfile {
    let json = fs::read_to_string(path).unwrap();
    serde_json::from_str(&json).unwrap()
}
