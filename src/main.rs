mod apply;
mod display_info;
mod profile;
mod query;

use apply::apply_profile;
use profile::{profile_detector, save_profile};
use query::get_display_profile;

use std::io::{self, Write};

fn main() {
    loop {
        println!("\n==== Display Profile 관리 메뉴 ====");
        println!("[1] 현재 디스플레이 구성 저장");
        println!("[2] 저장된 프로필 목록 불러오기 및 적용");
        println!("[3] 종료");
        print!("옵션 번호를 입력하세요: ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let trimmed = input.trim();

        match trimmed {
            "1" => {
                print!("저장할 프로필 파일 경로를 입력하세요: ");
                io::stdout().flush().unwrap();
                let mut save_path = String::new();
                io::stdin().read_line(&mut save_path).unwrap();
                let save_path = save_path.trim();
                let profile = get_display_profile();
                save_profile(&profile, save_path);
                println!("프로필을 저장했습니다: {}", save_path);
            }
            "2" => {
                let profiles = profile_detector();
                if profiles.is_empty() {
                    println!("저장된 프로필이 없습니다.");
                    continue;
                }

                println!("=== 저장된 프로필 목록 ===");
                for (i, p) in profiles.iter().enumerate() {
                    println!("[{}] : NAME: {}", i, p.name);
                }

                print!("적용할 프로필 번호 선택: ");
                std::io::stdout().flush().unwrap();
                let mut sel = String::new();
                std::io::stdin().read_line(&mut sel).unwrap();

                if let Ok(index) = sel.trim().parse::<usize>() {
                    if let Some(selected) = profiles.get(index) {
                        println!("프로필 {} 적용 중...", selected.name);
                        apply_profile(&selected.profile); // <- 실제 적용 함수
                    } else {
                        println!("잘못된 인덱스입니다.");
                    }
                } else {
                    println!("숫자를 입력해주세요.");
                }
            }
            "3" => {
                break;
            }
            _ => println!("잘못된 입력입니다."),
        }
    }
    println!("Press Enter to exit...");
    io::stdout().flush().unwrap();
    let _ = io::stdin().read_line(&mut String::new());
}
