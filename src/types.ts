// Rust scan_ports가 반환하는 프로세스 하나. (Rust PortProcess와 1:1)
export interface PortProcess {
  pid: number;
  port: number;
  protocol: string; // "TCP" | "UDP"
  command: string; // lsof COMMAND (예: "node", "rapportd")
  processName: string; // 지금은 command와 동일
  user: string; // 소유 사용자
  address: string; // "*:3000 (LISTEN)" 등
}

// process_meta가 pid별로 반환하는 사람이 읽을 메타 (Rust ProcessMeta와 1:1)
export interface ProcessMeta {
  pid: number;
  cwd: string; // 작업 폴더
  fullCommand: string; // 풀 명령줄
}

// 상세 모달용 (Rust ProcessDetail와 1:1)
export interface ProcessDetail {
  pid: number;
  cwd: string;
  fullCommand: string;
  memPercent: number; // 전체 메모리 대비 %
  rssKb: number; // 물리 메모리 KB
}

// TS에서 그룹을 붙인 뷰 모델 (P3에서 사용)
export type ProcessGroup = "dev" | "system" | "other";

export interface DecoratedProcess extends PortProcess {
  group: ProcessGroup;
  isMine: boolean; // Portal이 띄운 프로세스인지 (P5에서 채움)
  // 사람이 읽는 정보 (process_meta로 보강, 없으면 undefined)
  label?: string; // 예: "portal"
  framework?: string; // 예: "vite" / "next"
  cwd?: string;
}
