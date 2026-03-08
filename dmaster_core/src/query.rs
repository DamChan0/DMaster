use crate::backend;
use crate::display_info::DisplayProfile;

pub fn get_display_profile() -> Result<DisplayProfile, String> {
    backend::get_backend().get_display_profile()
}
