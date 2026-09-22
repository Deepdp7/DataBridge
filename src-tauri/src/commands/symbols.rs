/// Symbol commands — list, add, remove, enable, disable, import CSV

use tauri::State;

use crate::models::symbol::{CsvSymbolRow, ImportError, ImportSummary, SymbolStatus};
use crate::storage::repository::{NewSymbol, NewSymbolMapping, SymbolFilter};
use crate::AppState;

// ──────────────────────────────────────────────────────────────
// List / search
// ──────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn list_symbols(
    state: State<'_, AppState>,
    exchange: Option<String>,
    status_filter: Option<String>,
    search: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<crate::models::Symbol>, String> {
    let status = status_filter.map(|s| {
        if s == "enabled" { SymbolStatus::Enabled } else { SymbolStatus::Disabled }
    });

    state.storage.symbols
        .list_symbols(&SymbolFilter {
            exchange,
            status,
            search,
            limit,
            offset,
        })
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn search_symbols(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<crate::models::Symbol>, String> {
    state.storage.symbols
        .list_symbols(&SymbolFilter {
            search: Some(query),
            limit: Some(50),
            ..Default::default()
        })
        .await
        .map_err(|e| e.to_string())
}

// ──────────────────────────────────────────────────────────────
// Add / remove
// ──────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn add_symbol(
    state: State<'_, AppState>,
    symbol: String,
    exchange: String,
    token: String,
    instrument_type: Option<String>,
) -> Result<i64, String> {
    if symbol.trim().is_empty() {
        return Err("Symbol cannot be empty".into());
    }
    if exchange.trim().is_empty() {
        return Err("Exchange cannot be empty".into());
    }
    if token.trim().is_empty() {
        return Err("Token cannot be empty".into());
    }

    let symbol_id = state.storage.symbols
        .upsert_symbol(&NewSymbol {
            symbol: symbol.trim().to_uppercase(),
            exchange: exchange.trim().to_uppercase(),
            instrument_type: instrument_type.unwrap_or_else(|| "EQ".into()),
            expiry: None,
            strike: None,
            option_type: None,
            tick_size: None,
            lot_size: None,
            status: "enabled".into(),
        })
        .await
        .map_err(|e| e.to_string())?;

    // Record broker token mapping (using active broker)
    let broker = state.storage.brokers
        .get_broker_by_key(&state.active_broker_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Active broker not found".to_string())?;

    state.storage.symbols
        .upsert_symbol_mapping(&NewSymbolMapping {
            symbol_id,
            broker_id: broker.id,
            broker_symbol: symbol.trim().to_uppercase(),
            broker_token: token,
        })
        .await
        .map_err(|e| e.to_string())?;

    Ok(symbol_id)
}

#[tauri::command]
pub async fn remove_symbol(
    state: State<'_, AppState>,
    symbol_id: i64,
) -> Result<(), String> {
    state.storage.symbols
        .delete_symbol(symbol_id)
        .await
        .map_err(|e| e.to_string())
}

// ──────────────────────────────────────────────────────────────
// Enable / disable
// ──────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn enable_symbol(
    state: State<'_, AppState>,
    symbol_id: i64,
) -> Result<(), String> {
    state.storage.symbols
        .set_symbol_status(symbol_id, SymbolStatus::Enabled)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn disable_symbol(
    state: State<'_, AppState>,
    symbol_id: i64,
) -> Result<(), String> {
    state.storage.symbols
        .set_symbol_status(symbol_id, SymbolStatus::Disabled)
        .await
        .map_err(|e| e.to_string())
}

// ──────────────────────────────────────────────────────────────
// CSV import — PRD §9.2, §FR-8, §FR-9
// ──────────────────────────────────────────────────────────────

/// Required CSV columns: symbol, exchange, token
/// Optional: instrument_type, expiry, strike, option_type
#[tauri::command]
pub async fn import_csv(
    state: State<'_, AppState>,
    csv_content: String,
) -> Result<ImportSummary, String> {
    let broker = state.storage.brokers
        .get_broker_by_key(&state.active_broker_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Active broker not found".to_string())?;

    let mut summary = ImportSummary {
        total_rows: 0,
        valid_rows: 0,
        duplicate_rows: 0,
        invalid_rows: 0,
        imported_rows: 0,
        skipped_rows: 0,
        errors: Vec::new(),
    };

    let mut reader = csv::Reader::from_reader(csv_content.as_bytes());
    let headers = reader.headers().map_err(|e| format!("CSV header error: {}", e))?.clone();

    // Validate required columns exist
    let required = ["symbol", "exchange", "token"];
    for col in &required {
        if !headers.iter().any(|h| h.to_lowercase() == *col) {
            return Err(format!("Missing required column: '{}'", col));
        }
    }

    for (row_idx, result) in reader.records().enumerate() {
        summary.total_rows += 1;
        let row_num = row_idx + 2; // 1-indexed + header row

        let record = match result {
            Ok(r) => r,
            Err(e) => {
                summary.invalid_rows += 1;
                summary.errors.push(ImportError {
                    row: row_num,
                    symbol: None,
                    reason: format!("CSV parse error: {}", e),
                });
                continue;
            }
        };

        // Extract fields by header name
        let get_field = |name: &str| -> Option<String> {
            headers.iter().position(|h| h.to_lowercase() == name)
                .and_then(|i| record.get(i))
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        };

        let symbol = match get_field("symbol") {
            Some(s) => s.to_uppercase(),
            None => {
                summary.invalid_rows += 1;
                summary.errors.push(ImportError {
                    row: row_num,
                    symbol: None,
                    reason: "Missing symbol".into(),
                });
                continue;
            }
        };

        let exchange = match get_field("exchange") {
            Some(e) => e.to_uppercase(),
            None => {
                summary.invalid_rows += 1;
                summary.errors.push(ImportError {
                    row: row_num,
                    symbol: Some(symbol.clone()),
                    reason: "Missing exchange".into(),
                });
                continue;
            }
        };

        let token = match get_field("token") {
            Some(t) => t,
            None => {
                summary.invalid_rows += 1;
                summary.errors.push(ImportError {
                    row: row_num,
                    symbol: Some(symbol.clone()),
                    reason: "Missing token".into(),
                });
                continue;
            }
        };

        let instrument_type = get_field("instrument_type").unwrap_or_else(|| "EQ".into());
        let expiry      = get_field("expiry");
        let option_type = get_field("option_type");
        let strike: Option<f64> = get_field("strike")
            .and_then(|s| s.parse().ok());

        summary.valid_rows += 1;

        // Upsert symbol
        let symbol_id = match state.storage.symbols.upsert_symbol(&NewSymbol {
            symbol: symbol.clone(),
            exchange: exchange.clone(),
            instrument_type,
            expiry,
            strike,
            option_type,
            tick_size: None,
            lot_size: None,
            status: "enabled".into(),
        }).await {
            Ok(id) => id,
            Err(e) if e.to_string().contains("UNIQUE") => {
                summary.duplicate_rows += 1;
                summary.skipped_rows += 1;
                continue;
            }
            Err(e) => {
                summary.invalid_rows += 1;
                summary.errors.push(ImportError {
                    row: row_num,
                    symbol: Some(symbol.clone()),
                    reason: format!("DB error: {}", e),
                });
                continue;
            }
        };

        // Upsert token mapping
        let _ = state.storage.symbols.upsert_symbol_mapping(&NewSymbolMapping {
            symbol_id,
            broker_id: broker.id,
            broker_symbol: symbol.clone(),
            broker_token: token,
        }).await;

        summary.imported_rows += 1;
    }

    summary.skipped_rows = summary.total_rows - summary.imported_rows - summary.invalid_rows;

    Ok(summary)
}
