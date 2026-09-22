/// Tauri commands — symbols, CSV import, backfill, history, live, logs, settings, system

pub mod auth;
pub mod backfill;
pub mod history;
pub mod live;
pub mod logs;
pub mod settings;
pub mod symbols;
pub mod system;

/// Register all commands with the Tauri builder.
/// Call this in `lib.rs` → `tauri::Builder::invoke_handler`.
#[macro_export]
macro_rules! all_handlers {
    () => {
        tauri::generate_handler![
            // Auth
            crate::commands::auth::save_broker_credentials,
            crate::commands::auth::get_session_status,
            crate::commands::auth::trigger_authentication,
            // Symbols
            crate::commands::symbols::list_symbols,
            crate::commands::symbols::add_symbol,
            crate::commands::symbols::remove_symbol,
            crate::commands::symbols::enable_symbol,
            crate::commands::symbols::disable_symbol,
            crate::commands::symbols::import_csv,
            crate::commands::symbols::search_symbols,
            // Backfill
            crate::commands::backfill::get_backfill_status,
            crate::commands::backfill::trigger_backfill,
            crate::commands::backfill::cancel_backfill,
            // History
            crate::commands::history::get_history,
            crate::commands::history::get_bar_count,
            crate::commands::history::get_gaps,
            crate::commands::history::repair_gaps,
            // Live
            crate::commands::live::get_live_status,
            crate::commands::live::get_tick_stats,
            // Logs
            crate::commands::logs::get_logs,
            crate::commands::logs::clear_logs,
            // Settings
            crate::commands::settings::get_settings,
            crate::commands::settings::set_setting,
            crate::commands::settings::get_all_settings,
            // System
            crate::commands::system::get_system_status,
            crate::commands::system::set_start_with_windows,
            crate::commands::system::get_ipc_port,
        ]
    };
}
