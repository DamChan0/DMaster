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

#[cfg(target_os = "linux")]
mod gnome_wayland;

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
mod unsupported;

fn is_wayland_session() -> bool {
    std::env::var("WAYLAND_DISPLAY").is_ok()
}

fn is_gnome_session() -> bool {
    std::env::var("XDG_CURRENT_DESKTOP")
        .map(|v| v.to_lowercase().contains("gnome"))
        .unwrap_or(false)
}

pub fn get_backend() -> Box<dyn DisplayBackend> {
    #[cfg(target_os = "windows")]
    {
        Box::new(windows::WindowsDisplayBackend)
    }

    #[cfg(target_os = "linux")]
    {
        if is_wayland_session() && is_gnome_session() {
            Box::new(gnome_wayland::GnomeWaylandBackend)
        } else {
            Box::new(linux::LinuxDisplayBackend)
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        Box::new(unsupported::UnsupportedDisplayBackend)
    }
}
