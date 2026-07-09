# Portly 🔌

> 로컬 dev 서버를 메뉴바에서 끄고 켜는 macOS 앱.

떠 있는 서버를 클릭 한 번에 종료하고, 등록해둔 프로젝트는 ▶ 버튼으로 바로 실행합니다. 터미널에서 `lsof`/`kill` 치던 일을 메뉴바에서.

<p align="center">
  <a href="https://github.com/rrne/portly/releases/latest/download/Portly_0.2.1_universal.dmg">
    <img src="https://img.shields.io/badge/⬇%20Download-Portly%20v0.2.1%20(.dmg)-2ea44f?style=for-the-badge" alt="Download Portly">
  </a>
</p>

## 핵심

### 🔪 로컬 서버, 버튼 하나로 종료
지금 포트를 물고 있는 dev 서버를 메뉴바에서 보고 클릭 한 번에 끕니다. 안 죽는 좀비 서버, 포트 충돌을 터미널 없이 정리하세요.

### ▶ 등록해두고, 언제든 다시 실행
자주 쓰는 프로젝트 폴더를 한 번 등록하면 ▶ 버튼으로 dev 서버를 바로 켭니다. 실행 명령은 폴더를 보고 자동으로 채워집니다.

## 그 외

- **자동 분류** — `내 dev 서버` / `기타` / `시스템`으로 그룹, 내 것만 위에
- **읽기 쉬운 이름** — `node` 대신 `my-app (Vite)`처럼 표시
- **브라우저 열기** — 포트 행에서 `localhost:포트`를 바로
- **안전장치** — 시스템·DB 종료 시 확인창
- **상세 정보** — 메모리·명령줄·폴더를 모달로

## 설치

### 다운로드 (권장)

1. **[⬇ Portly v0.2.1 다운로드 (.dmg)](https://github.com/rrne/portly/releases/latest/download/Portly_0.2.1_universal.dmg)** — Intel·Apple Silicon 모두 지원
2. `.dmg`를 열고 `Portly`를 `Applications`로 드래그
3. 첫 실행 시 **우클릭 → 열기** (미서명 앱이라 macOS가 한 번 확인함)

> ⚠️ **메뉴바에 아이콘이 안 보이면?** macOS 노치 맥북은 메뉴바가 꽉 차면 아이콘을 숨깁니다. 다른 메뉴바 앱을 정리하거나, 라이브 액티비티(예: 노션 녹음)를 끄면 나타납니다.

### 직접 빌드

```bash
# 사전 준비: Rust(rustup), Node 20.19+ 또는 22+, pnpm
pnpm install
pnpm tauri build      # → src-tauri/target/release/bundle/dmg/*.dmg
```

개발 모드:

```bash
pnpm tauri dev
```

## 스택

- [Tauri v2](https://v2.tauri.app) (Rust) — 트레이·프로세스 제어
- React + Vite + TypeScript — UI
- 백엔드는 얇게(`lsof`/`kill`/`ps` 호출만), 분류·표시 로직은 전부 프론트

## 동작 원리

- `lsof -i -P -n -sTCP:LISTEN`로 LISTEN 중인 포트를 수집
- `ps` / `lsof -d cwd`로 작업 폴더·명령줄·메모리 보강
- cwd가 홈(또는 지정 폴더) 하위면 "내 dev 서버"로 분류
- 종료는 `kill`(SIGTERM). 시스템/DB는 확인 후에만

설정은 `~/.portal/config.json`에 저장됩니다.

## 라이선스

MIT
