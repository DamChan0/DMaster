use crate::display_info::DisplayInfo;
use serde_json;
use std::{fs, path};

pub fn save_profile(profile: &[DisplayInfo], config_file_path: &str) {
    let json = serde_json::to_string_pretty(profile).unwrap();
    fs::write(path::Path::new(config_file_path), json).unwrap();
}

pub fn load_profile(path: &str) -> Vec<DisplayInfo> {
    let json = fs::read_to_string(path).unwrap();
    serde_json::from_str(&json).unwrap()
}
