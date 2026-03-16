# 05. Linux 백엔드 심화

## 이 문서에서 배우는 것

- `std::process::Command`로 외부 프로세스 실행
- xrandr 출력 파싱 패턴
- GNOME Mutter D-Bus 연동 (Python 브릿지)
- 두 백엔드의 설계 차이

---

## 1. 전체 흐름

```
Linux 세션
├── X11 세션 (대부분의 데스크톱, WSL)
│   └── LinuxDisplayBackend
│       └── xrandr 명령어 실행/파싱
│
└── Wayland + GNOME
    └── GnomeWaylandBackend
        └── Python3 → D-Bus → Mutter API
```

런타임 감지:
```rust
fn is_wayland_session() -> bool {
    std::env::var("WAYLAND_DISPLAY").is_ok()    // 환경변수 존재 여부
}

fn is_gnome_session() -> bool {
    std::env::var("XDG_CURRENT_DESKTOP")
        .map(|v| v.to_lowercase().contains("gnome"))
        .unwrap_or(false)
}
```

---

## 2. std::process::Command — 외부 프로세스 실행

### 기본 사용법

```rust
use std::process::Command;

let output = Command::new("xrandr")     // 실행할 프로그램
    .args(&["--query"])                 // 인자들
    .output()                           // 실행하고 결과 수집
    .map_err(|e| format!("failed: {e}"))?;
```

### output의 구조

```
Output {
    status: ExitStatus,       // 종료 코드 (0 = 성공)
    stdout: Vec<u8>,          // 표준 출력 (바이트)
    stderr: Vec<u8>,          // 표준 에러 (바이트)
}
```

### DMaster의 run_xrandr 함수

```rust
fn run_xrandr(args: &[&str]) -> Result<String, String> {
    // 1. 사전 조건 검증
    if std::env::var("DISPLAY").is_err() {
        return Err(String::from("DISPLAY is not set; X11/xrandr is unavailable"));
    }

    // 2. 명령어 실행
    let output = Command::new("xrandr")
        .args(args)
        .output()
        .map_err(|error| format!("failed to execute xrandr: {error}"))?;

    // 3. 종료 코드 확인
    if !output.status.success() {
        return Err(format!(
            "xrandr failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    // 4. stdout을 String으로 변환
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
```

**패턴 분해:**

| 단계 | 코드 | 하는 일 |
|------|------|---------|
| 사전 검증 | `env::var("DISPLAY").is_err()` | X11 없으면 빠른 실패 |
| 실행 | `.output()` | 프로세스 실행 + 완료 대기 |
| 에러 변환 | `.map_err(\|e\| format!(...))` | `io::Error` → `String` |
| 성공 확인 | `.status.success()` | 종료 코드 0인지 |
| 바이트→문자열 | `String::from_utf8_lossy` | 유효하지 않은 UTF-8은 `�`로 대체 |

📌 **`from_utf8_lossy`**: xrandr 출력이 항상 유효한 UTF-8이 아닐 수 있으므로, 안전한 변환 함수 사용. `from_utf8`은 에러를 반환하지만 `from_utf8_lossy`는 깨진 문자를 대체.

---

## 3. xrandr 출력 파싱

### xrandr --query 출력 예시

```
Screen 0: minimum 8 x 8, current 3840 x 1080, maximum 32767 x 32767
DP-1 connected primary 1920x1080+0+0 (normal left inverted right ...) 527mm x 296mm
   1920x1080     60.00*+  59.94    50.00  
   1680x1050     59.95  
eDP-1 connected 1920x1080+1920+0 (normal left inverted right ...) 344mm x 193mm
   1920x1080     60.01*+  60.01    59.97  
HDMI-1 disconnected (normal left inverted right x axis y axis)
```

### parse_xrandr_query 함수

```rust
fn parse_xrandr_query(output: &str) -> Result<DisplayProfile, String> {
    let mut displays = Vec::new();

    for line in output.lines() {
        // "connected"가 포함된 줄만 처리
        if !line.contains(" connected") {
            continue;   // disconnected, 해상도 줄, Screen 줄 등은 건너뜀
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        // parts = ["DP-1", "connected", "primary", "1920x1080+0+0", ...]

        let device_name = parts[0].to_string();  // "DP-1"

        // geometry 토큰 찾기: "1920x1080+0+0" 형태
        let geometry_token = parts
            .iter()
            .find(|token| token.contains('x') && token.contains('+'))
            .copied();

        let (width, height, position_x, position_y) = geometry_token
            .map(parse_geometry)       // Some("1920x1080+0+0") → parse
            .transpose()?             // Option<Result> → Result<Option>
            .unwrap_or((0, 0, 0, 0)); // None이면 기본값

        displays.push(DisplayConfig { /* ... */ });
    }

    // 토폴로지 추론
    let topology = if displays.len() > 1 {
        DisplayTopology::Extend
    } else if displays[0].device_name.starts_with("eDP")
           || displays[0].device_name.starts_with("LVDS") {
        DisplayTopology::Internal    // 노트북 내장 디스플레이
    } else {
        DisplayTopology::External
    };

    Ok(DisplayProfile { topology, displays })
}
```

### parse_geometry — 문자열 파싱 기법

```rust
// "1920x1080+0+0" → (1920, 1080, 0, 0)
fn parse_geometry(token: &str) -> Result<(u32, u32, i32, i32), String> {
    // Step 1: "1920x1080" + "0+0" 분리
    let (size, position) = token.split_once('+')
        .ok_or_else(|| format!("failed to parse '{token}'"))?;

    // Step 2: position "0+0" → x=0, y=0
    let mut position_parts = position.split('+');
    let x = position_parts.next()         // Some("0")
        .ok_or_else(|| "missing x")?
        .parse::<i32>()?;                 // "0" → 0i32
    let y = position_parts.next()
        .ok_or_else(|| "missing y")?
        .parse::<i32>()?;

    // Step 3: size "1920x1080" → w=1920, h=1080
    let (width, height) = size.split_once('x')
        .ok_or_else(|| "missing size")?;

    Ok((width.parse()?, height.parse()?, x, y))
}
```

**파싱 흐름:**

```
"1920x1080+0+0"
     │
     │ split_once('+')
     ▼
("1920x1080", "0+0")
     │              │
     │ split_once('x')   split('+')
     ▼              ▼
("1920","1080")  ["0", "0"]
     │              │
     │ parse()      │ parse()
     ▼              ▼
(1920u32, 1080u32, 0i32, 0i32)
```

📌 **`.transpose()`의 마법**: `Option<Result<T, E>>` → `Result<Option<T>, E>`. geometry가 없는 모니터(`None`)는 에러가 아닌 기본값 처리, 있는데 파싱 실패(`Some(Err)`)하면 에러 전파.

---

## 4. xrandr로 설정 적용

```rust
fn apply_linux_profile(profile: &DisplayProfile) -> Result<(), String> {
    let mut args: Vec<String> = Vec::new();
    for display in &profile.displays {
        args.push(String::from("--output"));
        args.push(display.device_name.clone());
        args.push(String::from("--mode"));
        args.push(format!("{}x{}", display.width, display.height));
        args.push(String::from("--pos"));
        args.push(format!("{}x{}", display.position_x, display.position_y));
        args.push(String::from("--rotate"));
        args.push(rotation_to_xrandr(display.orientation).to_string());
    }

    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    run_xrandr(&arg_refs).map(|_| ())
}
```

생성되는 xrandr 명령어:
```bash
xrandr --output DP-1 --mode 1920x1080 --pos 0x0 --rotate normal \
       --output eDP-1 --mode 1920x1080 --pos 1920x0 --rotate normal
```

**`args.iter().map(String::as_str).collect()`** — 왜 필요한가?

`run_xrandr`은 `&[&str]`을 받는데, `args`는 `Vec<String>`. 변환 필요:
```
Vec<String> → iter() → Iterator<Item=&String> → map(as_str) → Iterator<Item=&str> → collect() → Vec<&str>
```

---

## 5. GNOME Wayland 백엔드 — D-Bus 연동

### 왜 Python을 사용하는가?

Wayland에서는 xrandr이 동작하지 않는다. GNOME은 `org.gnome.Mutter.DisplayConfig` D-Bus 인터페이스를 제공하는데, Rust에서 D-Bus를 직접 호출하려면 별도 crate(zbus, dbus 등)이 필요하다.

DMaster는 의존성을 최소화하기 위해 **Python 스크립트를 중간 다리**로 사용:

```
Rust → Command("python3") → Python 스크립트 → D-Bus → Mutter
                                              ↓
                                         JSON stdout
                                              ↓
                              Rust ← serde_json::from_str
```

### 조회 스크립트 (QUERY_SCRIPT)

```python
import json, dbus

bus = dbus.SessionBus()
proxy = bus.get_object(
    'org.gnome.Mutter.DisplayConfig',
    '/org/gnome/Mutter/DisplayConfig',
)
iface = dbus.Interface(proxy, 'org.gnome.Mutter.DisplayConfig')
serial, monitors, logical_monitors, props = iface.GetCurrentState()

# ... monitors와 logical_monitors를 조합하여 displays 목록 생성 ...

print(json.dumps({'serial': int(serial), 'displays': displays}))
```

### Rust 측 파싱 구조체

```rust
#[derive(serde::Deserialize)]
struct QueryResult {
    serial: u32,                      // Mutter 상태 시리얼 (적용 시 필요)
    displays: Vec<QueryDisplay>,
}

#[derive(serde::Deserialize)]
struct QueryDisplay {
    connector: String,                // "DP-1"
    display_name: String,             // "DELL U2720Q"
    vendor: String,
    product: String,
    serial: String,
    mode_id: String,                  // "1920x1080@60.000"
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    transform: u32,
    scale: f64,
    primary: bool,
    #[serde(default)]                 // JSON에 없으면 빈 Vec
    available_modes: Vec<AvailableMode>,
}
```

📌 **`#[serde(default)]`**: JSON에 해당 필드가 없어도 에러 없이 기본값(`Vec::new()`) 사용.

### 모드 ID 탐색 로직

프로필에 저장된 해상도(1920x1080)를 현재 모니터에서 사용 가능한 모드 ID로 변환:

```rust
fn find_mode_id_for_resolution(display: &QueryDisplay, width: u32, height: u32) -> String {
    // 1. 현재 모드가 일치하면 그대로 사용
    if display.width == width && display.height == height {
        return display.mode_id.clone();
    }

    // 2. 사용 가능한 모드에서 해상도 일치하는 것 필터
    let matching: Vec<&AvailableMode> = display.available_modes.iter()
        .filter(|m| m.width == width && m.height == height)
        .collect();

    // 3. 우선순위: preferred > 최고 주사율 > 기본값
    if matching.is_empty() {
        return format!("{}x{}@60.000", width, height);
    }
    if let Some(preferred) = matching.iter().find(|m| m.is_preferred) {
        return preferred.mode_id.clone();
    }
    matching.iter()
        .max_by(|a, b| a.refresh.partial_cmp(&b.refresh).unwrap_or(std::cmp::Ordering::Equal))
        .map(|m| m.mode_id.clone())
        .unwrap_or_else(|| format!("{}x{}@60.000", width, height))
}
```

---

## 6. 두 백엔드 비교

| 항목 | LinuxDisplayBackend (X11) | GnomeWaylandBackend |
|------|--------------------------|---------------------|
| 외부 도구 | `xrandr` | `python3` + `dbus` |
| 데이터 형식 | 텍스트 파싱 | JSON |
| 모니터 이름 | device_name만 | display_name + connector |
| 토폴로지 | 추론 (모니터 수, 이름 패턴) | 추론 (동일) |
| 적용 방식 | xrandr 인자 조합 | D-Bus ApplyMonitorsConfig |
| Scale 지원 | 없음 | 있음 (scale 보존) |
| Serial 필요 | 없음 | 필수 (동시성 제어) |

---

## 정리: Linux 백엔드에서 배운 패턴

| 패턴 | 적용 |
|------|------|
| `Command::new().args().output()` | 외부 프로세스 실행 |
| `String::from_utf8_lossy` | 바이트 → 문자열 안전 변환 |
| `split_once`, `split`, `parse` | 텍스트 파싱 체인 |
| `Option::map().transpose()` | Optional 파싱 + 에러 전파 |
| 임베디드 Python 스크립트 | D-Bus같은 복잡한 IPC의 간편 브릿지 |
| `#[serde(default)]` | 누락 가능한 JSON 필드 처리 |

## 확인 문제

1. `run_xrandr`에서 `DISPLAY` 환경변수를 먼저 확인하는 이유는?
2. `.transpose()`가 `Option<Result<T, E>>`에서 하는 일은?
3. GNOME 백엔드가 Python을 사용하는 이유와, 이 방식의 단점은?
4. `find_mode_id_for_resolution`의 우선순위 3단계를 설명하라.

<details>
<summary>정답</summary>

1. X11 환경이 아니면(SSH, TTY 등) xrandr 자체가 동작하지 않는다. 프로세스를 실행하기 전에 빠르게 실패(fast fail)하여 불필요한 에러 메시지를 방지.
2. `Some(Ok(v))` → `Ok(Some(v))`, `Some(Err(e))` → `Err(e)`, `None` → `Ok(None)`. 외부 `Result`로 에러를 끌어올려 `?`로 전파 가능하게 만든다.
3. 이유: D-Bus 바인딩 crate 의존성 없이 구현 가능. 단점: python3 + python-dbus 패키지가 시스템에 설치되어 있어야 함, 프로세스 생성 오버헤드, Python 스크립트의 에러 처리가 Rust보다 불안정.
4. (1) 현재 모드가 이미 같은 해상도면 그대로, (2) preferred 모드가 있으면 우선, (3) 같은 해상도 중 최고 주사율, (4) 아무것도 못 찾으면 `"WxH@60.000"` 기본값.
</details>
