// 프로세스 종료 — kill 시스템 명령을 감싼다. 판단(뭘 죽일지)은 프론트가 한다.

use std::process::Command;

/// pid를 종료한다. force=false면 SIGTERM(정상 종료), true면 SIGKILL(강제).
#[tauri::command]
pub fn kill_pid(pid: i32, force: bool) -> Result<(), String> {
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
