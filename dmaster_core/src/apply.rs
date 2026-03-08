use crate::backend;
use crate::display_info::{DisplayMapping, DisplayProfile};

pub fn apply_profile(profile: &DisplayProfile) -> Result<(), String> {
    backend::get_backend().apply_profile(profile)
}

pub fn apply_profile_with_mapping(
    profile: &DisplayProfile,
    mappings: &[DisplayMapping],
) -> Result<(), String> {
    backend::get_backend().apply_with_mapping(profile, mappings)
}
