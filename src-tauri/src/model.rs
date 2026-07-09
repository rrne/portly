// Portal 데이터 모델 — Rust ↔ TS 계약.
// serde(rename_all="camelCase")로 TS 관례(camelCase)에 맞춘다.

use serde::Serialize;

/// 포트를 물고 있는 프로세스 하나. scan_ports가 이 목록을 반환한다.
/// group/isMine 같은 파생 필드는 여기 없다 — TS가 붙인다(얇은 백엔드 원칙).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortProcess {
    pub pid: i32,
    pub port: u16,
    pub protocol: String,   // "TCP" | "UDP"
    pub command: String,    // lsof COMMAND 컬럼 (예: "node", "rapportd")
    pub process_name: String, // 지금은 command와 동일. 나중에 ps로 풀 경로 보강.
    pub user: String,       // 소유 사용자 (예: "coco", "root")
    pub address: String,    // "*:3000", "127.0.0.1:5432 (LISTEN)" 등
}

/// pid 하나의 사람이 읽을 만한 메타. process_meta가 배치로 반환한다.
/// "이게 내 어떤 프로젝트인지" 알려주는 재료: 작업 폴더 + 풀 명령줄.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessMeta {
    pub pid: i32,
    pub cwd: String,          // 작업 디렉토리 (예: /Users/coco/Desktop/factory/portal)
    pub full_command: String, // 풀 명령줄 (예: node .../vite/bin/vite.js)
}

/// 상세 모달용 정보. process_detail(pid)가 반환한다.
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProcessDetail {
    pub pid: i32,
    pub cwd: String,
    pub full_command: String,
    pub mem_percent: f32, // 전체 메모리 대비 % (ps %mem)
    pub rss_kb: u64,      // 실제 물리 메모리 (KB, ps rss)
}
