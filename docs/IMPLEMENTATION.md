# Portal — 구현 계획 (IMPLEMENTATION)

> 기술 구현 계획서 (2026-07-08). 제품 범위는 `PLAN.md`에서 이미 확정됨 — 이 문서는 **어떻게 만드는가**만 다룬다.
> 대상 독자: React/TS 숙련 + Rust 첫 경험. 그래서 Rust 백엔드는 **얇게**(OS 호출만), 로직(그룹핑/정렬/필터)은 전부 TS로 민다.
> 스택: Tauri v2 (Rust) + React + Vite + TypeScript, pnpm. macOS 우선.

---

## 0. API 검증 요약 (2026-07 기준, 출처 명시)

이 계획은 다음을 **공식 문서로 확인**하고 작성했다. Tauri v1과 v2는 트레이/설정 API가 크게 다르므로 v1 예제를 그대로 쓰면 안 된다.

| 주제 | 확정 결론 | 출처 |
|---|---|---|
| 스캐폴딩 | `pnpm create tauri-app` → 대화형으로 pnpm + React + TypeScript 선택 | https://v2.tauri.app/start/create-project/ , https://github.com/tauri-apps/create-tauri-app |
| 트레이 | v2는 `tauri::tray::TrayIconBuilder` + `TrayIconEvent`. **v1의 `SystemTray`/`SystemTrayEvent`/`.system_tray()`는 폐기됨 — 쓰지 말 것** | https://v2.tauri.app/learn/system-tray/ |
| Dock 숨김 | **런타임 `app.set_activation_policy(tauri::ActivationPolicy::Accessory)`를 `setup` 훅에서 `#[cfg(target_os="macos")]`로 호출**. (config에 `activationPolicy` 키는 v2 스키마에 **없음** — 아래 "주의" 참고) | https://v2.tauri.app/learn/system-tray/ , https://github.com/tauri-apps/tauri/issues/9244 |
| command / invoke | `#[tauri::command]` + `tauri::generate_handler![...]`, 프론트 `invoke('name', {args})` (`@tauri-apps/api/core`) | https://v2.tauri.app/develop/calling-rust/ |
| 이벤트 | Rust: `use tauri::Emitter; app.emit("evt", payload)`. TS: `listen<T>('evt', cb)` (`@tauri-apps/api/event`) | https://v2.tauri.app/develop/calling-frontend/ |
| 상태 공유 | `.manage(Mutex::new(State))` + 커맨드 인자 `state: State<'_, Mutex<T>>`. 동기 커맨드는 `std::sync::Mutex`, async 커맨드에서 lock을 await 넘어 잡으려면 `tokio::sync::Mutex` | https://v2.tauri.app/develop/state-management/ |
| shell/process 권한 | `capabilities/*.json`에 명시 grant 필요. v2는 v1과 달리 기본 전면 차단 | https://v2.tauri.app/plugin/shell/ , https://v2.tauri.app/security/permissions/ |
| config 스키마 | 최상위 `app.windows[]`, `app.trayIcon`, `bundle`. **v1의 `"tauri": {...}` 네스팅은 사라짐** | https://v2.tauri.app/reference/config/ , https://schema.tauri.app/config/2 |

### 검증에서 나온 결정적 주의사항 (반드시 읽을 것)

1. **웹의 "Tauri v2 menubar" 튜토리얼 상당수가 v1 코드다.** 검색 중 나온 인기 dev.to 글은 `SystemTray::new()`, `.system_tray()`, `SystemTrayEvent::LeftClick { position, size }`, 그리고 `"tauri": { "macOS": { "activationPolicy" } }` config를 보여줬는데 **전부 v1**이다. v2에서는:
   - 트레이는 `TrayIconBuilder::new().on_tray_icon_event(|tray, event| ...)` + `TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. }`.
   - config에 `activationPolicy` 키가 **없다** (스키마 확인함). Dock 숨김은 **런타임 API**로 한다.
2. **`set_activation_policy`는 과거 setup+빌드후 이중 호출 시 패닉 이슈가 있었다** (tauri#9244, #8713에서 수정됨). 우리는 **`setup` 훅에서 한 번만** 호출한다. 최신 tauri 2.x에서 안정적이지만, 실패하면 대안(빈 dock 아이콘 리소스 트릭)이 있으니 아래 Phase 1 트러블슈팅 참조.
3. **트레이 클릭 시 창을 트레이 근처에 띄우는 좌표 계산**은 v2 공식 system-tray 문서가 예제를 주지 않는다. `TrayIconEvent::Click`의 `rect`(트레이 아이콘의 전역 위치/크기)를 읽어 `window.set_position(PhysicalPosition)`로 배치한다. 이 부분은 **버전 민감/실험 필요** 영역으로 명시한다 — 처음엔 화면 우상단 고정 좌표로 시작하고, rect 기반 정밀 배치는 Phase 1 말미에 튜닝한다.
4. **detached spawn은 순수 Rust `std::process::Command`로 한다** (shell 플러그인 아님). 이유는 Phase 5 참조 — 앱 종료 후에도 자식이 살아남아야 하는데, 플러그인의 스코프/수명 모델보다 `Command` + `setsid`(pre_exec) 직접 제어가 명확하다. shell 플러그인은 kill/열기 등 보조로만.

---

## 아키텍처 원칙 (얇은 Rust)

```
React/TS  ──invoke──▶  Rust command (얇음: lsof/kill/spawn 실행 → JSON 반환)
   ▲                        │
   │◀──── emit(event) ──────┘  (watch 스레드가 주기적으로 스캔 후 emit)

TS가 담당: 그룹핑, 정렬, 필터, "내 dev/시스템/기타" 분류, uptime 계산 표시, UI 상태
Rust가 담당: OS 호출과 직렬화뿐. 판단 로직 없음.
```

Rust 커맨드는 5개로 고정한다: `scan_ports`, `kill_pid`, `start_app`, `list_projects`, `save_project`(+ 삭제). 그 외 전부 TS.

---

## 데이터 모델 (Rust ↔ TS 계약)

두 언어가 같은 JSON을 주고받는다. Rust struct에 `#[serde(rename_all = "camelCase")]`를 붙여 TS 관례(camelCase)에 맞춘다.

### 발견된 프로세스 (scan_ports 반환 원소)

**TypeScript** (`src/types.ts`):
```ts
export interface PortProcess {
  pid: number;
  port: number;
  protocol: string;      // "TCP" | "UDP"
  command: string;       // lsof COMMAND 컬럼 (예: "node", "rapportd")
  processName: string;   // 풀 실행 경로/이름 (가능하면 ps로 보강, 없으면 command)
  user: string;          // 소유 사용자 (예: "coco", "root")
  address: string;       // "*:3000", "127.0.0.1:5432" 등
  // ↓ Rust가 주지 않는, TS가 파생/보강하는 필드는 별도 타입으로 분리
}

// TS에서 그룹/uptime을 붙인 뷰 모델
export type ProcessGroup = "dev" | "system" | "other";
export interface DecoratedProcess extends PortProcess {
  group: ProcessGroup;
  isMine: boolean;       // Portal이 spawn했고 추적 중인 PID인지 (registry PID 매칭)
}
```

**Rust** (`src-tauri/src/model.rs`):
```rust
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortProcess {
    pub pid: i32,
    pub port: u16,
    pub protocol: String,
    pub command: String,
    pub process_name: String,
    pub user: String,
    pub address: String,
}
```
> 주: `group`/`isMine`은 **Rust가 계산하지 않는다**(얇은 백엔드 원칙). TS가 registry와 lsof 경로 시그니처로 붙인다.

### 등록 프로젝트 (registry.json 원소)

**TypeScript**:
```ts
export interface Project {
  id: string;            // uuid or slug
  name: string;          // "learn-api"
  cwd: string;           // 절대경로 실행 디렉토리
  command: string;       // "pnpm dev" (셸 파싱은 Rust가 최소한으로)
  port?: number;         // 기대 포트 (충돌 감지/매칭용, 선택)
  createdAt: string;     // ISO
}

export interface Registry { projects: Project[]; }
```

**Rust** (`src-tauri/src/model.rs`):
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub cwd: String,
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    pub created_at: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Registry {
    pub projects: Vec<Project>,
}
```

### spawn 추적 상태 (앱 메모리, 저장 안 함)

**Rust** (`src-tauri/src/state.rs`):
```rust
use std::collections::HashMap;

#[derive(Default)]
pub struct AppState {
    // project.id → 우리가 spawn한 자식의 PID (restart / "내가 띄운 것" 매칭용)
    pub spawned: HashMap<String, u32>,
}
```

---

## Phase 0 — 툴체인 설치 (Rust)

**Goal**: `cargo`/`rustc`가 동작. (이 머신엔 현재 없음. Xcode CLT는 설치됨.)

**명령** (터미널에서 사용자가 직접 실행 — 대화형이라 승인 필요):
```bash
# rustup 설치 (기본 stable 툴체인)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

# 현재 셸에 PATH 반영
source "$HOME/.cargo/env"

# 검증
rustc --version   # 예: rustc 1.8x.x
cargo --version
```
- Xcode Command Line Tools는 이미 설치됨(확인함) → 링커 준비 OK. 미설치였다면 `xcode-select --install` 선행.

**Done when**: `rustc --version`과 `cargo --version`이 버전을 출력한다.

---

## Phase 1 — 스캐폴딩 + 메뉴바 트레이 + 빈 팝오버 창 (Dock 숨김)

**Goal**: 트레이 아이콘이 상단바에 뜨고, 클릭하면 빈 창이 트레이 근처에 나타난다. Dock 아이콘 없음. 여기까지가 "돌아가는 뼈대".

**스캐폴딩 명령**:
```bash
# /Users/coco/Desktop/factory/portal 안에서 (PLAN.md 옆)
cd /Users/coco/Desktop/factory/portal
pnpm create tauri-app@latest
#  ? Project name        → portal   (또는 . 로 현재 폴더)
#  ? Frontend language   → TypeScript / JavaScript
#  ? Package manager      → pnpm
#  ? UI template          → React
#  ? UI flavor            → TypeScript

pnpm install
pnpm tauri dev     # 첫 실행 = Rust 크레이트 컴파일(수 분 소요)
```
> 폴더에 이미 `PLAN.md`가 있으니 `portal` 하위 폴더로 만든 뒤 `PLAN.md`/`IMPLEMENTATION.md`를 그 안으로 옮기거나, 프로젝트명을 `.`로 지정해 현재 폴더에 스캐폴딩한다(권장: 하위 폴더 `portal/app` 대신 리포 루트 = `portal/`). 사용자 판단.

**생성/수정 파일**:
- `src-tauri/src/lib.rs` — 트레이 + activation policy + setup (핵심)
- `src-tauri/tauri.conf.json` — 창/트레이/번들 config
- `src-tauri/Cargo.toml` — feature `tray-icon` 확인
- `src-tauri/capabilities/default.json` — 창 제어 권한
- `src/App.tsx` — 빈 팝오버 UI

**`Cargo.toml`** (tauri 의존성에 트레이 feature):
```toml
[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

**`tauri.conf.json`** (v2 스키마 — 최상위 `app.windows`/`app.trayIcon`/`bundle`):
```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "Portal",
  "version": "0.1.0",
  "identifier": "com.handys.portal",
  "build": {
    "frontendDist": "../dist",
    "devUrl": "http://localhost:1420",
    "beforeDevCommand": "pnpm dev",
    "beforeBuildCommand": "pnpm build"
  },
  "app": {
    "macOSPrivateApi": true,
    "windows": [
      {
        "label": "main",
        "title": "Portal",
        "width": 360,
        "height": 520,
        "resizable": false,
        "decorations": false,
        "transparent": true,
        "alwaysOnTop": true,
        "visible": false,
        "skipTaskbar": true,
        "focus": true
      }
    ],
    "trayIcon": {
      "id": "main",
      "iconPath": "icons/tray.png",
      "iconAsTemplate": true,
      "tooltip": "Portal"
    },
    "security": { "csp": null }
  },
  "bundle": {
    "active": true,
    "targets": "app",
    "icon": ["icons/32x32.png", "icons/128x128.png", "icons/icon.icns"]
  }
}
```
> 주의: `activationPolicy`는 config 키가 아니다(스키마에 없음, 검증함). 아래 Rust에서 런타임 설정. `iconAsTemplate: true`는 macOS 다크/라이트 메뉴바에서 아이콘이 자동 반전되게 한다. `transparent: true` 쓰려면 `macOSPrivateApi: true` 필요.

**`src-tauri/src/lib.rs`** (핵심 — 트레이 + Dock 숨김 + 클릭 토글):
```rust
use tauri::{
    Manager,
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // macOS: Dock 아이콘 숨기고 메뉴바 전용으로
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            // 트레이 아이콘 — 좌클릭 메뉴 끄고, 클릭 이벤트로 창 토글
            let _tray = TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().unwrap().clone())
                .show_menu_on_left_click(false)
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        rect,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(win) = app.get_webview_window("main") {
                            if win.is_visible().unwrap_or(false) {
                                let _ = win.hide();
                            } else {
                                position_window_at_tray(&win, rect);
                                let _ = win.show();
                                let _ = win.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Portal");
}

// 트레이 rect 기준으로 창을 아래·중앙 정렬. (좌표계는 실험 필요 영역)
fn position_window_at_tray(win: &tauri::WebviewWindow, rect: tauri::tray::Rect) {
    use tauri::{PhysicalPosition, Position};
    // rect.position / rect.size 는 트레이 아이콘의 전역 좌표.
    // 초기엔 단순히 rect 아래에 붙인다. 정밀 튜닝은 아래 트러블슈팅 참조.
    if let (Position::Physical(pos), _) = (rect.position, rect.size) {
        let win_w = win.outer_size().map(|s| s.width as f64).unwrap_or(360.0);
        let x = pos.x as f64 - win_w / 2.0;
        let y = pos.y as f64; // 메뉴바 바로 아래
        let _ = win.set_position(PhysicalPosition::new(x.max(0.0), y));
    }
}
```
> `main.rs`는 `portal_lib::run()`을 호출하는 얇은 진입점(스캐폴드가 생성). 로직은 `lib.rs`.

**포커스 잃으면 창 숨기기** — TS에서 (`src/App.tsx`):
```ts
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect } from "react";

useEffect(() => {
  const w = getCurrentWindow();
  const un = w.onFocusChanged(({ payload: focused }) => {
    if (!focused) w.hide();
  });
  return () => { un.then(f => f()); };
}, []);
```

**`capabilities/default.json`** (창 show/hide/position 권한):
```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "core:window:allow-show",
    "core:window:allow-hide",
    "core:window:allow-set-position",
    "core:window:allow-set-focus",
    "core:event:default"
  ]
}
```

**Done when**:
- `pnpm tauri dev` 실행 → 상단 메뉴바에 아이콘, **Dock에는 아이콘 없음**.
- 트레이 클릭 → 빈 창이 트레이 아래에 뜬다. 다시 클릭하거나 창 밖 클릭 → 사라진다.

**트러블슈팅 (버전 민감 영역)**:
- Dock이 계속 보이면: `set_activation_policy` 호출 시점 문제(tauri#9244 계열). setup에서 **한 번만** 호출하는지 확인. 그래도 안 되면 macOS `Info.plist`에 `LSUIElement = true`를 번들 시 주입하는 방법으로 폴백(빌드된 앱에만 적용, dev에선 안 보일 수 있음).
- `rect` 좌표가 예상과 다르면(멀티 모니터/스케일): 우선 화면 우상단 고정 좌표(`monitor.size().width - win_w - 8`, `y = 28`)로 시작하고 rect 기반은 나중에.

---

## Phase 2 — `scan_ports()`: lsof 파싱 → 타입 JSON → 읽기 전용 목록

**Goal**: 창을 열면 지금 포트를 물고 있는 프로세스 목록이 뜬다. (이것만으로도 유용.)

**핵심 명령** (이 머신에서 동작 확인됨):
```bash
lsof -i -P -n -sTCP:LISTEN
# rapportd 618 coco 10u IPv4 ... TCP *:65499 (LISTEN)
```
LISTEN만 볼지 전체 연결을 볼지: MVP는 **LISTEN 서버만**(dev 서버 관제 목적) → `-sTCP:LISTEN`. UDP까지 필요하면 `-i` 전체.

**생성/수정 파일**:
- `src-tauri/src/model.rs` — `PortProcess` (위 데이터 모델)
- `src-tauri/src/scan.rs` — lsof 실행 + 파싱
- `src-tauri/src/lib.rs` — `invoke_handler`에 `scan_ports` 등록
- `src/types.ts` — TS 타입
- `src/api.ts` — invoke 래퍼
- `src/components/ProcessList.tsx` — 렌더

**`src-tauri/src/scan.rs`**:
```rust
use crate::model::PortProcess;
use std::process::Command;

#[tauri::command]
pub fn scan_ports() -> Result<Vec<PortProcess>, String> {
    let out = Command::new("lsof")
        .args(["-i", "-P", "-n", "-sTCP:LISTEN"])
        .output()
        .map_err(|e| format!("lsof 실행 실패: {e}"))?;
    // lsof는 결과 없으면 exit 1 → stderr 비었으면 정상(빈 목록)으로 취급
    let text = String::from_utf8_lossy(&out.stdout);
    Ok(parse_lsof(&text))
}

// 얇게: 파싱만. 그룹 분류/정렬은 TS가 한다.
fn parse_lsof(text: &str) -> Vec<PortProcess> {
    let mut rows = Vec::new();
    for line in text.lines().skip(1) { // 헤더 스킵
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 9 { continue; }
        let command = cols[0].to_string();
        let pid = cols[1].parse::<i32>().unwrap_or(-1);
        let user = cols[2].to_string();
        let proto = cols[7].to_string();           // TCP/UDP
        let name = cols[8].to_string();            // *:3000 (LISTEN)
        let address = name.clone();
        let port = name.rsplit(':').next()
            .and_then(|p| p.trim_end_matches(|c: char| !c.is_ascii_digit()).parse::<u16>().ok())
            .unwrap_or(0);
        rows.push(PortProcess {
            pid, port, protocol: proto, command: command.clone(),
            process_name: command, user, address,
        });
    }
    rows
}
```
> `processName` 정밀화(풀 경로)는 Should 단계에서 `ps -p <pid> -o comm=`/`-o command=`로 보강. MVP는 lsof COMMAND로 충분.

**`lib.rs`** 등록 (setup은 Phase 1 그대로 유지, 여기에 추가):
```rust
.invoke_handler(tauri::generate_handler![crate::scan::scan_ports])
```

**`src/api.ts`**:
```ts
import { invoke } from "@tauri-apps/api/core";
import type { PortProcess } from "./types";
export const scanPorts = () => invoke<PortProcess[]>("scan_ports");
```

**`src/App.tsx`** (읽기 전용 렌더):
```ts
const [procs, setProcs] = useState<PortProcess[]>([]);
useEffect(() => { scanPorts().then(setProcs).catch(console.error); }, []);
```

**Done when**: 창을 열면 `lsof` 결과가 목록으로 보이고, PID/포트/커맨드/사용자/주소가 표시된다. 터미널에서 `python3 -m http.server 8000` 띄우면 목록에 8000이 나타난다(수동 새로고침).

---

## Phase 3 — 그룹핑 (내 dev / 시스템 / 기타) — **전부 TS**

**Goal**: 목록이 `내 dev 서버 / 시스템 / 기타` 3그룹으로 나뉜다. Rust 변경 **없음**.

**생성/수정 파일**:
- `src/grouping.ts` — 분류 휴리스틱
- `src/components/ProcessList.tsx` — 그룹 섹션 렌더

**`src/grouping.ts`**:
```ts
import type { PortProcess, DecoratedProcess, Project } from "./types";

const DEV_SIGNATURES = ["node", "next", "vite", "esbuild", "pnpm", "bun", "deno", "ruby", "rails", "python", "webpack"];

export function classify(p: PortProcess, projects: Project[], me: string): DecoratedProcess {
  const isMine = /* Phase5에서 spawned PID 매칭으로 채움 */ false;
  let group: DecoratedProcess["group"];
  if (p.user === "root" || p.command.startsWith("/System") || p.command.startsWith("/usr")) {
    group = "system";
  } else if (DEV_SIGNATURES.some(s => p.command.toLowerCase().includes(s)) || p.user === me) {
    group = "dev";
  } else {
    group = "other";
  }
  return { ...p, group, isMine };
}

export function groupAll(procs: PortProcess[], projects: Project[], me: string) {
  const decorated = procs.map(p => classify(p, projects, me));
  return {
    dev: decorated.filter(d => d.group === "dev"),
    system: decorated.filter(d => d.group === "system"),
    other: decorated.filter(d => d.group === "other"),
  };
}
```
> `me`(현재 사용자명)는 어디서? lsof의 소유자 컬럼과 비교해야 하니, `scan_ports`가 별도로 주거나 TS에서 `@tauri-apps/plugin-os`의 사용자명, 또는 첫 스캔에서 최빈 user를 "나"로 추정. MVP는 **"root가 아니고 dev 시그니처면 dev, root/System/usr면 system, 나머지 other"** 로 충분 — `me` 매칭은 정확도 개선용.

**Done when**: rapportd(Apple) 류는 `시스템`(root/경로), `node`/`vite`는 `내 dev`, 나머지는 `기타`로 자동 분류돼 섹션 헤더와 함께 보인다.

---

## Phase 4 — `kill_pid()` + 자동 갱신

**Goal**: 각 행에 kill 버튼. 누르면 죽고 목록이 즉시 갱신된다. 실패(권한 부족)는 우아하게.

**생성/수정 파일**:
- `src-tauri/src/kill.rs` — kill 커맨드
- `src-tauri/src/lib.rs` — 등록
- `src/api.ts`, `src/components/ProcessRow.tsx`

**`src-tauri/src/kill.rs`**:
```rust
use std::process::Command;

#[tauri::command]
pub fn kill_pid(pid: i32) -> Result<(), String> {
    // 우선 SIGTERM(정상 종료). 필요 시 프론트에서 force=true로 재호출 → SIGKILL.
    let out = Command::new("kill")
        .arg(pid.to_string())
        .output()
        .map_err(|e| format!("kill 실행 실패: {e}"))?;
    if out.status.success() { Ok(()) }
    else {
        let err = String::from_utf8_lossy(&out.stderr);
        Err(if err.contains("Operation not permitted") {
            format!("권한 없음: PID {pid}는 시스템/타 사용자 프로세스일 수 있습니다.")
        } else { format!("종료 실패: {}", err.trim()) })
    }
}
```
> force 옵션: 시그니처를 `kill_pid(pid: i32, force: bool)`로 두고 `["-9", pid]` vs `[pid]`. MVP는 SIGTERM만, force는 Should.

**TS 흐름** (버튼 → kill → 재스캔):
```ts
export const killPid = (pid: number) => invoke<void>("kill_pid", { pid });

async function onKill(pid: number) {
  try { await killPid(pid); }
  catch (e) { toast(String(e)); }        // 권한 에러 표시
  finally { setProcs(await scanPorts()); } // 즉시 갱신
}
```

**Done when**: `python3 -m http.server 9999` 띄우고 → Portal에서 9999 행 kill → 목록에서 사라지고 실제 프로세스도 죽는다. rapportd(root) kill 시도 → "권한 없음" 토스트가 뜨고 앱은 안 죽는다.

---

## Phase 5 — 프로젝트 등록(JSON) + `start_app()` detached spawn

**Goal**: 폴더/커맨드를 등록하고, ▶ 버튼으로 실행. 실행한 프로세스는 **Portal을 꺼도 살아있다**(detached).

**생성/수정 파일**:
- `src-tauri/src/registry.rs` — registry.json 읽기/쓰기 + `list_projects`/`save_project`/`delete_project`
- `src-tauri/src/spawn.rs` — `start_app` detached
- `src-tauri/src/state.rs` — `AppState { spawned }`
- `src-tauri/src/lib.rs` — `.manage(Mutex::new(AppState::default()))` + 등록
- `src-tauri/capabilities/default.json` — dialog(폴더 선택) 권한
- `src/components/RegisterForm.tsx`, `src/components/ProjectList.tsx`

**registry 위치**: `~/.portal/registry.json` (PLAN.md §8 권장 그대로. 아래 열린 질문 참조). Rust에서 `dirs`/`tauri::Manager::path()`로 홈 경로 획득:
```toml
# Cargo.toml
dirs = "5"
uuid = { version = "1", features = ["v4"] }
libc = "0.2"
```

**`src-tauri/src/registry.rs`** (얇은 파일 I/O):
```rust
use crate::model::{Project, Registry};
use std::{fs, path::PathBuf};

fn registry_path() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("홈 디렉토리를 찾을 수 없음")?;
    let dir = home.join(".portal");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("registry.json"))
}

#[tauri::command]
pub fn list_projects() -> Result<Vec<Project>, String> {
    let path = registry_path()?;
    if !path.exists() { return Ok(vec![]); }
    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let reg: Registry = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    Ok(reg.projects)
}

#[tauri::command]
pub fn save_project(project: Project) -> Result<Vec<Project>, String> {
    let path = registry_path()?;
    let mut reg: Registry = if path.exists() {
        serde_json::from_str(&fs::read_to_string(&path).map_err(|e| e.to_string())?)
            .unwrap_or_default()
    } else { Registry::default() };
    // id 있으면 갱신, 없으면 추가 (프론트가 id 채워 보냄)
    if let Some(existing) = reg.projects.iter_mut().find(|p| p.id == project.id) {
        *existing = project;
    } else {
        reg.projects.push(project);
    }
    fs::write(&path, serde_json::to_string_pretty(&reg).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    Ok(reg.projects)
}
```

**`src-tauri/src/spawn.rs`** (detached — 핵심 난관):
```rust
use crate::{model::Project, state::AppState};
use std::{process::Command, sync::Mutex};
use tauri::State;

#[tauri::command]
pub fn start_app(project: Project, state: State<'_, Mutex<AppState>>) -> Result<u32, String> {
    // 셸을 통해 "pnpm dev" 같은 커맨드 문자열을 실행 (cwd 지정)
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(&project.command).current_dir(&project.cwd);

    // ── detached: 새 세션 리더로 만들어 Portal 종료와 무관하게 살림 ──
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        unsafe {
            cmd.pre_exec(|| {
                // 새 세션 생성 → Portal(부모)이 죽어도 SIGHUP 안 받음
                libc::setsid();
                Ok(())
            });
        }
    }
    // 로그 tail(Should)을 위해선 여기서 stdout을 파일로 리다이렉트하거나 piped.
    // MVP detached는 부모 파이프를 물면 Portal 종료 시 깨지므로, 로그는 파일로:
    //   cmd.stdout(File::create(log_path)?); cmd.stderr(...same...);
    // MVP는 로그 없이 inherit/null.

    let child = cmd.spawn().map_err(|e| format!("실행 실패: {e}"))?;
    let pid = child.id();
    // 추적: project.id → pid (restart/"내가 띄운 것" 매칭)
    state.lock().unwrap().spawned.insert(project.id.clone(), pid);
    // child를 drop해도 죽지 않음(Rust는 Child에 Drop=kill 없음). 명시적으로 forget.
    std::mem::forget(child);
    Ok(pid)
}
```
> `libc = "0.2"`를 Cargo.toml에 추가. `setsid`로 detach하는 게 핵심 — 이게 없으면 Portal 종료 시 자식이 SIGHUP으로 죽어 "오늘 겪은 좀비 문제"의 반대 상황이 재현된다. **트레이드오프**: detach하면 로그 stdout을 파이프로 못 잡는다(부모 파이프가 앱과 함께 닫힘) → 로그 tail이 필요하면 **파일로 리다이렉트**해야 한다(위 주석). MVP는 로그 생략, Should에서 파일 방식.

**`state.lock().unwrap()`** — 동기 커맨드라 `std::sync::Mutex`로 충분(await 안 넘김). async로 바꾸면 `tokio::sync::Mutex` 필요(검증한 caveat).

**폴더 선택** — dialog 플러그인:
```bash
pnpm tauri add dialog        # 플러그인 설치 + lib.rs .plugin(...) + capabilities 자동 반영
```
TS:
```ts
import { open } from "@tauri-apps/plugin-dialog";
const dir = await open({ directory: true });
```
capabilities에 `"dialog:allow-open"` 추가 필요(add 명령이 대개 자동 처리).

**Done when**: RegisterForm에서 폴더 선택 + `pnpm dev` + 포트 3000 등록 → `~/.portal/registry.json`에 기록됨. ▶ 클릭 → 서버 기동, scan 목록에 3000 등장. **Portal을 완전히 종료(cmd+Q)해도** 그 서버는 계속 살아있다(`lsof -i:3000`으로 확인).

---

## Phase 6 — watch loop / 실시간 갱신

**Goal**: 수동 새로고침 없이 주기적으로 스캔해 자동 갱신.

**두 가지 방식 — 권장: TS 폴링(얇은 Rust 원칙에 부합)**:

**옵션 A (권장, 단순): TS setInterval 폴링**
```ts
useEffect(() => {
  const tick = () => scanPorts().then(setProcs).catch(() => {});
  tick();
  const id = setInterval(tick, 3000);   // 3초 (아래 열린 질문 참조)
  // 창이 숨겨져 있을 땐 멈춰 리소스 절약:
  return () => clearInterval(id);
}, []);
```
창 표시 여부와 연동: `onFocusChanged`로 보일 때만 폴링.

**옵션 B (이벤트): Rust watch 스레드 + emit** — 창이 닫혀 있어도 트레이 배지 갱신 등 백그라운드가 필요할 때만.
```rust
// setup 훅 안에서
let handle = app.handle().clone();
std::thread::spawn(move || {
    use tauri::Emitter;
    loop {
        if let Ok(procs) = crate::scan::scan_ports_inner() {
            let _ = handle.emit("ports-updated", procs);
        }
        std::thread::sleep(std::time::Duration::from_secs(3));
    }
});
```
```ts
import { listen } from "@tauri-apps/api/event";
useEffect(() => {
  const un = listen<PortProcess[]>("ports-updated", e => setProcs(e.payload));
  return () => { un.then(f => f()); };
}, []);
```

**권장 결정**: MVP는 **옵션 A(창 보일 때만 3초 폴링)**. Rust를 얇게 유지하고 배터리 영향 최소. 트레이 배지/백그라운드 알림이 필요해지면 옵션 B로 승격.

**Done when**: 창을 열어둔 채 터미널에서 서버를 띄우거나 죽이면, 3초 내 목록이 저절로 반영된다.

---

## Phase 7+ — Should 항목 (순차)

각각 독립적으로 추가 가능. Rust 최소 변경 유지.

1. **좀비/오래된 프로세스 강조 (uptime)** — `scan_ports`가 `ps -p <pid> -o etime=,comm=`를 pid별로 붙여 `startedAt`/`etime` 추가, 또는 별도 `process_meta(pids)` 커맨드. TS가 임계값(예 12h+) 넘으면 경고색. lsof 결과의 PID들을 한 번의 `ps` 호출로 배치 조회(호출 횟수 최소화).
2. **restart** — `AppState.spawned`에서 project.id→pid 찾아 `kill_pid` 후 `start_app`. "내가 띄운 것"만 restart 노출(isMine).
3. **포트 충돌 감지** — TS: registry의 `port`와 현재 scan을 대조해 "3000은 이미 X가 씀" 배지.
4. **로그 tail** — Phase 5 spawn 시 stdout/stderr를 `~/.portal/logs/<id>.log`로 리다이렉트(detach 호환). `tail_log(id, lines)` 커맨드가 파일 끝 N줄 반환. TS가 폴링해 표시.
5. **다크모드** — 순수 프론트(CSS `prefers-color-scheme`). Rust 무관.
6. **kill 확인 다이얼로그(Could)** — system/other 그룹에만 확인 모달(마찰 추가), dev는 즉시.

---

## 핵심 기술 난관 (PLAN.md §6) — 각 Phase의 완화책

| 난관 | 완화 | 해당 Phase |
|---|---|---|
| **detached spawn / 프로세스 소유권** | `setsid`(pre_exec) + `std::mem::forget(child)`로 Portal 종료와 자식 수명 분리. detach가 로그 파이프를 끊는 트레이드오프는 로그를 **파일 리다이렉트**로 해결 | Phase 5, 7 |
| **"이미 떠 있는 것" vs "내가 띄운 것"** | `AppState.spawned: HashMap<project.id, pid>`에 spawn PID 기록 → scan 결과 PID와 TS에서 매칭해 `isMine` 표시. restart는 isMine에만 | Phase 5, 7 |
| **lsof 파싱 & 그룹 휴리스틱** | Rust는 **파싱만**(공백 split, 마지막 컬럼에서 포트 추출). 분류는 TS `grouping.ts`에서 root/경로→system, dev 시그니처→dev, 나머지→other. 규칙 조정이 프론트에서 빠름 | Phase 2, 3 |
| **권한(시스템 프로세스 kill 실패)** | `kill_pid`가 stderr의 "Operation not permitted"를 잡아 한국어 에러 메시지로 변환, 앱은 안 죽고 토스트만 | Phase 4 |
| **Rust 첫 경험** | 커맨드 5개로 고정, 각 파일 30줄 내외. 로직은 전부 TS. Mutex는 `std::sync`(동기 커맨드)만 써서 async/await 지옥 회피. 검증된 최소 스니펫만 사용 | 전 Phase |

---

## 열린 질문 (PLAN.md §8) — 권장 기본값

| 질문 | 권장 기본값 | 근거 |
|---|---|---|
| Rust 깊이 | **백엔드 최소만 Rust(커맨드 5개), 로직 전부 TS** | PLAN 원칙 + 학습 곡선 최소화. Rust는 lsof/kill/spawn/파일I/O만 |
| registry 위치 | **`~/.portal/registry.json`** | PLAN 제안 그대로. 홈 하위 단일 폴더에 logs/도 함께(`~/.portal/logs/`) |
| 갱신 주기 | **3초 폴링, 창이 보일 때만** | 2초는 lsof 부하가 잦고 배터리에 불리. 3초 + 숨김 시 정지가 균형. 수동 새로고침 버튼도 병행 |
| 크로스플랫폼 | **초기 macOS 전용** | lsof/kill/setsid/ActivationPolicy가 macOS 전제. Windows는 `netstat`/`taskkill`로 커맨드만 갈아끼우면 되도록 scan/kill을 `#[cfg]`로 분리해 둠(구조만 대비, 구현은 나중) |
| 아이콘/브랜딩 | **템플릿 아이콘(iconAsTemplate)으로 시작**, 브랜딩은 릴리스 직전 | 메뉴바 아이콘은 흑백 template 이미지가 다크/라이트 자동 대응 |

---

## 실행 순서 체크리스트 (요약)

- [ ] **P0** `rustup` 설치 → `cargo --version` OK
- [ ] **P1** `pnpm create tauri-app`(pnpm/React/TS) → 트레이 + Dock 숨김 + 빈 팝오버 (`pnpm tauri dev`)
- [ ] **P2** `scan_ports` + lsof 파싱 + 읽기 전용 목록
- [ ] **P3** `grouping.ts` — 내 dev/시스템/기타 (TS만)
- [ ] **P4** `kill_pid` + 창 보일 때 즉시 재스캔 + 권한 에러 처리
- [ ] **P5** registry(JSON) + dialog 폴더선택 + `start_app` detached(setsid)
- [ ] **P6** 3초 폴링 자동 갱신
- [ ] **P7+** uptime 강조 / restart / 충돌감지 / 로그tail / 다크모드

---

## 참고 문서 (검증 출처)

- Create a Project — https://v2.tauri.app/start/create-project/
- System Tray (TrayIconBuilder) — https://v2.tauri.app/learn/system-tray/
- Calling Rust (commands/invoke) — https://v2.tauri.app/develop/calling-rust/
- Calling Frontend (emit/listen) — https://v2.tauri.app/develop/calling-frontend/
- State Management (Mutex/State) — https://v2.tauri.app/develop/state-management/
- Shell plugin & permissions — https://v2.tauri.app/plugin/shell/
- Permissions/Capabilities — https://v2.tauri.app/security/permissions/
- Config reference / schema — https://v2.tauri.app/reference/config/ , https://schema.tauri.app/config/2
- set_activation_policy 런타임 이슈 — https://github.com/tauri-apps/tauri/issues/9244
- Rust std::process::Command / Child — https://doc.rust-lang.org/std/process/struct.Command.html
