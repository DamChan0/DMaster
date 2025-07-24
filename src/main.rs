mod apply;
mod display_info;
mod profile;
mod query;

use apply::apply_profile;
use profile::{load_profile, save_profile};
use query::get_display_profile;

use std::io::{self, Write};

fn main() {
    loop {
        println!("\n==== Display Profile 관리 메뉴 ====");
        println!("1) 현재 디스플레이 프로필 저장");
        println!("2) 저장된 프로필 적용");
        println!("3) 종료");
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
                print!("적용할 프로필 파일 경로를 입력하세요: ");
                io::stdout().flush().unwrap();
                let mut path = String::new();
                io::stdin().read_line(&mut path).unwrap();
                let path = path.trim();
                let profile = load_profile(path);
                apply_profile(&profile);
                println!("프로필을 적용했습니다: {}", path);
            }
            "3" => {
                println!("프로그램을 종료합니다.");
                break;
            }
            _ => {
                println!("잘못된 입력입니다. 다시 선택하세요.");
            }
        }
    }
}
