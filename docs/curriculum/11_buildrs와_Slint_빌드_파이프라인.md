# 11. build.rs와 Slint 빌드 파이프라인

## 이 문서에서 배우는 것

- `build.rs`가 왜 필요한지
- `slint_build::compile("ui/app.slint")`의 의미
- 변경 시 어떤 파일이 다시 빌드되는지

---

## 1. DMaster의 build.rs

대상 파일: `dmaster_gui/build.rs`

```rust
fn main() {
    slint_build::compile("ui/app.slint").unwrap();
}
```

이 3줄이 하는 일:

1. Cargo가 `dmaster_gui` 빌드 전에 `build.rs`를 실행
2. Slint 소스(`ui/app.slint`)를 Rust 코드로 생성
3. 생성 코드를 `src/main.rs`에서 `slint::include_modules!()`로 포함

---

## 2. Cargo.toml 연결점

`dmaster_gui/Cargo.toml`:

```toml
[package]
build = "build.rs"

[dependencies]
slint = "1.15"

[build-dependencies]
slint-build = "1.15"
```

포인트:

- `build = "build.rs"`: build script 진입점 지정
- `slint-build`: build script 전용 의존성
- `slint`: 런타임 UI 라이브러리

`build-dependencies`와 `dependencies`를 분리한 이유는 "빌드 시점 코드"와 "실행 시점 코드"가 다르기 때문이다.

---

## 3. 컴파일 흐름

```text
cargo build -p dmaster_gui
  ├─ Step 1: build.rs 실행
  │    └─ slint_build::compile("ui/app.slint")
  │         └─ app.slint + import된 *.slint 파싱/코드생성
  │
  ├─ Step 2: Rust 본 컴파일
  │    └─ src/main.rs 의 slint::include_modules!() 확장
  │
  └─ Step 3: dmaster_gui 바이너리 생성
```

---

## 4. main.rs 연결 방식

`dmaster_gui/src/main.rs`에서 핵심 연결은 다음 두 줄이다.

```rust
slint::include_modules!();

let app = App::new().expect("failed to create UI");
```

`App` 타입은 `app.slint`의 `export component App`에서 생성된다. 즉, `.slint` 정의가 Rust 타입 시스템으로 들어오는 경로가 `build.rs`다.

---

## 5. 실전 수정 시 체크포인트

`.slint` 수정 후 빌드가 실패하면 아래 순서로 확인하면 된다.

1. `app.slint`에 문법 오류가 없는지
2. `import`된 파일 경로가 유효한지
3. `export`된 타입/필드 이름과 `main.rs` 사용 이름이 맞는지

대표 명령:

```bash
cargo build -p dmaster_gui
```

---

## 6. 왜 build.rs를 유지해야 하나?

Slint UI를 Rust 소스 코드에 하드코딩하지 않고 `.slint` 파일로 분리할 수 있고, UI 변경 시 Rust 로직과 독립적으로 반복 수정이 가능해진다.

즉 DMaster에서 `build.rs`는 "UI DSL ↔ Rust 코드" 브릿지다.

---

## 확인 문제

1. `build-dependencies`에 `slint-build`가 있어야 하는 이유는?
2. `build.rs`를 제거하면 어떤 단계에서 빌드가 깨질 가능성이 큰가?
3. `slint::include_modules!()`는 어떤 코드에 의존하는가?

<details>
<summary>정답</summary>

1. build script 실행 시점에만 필요하기 때문이다.
2. `App` 같은 Slint 생성 타입을 찾지 못해 Rust 컴파일 단계에서 실패한다.
3. `build.rs`가 생성한 Slint Rust 코드에 의존한다.
</details>
