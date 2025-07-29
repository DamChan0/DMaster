pub mod apply;
pub mod display_info;
pub mod profile;
pub mod query;

pub use apply::apply_profile;
pub use display_info::*;
pub use profile::{ProfileInfo, profile_detector, save_profile};
pub use query::get_display_profile;
