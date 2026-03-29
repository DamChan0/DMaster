# 12. Cargo 의존성 관리와 기능 플래그

## 이 문서에서 배우는 것

- DMaster Workspace의 의존성 구조
- `features = [..]`를 현재 코드에서 어떻게 쓰는지
- `dependencies`/`build-dependencies`/path dependency 차이

---

## 1. Workspace 기준 구조

루트 `Cargo.toml`:

```toml
[workspace]
members = ["dmaster_core", "dmaster_gui", "dmaster_cli"]
```

의미:

- 세 crate를 하나의 workspace로 묶음
- 공통 lockfile/target 캐시 사용
- `cargo build -p dmaster_gui`처럼 패키지 단위 빌드 가능

---

## 2. 현재 DMaster 의존성 맵

```text
dmaster_core
  ├─ serde      (derive feature 사용)
  ├─ serde_json
  └─ winapi     (winuser, wingdi feature 사용)

dmaster_gui
  ├─ dmaster_core (path = ../dmaster_core)
  ├─ slint
  └─ slint-build (build-dependencies)

dmaster_cli
  └─ dmaster_core (path = ../dmaster_core)
```

핵심 파일:

- `dmaster_core/Cargo.toml`
- `dmaster_gui/Cargo.toml`
- `dmaster_cli/Cargo.toml`

---

## 3. feature 사용 예시 (현재 코드)

### 3-1. serde derive feature

```toml
serde = { version = "1.0", features = ["derive"] }
```

이게 없으면 `#[derive(Serialize, Deserialize)]`를 사용할 수 없다.

### 3-2. winapi 세부 모듈 feature

```toml
winapi = { version = "0.3", features = ["winuser", "wingdi"] }
```

`windows.rs`에서 쓰는 타입/함수:

- `winapi::um::winuser::*`
- `winapi::um::wingdi::*`

필요한 모듈만 켜서 컴파일 범위를 줄이는 방식이다.

---

## 4. dependency 종류 차이

### `dependencies`

실행 코드가 직접 사용하는 의존성.

예: `dmaster_gui`의 `slint`, `dmaster_core`

### `build-dependencies`

`build.rs`에서만 쓰는 의존성.

예: `dmaster_gui`의 `slint-build`

### path dependency

같은 저장소의 로컬 crate 연결.

```toml
dmaster_core = { path = "../dmaster_core" }
```

이 설정 덕분에 GUI/CLI가 동일한 core 로직을 공유한다.

---

## 5. 버전 표기와 업데이트

현재 프로젝트 예시:

```toml
serde_json = "1.0.141"
slint = "1.15"
```

Cargo 기본 규칙(호환 범위 허용)으로, patch/minor 업데이트를 자동 수용한다(semver 호환 범위 내).

실무에서 자주 쓰는 명령:

```bash
# 현재 의존성 트리 확인
cargo tree

# 빌드 테스트
cargo build

# 특정 패키지만
cargo build -p dmaster_core
```

---

## 6. DMaster에 바로 적용 가능한 확장 포인트

1. Core에 새 직렬화 포맷 지원 시: `serde_*` 계열 의존성 추가
2. GUI 빌드 처리 확장 시: `build-dependencies`에 도구 추가
3. 플랫폼 분기 확장 시: `winapi` feature 또는 타겟별 dependency 테이블 검토

---

## 확인 문제

1. 왜 `slint-build`는 `dependencies`가 아니라 `build-dependencies`인가?
2. `path = "../dmaster_core"`를 쓰는 장점은?
3. `serde`에서 `derive` feature를 빼면 어떤 코드가 먼저 깨질 가능성이 큰가?

<details>
<summary>정답</summary>

1. 런타임이 아니라 build script 실행 시점에만 필요하기 때문이다.
2. 동일 저장소에서 core 변경을 즉시 GUI/CLI에 반영하며 버전 퍼블리시 없이 통합 개발 가능하다.
3. `display_info.rs` 같은 `#[derive(Serialize, Deserialize)]` 선언부.
</details>
