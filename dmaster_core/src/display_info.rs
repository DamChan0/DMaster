use serde::{Deserialize, Serialize};

fn default_enabled() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    pub label: Option<String>,
    pub device_name: String,
    pub device_id: String,
    pub device_key: String,
    pub width: u32,
    pub height: u32,
    pub position_x: i32,
    pub position_y: i32,
    pub orientation: u32,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DisplayTopology {
    Extend,
    Clone,
    Internal,
    External,
    Unknown(i32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayProfile {
    pub topology: DisplayTopology,
    pub displays: Vec<DisplayConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorProfile {
    pub schema_version: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub topology: DisplayTopology,
    pub displays: Vec<DisplayConfig>,
}

impl MonitorProfile {
    pub fn new(
        name: String,
        description: Option<String>,
        created_at: String,
        profile: DisplayProfile,
    ) -> Self {
        Self {
            schema_version: String::from("0.2.0"),
            name,
            description,
            created_at,
            topology: profile.topology,
            displays: profile.displays,
        }
    }

    pub fn to_display_profile(&self) -> DisplayProfile {
        DisplayProfile {
            topology: self.topology.clone(),
            displays: self.displays.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayMapping {
    pub current_display_name: String,
    pub profile_display_index: usize,
}
