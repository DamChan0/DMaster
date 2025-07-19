use crate::display_info::{DisplayConfig, DisplayProfile, DisplayTopology};
use std::ptr;
use winapi::shared::minwindef::{DWORD, UINT};
use winapi::shared::ntdef::LPWSTR;
use winapi::shared::windef::HMONITOR;
use winapi::um::wingdi::{DEVMODEW, DISPLAY_DEVICEW};
use winapi::um::winuser::{
    EDD_GET_DEVICE_INTERFACE_NAME, EnumDisplayDevicesW, EnumDisplaySettingsW,
};

// SetDisplayConfig/GetDisplayConfig FFI 선언 (winuser.h)
#[link(name = "user32")]
unsafe extern "system" {
    fn GetDisplayConfigBufferSizes(
        flags: UINT,
        numPathArrayElements: *mut UINT,
        numModeInfoArrayElements: *mut UINT,
    ) -> i32;
    fn QueryDisplayConfig(
        flags: UINT,
        numPathArrayElements: *mut UINT,
        pathArray: *mut core::ffi::c_void,
        numModeInfoArrayElements: *mut UINT,
        modeInfoArray: *mut core::ffi::c_void,
        currentTopologyId: *mut UINT,
    ) -> i32;
}

const QDC_ALL_PATHS: u32 = 1;
const SDC_TOPOLOGY_INTERNAL: u32 = 0x00000001;
const SDC_TOPOLOGY_CLONE: u32 = 0x00000002;
const SDC_TOPOLOGY_EXTEND: u32 = 0x00000004;
const SDC_TOPOLOGY_EXTERNAL: u32 = 0x00000008;

fn query_display_topology() -> DisplayTopology {
    unsafe {
        let mut current_topology: UINT = 0;
        // buffer 크기 0으로, topology 정보만 받음 (docs 참고)
        let status = QueryDisplayConfig(
            0,                    // flags: 0 = only current topology
            &mut 0u32,            // numPathArrayElements
            std::ptr::null_mut(), // pathArray
            &mut 0u32,            // numModeInfoArrayElements
            std::ptr::null_mut(), // modeInfoArray
            &mut current_topology,
        );
        if status == 0 {
            match current_topology {
                SDC_TOPOLOGY_EXTEND => DisplayTopology::Extend,
                SDC_TOPOLOGY_CLONE => DisplayTopology::Clone,
                SDC_TOPOLOGY_INTERNAL => DisplayTopology::Internal,
                SDC_TOPOLOGY_EXTERNAL => DisplayTopology::External,
                other => DisplayTopology::Unknown(other as i32),
            }
        } else {
            DisplayTopology::Unknown(status)
        }
    }
}

pub fn get_display_info() -> DisplayProfile {
    let mut display_info_list: Vec<DisplayConfig> = vec![];
    let mut device_num = 0;
    loop {
        let mut device: DISPLAY_DEVICEW = unsafe { std::mem::zeroed() };
        device.cb = std::mem::size_of::<DISPLAY_DEVICEW>() as u32;
        // 디스플레이 장치 정보 가져오기
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
        // DEVMODEW 초기화
        let mut devmode: DEVMODEW = unsafe { std::mem::zeroed() };
        devmode.dmSize = std::mem::size_of::<DEVMODEW>() as u16;

        // 현재 장치의 디스플레이 설정 얻기
        if unsafe { EnumDisplaySettingsW(device.DeviceName.as_ptr(), u32::MAX, &mut devmode) } != 0
        {
            // 유니코드 문자열에서 null 문자 제거
            let device_name = String::from_utf16_lossy(&device.DeviceName)
                .trim_end_matches('\0')
                .to_string();

            let device_id = String::from_utf16_lossy(&device.DeviceID)
                .trim_end_matches('\0')
                .to_string();

            let device_key = String::from_utf16_lossy(&device.DeviceKey)
                .trim_end_matches('\0')
                .to_string();

            // 위치, 방향, 해상도 필드 안전하게 추출
            let (position_x, position_y, orientation) = unsafe {
                let s2 = devmode.u1.s2();
                (s2.dmPosition.x, s2.dmPosition.y, s2.dmDisplayOrientation)
            };

            display_info_list.push(DisplayConfig {
                device_name,
                device_id,
                device_key,
                width: devmode.dmPelsWidth,
                height: devmode.dmPelsHeight,
                position_x,
                position_y,
                orientation,
            });
        }

        device_num += 1;
    }

    DisplayProfile {
        topology: query_display_topology(),
        displays: display_info_list,
    }
}
