# 07. Slint 기초

## 이 문서에서 배우는 것

- Slint 언어 문법 (컴포넌트, 속성, 콜백)
- 레이아웃 시스템 (VerticalBox, HorizontalBox)
- Rust ↔ Slint 연결 구조
- DMaster의 UI 파일 구조

---

## 1. Slint란?

Rust 네이티브 GUI 프레임워크. `.slint` 파일에 UI를 선언적으로 작성하고, Rust 코드에서 로직을 연결한다.

```
┌────────────────────────────────────────────────────┐
│  .slint 파일 (UI 선언)        Rust 코드 (로직)      │
│  ─────────────────           ─────────────────      │
│  컴포넌트 정의                 이벤트 처리             │
│  레이아웃                     데이터 바인딩            │
│  스타일링                     비즈니스 로직            │
│  애니메이션                    dmaster_core 호출      │
│                                                      │
│         build.rs에서 .slint → Rust 코드로 컴파일      │
└────────────────────────────────────────────────────┘
```

---

## 2. 빌드 파이프라인

```rust
// dmaster_gui/build.rs
fn main() {
    slint_build::compile("ui/app.slint").unwrap();
}
```

```rust
// dmaster_gui/src/main.rs
slint::include_modules!();   // build.rs가 생성한 코드를 포함

fn main() {
    let app = App::new().unwrap();   // .slint에서 정의한 App 컴포넌트 인스턴스 생성
    // ...
    app.run().unwrap();              // 이벤트 루프 시작
}
```

**빌드 흐름:**
```
ui/app.slint ──▶ slint_build::compile() ──▶ 생성된 Rust 코드
                                              │
                                     slint::include_modules!()
                                              │
                                         src/main.rs에서 사용
                                              │
                                         App::new(), app.run()
```

---

## 3. Slint 언어 기초 문법

### 3.1 컴포넌트 정의

```slint
// app.slint
export component App inherits Window {
    title: "DMaster";
    preferred-width: 860px;
    preferred-height: 560px;
    background: #1e1f22;
}
```

- `component App` — 컴포넌트 이름 (Rust에서 `App`으로 접근)
- `inherits Window` — Window를 기반으로 확장
- `export` — Rust 코드에서 사용 가능하게 노출
- 속성 설정은 `키: 값;` 형태

### 3.2 Struct 정의

```slint
export struct ProfileEntry {
    name: string,
    description: string,
    display-count: int,
    created-at: string,
    topology: string,
}
```

📌 **Slint의 이름 규칙**: kebab-case (`display-count`). Rust에서는 snake_case (`display_count`)로 자동 변환.

### 3.3 속성 (Property)

```slint
export component App inherits Window {
    // in-out: Rust에서 읽기/쓰기 가능
    in-out property <[ProfileEntry]> profiles;
    in-out property <int> selected-index: -1;      // 기본값 -1
    in-out property <string> status-message: "Ready";
    in-out property <bool> status-is-error: false;
}
```

**속성 방향:**

| 방향 | 의미 | 사용처 |
|------|------|--------|
| `in` | 외부에서 설정만 가능 | 부모 → 자식 데이터 전달 |
| `out` | 외부에서 읽기만 가능 | 자식 → 부모 상태 전달 |
| `in-out` | 읽기/쓰기 모두 가능 | Rust ↔ Slint 양방향 바인딩 |
| (없음) | 컴포넌트 내부 전용 | 내부 상태 |

**Rust에서의 접근:**
```rust
// 쓰기
app.set_profiles(ModelRc::from(model.clone()));
app.set_status_message("Ready".into());
app.set_selected_index(-1);

// 읽기
let idx = app.get_selected_index();
```

### 3.4 콜백 (Callback)

```slint
export component App inherits Window {
    callback refresh-profiles();
    callback save-profile(string, string);
    callback select-profile(int);
    callback delete-profile(string);
}
```

**Slint 측에서 호출:**
```slint
Button {
    text: "⟳ Refresh";
    clicked => { root.refresh-profiles(); }   // 버튼 클릭 시 콜백 호출
}
```

**Rust 측에서 구현:**
```rust
app.on_refresh_profiles(move || {
    let loaded = load_all_profiles();
    model_clone.set_vec(loaded.entries);
    // ...
});
```

**콜백 흐름:**
```
사용자 클릭 → Slint Button.clicked → root.refresh-profiles()
                                            │
                                     Rust: on_refresh_profiles(closure)
                                            │
                                     load_all_profiles() → UI 업데이트
```

---

## 4. 레이아웃 시스템

### VerticalBox / HorizontalBox

```slint
VerticalBox {                    HorizontalBox {
    // 위에서 아래로 배치             // 왼쪽에서 오른쪽으로 배치
    spacing: 8px;                    spacing: 8px;
    padding: 12px;                   alignment: start;

    Text { text: "A"; }             Text { text: "A"; }
    Text { text: "B"; }             Text { text: "B"; }
    Text { text: "C"; }             Text { text: "C"; }
}                                }

결과:                             결과:
┌──────────┐                     ┌──────────────────┐
│    A     │                     │  A    B    C     │
│    B     │                     └──────────────────┘
│    C     │
└──────────┘
```

### DMaster의 메인 레이아웃

```
┌─────────────────────────────────────────────────────┐
│  Header Bar  (Rectangle, height: 52px)               │
│  ┌─────────────────────────────────────────────────┐ │
│  │  "DMaster"              [⟳ Refresh] [＋ Save]  │ │
│  └─────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────┤
│  Content (HorizontalBox, vertical-stretch: 1)        │
│  ┌──────────────┐ │ ┌────────────────────────────┐  │
│  │ Profile List  │ │ │ Detail Panel               │  │
│  │ (width:300px) │ │ │ (horizontal-stretch: 1)    │  │
│  │               │ │ │                            │  │
│  │ > home_dual   │ │ │  home_dual                 │  │
│  │   work_single │ │ │  "집 듀얼 설정"              │  │
│  │               │ │ │  DISPLAYS: 2               │  │
│  │               │ │ │  ┌────────────────────┐    │  │
│  │               │ │ │  │ DP-1  1920x1080    │    │  │
│  │               │ │ │  │ eDP-1 1920x1080    │    │  │
│  │               │ │ │  └────────────────────┘    │  │
│  │               │ │ │  [Apply Profile] [Delete]  │  │
│  └──────────────┘ │ └────────────────────────────┘  │
├─────────────────────────────────────────────────────┤
│  Status Bar  (Rectangle, height: 28px)               │
│  ┌─────────────────────────────────────────────────┐ │
│  │  ● Ready                                        │ │
│  └─────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────┘
```

대응하는 Slint 코드 구조:
```slint
VerticalBox {                          // 전체 세로 배치
    Rectangle { height: 52px; ... }     // Header
    Rectangle { height: 1px; ... }      // 구분선
    HorizontalBox {                     // Content
        VerticalBox { width: 300px; }   // Profile List
        Rectangle { width: 1px; }       // 세로 구분선
        VerticalBox { }                 // Detail Panel
    }
    Rectangle { height: 1px; }          // 구분선
    Rectangle { height: 28px; }         // Status Bar
}
```

### stretch 속성

```slint
HorizontalBox {
    VerticalBox { width: 300px; }          // 고정 너비
    VerticalBox { horizontal-stretch: 1; }  // 나머지 공간 전부 차지
}
```

`stretch`는 남은 공간을 비율로 분배:
```
stretch: 1, stretch: 2 → 1:2 비율로 공간 분배
stretch: 1, (없음)     → stretch가 있는 쪽이 남은 공간 전부 차지
```

---

## 5. 조건부 렌더링

```slint
// 프로필이 없을 때
if profiles.length == 0 : VerticalBox {
    Text { text: "No profiles saved yet."; }
}

// 프로필이 있을 때
if profiles.length > 0 : ListView {
    for profile[i] in root.profiles : Rectangle {
        // 각 프로필 항목 렌더링
    }
}
```

### 선택 상태 표시

```slint
for profile[i] in root.profiles : Rectangle {
    // 선택된 항목이면 배경색 변경
    background: i == root.selected-index ? #3a7bd540 : transparent;

    // 선택 인디케이터 (왼쪽 파란 줄)
    Rectangle {
        width: i == root.selected-index ? 3px : 0px;
        background: #3a7bd5;
    }

    // 클릭 처리
    TouchArea {
        clicked => {
            root.selected-index = i;
            root.select-profile(i);
        }
    }
}
```

📌 **삼항 연산자**: `조건 ? 참값 : 거짓값` — CSS와 같은 문법.

---

## 6. 대화상자 (Dialog) 패턴

### 오버레이 구현

```slint
// 반투명 배경 오버레이
if root.show-save-dialog : Rectangle {
    background: #00000066;          // 검은색 40% 투명도
    x: 0; y: 0;
    width: root.width;
    height: root.height;

    // 대화상자 컴포넌트 (중앙 배치)
    SaveDialog {
        x: (parent.width - self.width) / 2;
        y: (parent.height - self.height) / 2;

        confirmed(name, desc) => {
            root.save-profile(name, desc);
            root.show-save-dialog = false;
        }
        cancelled => {
            root.show-save-dialog = false;
        }
    }
}
```

### SaveDialog 컴포넌트

```slint
// save_dialog.slint
export component SaveDialog inherits Rectangle {
    width: 420px;
    height: 210px;
    background: #2b2d30;
    border-radius: 10px;
    drop-shadow-blur: 24px;

    callback confirmed(string, string);   // (name, description)
    callback cancelled();

    VerticalBox {
        Text { text: "Save Current Profile"; }

        name-edit := LineEdit {
            placeholder-text: "Profile name (required)";
        }

        desc-edit := LineEdit {
            placeholder-text: "Description (optional)";
        }

        HorizontalBox {
            Button {
                text: "Cancel";
                clicked => { root.cancelled(); }
            }
            Button {
                text: "Save";
                enabled: name-edit.text != "";      // 이름 비어있으면 비활성화
                clicked => {
                    root.confirmed(name-edit.text, desc-edit.text);
                }
            }
        }
    }
}
```

📌 **`:= 바인딩`**: `name-edit := LineEdit { }` — 컴포넌트에 이름을 부여. 다른 곳에서 `name-edit.text`로 접근 가능.

---

## 7. 파일 구조와 import

```slint
// app.slint
import { Button, VerticalBox, HorizontalBox, ListView } from "std-widgets.slint";
import { SaveDialog } from "save_dialog.slint";
import { MappingDialog, MappingRow } from "mapping_dialog.slint";

export { MappingRow }   // MappingRow를 Rust에서도 접근 가능하게 re-export
```

**import 규칙:**
- `"std-widgets.slint"` — Slint 내장 위젯 (Button, LineEdit, ComboBox 등)
- `"save_dialog.slint"` — 같은 디렉토리의 커스텀 컴포넌트
- `export { }` — Rust 코드에서 접근 가능하게 노출

---

## 8. Slint ↔ Rust 타입 대응표

| Slint 타입 | Rust 타입 |
|-----------|-----------|
| `string` | `SharedString` |
| `int` | `i32` |
| `float` | `f32` |
| `bool` | `bool` |
| `color` | `Color` |
| `[T]` (배열) | `ModelRc<T>` |
| `struct` | 동명의 Rust struct (자동 생성) |
| `callback(args)` | `.on_callback_name(closure)` |

---

## 정리

| 개념 | Slint 문법 | 역할 |
|------|-----------|------|
| 컴포넌트 | `component X inherits Y { }` | UI 요소 정의 |
| 속성 | `in-out property <T> name: default;` | 상태 저장 |
| 콜백 | `callback name(arg_types);` | 이벤트 → Rust 연결 |
| 레이아웃 | `VerticalBox`, `HorizontalBox` | 자식 배치 |
| 반복 | `for item[i] in list : ...` | 리스트 렌더링 |
| 조건 | `if condition : ...` | 조건부 표시 |
| 바인딩 | `name := Component { }` | 다른 곳에서 참조 |
| import | `import { X } from "file.slint";` | 컴포넌트 가져오기 |

## 확인 문제

1. `in-out property <[ProfileEntry]> profiles;`에서 `[ProfileEntry]`는 어떤 타입인가?
2. `clicked => { root.refresh-profiles(); }`에서 `root`는 무엇을 가리키는가?
3. Slint의 `display-count`가 Rust에서 어떤 이름으로 접근되는가?
4. `horizontal-stretch: 1`의 효과는?

<details>
<summary>정답</summary>

1. `ProfileEntry`의 배열(모델). Rust에서는 `ModelRc<ProfileEntry>`로 매핑.
2. 현재 속한 최상위 `component` (이 경우 `App`). `root.`을 통해 App의 속성과 콜백에 접근.
3. `display_count` (kebab-case → snake_case 자동 변환).
4. 부모 레이아웃에서 남은 가로 공간을 이 요소가 차지. stretch 값이 높을수록 더 많은 비율을 차지.
</details>
