# 09. Windows 백엔드 심화

## 이 문서에서 배우는 것

- `unsafe extern "system"`으로 WinAPI를 연결하는 방법
- `QueryDisplayConfig`/`SetDisplayConfig` 기반 토폴로지 처리 흐름
- DMaster의 Windows 적용 로직(`enabled` 포함) 동작 방식

---

## 1. 파일 역할과 진입점

대상 파일: `dmaster_core/src/backend/windows.rs`

핵심 구조:

```rust
pub struct WindowsDisplayBackend;

impl DisplayBackend for WindowsDisplayBackend {
    fn get_display_profile(&self) -> Result<DisplayProfile, String> {
        get_display_profile_impl()
    }

    fn apply_profile(&self, profile: &DisplayProfile) -> Result<(), String> {
        apply_resolved_profile(profile)
    }
}
```

`DisplayBackend` trait 구현체로서, 외부(`query.rs`, `apply.rs`)는 내부 WinAPI 디테일을 몰라도 된다.

---

## 2. FFI 레이어: extern 선언

```rust
#[link(name = "user32")]
unsafe extern "system" {
    fn GetDisplayConfigBufferSizes(... ) -> i32;
    fn QueryDisplayConfig(... ) -> i32;
    fn SetDisplayConfig(... ) -> i32;
}
```

포인트:

- `#[link(name = "user32")]`: 링크 대상 DLL/라이브러리 지정
- `extern "system"`: Windows 호출 규약(ABI)
- `unsafe`: Rust가 포인터 유효성/버퍼 크기를 보장할 수 없기 때문

---

## 3. 조회 경로: 현재 디스플레이 상태 수집

### 3-1. 기본 디바이스 정보 열거

`get_display_profile_impl()`에서 반복:

1. `EnumDisplayDevicesW`로 디바이스 목록 열거
2. `EnumDisplaySettingsW`로 해상도/위치/회전 획득
3. `DISPLAY_DEVICEW`/`DEVMODEW`를 `DisplayConfig`로 변환

```rust
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
```

현재 코드는 조회 시 `enabled = true`로 채운다.

### 3-2. 토폴로지 추론

`query_display_topology()`:

1. `GetDisplayConfigBufferSizes`로 필요한 배열 크기 확인
2. `DISPLAYCONFIG_PATH_INFO`/`DISPLAYCONFIG_MODE_INFO` 버퍼 할당
3. `QueryDisplayConfig(QDC_ONLY_ACTIVE_PATHS, ...)` 실행
4. `sourceInfo.id` 중복 여부로 `Clone` vs `Extend` 추론

에러 시 `Err`가 아니라 `DisplayTopology::Unknown(status)`로 보존하는 것도 특징.

---

## 4. 적용 경로: 토폴로지 + 개별 디스플레이 설정

`apply_resolved_profile()` 동작:

```text
입력 검증
  ├─ displays 비어있으면 에러
  ├─ enabled=true 항목 하나도 없으면 에러
  └─ 대상 device_name 연결 여부 검증

토폴로지 적용
  └─ SetDisplayConfig(..., SDC_TOPOLOGY_* | SDC_APPLY)

개별 반영
  ├─ enabled=true  -> apply_single_display()
  └─ enabled=false -> detach_display()
```

### 4-1. 활성 디스플레이 적용

`apply_single_display()`는 `DEVMODEW`를 채우고 `ChangeDisplaySettingsExW` 호출:

- `dmPelsWidth`, `dmPelsHeight`
- `dmPosition`
- `dmDisplayOrientation`
- `dmFields = DM_PELSWIDTH | DM_PELSHEIGHT | DM_POSITION | DM_DISPLAYORIENTATION`

### 4-2. 비활성(분리) 처리

`detach_display()`는 `width/height = 0`으로 전달해 디스플레이 detach를 수행한다.

---

## 5. 매핑 적용

`apply_with_mapping()`은 저장 프로필을 현재 연결 상태에 맞게 변환한다.

핵심 아이디어:

- 현재 연결 디바이스의 `device_name/device_id/device_key`는 유지
- 해상도/위치/회전/enabled는 소스 프로필에서 가져옴

실제 병합 함수: `merge_display_config()`

```rust
enabled: source_display.enabled,
```

즉, 매핑 적용에서도 `enabled` 상태가 그대로 반영된다.

---

## 6. 안전성 관점 체크리스트

이 파일의 `unsafe` 블록에서 반드시 지켜야 할 항목:

1. 구조체 크기 필드 정확히 세팅 (`cb`, `dmSize`)
2. API 호출 전 버퍼 길이/포인터 일치
3. UTF-16 문자열의 NUL 종료 보장
4. 반환 코드 0(success) 여부 즉시 검증

현재 DMaster는 위 4가지를 모두 코드로 명시하고 있다.

---

## 확인 문제

1. `QueryDisplayConfig`에서 `sourceInfo.id` 중복으로 `Clone`을 판단하는 이유는?
2. `enabled=false`는 Windows 경로에서 어떤 함수로 처리되는가?
3. `DisplayTopology::Unknown(status)`로 감싸는 설계의 장점은?

<details>
<summary>정답</summary>

1. 여러 path가 같은 source를 공유하면 같은 화면 소스를 복제해 여러 대상에 출력하는 구조로 해석할 수 있기 때문이다.
2. `detach_display()`.
3. 실패 원인 코드를 잃지 않고 상위 계층(UI/로그)에서 해석할 수 있다.
</details>
