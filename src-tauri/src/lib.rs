// Portal — 로컬 포트 관제탑 & 런처
// P1: 메뉴바 트레이 아이콘 + Dock 숨김 + 클릭 시 팝오버 창 토글
// 설계 원칙: Rust는 얇게(OS 호출/트레이만), 로직은 프론트(React/TS)에서.

mod config;
mod detect;
mod kill;
mod model;
mod registry;
mod scan;
mod spawn;

use tauri::{
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, PhysicalPosition, Position, Size,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // single-instance는 반드시 첫 번째 플러그인으로 등록.
        // 이미 떠 있는데 또 실행하면, 기존 창을 보여주고 새 프로세스는 종료된다.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.show();
                let _ = win.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // macOS: Dock 아이콘 숨기고 메뉴바 전용 앱으로.
            // (v2에선 config 키가 아니라 런타임 API. setup에서 딱 한 번만 호출 — tauri#9244)
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            // 트레이 아이콘 — 좌클릭 메뉴 없이, 클릭 이벤트로 창을 토글한다.
            // 트레이 전용 22px 아이콘을 파일에서 명시 로드.
            // (default_window_icon은 512px 창용이라 메뉴바에서 안 보이는 경우가 있음.)
            let tray_icon_path = app
                .path()
                .resolve("icons/tray.png", tauri::path::BaseDirectory::Resource)
                .ok()
                .filter(|p| p.exists())
                .or_else(|| {
                    // dev 폴백: 소스 트리의 아이콘 경로
                    let dev = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                        .join("icons/tray.png");
                    dev.exists().then_some(dev)
                });
            let tray_icon = match tray_icon_path {
                Some(p) => tauri::image::Image::from_path(&p)
                    .unwrap_or_else(|_| app.default_window_icon().unwrap().clone()),
                None => app.default_window_icon().unwrap().clone(),
            };

            // 우클릭 메뉴 — 종료 수단.
            use tauri::menu::{MenuBuilder, MenuItemBuilder};
            let quit = MenuItemBuilder::with_id("quit", "Portly 종료").build(app)?;
            let menu = MenuBuilder::new(app).item(&quit).build()?;

            let _tray = TrayIconBuilder::with_id("main")
                .icon(tray_icon)
                // 단색 실루엣 아이콘이라 template 켬 → macOS가 다크/라이트에 맞춰 자동 반전.
                .icon_as_template(true)
                .tooltip("Portly")
                .menu(&menu)
                // 좌클릭은 창 토글, 우클릭만 메뉴가 뜨게.
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| {
                    if event.id() == "quit" {
                        app.exit(0);
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        rect,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(win) = app.get_webview_window("main") {
                            if win.is_visible().unwrap_or(false) {
                                let _ = win.hide();
                            } else {
                                // 트레이 아이콘 위치 기준으로 창을 메뉴바 바로 아래·중앙 정렬.
                                // (tauri::tray::Rect는 private이라 타입명 없이 필드로 추론시킨다.
                                //  좌표 기반 배치는 v2 공식 예제가 없는 버전 민감 영역 — 우선 단순 버전.)
                                if let (Position::Physical(pos), Size::Physical(size)) =
                                    (rect.position, rect.size)
                                {
                                    let win_w =
                                        win.outer_size().map(|s| s.width as f64).unwrap_or(360.0);
                                    let icon_center_x = pos.x as f64 + size.width as f64 / 2.0;
                                    let x = (icon_center_x - win_w / 2.0).max(0.0);
                                    let y = pos.y as f64 + size.height as f64;
                                    let _ = win.set_position(PhysicalPosition::new(x, y));
                                }
                                let _ = win.show();
                                let _ = win.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            scan::scan_ports,
            scan::process_meta,
            scan::process_detail,
            kill::kill_pid,
            config::home_dir,
            config::load_config,
            config::save_config,
            detect::detect_command,
            registry::list_projects,
            registry::save_project,
            registry::delete_project,
            spawn::start_app,
            spawn::tail_log
        ])
        .run(tauri::generate_context!())
        .expect("error while running Portly");
}
