# 08. Slint 실전 수정 가이드

## 이 문서에서 배우는 것

- Rust ↔ Slint 바인딩의 전체 메커니즘
- DMaster GUI의 데이터 흐름 완전 분석
- 직접 UI를 수정하는 실습 예제

---

## 1. Rust ↔ Slint 바인딩 메커니즘

### 데이터 모델: VecModel과 ModelRc

Slint에서 배열(`[T]`) 속성에 데이터를 넣으려면 `ModelRc<T>`를 사용한다.

```rust
use slint::{ModelRc, VecModel};
use std::rc::Rc;

// 1. VecModel 생성 (수정 가능한 벡터 모델)
let model: Rc<VecModel<ProfileEntry>> = Rc::new(VecModel::from(loaded.entries));

// 2. ModelRc로 변환하여 Slint에 전달
app.set_profiles(ModelRc::from(model.clone()));

// 3. 나중에 데이터 교체
model.set_vec(new_entries);    // UI가 자동으로 업데이트됨
```

**왜 `Rc<VecModel<T>>`인가?**

```
Rc<VecModel<T>>
│
├── Rc: 참조 카운트 스마트 포인터
│   └── 여러 콜백에서 같은 model을 공유하기 위해
│
└── VecModel<T>: Slint의 리스트 모델
    └── .set_vec(), .push(), .remove() 등 제공
```

```rust
// 여러 콜백에서 같은 model을 사용
let model_clone1 = model.clone();   // Rc::clone → 참조 카운트 +1
let model_clone2 = model.clone();   // Rc::clone → 참조 카운트 +1

app.on_refresh_profiles(move || {
    model_clone1.set_vec(new_data);   // 콜백 1에서 모델 수정
});

app.on_save_profile(move |name, desc| {
    model_clone2.set_vec(new_data);   // 콜백 2에서 같은 모델 수정
});
```

### SharedString

Slint의 string ↔ Rust의 `SharedString`:

```rust
use slint::SharedString;

// String → SharedString
let ss = SharedString::from("hello");
let ss = SharedString::from(my_string.as_str());

// SharedString → &str
let s: &str = ss.as_str();

// format! → SharedString
let ss: SharedString = format!("{}x{}", w, h).into();
```

---

## 2. 콜백 등록 패턴

### DMaster의 콜백 구조

```rust
fn main() {
    let app = App::new().unwrap();

    // 1. 초기 데이터 로드
    let loaded = load_all_profiles();
    let model = Rc::new(VecModel::from(loaded.entries));
    app.set_profiles(ModelRc::from(model.clone()));

    // 2. 콜백 등록 (여러 개)
    // 각 콜백은 필요한 변수를 clone해서 move closure로 캡처

    let model_clone = model.clone();          // clone해서
    app.on_refresh_profiles(move || {          // move closure에 캡처
        let loaded = load_all_profiles();
        model_clone.set_vec(loaded.entries);   // closure 안에서 사용
    });

    // 3. 이벤트 루프 시작
    app.run().unwrap();
}
```

### move closure 패턴

```rust
// 콜백에서 여러 변수 사용 시:
let app_weak = app.as_weak();           // app의 약한 참조 (순환 참조 방지)
let model_clone = model.clone();
let data_clone = profile_data.clone();

app.on_save_profile(move |name: SharedString, desc: SharedString| {
    // 1. 비즈니스 로직 실행
    let result = get_display_profile().and_then(|profile| {
        save_profile(name.as_str(), desc_opt, &profile)
    });

    // 2. UI 업데이트 (app_weak 사용)
    if let Some(app) = app_weak.upgrade() {    // Weak → Option<App>
        match result {
            Ok(path) => {
                app.set_status_message(format!("Saved to {path}").into());
                app.set_status_is_error(false);
                // 프로필 목록 갱신
                let loaded = load_all_profiles();
                model_clone.set_vec(loaded.entries);
            }
            Err(e) => {
                app.set_status_message(format!("Save failed: {e}").into());
                app.set_status_is_error(true);
            }
        }
    }
});
```

**`app.as_weak()` — 왜 필요한가?**

```
app ────▶ 콜백 클로저 ────▶ app (순환 참조!)
 │                            ▲
 └────────────────────────────┘

해결: Weak 참조 사용
app ────▶ 콜백 클로저 ────▶ app_weak (약한 참조)
                              │
                     .upgrade() → Some(app) 또는 None
```

---

## 3. DMaster GUI 전체 데이터 흐름

```
┌──────────────────────────────────────────────────────────────┐
│                        main.rs                                │
│                                                                │
│  ┌──────────────────────────────────────────────┐             │
│  │ 초기화                                        │             │
│  │ load_all_profiles() → model, profile_data    │             │
│  │ app.set_profiles(model)                       │             │
│  └──────────────────────────────────────────────┘             │
│                                                                │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐    │
│  │on_select_     │  │on_refresh_   │  │on_save_profile   │    │
│  │profile(i)     │  │profiles()    │  │(name, desc)      │    │
│  │               │  │              │  │                   │    │
│  │ data[i] →     │  │ reload all → │  │ get_display →    │    │
│  │ display_model │  │ model.set()  │  │ save_profile →   │    │
│  └──────────────┘  └──────────────┘  │ reload all       │    │
│                                       └──────────────────┘    │
│  ┌──────────────┐  ┌──────────────────────────────────────┐  │
│  │on_delete_     │  │on_request_apply(name)                │  │
│  │profile(name)  │  │                                      │  │
│  │               │  │ 커넥터 일치? ──Yes──▶ apply_profile   │  │
│  │ delete →      │  │              │                        │  │
│  │ reload all    │  │              No                       │  │
│  └──────────────┘  │              ▼                        │  │
│                     │ show_mapping_dialog = true            │  │
│                     │ mapping_model 구성                    │  │
│                     └──────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────────┐    │
│  │on_mapping_confirmed()                                 │    │
│  │                                                       │    │
│  │ selections → DisplayMapping[] → apply_with_mapping    │    │
│  └──────────────────────────────────────────────────────┘    │
└──────────────────────────────────────────────────────────────┘
```

---

## 4. 실습: UI 수정 예제

### 실습 1: 상태 바에 프로필 개수 표시

**현재:**
```
● Ready
```

**목표:**
```
● Ready | 3 profiles
```

**수정할 파일: `ui/app.slint`**

현재 코드:
```slint
// Status bar 안의 Text
Text {
    text: root.status-message;
    font-size: 11px;
    color: root.status-is-error ? #e05252 : #9e9e9e;
    vertical-alignment: center;
}
```

수정:
```slint
Text {
    text: root.status-message;
    font-size: 11px;
    color: root.status-is-error ? #e05252 : #9e9e9e;
    vertical-alignment: center;
}

// 이 부분 추가 ▼
Rectangle { horizontal-stretch: 1; }   // 스페이서

Text {
    text: root.profiles.length == 1
        ? "1 profile"
        : root.profiles.length + " profiles";
    font-size: 11px;
    color: #6b6b6b;
    vertical-alignment: center;
}
```

📌 Slint에서 `root.profiles.length`로 배열 길이에 직접 접근 가능. Rust 코드 수정 불필요.

---

### 실습 2: 프로필 카드에 생성일 표시

**현재**: 프로필 목록에 이름과 설명만 표시
**목표**: 설명 아래에 생성일도 표시

**수정할 파일: `ui/app.slint`**

프로필 항목의 VerticalBox에 추가:

```slint
VerticalBox {
    // 기존 코드
    Text {
        text: profile.name;
        font-size: 13px;
        font-weight: 600;
    }
    Text {
        text: profile.description != "" ? profile.description : "(no description)";
        font-size: 11px;
        color: #6b6b6b;
    }
    // 이 부분 추가 ▼
    Text {
        text: profile.created-at;
        font-size: 10px;
        color: #505050;
    }
}
```

📌 `ProfileEntry` struct에 이미 `created-at` 필드가 있으므로 Rust 코드 수정 불필요.

---

### 실습 3: 배경색 테마 변경

**현재**: 다크 테마 (#1e1f22)
**목표**: 색상만 변경

변경할 색상 맵:

| 용도 | 현재 (어두운) | 예: 밝은 테마 |
|------|-------------|------------|
| 메인 배경 | `#1e1f22` | `#f5f5f5` |
| 패널 배경 | `#2b2d30` | `#ffffff` |
| 구분선 | `#3c3f41` | `#e0e0e0` |
| 제목 텍스트 | `#e0e0e0` | `#1e1e1e` |
| 본문 텍스트 | `#b0b0b0` | `#4a4a4a` |
| 보조 텍스트 | `#6b6b6b` | `#999999` |
| 강조색 | `#3a7bd5` | `#3a7bd5` (유지) |

`app.slint`에서 색상 코드만 치환하면 된다. 전역 변수로 만들면 더 좋다:

```slint
// app.slint 상단에 추가
global Theme {
    out property <color> bg-primary: #1e1f22;
    out property <color> bg-secondary: #2b2d30;
    out property <color> divider: #3c3f41;
    out property <color> text-primary: #e0e0e0;
    out property <color> text-secondary: #b0b0b0;
    out property <color> text-muted: #6b6b6b;
    out property <color> accent: #3a7bd5;
}
```

사용:
```slint
export component App inherits Window {
    background: Theme.bg-primary;
    // ...
    Text { color: Theme.text-primary; }
}
```

---

### 실습 4: 새 필드 추가 (Rust + Slint 양쪽 수정)

**목표**: 프로필 상세에 "총 해상도" 정보 추가 (예: "3840 x 1080")

**Step 1: Slint struct에 필드 추가** (`app.slint`):
```slint
export struct ProfileEntry {
    name: string,
    description: string,
    display-count: int,
    created-at: string,
    topology: string,
    total-resolution: string,     // 추가
}
```

**Step 2: Rust에서 값 계산** (`gui/src/main.rs`의 `load_all_profiles`):
```rust
let entries = profiles.iter().map(|p| {
    let total_width: u32 = p.profile.displays.iter().map(|d| d.width).sum();
    let max_height: u32 = p.profile.displays.iter().map(|d| d.height).max().unwrap_or(0);

    ProfileEntry {
        name: SharedString::from(p.name.as_str()),
        description: SharedString::from(p.profile.description.as_deref().unwrap_or_default()),
        display_count: p.profile.displays.len() as i32,
        created_at: format_timestamp(&p.profile.created_at).into(),
        topology: SharedString::from(topology_label(&p.profile.topology)),
        total_resolution: format!("{total_width} x {max_height}").into(),  // 추가
    }
}).collect();
```

**Step 3: Slint UI에 표시** (`app.slint`의 상세 패널):
```slint
VerticalBox {
    spacing: 4px;
    Text { text: "TOTAL RESOLUTION"; font-size: 10px; color: #6b6b6b; }
    Text {
        text: root.profiles[root.selected-index].total-resolution;
        font-size: 13px;
        color: #b0b0b0;
    }
}
```

---

## 5. 디버깅 팁

### Slint 컴파일 에러

```bash
cargo build -p dmaster_gui 2>&1 | head -20
```

흔한 에러:
- `Unknown property 'x'` — 속성 이름 오타 (kebab-case 확인)
- `Cannot convert string to int` — 타입 불일치
- `Expected ';'` — 세미콜론 누락

### 런타임 확인

```rust
// 콜백 안에서 디버그 출력
app.on_select_profile(move |index: i32| {
    eprintln!("[DEBUG] selected profile index: {index}");
    // ...
});
```

### slint-viewer (미리보기 도구)

```bash
cargo install slint-viewer
slint-viewer dmaster_gui/ui/app.slint
```

Rust 코드 없이 `.slint` 파일만으로 UI 미리보기. 데이터는 비어있지만 레이아웃과 스타일 확인 가능.

---

## 정리: 수정 체크리스트

| 수정 범위 | 필요한 파일 수정 |
|-----------|----------------|
| 색상/크기/폰트만 변경 | `.slint` 파일만 |
| 기존 데이터 다른 형태로 표시 | `.slint` 파일만 |
| 새 데이터 필드 추가 | `.slint` struct + `main.rs` |
| 새 동작 (버튼, 콜백) 추가 | `.slint` callback + `main.rs` on_* |
| 새 대화상자 추가 | 새 `.slint` 파일 + `app.slint` import + `main.rs` |

## 확인 문제

1. `app.as_weak()`를 쓰지 않고 `app`을 직접 클로저에 캡처하면 어떤 문제가 생기는가?
2. `ModelRc`와 `VecModel`의 관계는?
3. Slint에서 `display-count`라고 정의한 필드를 Rust에서 접근할 때 이름은?
4. `.slint` 파일만 수정해서 할 수 있는 것과, Rust 코드도 수정해야 하는 것의 기준은?

<details>
<summary>정답</summary>

1. `app`이 클로저로 move되면 이후 다른 클로저에서 `app`을 사용할 수 없다 (소유권 이동). 또한 `app`이 자신의 콜백을 소유하고, 콜백이 다시 `app`을 소유하면 순환 참조(memory leak) 발생. `Weak` 참조는 소유권을 갖지 않으므로 순환을 끊는다.
2. `VecModel<T>`은 데이터를 저장하고 수정하는 실제 모델. `ModelRc`는 Slint가 모델을 참조하기 위한 타입-erased 래퍼. `Rc<VecModel<T>>`에서 `ModelRc::from()`으로 변환하여 Slint property에 설정.
3. `display_count` (kebab-case → snake_case 자동 변환).
4. 기준은 **새로운 데이터가 필요한가?**. 기존 데이터의 표시 방식(색상, 위치, 조건부 렌더링)만 바꾸면 `.slint`만. 새 데이터를 계산하거나 전달해야 하면 Rust도 수정.
</details>
