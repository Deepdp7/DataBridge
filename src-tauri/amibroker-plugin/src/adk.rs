use std::os::raw::{c_char, c_int, c_void};

pub const PLUGIN_VERSION: c_int = 2; // Modern AmiBroker plugins usually report 2 or higher

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct AmiDate {
    pub date: u64,
}

impl AmiDate {
    pub fn new(year: u32, month: u32, day: u32, hour: u32, minute: u32, second: u32, microsec: u32) -> Self {
        // AB AmiDate bit format on x64:
        // LSB to MSB:
        // MicroSec: 10, MilliSec: 10, Second: 6, Minute: 6 (Total 32 bits = 1 DWORD)
        // Hour: 5, Day: 5, Month: 4, Year: 12, Reserved: 6 (Total 32 bits = 1 DWORD)
        let mut d: u64 = 0;
        
        let ms = microsec % 1000;
        let millis = microsec / 1000;

        d |= (ms as u64) & 0x3FF; // 10 bits
        d |= ((millis as u64) & 0x3FF) << 10; // 10 bits
        d |= ((second as u64) & 0x3F) << 20; // 6 bits
        d |= ((minute as u64) & 0x3F) << 26; // 6 bits
        d |= ((hour as u64) & 0x1F) << 32; // 5 bits
        d |= ((day as u64) & 0x1F) << 37; // 5 bits
        d |= ((month as u64) & 0xF) << 42; // 4 bits
        d |= ((year as u64) & 0xFFF) << 46; // 12 bits
        // Remaining 6 bits are reserved (0)

        AmiDate { date: d }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Quotation {
    pub date_time: AmiDate,
    pub price: f32,
    pub open: f32,
    pub high: f32,
    pub low: f32,
    pub volume: f32,
    pub open_int: f32,
    pub aux_data1: f32,
    pub aux_data2: f32,
}

#[repr(C)]
pub struct PluginInfo {
    pub struct_size: c_int,
    pub type_: c_int,
    pub version: c_int,
    pub id_code: c_int,
    pub name: [c_char; 256],
    pub vendor: [c_char; 256],
    pub certificate: c_int,
    pub min_amibroker_version: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RecentInfo {
    pub name: [c_char; 64],
    pub last: f32,
    pub open: f32,
    pub high: f32,
    pub low: f32,
    pub close: f32,
    pub change: f32,
    pub volume: f32,
    pub trade_volume: f32,
    pub open_int: f32,
    pub eps: f32,
    pub ep_sw: f32,
    pub dividend: f32,
    pub div_date: f32,
    pub ex_div_date: f32,
    pub nav: f32,
    pub date_time: AmiDate,
    pub bid: f32,
    pub ask: f32,
    pub bid_size: f32,
    pub ask_size: f32,
    pub prev_ask: f32,
    pub prev_bid: f32,
    pub total_vol: f32,
    pub fifty_two_wk_high: f32,
    pub fifty_two_wk_low: f32,
    pub flags: u32,
    pub update_flags: u32,
}

impl Default for RecentInfo {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

#[repr(C)]
pub struct AmiVar {
    pub type_: c_int,
    pub value: AmiVarValue,
}

#[repr(C)]
pub union AmiVarValue {
    pub val: f32,
    pub array: *mut f32,
    pub string: *mut c_char,
    pub disp: *mut c_void,
    pub recent_info: *mut RecentInfo,
}

#[repr(C)]
pub struct InfoSite {
    pub struct_size: c_int,
    pub get_stock_info: *const c_void,
    pub padding: [u8; 1024], // Opaque padding since we don't call callbacks in this simple plugin
}
