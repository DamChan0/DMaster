use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayInfo {
    pub device_name: String,
    pub width: u32,
    pub height: u32,
    pub position_x: i32,
    pub position_y: i32,
    pub orientation: u32,
}
