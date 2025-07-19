use crate::display_info::{DisplayConfig, DisplayProfile, DisplayTopology};
use std::ptr;
use winapi::shared::windef::POINTL;
use winapi::um::wingdi::{
    DEVMODEW, DM_DISPLAYORIENTATION, DM_PELSHEIGHT, DM_PELSWIDTH, DM_POSITION,
};
use winapi::um::winuser::{CDS_RESET, CDS_UPDATEREGISTRY, ChangeDisplaySettingsExW};

const SDC_TOPOLOGY_INTERNAL: u32 = 0x00000001;
const SDC_TOPOLOGY_CLONE: u32 = 0x00000002;
const SDC_TOPOLOGY_EXTEND: u32 = 0x00000004;
const SDC_TOPOLOGY_EXTERNAL: u32 = 0x00000008;
const SDC_APPLY: u32 = 0x00000080;

unsafe extern "system" {
    fn SetDisplayConfig(
        numPathArrayElements: u32,
        pathArray: *const core::ffi::c_void,
        numModeInfoArrayElements: u32,
        modeInfoArray: *const core::ffi::c_void,
        flags: u32,
    ) -> i32;
}

fn get_topology_flag(topology: &DisplayTopology) -> u32 {
    match topology {
        DisplayTopology::Extend => SDC_TOPOLOGY_EXTEND,
        DisplayTopology::Clone => SDC_TOPOLOGY_CLONE,
        DisplayTopology::Internal => SDC_TOPOLOGY_INTERNAL,
        DisplayTopology::External => SDC_TOPOLOGY_EXTERNAL,
        DisplayTopology::Unknown(_) => SDC_TOPOLOGY_EXTEND,
    }
}

pub fn apply_profile(profile: &DisplayProfile) {
    let flag = get_topology_flag(&profile.topology) | SDC_APPLY;
    unsafe {
        SetDisplayConfig(0, ptr::null(), 0, ptr::null(), flag);
    }
    for display_config in &profile.displays {
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
