# 04. Trait와 Backend 패턴

## 이 문서에서 배우는 것

- Trait 정의와 구현
- Dynamic Dispatch (`Box<dyn Trait>`)
- 조건부 컴파일 (`#[cfg]`)
- Strategy Pattern이 Rust에서 어떻게 구현되는가

---

## 1. Trait — 인터페이스 정의

### 개념

Trait는 "이런 메서드를 가져야 한다"는 **계약(contract)**이다.

```rust
// backend/mod.rs
pub trait DisplayBackend {
    fn get_display_profile(&self) -> Result<DisplayProfile, String>;
    fn apply_profile(&self, profile: &DisplayProfile) -> Result<(), String>;
    fn apply_with_mapping(
        &self,
        profile: &DisplayProfile,
        mappings: &[DisplayMapping],
    ) -> Result<(), String>;
}
```

이 trait를 구현(implement)하면, 어떤 플랫폼이든 동일한 인터페이스로 사용 가능.

### 구현

```rust
// backend/linux.rs
pub struct LinuxDisplayBackend;

impl DisplayBackend for LinuxDisplayBackend {
    fn get_display_profile(&self) -> Result<DisplayProfile, String> {
        let output = run_xrandr(&["--query"])?;
        parse_xrandr_query(&output)
    }

    fn apply_profile(&self, profile: &DisplayProfile) -> Result<(), String> {
        apply_linux_profile(profile)
    }

    fn apply_with_mapping(&self, profile: &DisplayProfile, 
                          mappings: &[DisplayMapping]) -> Result<(), String> {
        // ... 매핑 로직
    }
}
```

```rust
// backend/windows.rs
pub struct WindowsDisplayBackend;

impl DisplayBackend for WindowsDisplayBackend {
    fn get_display_profile(&self) -> Result<DisplayProfile, String> {
        get_display_profile_impl()       // WinAPI 호출
    }
    // ...
}
```

```rust
// backend/gnome_wayland.rs
pub struct GnomeWaylandBackend;

impl DisplayBackend for GnomeWaylandBackend {
    fn get_display_profile(&self) -> Result<DisplayProfile, String> {
        let state = query_current_state()?;  // Mutter D-Bus 호출
        // ...
    }
    // ...
}
```

📌 **같은 trait, 다른 구현**: 3개의 struct가 모두 `DisplayBackend`를 구현하지만, 내부 동작은 완전히 다르다.

---

## 2. Dynamic Dispatch — Box\<dyn Trait\>

### 문제: 컴파일 타임에 타입을 모른다

```rust
// 이건 안 됨:
fn get_backend() -> ??? {   // LinuxDisplayBackend? WindowsDisplayBackend?
    if cfg!(windows) {
        WindowsDisplayBackend     // 타입 A
    } else {
        LinuxDisplayBackend       // 타입 B ← 타입이 다름!
    }
}
```

Rust에서 함수 반환 타입은 하나여야 한다. 다른 타입을 반환할 수 없다.

### 해결: Trait Object (`Box<dyn Trait>`)

```rust
pub fn get_backend() -> Box<dyn DisplayBackend> {
    //                   ^^^^^^^^^^^^^^^^^^^^^^^^
    //                   "DisplayBackend를 구현한 어떤 타입이든"
    //                   Box로 힙에 할당해서 반환
    
    #[cfg(target_os = "windows")]
    { Box::new(windows::WindowsDisplayBackend) }

    #[cfg(target_os = "linux")]
    {
        if is_wayland_session() && is_gnome_session() {
            Box::new(gnome_wayland::GnomeWaylandBackend)
        } else {
            Box::new(linux::LinuxDisplayBackend)
        }
    }
}
```

**호출하는 쪽:**

```rust
// query.rs
pub fn get_display_profile() -> Result<DisplayProfile, String> {
    backend::get_backend().get_display_profile()
    //      ^^^^^^^^^^^^^^ Box<dyn DisplayBackend>
    //                     .get_display_profile() ← trait 메서드 호출
}
```

호출자는 어떤 Backend인지 **모르고, 알 필요도 없다**.

### 구조도

```
query.rs / apply.rs
    │
    │  backend::get_backend()
    ▼
Box<dyn DisplayBackend>   ◀── "어떤 구현체든 이 인터페이스를 따른다"
    │
    │  .get_display_profile()
    ▼
┌─────────────┬─────────────────┬─────────────────┐
│ Windows     │ Linux (X11)     │ Linux (Wayland)  │
│ WinAPI 호출  │ xrandr 호출     │ Mutter D-Bus    │
└─────────────┴─────────────────┴─────────────────┘
```

### Static vs Dynamic Dispatch

```
Static Dispatch (제네릭)              Dynamic Dispatch (trait object)
───────────────────────              ────────────────────────────
fn do_thing<T: Backend>(b: T)        fn do_thing(b: Box<dyn Backend>)
• 컴파일 타임에 타입 결정               • 런타임에 타입 결정
• 각 타입별 코드 복사 (monomorphization) • vtable로 메서드 탐색
• 약간 더 빠름                        • 바이너리 크기 작음
• 타입이 컴파일 타임에 결정될 때         • 타입이 런타임에 결정될 때
```

DMaster는 **런타임에** 플랫폼 + 세션 타입을 확인하므로 Dynamic Dispatch가 적합.

---

## 3. 조건부 컴파일 (#[cfg])

### 기본 문법

```rust
#[cfg(target_os = "windows")]    // Windows에서만 컴파일
mod windows;

#[cfg(target_os = "linux")]      // Linux에서만 컴파일
mod linux;

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
mod unsupported;                  // Windows도 Linux도 아닌 경우
```

### DMaster의 cfg 전략

```rust
// backend/mod.rs — 전체 구조

// 1. 플랫폼별 모듈 선언 (컴파일 대상 선택)
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
mod gnome_wayland;

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
mod unsupported;

// 2. 런타임 환경 감지 (Linux 내에서 추가 분기)
fn is_wayland_session() -> bool {
    std::env::var("WAYLAND_DISPLAY").is_ok()
}

fn is_gnome_session() -> bool {
    std::env::var("XDG_CURRENT_DESKTOP")
        .map(|v| v.to_lowercase().contains("gnome"))
        .unwrap_or(false)
}

// 3. 팩토리 함수
pub fn get_backend() -> Box<dyn DisplayBackend> {
    #[cfg(target_os = "windows")]
    { Box::new(windows::WindowsDisplayBackend) }

    #[cfg(target_os = "linux")]
    {
        if is_wayland_session() && is_gnome_session() {
            Box::new(gnome_wayland::GnomeWaylandBackend)
        } else {
            Box::new(linux::LinuxDisplayBackend)
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    { Box::new(unsupported::UnsupportedDisplayBackend) }
}
```

### 컴파일 vs 런타임 분기

```
                    컴파일 타임 (#[cfg])
                    ├── Windows → windows.rs만 컴파일
                    └── Linux → linux.rs + gnome_wayland.rs 둘 다 컴파일
                                    │
                                런타임 (if/else)
                                ├── GNOME + Wayland → GnomeWaylandBackend
                                └── 그 외 → LinuxDisplayBackend (xrandr)
```

⚠️ **`#[cfg]`는 코드 자체를 포함/제외한다**. `#[cfg(windows)]`로 감싼 코드는 Linux에서 아예 컴파일되지 않는다. `if cfg!(windows)`와 다르다 — 후자는 코드가 컴파일은 되되 실행만 안 됨.

---

## 4. Strategy Pattern in Rust

### 디자인 패턴 관점

DMaster의 backend 구조는 **Strategy Pattern**이다:

```
┌────────────────────────────────────────────────┐
│ Strategy Pattern                                │
│                                                  │
│ Context: query.rs, apply.rs                     │
│   "나는 DisplayBackend를 사용할 뿐,              │
│    어떤 구현인지 모른다"                           │
│                                                  │
│ Strategy Interface: trait DisplayBackend         │
│   "이 3개 메서드를 구현하라"                       │
│                                                  │
│ Concrete Strategies:                             │
│   ├── WindowsDisplayBackend  (WinAPI)           │
│   ├── LinuxDisplayBackend    (xrandr)           │
│   ├── GnomeWaylandBackend    (D-Bus)            │
│   └── UnsupportedBackend     (에러)              │
│                                                  │
│ Factory: get_backend()                           │
│   "환경을 보고 적절한 Strategy를 선택"             │
└────────────────────────────────────────────────┘
```

### Rust에서의 장점

1. **Trait이 인터페이스를 강제**: 메서드를 빼먹으면 컴파일 에러
2. **cfg로 불필요한 코드 제거**: Windows 빌드에 Linux 코드가 포함되지 않음
3. **Box\<dyn Trait\>로 유연한 반환**: 런타임에 구현체 교체 가능

---

## 5. Unit Struct (필드 없는 구조체)

```rust
pub struct LinuxDisplayBackend;       // 필드 없음
pub struct WindowsDisplayBackend;     // 필드 없음
pub struct GnomeWaylandBackend;       // 필드 없음
```

왜 필드가 없는 struct를 쓰는가?

이 Backend들은 **상태(state)를 가질 필요가 없다**. 매 호출마다 OS에 직접 질의하고, 내부에 캐시할 것이 없다. 하지만 trait를 구현하려면 struct가 필요하므로, 빈 struct를 사용한다.

```rust
impl DisplayBackend for LinuxDisplayBackend {
    fn get_display_profile(&self) -> ... {
        // self는 비어있지만, trait 시그니처에 필요
        run_xrandr(&["--query"])  // 매번 새로 호출
    }
}
```

📌 이런 패턴을 **marker type** 또는 **zero-sized type (ZST)**이라 한다. 메모리를 0바이트 차지.

---

## 정리: DMaster Backend 아키텍처

| 구성 요소 | 파일 | 역할 |
|-----------|------|------|
| Trait 정의 | `backend/mod.rs` | 인터페이스 계약 |
| 팩토리 함수 | `backend/mod.rs::get_backend()` | 환경에 맞는 구현체 선택 |
| Windows 구현 | `backend/windows.rs` | WinAPI 호출 |
| X11 구현 | `backend/linux.rs` | xrandr 호출 |
| Wayland 구현 | `backend/gnome_wayland.rs` | Mutter D-Bus 호출 |
| 폴백 | `backend/unsupported.rs` | 에러 반환 |
| API 래퍼 | `query.rs`, `apply.rs` | backend 선택을 숨김 |

## 확인 문제

1. `Box<dyn DisplayBackend>`에서 `dyn`은 무슨 뜻인가?
2. `#[cfg(target_os = "linux")]`와 `if cfg!(target_os = "linux")`의 차이는?
3. `LinuxDisplayBackend`에 필드가 없는 이유는?
4. 새 플랫폼(macOS)을 추가하려면 어떤 파일들을 만들어야 하는가?

<details>
<summary>정답</summary>

1. **dynamic dispatch**를 의미. 컴파일 타임에 구체 타입을 모르고, 런타임에 vtable을 통해 메서드를 호출.
2. `#[cfg]`는 **컴파일 시 코드 자체를 포함/제거**. `cfg!()` 매크로는 코드를 컴파일은 하되 `true/false`를 반환하여 **런타임 분기**에 사용. `cfg!`를 쓰면 다른 플랫폼 코드도 컴파일되어야 하므로 타입 에러 발생 가능.
3. 상태를 보관할 필요 없이, 매번 OS에 직접 질의하면 되므로. trait 구현을 위한 타입이 필요할 뿐.
4. `backend/macos.rs` 파일 생성, `MacOSDisplayBackend` struct + `impl DisplayBackend`, `backend/mod.rs`에 `#[cfg(target_os = "macos")] mod macos;` 추가, `get_backend()`에 macOS 분기 추가.
</details>
