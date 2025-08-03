use dmaster_core::ProfileInfo;
use tauri::command;

// Section 1: 함수 설명
// ① get_profiles
//   역할      : 저장된 모든 프로필 반환
//   파라미터  : 없음
//   시나리오  : GUI가 프로필 목록 요청 시 호출
//   결과      : Vec<ProfileInfo>
#[command]
fn get_profiles() -> Vec<ProfileInfo> {
    dmaster_core::profile_detector()
}

// ② save_current_profile
//   역할      : 현재 디스플레이 구성을 파일로 저장
//   파라미터  : 없음
//   결과      : 저장된 파일 경로(String)
#[command]
fn save_profile(name: String) -> Result<(), String> {
    let profile = dmaster_core::get_display_profile();
    let info = ProfileInfo {
        name: name.clone(),
        profile,
        path: todo!(),
    };
    let dir = "profiles";
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let path = format!("{}/{}.json", dir, name);

    dmaster_core::save_profile(&info.profile, &path);
    Ok(())
}
// ③ apply_profile_cmd
//   역할      : 이름으로 프로필 찾아 적용
//   파라미터  : name(String) – 적용할 프로필명
//   결과      : 성공 시 Ok, 실패 시 Err(String)
#[command]
fn apply_profile_cmd(name: String) -> Result<(), String> {
    let profiles = dmaster_core::profile_detector();
    match profiles.into_iter().find(|p| p.name == name) {
        Some(p) => {
            dmaster_core::apply_profile(&p.profile);
            Ok(())
        }
        None => Err(format!("프로필 '{}' 을 찾을 수 없습니다.", name)),
    }
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_profiles,
            save_profile,
            apply_profile_cmd
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
