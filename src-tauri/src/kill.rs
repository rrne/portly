// 프로세스 종료 — kill 시스템 명령을 감싼다. 판단(뭘 죽일지)은 프론트가 한다.

use std::process::Command;

/// pid를 종료한다. force=false면 SIGTERM(정상 종료), true면 SIGKILL(강제).
#[tauri::command]
pub fn kill_pid(pid: i32, force: bool) -> Result<(), String> {
    let signal = if force { "-9" } else { "-15" }; // SIGKILL / SIGTERM
    let out = Command::new("kill")
        .args([signal, &pid.to_string()])
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
