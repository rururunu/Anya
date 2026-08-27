mod adapters;
mod app_state;
mod commands;
mod core;
mod models;
mod runtime;
mod services;

pub use core::chat::eval_harness;

use std::time::{SystemTime, UNIX_EPOCH};

use rdev::{listen, Event, EventType};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, RunEvent,
};
use windows::Win32::Foundation::POINT;
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

use app_state::AppState;
use commands::{
    app, ask, chat, diff, gemini, harness, icons, mcp, permission, remote, semantic, settings,
    skills, token_usage, updater, window, workspace,
};
use services::app_lifecycle;
use services::overlay_native::clear_minimize_pending;
use services::settings_store::{
    apply_runtime_settings, load_settings, register_enabled_mcp_tools, SettingsState,
};
use services::window::{
    cleanup_overlay_state, configure_overlay_window, handle_overlay_focused, is_overlay_label,
    mark_blur_guard, should_keep_overlay_visible, show_settings_window, show_workbench_window,
    toggle_overlay,
};

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn cursor_pos() -> Option<(i32, i32)> {
    let mut pt = POINT::default();
    unsafe { GetCursorPos(&mut pt).ok().map(|_| (pt.x, pt.y)) }
}

fn trigger_overlay(app: &AppHandle) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        crate::core::context::provider::force_release_modifiers_for_capture();
        toggle_overlay(&handle, cursor_pos());
    });
}

fn start_hotkey_listener(app: AppHandle) {
    std::thread::spawn(move || {
        let mut primary_detector = crate::services::hotkey::DoubleModifierDetector::default();
        let mut secondary = crate::services::hotkey::SecondaryHotkeyDetector::default();
        let callback = move |event: Event| {
            let primary_enabled = crate::services::hotkey::primary_hotkey_enabled();
            let secondary_enabled = crate::services::hotkey::secondary_hotkey_enabled();
            let primary = crate::services::hotkey::current_primary_hotkey();
            let chord = crate::services::hotkey::current_secondary_hotkey();
            let triggered = match event.event_type {
                EventType::KeyPress(key) => {
                    if primary_enabled {
                        primary_detector.key_press(key, now_millis(), primary);
                    } else {
                        primary_detector =
                            crate::services::hotkey::DoubleModifierDetector::default();
                    }
                    if secondary_enabled {
                        secondary.key_press(key, &chord);
                    } else {
                        secondary = crate::services::hotkey::SecondaryHotkeyDetector::default();
                    }
                    false
                }
                EventType::KeyRelease(key) => {
                    let primary_hit =
                        primary_enabled && primary_detector.key_release(key, now_millis(), primary);
                    let chord_hit = secondary_enabled && secondary.key_release(key, &chord);
                    primary_hit || chord_hit
                }
                _ => false,
            };

            if triggered {
                trigger_overlay(&app);
            }
        };

        if let Err(error) = listen(callback) {
            eprintln!("failed to listen for global shortcuts: {error:?}");
        }
    });
}

fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let workbench = MenuItem::with_id(app, "workbench", "Open Workbench", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "设置", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&workbench, &settings, &quit])?;
    let icon = app
        .default_window_icon()
        .cloned()
        .expect("missing application icon");

    TrayIconBuilder::new()
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip(app.package_info().name.clone())
        .on_menu_event(|app, event| match event.id.as_ref() {
            "workbench" => show_workbench_window(app),
            "settings" => show_settings_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                show_workbench_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let handle = app.clone();
            let _ = app.run_on_main_thread(move || {
                show_workbench_window(&handle);
            });
        }))
        .setup(|app| {
            let config_dir = app
                .path()
                .app_config_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."));
            crate::core::chat::telemetry::init_logging(&config_dir);
            // Stable root for mcp-remote OAuth tokens (package still appends mcp-remote-{ver}/).
            crate::core::mcp::init_mcp_remote_config_dir(config_dir.join("mcp-auth"));
            let settings = load_settings(app.handle());
            apply_runtime_settings(&settings);
            crate::services::pin_badge::start(app.handle().clone());
            app.manage(SettingsState::new(settings.clone()));
            app.manage(AppState::new(app.handle().clone()));
            crate::core::context::providers::local_api::start_server(app.handle().clone());
            register_enabled_mcp_tools(app.handle());
            if settings.semantic_search_enabled {
                crate::core::ai::embed::SemanticSearchEngine::enable(
                    crate::core::ai::embed::EmbeddingConfig {
                        backend: settings.semantic_search_backend,
                        local_model: settings.semantic_search_model,
                        api_base_url: settings.semantic_search_api_base_url.clone(),
                        api_key: settings.semantic_search_api_key.clone(),
                        api_model: settings.semantic_search_api_model.clone(),
                    },
                    crate::commands::semantic::model_cache_dir(app.handle()),
                );
            }
            setup_tray(app)?;
            if let Some(window) = app.get_webview_window("overlay") {
                configure_overlay_window(&window);
            }
            start_hotkey_listener(app.handle().clone());
            show_workbench_window(app.handle());
            crate::core::remote::restore_gateway_if_enabled(app.handle());
            Ok(())
        })
        .on_window_event(|window, event| {
            let label = window.label().to_string();

            if label == "workbench" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    // During Windows MSI updates the installer / updater may request
                    // close; blocking it deadlocks the install after download finishes.
                    if !app_lifecycle::allow_exit() {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                    return;
                }
                if matches!(event, tauri::WindowEvent::Resized(_)) {
                    crate::services::workbench_glass::sync_covering(window.app_handle());
                    return;
                }
            }

            if let tauri::WindowEvent::Destroyed = event {
                if is_overlay_label(&label) {
                    cleanup_overlay_state(&label);
                }
                return;
            }

            let tauri::WindowEvent::Focused(focused) = event else {
                return;
            };

            // 某个 overlay 获得焦点时，设置 blur guard，防止因窗口间焦点切换
            // 导致另一个 overlay 被错误地隐藏（如点击另一个 overlay 的关闭按钮）
            if *focused && is_overlay_label(&label) {
                mark_blur_guard();
                handle_overlay_focused(window.app_handle(), &label);
                return;
            }

            if !*focused && is_overlay_label(&label) {
                clear_minimize_pending();
            }

            if *focused
                || !window.is_visible().unwrap_or(false)
                || window.is_minimized().unwrap_or(false)
            {
                return;
            }

            if is_overlay_label(&label) {
                if should_keep_overlay_visible(&label) {
                    return;
                }
                use crate::services::overlay_native::hide_overlay_without_flash;
                if let Some(webview) = window.app_handle().get_webview_window(&label) {
                    if hide_overlay_without_flash(&webview).is_err() {
                        let _ = webview.hide();
                    }
                } else {
                    let _ = window.hide();
                }
                let _ = window.emit_to(&label, "overlay-hidden", ());
            }
        })
        .invoke_handler(tauri::generate_handler![
            window::open_settings,
            window::open_session_in_overlay,
            window::open_session_in_workbench,
            window::show_interaction_notification,
            window::dismiss_interaction_notification,
            window::set_window_session_view,
            window::hide_overlay_window,
            window::minimize_overlay_window,
            window::close_overlay_window,
            window::exit_app,
            window::set_overlay_chat_mode_command,
            window::set_overlay_popup_open_command,
            window::take_overlay_context,
            window::open_image_preview,
            window::get_preview_image,
            settings::get_app_settings,
            settings::set_app_settings,
            semantic::get_semantic_search_status,
            semantic::set_semantic_search,
            semantic::test_semantic_search_api,
            semantic::fetch_semantic_search_models,
            gemini::gemini_auth_status,
            gemini::gemini_oauth_login,
            gemini::gemini_oauth_cancel_login,
            gemini::gemini_oauth_logout,
            gemini::gemini_import_client_secrets,
            skills::list_skills,
            skills::install_skill,
            skills::install_skill_markdown,
            skills::write_skill_meta,
            skills::uninstall_skill,
            skills::get_skills_dir,
            skills::open_skills_dir,
            mcp::get_mcp_runtime_support,
            mcp::list_mcp_server_statuses,
            mcp::connect_mcp_server,
            mcp::reauthenticate_mcp_server,
            icons::cache_install_icon,
            icons::lookup_install_icon,
            icons::lookup_install_icons,
            icons::clear_install_icon,
            app::get_app_info,
            app::webview_gpu_disabled,
            app::relaunch_app,
            updater::download_and_install_update,
            diff::build_code_diff,
            chat::chat,
            chat::chat_cancel,
            chat::agent_debug_snapshot,
            chat::chat_history,
            chat::list_chat_sessions,
            chat::list_archived_chat_sessions,
            chat::list_chat_models,
            chat::list_custom_provider_models,
            chat::get_context_usage,
            chat::get_environment_context,
            chat::delete_chat_session,
            chat::branch_chat_session,
            chat::set_chat_session_archived,
            chat::set_chat_session_workspace,
            chat::clear_all_chat_sessions,
            token_usage::get_token_usage_report,
            workspace::list_workspaces,
            workspace::list_archived_workspaces,
            workspace::get_current_workspace,
            workspace::list_workspace_files,
            workspace::create_workspace,
            workspace::switch_workspace,
            workspace::clear_current_workspace,
            workspace::delete_workspace,
            workspace::update_workspace,
            workspace::open_workspace_folder,
            workspace::open_workspace_in_terminal,
            workspace::open_in_default_app,
            workspace::reveal_in_explorer,
            workspace::set_workspace_pinned,
            workspace::set_workspace_archived,
            workspace::reorder_workspaces,
            ask::respond_ask_user,
            permission::respond_path_permission,
            harness::respond_tool_approval,
            harness::set_plan_mode,
            harness::get_plan_mode,
            harness::list_checkpoints,
            harness::rewind_session,
            remote::remote_gateway_status,
            remote::remote_gateway_start,
            remote::remote_gateway_stop,
            remote::remote_create_pairing,
            remote::remote_list_devices,
            remote::remote_revoke_device,
            remote::remote_get_tunnel_prefs,
            remote::remote_set_tunnel_prefs,
            remote::remote_sync_session_compose,
            remote::remote_get_session_compose,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app_handle, event| {
            if let RunEvent::ExitRequested { api, code, .. } = event {
                // Tray-style: ignore window-driven exit unless an update is installing
                // (or the process requested an explicit exit code).
                if code.is_none() && !app_lifecycle::allow_exit() {
                    api.prevent_exit();
                }
            }
        });
}

pub fn configure_prestart_webview() {
    services::settings_store::configure_prestart_webview();
}
