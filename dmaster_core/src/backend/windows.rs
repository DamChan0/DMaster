use crate::backend::DisplayBackend;
use crate::display_info::{DisplayConfig, DisplayMapping, DisplayProfile, DisplayTopology};
use std::ptr;
use winapi::shared::windef::POINTL;
use winapi::um::wingdi::{
    DEVMODEW, DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_PATH_INFO, DISPLAY_DEVICEW,
    DISPLAY_DEVICE_ACTIVE, DM_DISPLAYORIENTATION, DM_PELSHEIGHT, DM_PELSWIDTH, DM_POSITION,
};
use winapi::um::winuser::{
    ChangeDisplaySettingsExW, EnumDisplayDevicesW, EnumDisplaySettingsW, CDS_RESET,
    CDS_UPDATEREGISTRY, EDD_GET_DEVICE_INTERFACE_NAME,
};

const SDC_TOPOLOGY_INTERNAL: u32 = 0x00000001;
const SDC_TOPOLOGY_CLONE: u32 = 0x00000002;
const SDC_TOPOLOGY_EXTEND: u32 = 0x00000004;
const SDC_TOPOLOGY_EXTERNAL: u32 = 0x00000008;
const SDC_APPLY: u32 = 0x00000080;
const QDC_ONLY_ACTIVE_PATHS: u32 = 0x00000002;

#[link(name = "user32")]
unsafe extern "system" {
    fn GetDisplayConfigBufferSizes(
        flags: u32,
        num_path_array_elements: *mut u32,
        num_mode_info_array_elements: *mut u32,
    ) -> i32;

    fn QueryDisplayConfig(
        flags: u32,
        num_path_array_elements: *mut u32,
        path_array: *mut DISPLAYCONFIG_PATH_INFO,
        num_mode_info_array_elements: *mut u32,
        mode_info_array: *mut DISPLAYCONFIG_MODE_INFO,
        current_topology_id: *mut u32,
    ) -> i32;

    fn SetDisplayConfig(
        num_path_array_elements: u32,
        path_array: *const core::ffi::c_void,
        num_mode_info_array_elements: u32,
        mode_info_array: *const core::ffi::c_void,
        flags: u32,
    ) -> i32;
}

pub struct WindowsDisplayBackend;

impl DisplayBackend for WindowsDisplayBackend {
    fn get_display_profile(&self) -> Result<DisplayProfile, String> {
        get_display_profile_impl()
    }

    fn apply_profile(&self, profile: &DisplayProfile) -> Result<(), String> {
        apply_resolved_profile(profile)
    }

    fn apply_with_mapping(
        &self,
        profile: &DisplayProfile,
        mappings: &[DisplayMapping],
    ) -> Result<(), String> {
        let current_profile = get_display_profile_impl()?;
        let mut mapped_displays = Vec::new();

        for mapping in mappings {
            let current_display = current_profile
                .displays
                .iter()
                .find(|display| display.device_name == mapping.current_display_name)
                .ok_or_else(|| {
                    format!(
                        "current display '{}' was not found while applying mapped profile",
                        mapping.current_display_name
                    )
                })?;

            let source_display = profile
                .displays
                .get(mapping.profile_display_index)
                .ok_or_else(|| {
                    format!(
                        "profile display index {} is out of range",
                        mapping.profile_display_index
                    )
                })?;

            mapped_displays.push(merge_display_config(current_display, source_display));
        }

        let mapped_profile = DisplayProfile {
            topology: profile.topology.clone(),
            displays: mapped_displays,
        };

        apply_resolved_profile(&mapped_profile)
    }
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

fn query_display_topology() -> Result<DisplayTopology, String> {
    use std::collections::HashMap;

    let mut num_paths: u32 = 0;
    let mut num_modes: u32 = 0;

    let status = unsafe {
        GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut num_paths, &mut num_modes)
    };
    if status != 0 {
        return Ok(DisplayTopology::Unknown(status));
    }

    let mut paths: Vec<DISPLAYCONFIG_PATH_INFO> =
        vec![unsafe { std::mem::zeroed() }; num_paths as usize];
    let mut modes: Vec<DISPLAYCONFIG_MODE_INFO> =
        vec![unsafe { std::mem::zeroed() }; num_modes as usize];

    let status = unsafe {
        QueryDisplayConfig(
            QDC_ONLY_ACTIVE_PATHS,
            &mut num_paths,
            paths.as_mut_ptr(),
            &mut num_modes,
            modes.as_mut_ptr(),
            ptr::null_mut(),
        )
    };
    if status != 0 {
        return Ok(DisplayTopology::Unknown(status));
    }

    if num_paths == 1 {
        return Ok(DisplayTopology::Internal);
    }

    let mut source_id_count: HashMap<u32, usize> = HashMap::new();
    for path in &paths {
        let src_id = path.sourceInfo.id;
        *source_id_count.entry(src_id).or_insert(0) += 1;
    }

    if source_id_count.values().any(|count| *count > 1) {
        Ok(DisplayTopology::Clone)
    } else {
        Ok(DisplayTopology::Extend)
    }
}

fn get_display_profile_impl() -> Result<DisplayProfile, String> {
    let mut display_info_list: Vec<DisplayConfig> = vec![];
    let mut device_num = 0;

    loop {
        let mut device: DISPLAY_DEVICEW = unsafe { std::mem::zeroed() };
        device.cb = std::mem::size_of::<DISPLAY_DEVICEW>() as u32;
        let success = unsafe {
            EnumDisplayDevicesW(
                ptr::null(),
                device_num,
                &mut device,
                EDD_GET_DEVICE_INTERFACE_NAME,
            )
        };
        if success == 0 {
            break;
        }

        let mut devmode: DEVMODEW = unsafe { std::mem::zeroed() };
        devmode.dmSize = std::mem::size_of::<DEVMODEW>() as u16;

        if unsafe { EnumDisplaySettingsW(device.DeviceName.as_ptr(), u32::MAX, &mut devmode) } != 0
        {
            let device_name = String::from_utf16_lossy(&device.DeviceName)
                .trim_end_matches('\0')
                .to_string();
            let device_id = String::from_utf16_lossy(&device.DeviceID)
                .trim_end_matches('\0')
                .to_string();
            let device_key = String::from_utf16_lossy(&device.DeviceKey)
                .trim_end_matches('\0')
                .to_string();
            let (position_x, position_y, orientation) = unsafe {
                let state = devmode.u1.s2();
                (
                    state.dmPosition.x,
                    state.dmPosition.y,
                    state.dmDisplayOrientation,
                )
            };

            display_info_list.push(DisplayConfig {
                label: Some(device_name.clone()),
                device_name,
                device_id,
                device_key,
                width: devmode.dmPelsWidth,
                height: devmode.dmPelsHeight,
                position_x,
                position_y,
                orientation,
                enabled: true,
            });
        }

        device_num += 1;
    }

    Ok(DisplayProfile {
        topology: query_display_topology()?,
        displays: display_info_list,
    })
}

fn apply_resolved_profile(profile: &DisplayProfile) -> Result<(), String> {
    if profile.displays.is_empty() {
        return Err(String::from(
            "profile does not contain any displays to apply",
        ));
    }

    if !profile.displays.iter().any(|d| d.enabled) {
        return Err(String::from("no enabled displays in profile"));
    }

    let all_display_names = query_all_display_names();
    for display in &profile.displays {
        if !all_display_names
            .iter()
            .any(|name| name == &display.device_name)
        {
            return Err(format!(
                "target display '{}' is not currently connected",
                display.device_name
            ));
        }
    }

    let flag = get_topology_flag(&profile.topology) | SDC_APPLY;
    let topology_status = unsafe { SetDisplayConfig(0, ptr::null(), 0, ptr::null(), flag) };
    if topology_status != 0 {
        return Err(format!(
            "failed to apply display topology with status code {}",
            topology_status
        ));
    }

    for display in &profile.displays {
        if display.enabled {
            apply_single_display(display)?;
        } else {
            detach_display(display)?;
        }
    }

    Ok(())
}

fn apply_single_display(display_config: &DisplayConfig) -> Result<(), String> {
    let mut devmode: DEVMODEW = unsafe { std::mem::zeroed() };
    devmode.dmSize = std::mem::size_of::<DEVMODEW>() as u16;
    devmode.dmPelsWidth = display_config.width;
    devmode.dmPelsHeight = display_config.height;
    devmode.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT | DM_POSITION | DM_DISPLAYORIENTATION;

    unsafe {
        let state = devmode.u1.s2_mut();
        state.dmPosition = POINTL {
            x: display_config.position_x,
            y: display_config.position_y,
        };
        state.dmDisplayOrientation = display_config.orientation;
    }

    let device_w: Vec<u16> = display_config
        .device_name
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let status = unsafe {
        ChangeDisplaySettingsExW(
            device_w.as_ptr(),
            &mut devmode,
            ptr::null_mut(),
            CDS_UPDATEREGISTRY | CDS_RESET,
            ptr::null_mut(),
        )
    };

    if status != 0 {
        return Err(format!(
            "failed to apply settings for '{}' with status code {}",
            display_config.device_name, status
        ));
    }

    Ok(())
}

fn detach_display(display_config: &DisplayConfig) -> Result<(), String> {
    let mut devmode: DEVMODEW = unsafe { std::mem::zeroed() };
    devmode.dmSize = std::mem::size_of::<DEVMODEW>() as u16;
    devmode.dmPelsWidth = 0;
    devmode.dmPelsHeight = 0;
    devmode.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT | DM_POSITION;

    unsafe {
        let state = devmode.u1.s2_mut();
        state.dmPosition = POINTL { x: 0, y: 0 };
    }

    let device_w: Vec<u16> = display_config
        .device_name
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let status = unsafe {
        ChangeDisplaySettingsExW(
            device_w.as_ptr(),
            &mut devmode,
            ptr::null_mut(),
            CDS_UPDATEREGISTRY,
            ptr::null_mut(),
        )
    };

    if status != 0 {
        return Err(format!(
            "failed to detach display '{}' with status code {}",
            display_config.device_name, status
        ));
    }

    Ok(())
}

fn query_all_display_names() -> Vec<String> {
    let mut names = vec![];
    let mut device_num = 0;

    loop {
        let mut device: DISPLAY_DEVICEW = unsafe { std::mem::zeroed() };
        device.cb = std::mem::size_of::<DISPLAY_DEVICEW>() as u32;
        let success = unsafe {
            EnumDisplayDevicesW(
                ptr::null(),
                device_num,
                &mut device,
                EDD_GET_DEVICE_INTERFACE_NAME,
            )
        };
        if success == 0 {
            break;
        }

        let device_name = String::from_utf16_lossy(&device.DeviceName)
            .trim_end_matches('\0')
            .to_string();
        names.push(device_name);

        device_num += 1;
    }

    names
}

fn query_current_display_names() -> Vec<String> {
    let mut names = vec![];
    let mut device_num = 0;

    loop {
        let mut device: DISPLAY_DEVICEW = unsafe { std::mem::zeroed() };
        device.cb = std::mem::size_of::<DISPLAY_DEVICEW>() as u32;
        let success = unsafe {
            EnumDisplayDevicesW(
                ptr::null(),
                device_num,
                &mut device,
                EDD_GET_DEVICE_INTERFACE_NAME,
            )
        };
        if success == 0 {
            break;
        }

        if device.StateFlags & DISPLAY_DEVICE_ACTIVE != 0 {
            let device_name = String::from_utf16_lossy(&device.DeviceName)
                .trim_end_matches('\0')
                .to_string();
            names.push(device_name);
        }

        device_num += 1;
    }

    names
}

fn merge_display_config(
    current_display: &DisplayConfig,
    source_display: &DisplayConfig,
) -> DisplayConfig {
    DisplayConfig {
        label: source_display
            .label
            .clone()
            .or_else(|| current_display.label.clone()),
        device_name: current_display.device_name.clone(),
        device_id: current_display.device_id.clone(),
        device_key: current_display.device_key.clone(),
        width: source_display.width,
        height: source_display.height,
        position_x: source_display.position_x,
        position_y: source_display.position_y,
        orientation: source_display.orientation,
        enabled: source_display.enabled,
    }
}
