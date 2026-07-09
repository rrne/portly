// 포트 스캔 — lsof를 실행해 LISTEN 소켓을 파싱한다. 판단 로직 없음(얇은 백엔드).

use crate::model::{PortProcess, ProcessDetail, ProcessMeta};
use std::process::Command;

#[tauri::command]
pub fn scan_ports() -> Result<Vec<PortProcess>, String> {
    // -i: 네트워크 소켓, -P: 포트 숫자 그대로, -n: 호스트 이름 해석 안 함(빠름),
    // -sTCP:LISTEN: 듣고 있는 TCP만(= dev 서버 관제 목적)
    let out = Command::new("lsof")
        .args(["-i", "-P", "-n", "-sTCP:LISTEN"])
        .output()
        .map_err(|e| format!("lsof 실행 실패: {e}"))?;

    // lsof는 결과가 없으면 exit 1을 낸다 → stdout 기준으로만 파싱(빈 목록 정상 처리).
    let text = String::from_utf8_lossy(&out.stdout);
    Ok(parse_lsof(&text))
}

/// lsof 출력 한 줄 예:
/// `node  12345 coco  23u  IPv4 0x...  0t0  TCP *:3000 (LISTEN)`
/// 컬럼: COMMAND PID USER FD TYPE DEVICE SIZE/OFF NODE NAME
fn parse_lsof(text: &str) -> Vec<PortProcess> {
    let mut rows = Vec::new();
    for line in text.lines().skip(1) {
        // 헤더 스킵
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 9 {
            continue;
        }
        let command = cols[0].to_string();
        let pid = cols[1].parse::<i32>().unwrap_or(-1);
        let user = cols[2].to_string();
        let protocol = cols[7].to_string(); // NODE 컬럼 = TCP/UDP
        // NAME 컬럼부터 끝까지(주소 + "(LISTEN)")를 주소로 합친다.
        let name = cols[8..].join(" ");
        let port = extract_port(cols[8]);

        // 포트가 0(=`*:*` 같은 주소 미확정 UDP 노이즈)인 행은 건너뛴다.
        if port == 0 {
            continue;
        }

        rows.push(PortProcess {
            pid,
            port,
            protocol,
            process_name: command.clone(),
            command,
            user,
            address: name,
        });
    }
    rows
}

/// pid 목록의 사람이 읽을 만한 메타(cwd + 풀 명령줄)를 배치로 가져온다.
/// 풀 명령줄은 `ps`로 한 번에, cwd는 pid별 `lsof -d cwd`로 (macOS ps엔 cwd가 없음).
#[tauri::command]
pub fn process_meta(pids: Vec<i32>) -> Result<Vec<ProcessMeta>, String> {
    if pids.is_empty() {
        return Ok(vec![]);
    }

    // 1) 풀 명령줄 배치 조회: `ps -p 1,2,3 -o pid=,command=`
    let pid_csv = pids
        .iter()
        .map(|p| p.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let ps_out = Command::new("ps")
        .args(["-p", &pid_csv, "-o", "pid=,command="])
        .output()
        .map_err(|e| format!("ps 실행 실패: {e}"))?;
    let ps_text = String::from_utf8_lossy(&ps_out.stdout);

    let mut result = Vec::new();
    for line in ps_text.lines() {
        let line = line.trim_start();
        // "<pid> <command...>" — 첫 공백 기준으로 분리
        let Some((pid_str, cmd)) = line.split_once(char::is_whitespace) else {
            continue;
        };
        let Ok(pid) = pid_str.trim().parse::<i32>() else {
            continue;
        };
        // 2) cwd는 pid별로 lsof (dev 그룹은 대개 몇 개뿐)
        let cwd = cwd_of(pid).unwrap_or_default();
        result.push(ProcessMeta {
            pid,
            cwd,
            full_command: cmd.trim().to_string(),
        });
    }
    Ok(result)
}

/// pid 하나의 상세 정보(메모리, 풀 명령, cwd)를 뽑는다. 상세 모달용.
#[tauri::command]
pub fn process_detail(pid: i32) -> Result<ProcessDetail, String> {
    // ps로 %mem, rss, 풀 명령을 한 번에.
    let out = Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "%mem=,rss=,command="])
        .output()
        .map_err(|e| format!("ps 실행 실패: {e}"))?;
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text.lines().next().unwrap_or("").trim_start();

    // "%mem rss command..." — 앞 두 토큰이 숫자, 나머지가 명령
    let mut it = line.splitn(3, char::is_whitespace);
    let mem_percent = it.next().and_then(|s| s.parse::<f32>().ok()).unwrap_or(0.0);
    let rss_kb = it.next().and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
    let full_command = it.next().unwrap_or("").trim().to_string();

    Ok(ProcessDetail {
        pid,
        cwd: cwd_of(pid).unwrap_or_default(),
        full_command,
        mem_percent,
        rss_kb,
    })
}

/// pid 하나의 작업 디렉토리를 lsof로 뽑는다. `lsof -a -p PID -d cwd -Fn`
fn cwd_of(pid: i32) -> Option<String> {
    let out = Command::new("lsof")
        .args(["-a", "-p", &pid.to_string(), "-d", "cwd", "-Fn"])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    // -Fn 출력: 'n'으로 시작하는 줄이 경로
    text.lines()
        .find(|l| l.starts_with('n'))
        .map(|l| l[1..].to_string())
}

/// "*:3000", "127.0.0.1:5432", "[::1]:8080" 등에서 마지막 :뒤의 포트 숫자를 뽑는다.
fn extract_port(name: &str) -> u16 {
    name.rsplit(':')
        .next()
        .and_then(|p| {
            p.trim_end_matches(|c: char| !c.is_ascii_digit())
                .parse::<u16>()
                .ok()
        })
        .unwrap_or(0)
}
