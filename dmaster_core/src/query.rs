use crate::display_info::{DisplayConfig, DisplayProfile, DisplayTopology};
use std::ptr;
use winapi::shared::minwindef::{DWORD, UINT};
use winapi::shared::ntdef::LPWSTR;
use winapi::shared::windef::HMONITOR;
use winapi::um::wingdi::{DEVMODEW, DISPLAY_DEVICEW};
use winapi::um::wingdi::{DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_PATH_INFO};
use winapi::um::winuser::{
    EDD_GET_DEVICE_INTERFACE_NAME, EnumDisplayDevicesW, EnumDisplaySettingsW,
};
// SetDisplayConfig/GetDisplayConfig FFI 선언 (winuser.h)
#[link(name = "user32")]
unsafe extern "system" {
    pub fn GetDisplayConfigBufferSizes(
        flags: u32,
        numPathArrayElements: *mut u32,
        numModeInfoArrayElements: *mut u32,
    ) -> i32;

    pub fn QueryDisplayConfig(
        flags: u32,
        numPathArrayElements: *mut u32,
        pathArray: *mut winapi::um::wingdi::DISPLAYCONFIG_PATH_INFO,
        numModeInfoArrayElements: *mut u32,
        modeInfoArray: *mut winapi::um::wingdi::DISPLAYCONFIG_MODE_INFO,
        currentTopologyId: *mut u32,
    ) -> i32;
}

const QDC_ALL_PATHS: u32 = 1;
const SDC_TOPOLOGY_INTERNAL: u32 = 0x00000001;
const SDC_TOPOLOGY_CLONE: u32 = 0x00000002;
const SDC_TOPOLOGY_EXTEND: u32 = 0x00000004;
const SDC_TOPOLOGY_EXTERNAL: u32 = 0x00000008;

fn query_display_topology() -> DisplayTopology {
    use std::collections::HashMap;
    use std::ptr;
    use winapi::um::wingdi::{DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_PATH_INFO};

    const QDC_ONLY_ACTIVE_PATHS: u32 = 0x00000002;

    let mut num_paths: u32 = 0;
    let mut num_modes: u32 = 0;

    let status = unsafe {
        GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut num_paths, &mut num_modes)
    };
    if status != 0 {
        return DisplayTopology::Unknown(status);
    }

    let mut paths: Vec<DISPLAYCONFIG_PATH_INFO> =
        vec![unsafe { std::mem::zeroed() }; num_paths as usize];
    let mut modes: Vec<DISPLAYCONFIG_MODE_INFO> =
        vec![unsafe { std::mem::zeroed() }; num_modes as usize];

    let status2 = unsafe {
        QueryDisplayConfig(
            QDC_ONLY_ACTIVE_PATHS,
            &mut num_paths,
            paths.as_mut_ptr(),
            &mut num_modes,
            modes.as_mut_ptr(),
            ptr::null_mut(),
        )
    };
    if status2 != 0 {
        return DisplayTopology::Unknown(status2);
    }

    // 해석: 복제(clone)인지, 확장(extend)인지, 내부/외부(single)인지
    if num_paths == 1 {
        return DisplayTopology::Internal; // 또는 External, 추가 판별 가능
    }

    // 복제: 여러 path의 sourceId가 같으면 복제
    let mut source_id_count: HashMap<u32, usize> = HashMap::new();
    for path in &paths {
        let src_id = path.sourceInfo.id;
        *source_id_count.entry(src_id).or_insert(0) += 1;
    }
    // 하나의 sourceId가 여러 path에 등장 → 복제
    if source_id_count.values().any(|&count| count > 1) {
        return DisplayTopology::Clone;
    } else {
        return DisplayTopology::Extend;
    }
}

pub fn debug_display_paths() {
    use std::ptr;
    use winapi::um::wingdi::{DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_PATH_INFO};

    const QDC_ONLY_ACTIVE_PATHS: u32 = 0x00000002;

    let mut num_paths: u32 = 0;
    let mut num_modes: u32 = 0;

    // 1. 먼저 필요한 버퍼 크기 파악 (flags를 꼭 QDC_ONLY_ACTIVE_PATHS로)
    let status = unsafe {
        GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut num_paths, &mut num_modes)
    };
    println!(
        "BufferSizes: status={}, num_paths={}, num_modes={}",
        status, num_paths, num_modes
    );

    if status != 0 {
        println!("GetDisplayConfigBufferSizes failed with code {}", status);
        return;
    }

    // 2. 버퍼 할당
    let mut paths: Vec<DISPLAYCONFIG_PATH_INFO> =
        vec![unsafe { std::mem::zeroed() }; num_paths as usize];
    let mut modes: Vec<DISPLAYCONFIG_MODE_INFO> =
        vec![unsafe { std::mem::zeroed() }; num_modes as usize];

    let status2 = unsafe {
        QueryDisplayConfig(
            QDC_ONLY_ACTIVE_PATHS, // 여기도 같은 flags 사용!
            &mut num_paths,
            paths.as_mut_ptr(),
            &mut num_modes,
            modes.as_mut_ptr(),
            ptr::null_mut(), // topology id는 불필요
        )
    };

    println!(
        "QueryDisplayConfig: status={}, num_paths={}, num_modes={}",
        status2, num_paths, num_modes
    );

    if status2 != 0 {
        println!("QueryDisplayConfig failed with code {}", status2);
        return;
    }

    // 3. path와 mode info 간단 출력
    for (i, path) in paths.iter().enumerate() {
        println!(
            "Path[{}]: sourceId = {:?}, targetId = {:?}, flags = {}",
            i, path.sourceInfo.id, path.targetInfo.id, path.flags
        );
    }
    // 필요하다면 mode info도 비슷하게 출력 가능
}

pub fn get_display_profile() -> DisplayProfile {
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
