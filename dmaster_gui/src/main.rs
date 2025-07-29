use dmaster_core::ProfileInfo;
use tauri::command;

/// Tauri 커맨드: 저장된 프로필 목록 반환
#[command]
fn get_profiles() -> Vec<ProfileInfo> {
    dmaster_core::profile_detector()
}

/// Tauri 커맨드: 현재 디스플레이 구성을 저장하고 파일 경로를 반환
#[command]
fn save_current_profile() -> String {
    let profile = dmaster_core::get_display_profile();
    let path = String::from("profile.json");
    dmaster_core::save_profile(&profile, &path);
    path
}

/// Tauri 커맨드: 주어진 이름의 프로필을 찾아 적용
#[command]
fn apply_profile_cmd(name: String) -> Result<(), String> {
    let profiles = dmaster_core::profile_detector();
    if let Some(p) = profiles.into_iter().find(|p| p.name == name) {
        dmaster_core::apply_profile(&p.profile);
        Ok(())
    } else {
        Err(format!("프로필 '{}' 을 찾을 수 없습니다.", name))
    }
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_profiles,
            save_current_profile,
            apply_profile_cmd
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
