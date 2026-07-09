// 등록 프로젝트 — ~/.portly/registry.json 에 저장. "폴더+명령"을 등록하고 ▶로 실행한다.

use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,      // 프론트가 생성(uuid/slug)
    pub name: String,    // 표시 이름 (예: "yolo-api")
    pub cwd: String,     // 실행 디렉토리(절대경로)
    pub command: String, // 실행 명령 (예: "pnpm dev")
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Registry {
    #[serde(default)]
    pub projects: Vec<Project>,
}

fn registry_path() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("홈 디렉토리를 찾을 수 없음")?;
    let dir = home.join(".portly");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("registry.json"))
}

fn read_registry() -> Result<Registry, String> {
    let path = registry_path()?;
    if !path.exists() {
        return Ok(Registry::default());
    }
    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

fn write_registry(reg: &Registry) -> Result<(), String> {
    let path = registry_path()?;
    let json = serde_json::to_string_pretty(reg).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_projects() -> Result<Vec<Project>, String> {
    Ok(read_registry()?.projects)
}

/// 프로젝트 등록/수정. 검증: cwd는 존재하는 절대경로 폴더, 명령은 비어있지 않을 것.
#[tauri::command]
pub fn save_project(project: Project) -> Result<Vec<Project>, String> {
    use std::path::Path;

    let name = project.name.trim();
    let cwd = project.cwd.trim();
    let command = project.command.trim();

    if name.is_empty() {
        return Err("이름을 입력하세요.".into());
    }
    if command.is_empty() {
        return Err("실행 명령을 입력하세요.".into());
    }
    let p = Path::new(cwd);
    if !p.is_absolute() {
        return Err(format!("절대경로만 지정할 수 있습니다: {cwd}"));
    }
    if !p.is_dir() {
        return Err(format!("존재하는 폴더가 아닙니다: {cwd}"));
    }

    let clean = Project {
        id: project.id.clone(),
        name: name.to_string(),
        cwd: cwd.to_string(),
        command: command.to_string(),
    };

    let mut reg = read_registry()?;
    if let Some(existing) = reg.projects.iter_mut().find(|x| x.id == clean.id) {
        *existing = clean; // id 있으면 갱신
    } else {
        reg.projects.push(clean); // 없으면 추가
    }
    write_registry(&reg)?;
    Ok(reg.projects)
}

#[tauri::command]
pub fn delete_project(id: String) -> Result<Vec<Project>, String> {
    let mut reg = read_registry()?;
    reg.projects.retain(|x| x.id != id);
    write_registry(&reg)?;
    Ok(reg.projects)
}

/// id로 프로젝트 조회 (spawn에서 사용)
pub fn get_project(id: &str) -> Result<Option<Project>, String> {
    Ok(read_registry()?.projects.into_iter().find(|x| x.id == id))
}
