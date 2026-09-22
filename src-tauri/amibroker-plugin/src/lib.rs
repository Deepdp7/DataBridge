pub mod adk;
pub mod ipc;

use std::os::raw::{c_char, c_int, c_void};
use std::ffi::CStr;
use once_cell::sync::Lazy;

use adk::*;
use ipc::IpcClient;

// Global IPC Client initialized on Init()
static mut IPC_CLIENT: Option<IpcClient> = None;

#[no_mangle]
pub extern "system" fn GetPluginInfo(info: *mut PluginInfo) -> c_int {
    if info.is_null() {
        return 0;
    }

    unsafe {
        (*info).type_ = 1; // 1 = data plugin
        (*info).version = 10000;
        (*info).id_code = 0x44425247; // "DBRG"
        (*info).certificate = 0;
        (*info).min_amibroker_version = 60000;

        let name = "DataBridge Plugin\0".as_bytes();
        let vendor = "DataBridge Engineering\0".as_bytes();

        std::ptr::copy_nonoverlapping(name.as_ptr() as *const i8, (*info).name.as_mut_ptr(), name.len());
        std::ptr::copy_nonoverlapping(vendor.as_ptr() as *const i8, (*info).vendor.as_mut_ptr(), vendor.len());
    }

    1
}

#[no_mangle]
pub extern "system" fn Init() -> c_int {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .try_init()
        .ok();

    tracing::info!("DataBridge Plugin Init()");

    unsafe {
        if IPC_CLIENT.is_none() {
            IPC_CLIENT = Some(IpcClient::new());
        }
    }

    1
}

#[no_mangle]
pub extern "system" fn Release() -> c_int {
    tracing::info!("DataBridge Plugin Release()");
    unsafe {
        IPC_CLIENT = None;
    }
    1
}

#[no_mangle]
pub extern "system" fn GetQuotesEx(
    ticker: *const c_char,
    _periodicity: c_int,
    _last_valid: c_int,
    size: c_int,
    quotes: *mut Quotation,
    _gqe_context: *mut c_void,
) -> c_int {
    if ticker.is_null() || quotes.is_null() {
        return 0;
    }

    let ticker_str = unsafe { CStr::from_ptr(ticker).to_string_lossy().into_owned() };
    
    let client = unsafe {
        match IPC_CLIENT.as_ref() {
            Some(c) => c,
            None => return 0,
        }
    };

    let bars = client.fetch_bars(&ticker_str);
    
    // AmiBroker asks for `size` bars maximum.
    // If we return N bars, we just fill the array and return N.
    let count = std::cmp::min(bars.len(), size as usize);
    if count == 0 {
        return 0;
    }

    unsafe {
        let out_quotes = std::slice::from_raw_parts_mut(quotes, count);
        for (i, bar) in bars.iter().take(count).enumerate() {
            let dt = chrono::DateTime::parse_from_rfc3339(&bar.timestamp).unwrap_or_default().with_timezone(&chrono::Utc);
            use chrono::Timelike;
            use chrono::Datelike;
            let date_time = AmiDate::new(
                dt.year() as u32,
                dt.month() as u32,
                dt.day() as u32,
                dt.hour() as u32,
                dt.minute() as u32,
                dt.second() as u32,
                (dt.nanosecond() / 1000) as u32
            );
            
            out_quotes[i] = Quotation {
                date_time,
                price: bar.close,
                open: bar.open,
                high: bar.high,
                low: bar.low,
                volume: bar.volume,
                open_int: bar.open_int,
                aux_data1: 0.0,
                aux_data2: 0.0,
            };
        }
    }

    count as c_int
}

// Global struct required by AmiBroker's GetRecentInfo which returns a variant pointer.
static mut LAST_RECENT_INFO: Lazy<parking_lot::Mutex<RecentInfo>> = Lazy::new(|| parking_lot::Mutex::new(RecentInfo::default()));

#[no_mangle]
pub extern "system" fn GetRecentInfo(ticker: *const c_char) -> AmiVar {
    let mut var = AmiVar {
        type_: 0, // None initially
        value: AmiVarValue { val: 0.0 },
    };

    if ticker.is_null() {
        return var;
    }

    let ticker_str = unsafe { CStr::from_ptr(ticker).to_string_lossy().into_owned() };

    let client = unsafe {
        match IPC_CLIENT.as_ref() {
            Some(c) => c,
            None => return var,
        }
    };

    let cache = client.recent_info_cache.read();
    if let Some(ri) = cache.get(&ticker_str) {
        unsafe {
            let mut guard = LAST_RECENT_INFO.lock();
            *guard = *ri;
            var.type_ = 5; // recent_info type
            var.value.recent_info = &mut *guard as *mut RecentInfo;
        }
    }

    var
}
