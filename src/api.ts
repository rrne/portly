// Rust 커맨드 호출 래퍼. 프론트는 여기만 통해 백엔드와 대화한다.
import { invoke } from "@tauri-apps/api/core";
import { openUrl, revealItemInDir } from "@tauri-apps/plugin-opener";
import type { PortProcess, ProcessDetail, ProcessMeta } from "./types";

/** 포트를 기본 브라우저로 연다. (http://localhost:포트) */
export const openPort = (port: number) => openUrl(`http://localhost:${port}`);

/** 포트를 물고 있는 프로세스 목록을 스캔한다. */
export const scanPorts = () => invoke<PortProcess[]>("scan_ports");

/** pid 목록의 사람이 읽을 메타(cwd + 풀 명령)를 배치로 가져온다. */
export const processMeta = (pids: number[]) =>
  invoke<ProcessMeta[]>("process_meta", { pids });

/** pid 하나의 상세 정보(메모리 등)를 가져온다. 상세 모달용. */
export const processDetail = (pid: number) =>
  invoke<ProcessDetail>("process_detail", { pid });

/** pid를 종료한다. expectedCommand로 PID 재사용을 방어한다. force=true면 SIGKILL. */
export const killPid = (pid: number, expectedCommand: string, force = false) =>
  invoke<void>("kill_pid", { pid, force, expectedCommand });

// ── 설정 ──
export interface Config {
  projectRoots: string[];
}

/** 현재 사용자의 홈 경로. */
export const homeDir = () => invoke<string>("home_dir");

// ── 설치 위치 진단 ──
export interface InstallStatus {
  exePath: string;
  location: "ok" | "dmg" | "translocated";
}
/** 앱이 DMG/translocation 경로에서 실행 중인지 확인한다(트레이 오작동 원인). */
export const installStatus = () => invoke<InstallStatus>("install_status");

/** Finder에서 해당 파일을 강조 표시한다(Applications로 옮기도록 안내용). */
export const revealInFinder = (path: string) => revealItemInDir(path);

export const loadConfig = () => invoke<Config>("load_config");
export const saveConfig = (config: Config) =>
  invoke<void>("save_config", { config });

// ── 등록 프로젝트 (폴더+명령 등록 → ▶ 실행) ──
export interface Project {
  id: string;
  name: string;
  cwd: string;
  command: string;
}

export interface Detected {
  manager: string; // pnpm | yarn | npm | bun | ""
  scripts: string[];
  suggestedCommand: string;
}
/** 폴더에서 패키지 매니저+스크립트를 감지해 추천 명령을 만든다. */
export const detectCommand = (cwd: string) =>
  invoke<Detected>("detect_command", { cwd });

export const listProjects = () => invoke<Project[]>("list_projects");
export const saveProject = (project: Project) =>
  invoke<Project[]>("save_project", { project });
export const deleteProject = (id: string) =>
  invoke<Project[]>("delete_project", { id });
/** 등록 프로젝트를 실행한다(detached). 반환=자식 pid. */
export const startApp = (id: string) => invoke<number>("start_app", { id });
export const tailLog = (id: string, lines = 40) =>
  invoke<string>("tail_log", { id, lines });
