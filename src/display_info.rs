use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    pub id: String,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub pos_x: f32,
    pub pos_y: f32,
    pub orientation: f32,
}
