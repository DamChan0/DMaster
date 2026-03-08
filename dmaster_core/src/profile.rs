use crate::display_info::{DisplayProfile, MonitorProfile};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const LEGACY_PROFILE_HEADER: &str = "DMaster_v0_1_1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileInfo {
    pub name: String,
    pub path: String,
    pub profile: MonitorProfile,
}

pub fn save_profile(
    name: &str,
    description: Option<String>,
    profile: &DisplayProfile,
) -> Result<PathBuf, String> {
    let dir = profiles_dir()?;
    fs::create_dir_all(&dir).map_err(|error| {
        format!(
            "failed to create profile directory '{}': {error}",
            dir.display()
        )
    })?;

    let stored_profile = MonitorProfile::new(
        name.to_string(),
        description,
        current_timestamp_string(),
        profile.clone(),
    );
    let file_path = dir.join(format!("{}.json", sanitize_profile_name(name)));
    let json = serde_json::to_string_pretty(&stored_profile)
        .map_err(|error| format!("failed to serialize profile '{name}': {error}"))?;

    fs::write(&file_path, json)
        .map_err(|error| format!("failed to write profile '{}': {error}", file_path.display()))?;

    Ok(file_path)
}

pub fn load_profiles() -> Result<Vec<ProfileInfo>, String> {
    let dir = profiles_dir()?;
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let entries = fs::read_dir(&dir).map_err(|error| {
        format!(
            "failed to read profile directory '{}': {error}",
            dir.display()
        )
    })?;
    let mut profiles = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|error| format!("failed to read profile entry: {error}"))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let content = fs::read_to_string(&path)
            .map_err(|error| format!("failed to read profile '{}': {error}", path.display()))?;
        let profile = parse_profile_content(&path, &content)?;
        profiles.push(ProfileInfo {
            name: profile.name.clone(),
            path: path.to_string_lossy().to_string(),
            profile,
        });
    }

    profiles.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(profiles)
}

pub fn load_profile_by_name(name: &str) -> Result<ProfileInfo, String> {
    let profiles = load_profiles()?;
    profiles
        .into_iter()
        .find(|profile| profile.name == name)
        .ok_or_else(|| format!("profile '{name}' was not found"))
}

pub fn delete_profile(name: &str) -> Result<(), String> {
    let path = profiles_dir()?.join(format!("{}.json", sanitize_profile_name(name)));
    if !path.exists() {
        return Err(format!("profile '{}' does not exist", path.display()));
    }

    fs::remove_file(&path)
        .map_err(|error| format!("failed to delete profile '{}': {error}", path.display()))
}

pub fn profiles_dir() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .ok_or_else(|| {
            String::from("HOME/USERPROFILE is not set; cannot resolve profile directory")
        })?;

    Ok(home.join(".dmaster").join("profiles"))
}

fn parse_profile_content(path: &Path, content: &str) -> Result<MonitorProfile, String> {
    if content.lines().next() == Some(LEGACY_PROFILE_HEADER) {
        return parse_legacy_profile(path, content);
    }

    serde_json::from_str::<MonitorProfile>(content).map_err(|error| {
        format!(
            "failed to parse profile '{}' as MonitorProfile JSON: {error}",
            path.display()
        )
    })
}

fn parse_legacy_profile(path: &Path, content: &str) -> Result<MonitorProfile, String> {
    let mut lines = content.lines();
    let _header = lines.next();
    let json_data = lines.collect::<Vec<&str>>().join("\n");
    let legacy_profile = serde_json::from_str::<DisplayProfile>(&json_data).map_err(|error| {
        format!(
            "failed to parse legacy profile '{}' as DisplayProfile JSON: {error}",
            path.display()
        )
    })?;

    Ok(MonitorProfile::new(
        path.file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
        Some(String::from("Imported from legacy DMaster profile format")),
        current_timestamp_string(),
        legacy_profile,
    ))
}

fn sanitize_profile_name(name: &str) -> String {
    let sanitized = name
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => ch,
            _ => '_',
        })
        .collect::<String>();

    if sanitized.is_empty() {
        String::from("profile")
    } else {
        sanitized
    }
}

fn current_timestamp_string() -> String {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_secs().to_string(),
        Err(_) => String::from("0"),
    }
}
