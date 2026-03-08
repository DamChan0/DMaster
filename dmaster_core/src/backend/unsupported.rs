use crate::backend::DisplayBackend;
use crate::display_info::{DisplayMapping, DisplayProfile};

pub struct UnsupportedDisplayBackend;

impl DisplayBackend for UnsupportedDisplayBackend {
    fn get_display_profile(&self) -> Result<DisplayProfile, String> {
        Err(String::from(
            "display querying is not supported on this platform",
        ))
    }

    fn apply_profile(&self, _profile: &DisplayProfile) -> Result<(), String> {
        Err(String::from(
            "display applying is not supported on this platform",
        ))
    }

    fn apply_with_mapping(
        &self,
        _profile: &DisplayProfile,
        _mappings: &[DisplayMapping],
    ) -> Result<(), String> {
        Err(String::from(
            "display mapping is not supported on this platform",
        ))
    }
}
