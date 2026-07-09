// Portal 설정 — ~/.portal/config.json 에 저장/로드. 얇게: 파일 I/O만.

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
    let dir = home.join(".portal");
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
pub fn save_config(config: Config) -> Result<(), String> {
    let path = config_path()?;
    let json = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())
}
