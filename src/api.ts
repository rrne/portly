// Rust 커맨드 호출 래퍼. 프론트는 여기만 통해 백엔드와 대화한다.
import { invoke } from "@tauri-apps/api/core";
import type { PortProcess, ProcessDetail, ProcessMeta } from "./types";

/** 포트를 물고 있는 프로세스 목록을 스캔한다. */
export const scanPorts = () => invoke<PortProcess[]>("scan_ports");

/** pid 목록의 사람이 읽을 메타(cwd + 풀 명령)를 배치로 가져온다. */
export const processMeta = (pids: number[]) =>
  invoke<ProcessMeta[]>("process_meta", { pids });

/** pid 하나의 상세 정보(메모리 등)를 가져온다. 상세 모달용. */
export const processDetail = (pid: number) =>
  invoke<ProcessDetail>("process_detail", { pid });

/** pid를 종료한다. force=true면 SIGKILL(강제). */
export const killPid = (pid: number, force = false) =>
  invoke<void>("kill_pid", { pid, force });

// ── 설정 ──
export interface Config {
  projectRoots: string[];
}

/** 현재 사용자의 홈 경로. */
export const homeDir = () => invoke<string>("home_dir");

export const loadConfig = () => invoke<Config>("load_config");
export const saveConfig = (config: Config) =>
  invoke<void>("save_config", { config });
