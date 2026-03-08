use crate::display_info::{DisplayMapping, DisplayProfile};

pub trait DisplayBackend {
    fn get_display_profile(&self) -> Result<DisplayProfile, String>;
    fn apply_profile(&self, profile: &DisplayProfile) -> Result<(), String>;
    fn apply_with_mapping(
        &self,
        profile: &DisplayProfile,
        mappings: &[DisplayMapping],
    ) -> Result<(), String>;
}

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
mod unsupported;

pub fn get_backend() -> Box<dyn DisplayBackend> {
    #[cfg(target_os = "windows")]
    {
        Box::new(windows::WindowsDisplayBackend)
    }

    #[cfg(target_os = "linux")]
    {
        Box::new(linux::LinuxDisplayBackend)
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        Box::new(unsupported::UnsupportedDisplayBackend)
    }
}
