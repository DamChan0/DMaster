use crate::display_info::DisplayInfo;
use std::ptr;
use winapi::um::wingdi::{
    DEVMODEW, DISPLAY_DEVICEW, DM_DISPLAYORIENTATION, DM_PELSHEIGHT, DM_PELSWIDTH, DM_POSITION,
};

use winapi::shared::windef::POINTL;

use winapi::um::winuser::{
    CDS_RESET, CDS_UPDATEREGISTRY, ChangeDisplaySettingsExW, EDD_GET_DEVICE_INTERFACE_NAME,
    EnumDisplayDevicesW, EnumDisplaySettingsW,
};

pub fn get_display_info() -> Vec<DisplayInfo> {
    let mut display_info_list: Vec<DisplayInfo> = vec![];
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

            // 위치, 방향, 해상도 필드 안전하게 추출
            let (position_x, position_y, orientation) = unsafe {
                let s2 = devmode.u1.s2();
                (s2.dmPosition.x, s2.dmPosition.y, s2.dmDisplayOrientation)
            };

            display_info_list.push(DisplayInfo {
                device_name,
                width: devmode.dmPelsWidth,
                height: devmode.dmPelsHeight,
                position_x,
                position_y,
                orientation,
            });
        }

        device_num += 1;
    }

    display_info_list
}
