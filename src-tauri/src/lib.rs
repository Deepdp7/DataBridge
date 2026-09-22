/// DataBridge — Tauri application core
///
/// This is the main library entry point. It wires together:
/// - AppState with all subsystems
/// - Database initialization
/// - Broker registry with MockBrokerAdapter
/// - All Tauri commands
/// - System tray
/// - IPC server (spawned as background task)

use std::sync::Arc;

use tauri::{Manager, State};
use tokio::sync::RwLock;
use tracing::info;
use tracing_subscriber::{EnvFilter, FmtSubscriber};

pub mod aggregation;
pub mod broker;
pub mod commands;
pub mod error;
pub mod historical;
pub mod ipc;
pub mod models;
pub mod recovery;
pub mod session;
pub mod storage;
pub mod websocket;

pub use error::{AppError, Result};
pub use models::*;

use broker::{BrokerRegistry, MockBrokerAdapter};
use storage::{AppStorage, credentials::{build_credential_store, CredentialStore}};

/// Global application state — injected into all Tauri commands via `State<AppState>`.
pub struct AppState {
    pub storage: AppStorage,
    pub broker_registry: Arc<RwLock<BrokerRegistry>>,
    pub credential_store: Arc<dyn CredentialStore>,
    pub active_broker_id: String,
    pub live_engine: Arc<tokio::sync::Mutex<Option<Arc<crate::websocket::LiveTickEngine>>>>,
}

/// Tauri application entry point.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize tracing subscriber
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_env("DATABRIDGE_LOG")
                .unwrap_or_else(|_| EnvFilter::new("databridge=info,warn")),
        )
        .with_target(true)
        .compact()
        .init();

    info!("DataBridge starting up");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let app_handle = app.handle().clone();

            // Resolve DB path in the app's data directory
            let data_dir = app.path().app_data_dir()
                .expect("Failed to resolve app data dir");
            std::fs::create_dir_all(&data_dir)
                .expect("Failed to create app data directory");
            let db_path = data_dir.join("databridge.db");
            let db_path_str = db_path.to_string_lossy().to_string();

            info!(db_path = %db_path_str, "Database path resolved");

            // Initialize storage asynchronously via the Tauri async runtime
            tauri::async_runtime::block_on(async move {
                // Initialize SQLite storage
                let storage = AppStorage::init(&db_path_str)
                    .await
                    .expect("Failed to initialize database");

                // Build broker registry
                let mut registry = BrokerRegistry::new();
                registry.register(Arc::new(crate::broker::mock::MockBrokerAdapter::new()));
                registry.register(Arc::new(crate::broker::fyers::FyersAdapter::new()));
                let registry = Arc::new(RwLock::new(registry));

                // Build credential store (Windows Credential Manager)
                let credential_store = build_credential_store();

                // Read active broker from settings
                let active_broker_id = storage.settings
                    .get("active_broker_id").await
                    .ok().flatten()
                    .unwrap_or_else(|| "mock".into());

                let app_state = AppState {
                    storage,
                    broker_registry: registry,
                    credential_store,
                    active_broker_id,
                    live_engine: Arc::new(tokio::sync::Mutex::new(None)),
                };

                // Register Tauri state
                app_handle.manage(app_state);

                // Start the local IPC server in the background
                let ipc_storage = app_handle.state::<AppState>().storage.clone();
                let ipc_port = ipc_storage.settings
                    .get("ipc_port").await
                    .ok().flatten()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(ipc::DEFAULT_IPC_PORT);

                let ipc_state = ipc::IpcState {
                    storage: ipc_storage,
                    ipc_port,
                };

                tokio::spawn(async move {
                    if let Err(e) = ipc::start_ipc_server(ipc_state, ipc_port).await {
                        tracing::error!("IPC server error: {}", e);
                    }
                });

                info!("DataBridge initialized successfully");
            });

            // Setup system tray
            setup_tray(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Auth
            commands::auth::save_broker_credentials,
            commands::auth::get_session_status,
            commands::auth::trigger_authentication,
            // Symbols
            commands::symbols::list_symbols,
            commands::symbols::add_symbol,
            commands::symbols::remove_symbol,
            commands::symbols::enable_symbol,
            commands::symbols::disable_symbol,
            commands::symbols::import_csv,
            commands::symbols::search_symbols,
            // Backfill
            commands::backfill::get_backfill_status,
            commands::backfill::trigger_backfill,
            commands::backfill::cancel_backfill,
            // History
            commands::history::get_history,
            commands::history::get_bar_count,
            commands::history::get_gaps,
            commands::history::repair_gaps,
            // Live
            commands::live::get_live_status,
            commands::live::get_tick_stats,
            commands::live::start_live_feed,
            commands::live::stop_live_feed,
            // Logs
            commands::logs::get_logs,
            commands::logs::clear_logs,
            // Settings
            commands::settings::get_settings,
            commands::settings::set_setting,
            commands::settings::get_all_settings,
            // System
            commands::system::get_system_status,
            commands::system::set_start_with_windows,
            commands::system::get_ipc_port,
            commands::system::run_health_check,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // Minimize to tray on close instead of quitting
                window.hide().unwrap();
                api.prevent_close();
            }
        })
        .run(tauri::generate_context!())
        .expect("Error while running DataBridge application");
}

fn setup_tray(app: &mut tauri::App) -> tauri::Result<()> {
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
    use tauri::menu::{Menu, MenuItem};

    let show = MenuItem::with_id(app, "show", "Show DataBridge", true, None::<&str>)?;
    let hide = MenuItem::with_id(app, "hide", "Hide DataBridge", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Exit", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&show, &hide, &quit])?;

    let _tray = TrayIconBuilder::new()
        .menu(&menu)
        .tooltip("DataBridge — Market Data Bridge")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "hide" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;

    Ok(())
}
