# 10. CLI 프론트엔드 설계

## 이 문서에서 배우는 것

- `dmaster_cli`가 `dmaster_core`를 사용하는 방식
- 메뉴 루프/입력 파싱/에러 처리 패턴
- 매핑 적용 + 비활성화(`off`) 플로우

---

## 1. 전체 구조

대상 파일: `dmaster_cli/src/main.rs`

핵심은 "UI만 CLI로 바꾼 thin frontend"다.

```text
main() 메뉴 루프
  ├─ save_current_profile_flow()
  ├─ list_profiles_flow()
  ├─ apply_profile_flow(false)
  ├─ apply_profile_flow(true)
  └─ delete_profile_flow()

실제 비즈니스 로직은 모두 dmaster_core 함수 호출
```

---

## 2. 메뉴 루프 패턴

```rust
loop {
    println!("[1] Save ...");
    // ...
    match prompt("Select an option: ").trim() {
        "1" => save_current_profile_flow(),
        "2" => list_profiles_flow(),
        "3" => apply_profile_flow(false),
        "4" => apply_profile_flow(true),
        "5" => delete_profile_flow(),
        "6" => break,
        _ => println!("Invalid option."),
    }
}
```

패턴 포인트:

- 잘못된 입력은 종료하지 않고 루프 유지
- `prompt(...).trim()`으로 공백 입력 정리
- 동작 분기만 하고, 실제 처리 함수는 분리

---

## 3. 입력 유틸리티 분리

```rust
fn prompt(message: &str) -> String {
    print!("{message}");
    io::stdout().flush().expect("stdout flush failed");
    let mut input = String::new();
    io::stdin().read_line(&mut input).expect("stdin read failed");
    input.trim_end().to_string()
}
```

의도:

- 입출력 보일러플레이트를 한 곳으로 모음
- 모든 flow 함수가 동일한 입력 UX를 재사용

`trimmed`/`optional_trimmed`도 같은 목적의 미니 헬퍼다.

---

## 4. Core API 재사용 방식

```rust
use dmaster_core::{
    apply_profile, apply_profile_with_mapping, delete_profile,
    get_display_profile, load_profiles, profiles_dir, save_profile, DisplayMapping,
};
```

CLI는 직접 OS API를 만지지 않는다.

- 조회: `get_display_profile()`
- 저장: `save_profile(...)`
- 적용: `apply_profile(...)` / `apply_profile_with_mapping(...)`
- 삭제: `delete_profile(...)`

즉, GUI와 CLI가 같은 core를 공유하고 입력/출력 채널만 다르다.

---

## 5. Apply 플로우 심화

### 5-1. 직접 적용 vs 매핑 적용

```rust
let result = if use_mapping {
    apply_with_manual_mapping(selected_profile)
} else {
    apply_profile(&selected_profile.profile.to_display_profile())
};
```

### 5-2. 수동 매핑 + off 처리

`apply_with_manual_mapping()`에서는 각 현재 모니터마다 입력을 받는다:

- 숫자 입력: 해당 profile index로 매핑
- `off` 입력: 해당 현재 모니터 비활성화 목록에 추가
- 빈 입력: 스킵

```rust
if trimmed_answer.eq_ignore_ascii_case("off") {
    disabled_names.push(current_display.device_name.clone());
    continue;
}
```

그 후 2단계 적용:

1. 매핑이 있으면 `apply_profile_with_mapping(...)`
2. 비활성 대상이 있으면 `enabled: false` 프로필을 구성해서 `apply_profile(...)`

이 구조 덕분에 CLI에서 "특정 디스플레이 끄기"까지 다룰 수 있다.

---

## 6. 에러 처리 스타일

CLI는 `Result`를 UI 경계에서 문자열 출력으로 변환한다.

```rust
match save_profile(...) {
    Ok(path) => println!("Saved profile to {}", path.display()),
    Err(error) => println!("Failed to save profile: {error}"),
}
```

핵심은 panic 대신 사용자 메시지로 복구 가능하게 처리하는 것이다.

---

## 확인 문제

1. CLI가 `dmaster_core`를 직접 호출하는 설계의 장점은?
2. `off` 입력은 최종적으로 어떤 데이터 구조로 전달되는가?
3. `select_profile<'a>`에 라이프타임이 필요한 이유는?

<details>
<summary>정답</summary>

1. 비즈니스 로직 중복 없이 GUI/CLI를 동시에 유지할 수 있고 테스트도 core 중심으로 단순화된다.
2. `DisplayConfig { enabled: false, ... }` 목록을 가진 `DisplayProfile`로 전달된다.
3. 반환 참조가 입력 슬라이스(`profiles`)의 수명에 묶여 있음을 명시해야 안전한 참조 반환이 가능하다.
</details>
