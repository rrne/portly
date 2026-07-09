// 등록 프로젝트 실행 — detached 로그인 셸로 spawn. 부작용 함수라 방어를 둔다.

use crate::registry::get_project;
use std::{fs, path::PathBuf, process::Command};

/// 로그 디렉토리 ~/.portly/logs/
fn log_dir() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("홈 디렉토리를 찾을 수 없음")?;
    let dir = home.join(".portly").join("logs");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

/// 등록 프로젝트 id를 실행한다.
/// - 로그인 셸(`$SHELL -lc`)로 감싸 nvm/PATH/.nvmrc를 살린다.
/// - setsid로 detach → Portly를 꺼도 서버는 계속 산다.
/// - stdout/stderr는 ~/.portly/logs/<id>.log 로 (detach라 파이프 못 씀).
#[tauri::command]
pub fn start_app(id: String) -> Result<u32, String> {
    let project = get_project(&id)?.ok_or("등록되지 않은 프로젝트입니다.")?;

    // cwd 재검증(등록 후 폴더가 사라졌을 수 있음)
    let cwd = std::path::Path::new(&project.cwd);
    if !cwd.is_dir() {
        return Err(format!("폴더가 없습니다: {}", project.cwd));
    }

    // 사용자 로그인 셸 (없으면 zsh). -l: 로그인(.zshrc 로드), -c: 명령 실행.
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());

    // 로그 파일 (append)
    let log_path = log_dir()?.join(format!("{id}.log"));
    let stdout = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(|e| format!("로그 파일 열기 실패: {e}"))?;
    let stderr = stdout
        .try_clone()
        .map_err(|e| format!("로그 파일 복제 실패: {e}"))?;

    let mut cmd = Command::new(&shell);
    cmd.arg("-lc")
        .arg(&project.command) // 명령은 인자로 전달(셸 인젝션 최소화: 별도 arg)
        .current_dir(&project.cwd)
        .stdin(std::process::Stdio::null())
        .stdout(stdout)
        .stderr(stderr);

    // detach: 새 세션 리더로 만들어 Portly(부모) 종료와 무관하게 산다.
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        unsafe {
            cmd.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }
    }

    let child = cmd.spawn().map_err(|e| format!("실행 실패: {e}"))?;
    let pid = child.id();
    // Child를 drop해도 Rust는 kill 안 하지만, 명시적으로 잊어 소유권을 뗀다.
    std::mem::forget(child);
    Ok(pid)
}

/// 로그 tail — 실행한 프로젝트의 최근 로그 N줄.
#[tauri::command]
pub fn tail_log(id: String, lines: usize) -> Result<String, String> {
    let log_path = log_dir()?.join(format!("{id}.log"));
    if !log_path.exists() {
        return Ok(String::new());
    }
    let text = fs::read_to_string(&log_path).map_err(|e| e.to_string())?;
    let tail: Vec<&str> = text.lines().rev().take(lines).collect();
    Ok(tail.into_iter().rev().collect::<Vec<_>>().join("\n"))
}
