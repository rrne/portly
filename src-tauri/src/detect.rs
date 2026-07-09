// 폴더에서 패키지 매니저 + 실행 스크립트를 감지해 추천 명령을 만든다. 조회성(부작용 없음).

use serde::Serialize;
use std::{fs, path::Path};

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Detected {
    pub manager: String,          // "pnpm" | "yarn" | "npm" | "bun" | ""
    pub scripts: Vec<String>,     // package.json의 scripts 키 목록
    pub suggested_command: String, // 예: "pnpm dev"
}

/// 폴더를 보고 패키지 매니저와 추천 실행 명령을 감지한다.
#[tauri::command]
pub fn detect_command(cwd: String) -> Result<Detected, String> {
    let dir = Path::new(&cwd);
    if !dir.is_dir() {
        return Err(format!("폴더가 아닙니다: {cwd}"));
    }

    // 1) lock 파일로 패키지 매니저 판별
    let manager = if dir.join("pnpm-lock.yaml").exists() {
        "pnpm"
    } else if dir.join("bun.lockb").exists() || dir.join("bun.lock").exists() {
        "bun"
    } else if dir.join("yarn.lock").exists() {
        "yarn"
    } else if dir.join("package-lock.json").exists() {
        "npm"
    } else if dir.join("package.json").exists() {
        "npm" // lock 없지만 package.json 있으면 npm 기본
    } else {
        ""
    };

    // 2) package.json의 scripts 키 읽기
    let scripts = read_scripts(dir);

    // 3) 실행 스크립트 우선순위: dev > start > serve > (첫 스크립트)
    let script = ["dev", "start", "serve"]
        .iter()
        .find(|s| scripts.iter().any(|k| k == **s))
        .map(|s| s.to_string())
        .or_else(|| scripts.first().cloned());

    // 4) 매니저 + 스크립트 → 명령. npm/bun은 run 필요, pnpm/yarn은 생략 가능.
    let suggested_command = match (manager, script.as_deref()) {
        ("", _) => String::new(),
        (m, Some(s)) => match m {
            "npm" => format!("npm run {s}"),
            "bun" => format!("bun run {s}"),
            _ => format!("{m} {s}"), // pnpm dev, yarn dev
        },
        (m, None) => format!("{m} dev"), // scripts 못 읽어도 관례상 dev 제안
    };

    Ok(Detected {
        manager: manager.to_string(),
        scripts,
        suggested_command,
    })
}

/// package.json의 "scripts" 객체 키만 뽑는다. (파싱 실패해도 빈 목록)
fn read_scripts(dir: &Path) -> Vec<String> {
    let path = dir.join("package.json");
    let Ok(text) = fs::read_to_string(&path) else {
        return vec![];
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else {
        return vec![];
    };
    json.get("scripts")
        .and_then(|s| s.as_object())
        .map(|obj| obj.keys().cloned().collect())
        .unwrap_or_default()
}
