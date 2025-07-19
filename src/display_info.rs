use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    pub device_name: String,
    pub device_id: String,
    pub device_key: String,
    pub width: u32,
    pub height: u32,
    pub position_x: i32,
    pub position_y: i32,
    pub orientation: u32,
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
