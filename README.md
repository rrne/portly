# Portal 🔌

> 로컬 dev 서버를 메뉴바에서 한눈에 보고, 클릭 한 번으로 끄는 macOS 앱.

"3000번 포트 누가 물고 있지?", "어제 띄운 서버가 안 죽었나?" — 매번 `lsof` 치는 대신, 메뉴바 아이콘 클릭 한 번으로 지금 포트를 쓰는 프로세스를 보고 정리하세요.

<p align="center">
  <a href="https://github.com/rrne/portly/releases/latest/download/Portal_0.1.0_aarch64.dmg">
    <img src="https://img.shields.io/badge/⬇%20Download-Portal%20v0.1.0%20(.dmg)-2ea44f?style=for-the-badge" alt="Download Portal">
  </a>
</p>

## 기능

- **한눈에 보기** — 포트를 쓰는 프로세스를 메뉴바 팝오버에 표시
- **똑똑한 분류** — `내 dev 서버` / `기타` / `시스템`으로 자동 그룹, 내 프로젝트만 위에
- **사람이 읽는 이름** — `node` 대신 `portal (Vite)`처럼 폴더명·프레임워크로 표시
- **원터치 종료** — dev 서버는 버튼 하나로, 시스템·DB는 확인창으로 실수 방지
- **상세 정보** — 메모리 사용량, 명령줄, 작업 폴더를 모달로
- **내 폴더 필터** — 홈(`~`) 하위 또는 직접 지정한 폴더의 것만 "내 dev"로

## 설치

### 다운로드 (권장)

1. **[⬇ Portal v0.1.0 다운로드 (.dmg)](https://github.com/rrne/portly/releases/latest/download/Portal_0.1.0_aarch64.dmg)** — Apple Silicon(M1 이상) 전용
2. `.dmg`를 열고 `Portal`을 `Applications`로 드래그
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
