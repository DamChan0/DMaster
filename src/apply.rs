use crate::display_info::DisplayInfo;
use std::ptr;
use winapi::shared::windef::POINTL;
use winapi::um::wingdi::{
    DEVMODEW, DM_DISPLAYORIENTATION, DM_PELSHEIGHT, DM_PELSWIDTH, DM_POSITION,
};
use winapi::um::winuser::{CDS_RESET, CDS_UPDATEREGISTRY, ChangeDisplaySettingsExW};

pub fn apply_profile(profile: &[DisplayInfo]) {
    for display_config in profile {
        let mut devmode: DEVMODEW = unsafe { std::mem::zeroed() };
        devmode.dmSize = std::mem::size_of::<DEVMODEW>() as u16;
        devmode.dmPelsWidth = display_config.width;
        devmode.dmPelsHeight = display_config.height;
        devmode.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT | DM_POSITION | DM_DISPLAYORIENTATION;

        unsafe {
            let s2 = devmode.u1.s2_mut();
            s2.dmPosition = POINTL {
                x: display_config.position_x,
                y: display_config.position_y,
            };
            s2.dmDisplayOrientation = display_config.orientation;
        }

        let device_w: Vec<u16> = display_config
            .device_name
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        unsafe {
            ChangeDisplaySettingsExW(
                device_w.as_ptr(),
                &mut devmode,
                ptr::null_mut(),
                CDS_UPDATEREGISTRY | CDS_RESET,
                ptr::null_mut(),
            );
        }
    }
}
