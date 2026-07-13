// Portly 설정 — ~/.portly/config.json 에 저장/로드. 얇게: 파일 I/O만.

use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    /// "내 프로젝트"로 볼 루트 폴더 목록. 비어 있으면 홈(~) 하위 전체를 내 것으로 본다.
    #[serde(default)]
    pub project_roots: Vec<String>,
}

fn config_path() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("홈 디렉토리를 찾을 수 없음")?;
    let dir = home.join(".portly");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("config.json"))
}

/// 현재 사용자의 홈 경로. 프론트가 "홈 하위" 필터 기준으로 쓴다.
#[tauri::command]
pub fn home_dir() -> Result<String, String> {
    dirs::home_dir()
        .map(|p| p.to_string_lossy().to_string())
        .ok_or_else(|| "홈 디렉토리를 찾을 수 없음".to_string())
}

/// 실행 위치 진단. 메뉴바 앱이 DMG(`/Volumes/…`)나 App Translocation 경로에서
/// 실행되면 트레이가 죽는 등 오작동하므로, 프론트가 경고를 띄우도록 상태를 알려준다.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallStatus {
    pub exe_path: String,
    /// "ok" | "dmg" | "translocated" — ok가 아니면 프론트가 이동 안내를 띄운다.
    pub location: String,
}

#[tauri::command]
pub fn install_status() -> InstallStatus {
    let exe = std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    // 디버그 빌드에서만: 배너 UI 검증용 강제 오버라이드(PORTLY_FORCE_LOCATION=dmg 등).
    #[cfg(debug_assertions)]
    if let Ok(forced) = std::env::var("PORTLY_FORCE_LOCATION") {
        return InstallStatus {
            exe_path: exe,
            location: forced,
        };
    }

    // DMG 마운트본 직접 실행: 읽기전용이라 config 저장 실패 + 언마운트 시 크래시.
    let location = if exe.starts_with("/Volumes/") {
        "dmg"
    // Gatekeeper App Translocation: quarantine된 앱을 무작위 읽기전용 경로로 옮겨 실행.
    // /private/var/folders/…/AppTranslocation/…/d/… 형태.
    } else if exe.contains("/AppTranslocation/") {
        "translocated"
    } else {
        "ok"
    }
    .to_string();

    InstallStatus {
        exe_path: exe,
        location,
    }
}

#[tauri::command]
pub fn load_config() -> Result<Config, String> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(Config::default());
    }
    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_config(mut config: Config) -> Result<(), String> {
    // ── 신뢰 경계 방어: project_roots를 무검증 저장하면 안 된다. ──
    // 잘못된 값(빈 문자열, "/", 상대경로 등)이 들어가면 필터가 무력화되거나
    // ("/"를 넣으면 모든 프로세스가 "내 dev"로 분류돼 노이즈 폭발) 오작동한다.
    config.project_roots = sanitize_roots(config.project_roots)?;

    let path = config_path()?;
    let json = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())
}

/// project_roots 검증: 존재하는 절대경로 디렉토리만, 위험한 최상위 루트는 거부, 중복 제거.
fn sanitize_roots(roots: Vec<String>) -> Result<Vec<String>, String> {
    use std::collections::BTreeSet;
    use std::path::Path;

    // 필터를 무의미하게 만드는(=사실상 전체) 경로는 거부한다.
    const FORBIDDEN: &[&str] = &["/", "/Users", "/System", "/Library", "/private"];

    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for raw in roots {
        let trimmed = raw.trim().trim_end_matches('/');
        if trimmed.is_empty() {
            continue; // 빈 값 무시
        }
        let p = Path::new(trimmed);
        if !p.is_absolute() {
            return Err(format!("절대경로만 지정할 수 있습니다: {raw}"));
        }
        if FORBIDDEN.contains(&trimmed) {
            return Err(format!(
                "너무 넓은 경로는 지정할 수 없습니다(필터가 무의미해짐): {raw}"
            ));
        }
        if !p.is_dir() {
            return Err(format!("존재하는 폴더가 아닙니다: {raw}"));
        }
        // 중복 제거(정규화된 경로 기준)
        if seen.insert(trimmed.to_string()) {
            out.push(trimmed.to_string());
        }
    }
    Ok(out)
}
