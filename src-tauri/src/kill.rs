// 프로세스 종료 — kill 시스템 명령을 감싼다. 판단(뭘 죽일지)은 프론트가 한다.

use std::process::Command;

/// pid를 종료한다. force=false면 SIGTERM(정상 종료), true면 SIGKILL(강제).
/// expected_command: 스캔 때 본 프로세스 이름(command). 넘기면 kill 직전에
/// 지금 그 pid의 이름과 대조해, pid가 재활용된 경우 엉뚱한 프로세스를 죽이지 않는다.
#[tauri::command]
pub fn kill_pid(pid: i32, force: bool, expected_command: Option<String>) -> Result<(), String> {
    // ── 신뢰 경계 방어: pid를 무비판 실행하면 안 된다. ──
    // kill은 음수 인자를 "프로세스 그룹"으로 해석한다. 예: `kill -9 -1`은
    // 내 권한의 모든 프로세스를 죽여 세션을 붕괴시킨다.
    // pid 0(=호출자 프로세스 그룹 전체), 1(=launchd, 시스템 붕괴)도 금지.
    // 파싱 실패로 -1이 흘러들어오거나(scan의 unwrap_or(-1)) 프론트 버그를 막는 최후 방어선.
    if pid <= 1 {
        return Err(format!(
            "종료 거부: PID {pid}는 유효하지 않거나 시스템 필수 프로세스입니다."
        ));
    }

    // ── PID 재사용 방어 ──
    // 스캔~클릭 사이에 그 pid가 죽고 다른 프로세스가 그 번호를 차지했을 수 있다.
    // 지금 pid의 이름을 다시 읽어, 스캔 때 본 이름과 다르면 종료를 거부한다.
    if let Some(expected) = expected_command.as_deref() {
        match current_command(pid) {
            None => {
                return Err(format!("PID {pid}는 이미 종료되었습니다."));
            }
            Some(now) if now != expected => {
                return Err(format!(
                    "종료 거부: PID {pid}가 그새 다른 프로세스({now})로 바뀌었습니다. 새로고침 후 다시 시도하세요."
                ));
            }
            _ => {}
        }
    }

    let signal = if force { "-9" } else { "-15" }; // SIGKILL / SIGTERM
    // "--"로 옵션 종료를 명시해, pid가 어떤 경우에도 플래그로 해석되지 않게 한다.
    let out = Command::new("kill")
        .args([signal, "--", &pid.to_string()])
        .output()
        .map_err(|e| format!("kill 실행 실패: {e}"))?;

    if out.status.success() {
        return Ok(());
    }

    let err = String::from_utf8_lossy(&out.stderr);
    Err(if err.contains("Operation not permitted") {
        format!("권한 없음: PID {pid}는 시스템/타 사용자 프로세스라 종료할 수 없습니다.")
    } else if err.contains("No such process") {
        format!("PID {pid}는 이미 종료되었습니다.")
    } else {
        format!("종료 실패: {}", err.trim())
    })
}

/// pid의 현재 프로세스명(lsof COMMAND와 같은 comm). 없으면 None(=이미 죽음).
/// scan.rs의 lsof COMMAND는 `ps -o comm`의 basename과 같은 값이다.
fn current_command(pid: i32) -> Option<String> {
    let out = Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "comm="])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text.trim();
    if line.is_empty() {
        return None;
    }
    // ps comm=은 풀 경로(/usr/bin/node)를 줄 수 있으니 basename만.
    Some(
        line.rsplit('/')
            .next()
            .unwrap_or(line)
            .to_string(),
    )
}
