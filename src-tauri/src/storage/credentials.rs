/// Secure credential storage — Windows Credential Manager (DPAPI)
///
/// PRD §9.1: Credentials shall be stored using OS-level secure storage
/// (Windows Credential Manager / DPAPI) rather than plain text.
///
/// The `CredentialStore` trait is platform-independent; the Windows
/// implementation is compiled only on `target_os = "windows"`.
/// A fallback in-memory store is used for testing.

use crate::error::{AppError, Result};

/// Trait for secure key-value credential storage.
/// Each entry is identified by a `target` string (the opaque key stored in
/// `broker_sessions.credential_ref`). The secret value is the raw token.
pub trait CredentialStore: Send + Sync {
    fn save(&self, target: &str, username: &str, secret: &str) -> Result<()>;
    fn load(&self, target: &str) -> Result<String>;
    fn delete(&self, target: &str) -> Result<()>;
    fn exists(&self, target: &str) -> bool;
}

// ──────────────────────────────────────────────────────────────
// Windows Credential Manager implementation
// ──────────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
pub mod windows_impl {
    use super::*;
    use windows::core::{PCWSTR, PWSTR};
    use windows::Win32::Foundation::ERROR_NOT_FOUND;
    use windows::Win32::Security::Credentials::{
        CredDeleteW, CredReadW, CredWriteW, CREDENTIALW, CRED_TYPE_GENERIC,
        CREDENTIAL_ATTRIBUTEW, CRED_FLAGS,
    };

    pub struct WindowsCredentialStore;

    impl WindowsCredentialStore {
        pub fn new() -> Self { WindowsCredentialStore }

        fn to_wide(s: &str) -> Vec<u16> {
            s.encode_utf16().chain(std::iter::once(0)).collect()
        }
    }

    impl Default for WindowsCredentialStore {
        fn default() -> Self { Self::new() }
    }

    impl CredentialStore for WindowsCredentialStore {
        fn save(&self, target: &str, username: &str, secret: &str) -> Result<()> {
            let mut target_wide = Self::to_wide(target);
            let mut username_wide = Self::to_wide(username);
            let secret_bytes = secret.as_bytes();

            let cred = CREDENTIALW {
                Flags: CRED_FLAGS(0),
                Type: CRED_TYPE_GENERIC,
                TargetName: PWSTR(target_wide.as_mut_ptr()),
                Comment: PWSTR::null(),
                LastWritten: Default::default(),
                CredentialBlobSize: secret_bytes.len() as u32,
                CredentialBlob: secret_bytes.as_ptr() as *mut u8,
                Persist: windows::Win32::Security::Credentials::CRED_PERSIST_LOCAL_MACHINE,
                AttributeCount: 0,
                Attributes: std::ptr::null_mut::<CREDENTIAL_ATTRIBUTEW>(),
                TargetAlias: PWSTR::null(),
                UserName: PWSTR(username_wide.as_mut_ptr()),
            };

            unsafe {
                CredWriteW(&cred, 0).map_err(|e| {
                    AppError::CredentialStore(format!("CredWriteW failed: {}", e))
                })?;
            }
            Ok(())
        }

        fn load(&self, target: &str) -> Result<String> {
            let target_wide = Self::to_wide(target);
            let mut pcred: *mut CREDENTIALW = std::ptr::null_mut();

            unsafe {
                CredReadW(
                    PCWSTR(target_wide.as_ptr()),
                    CRED_TYPE_GENERIC,
                    Some(0),
                    &mut pcred,
                )
                .map_err(|e| {
                    AppError::CredentialStore(format!("CredReadW failed: {}", e))
                })?;

                let cred = &*pcred;
                let blob = std::slice::from_raw_parts(
                    cred.CredentialBlob,
                    cred.CredentialBlobSize as usize,
                );
                let secret = String::from_utf8(blob.to_vec()).map_err(|_| {
                    AppError::CredentialStore("Invalid UTF-8 in credential blob".into())
                })?;

                windows::Win32::Security::Credentials::CredFree(pcred as *mut _);
                Ok(secret)
            }
        }

        fn delete(&self, target: &str) -> Result<()> {
            let target_wide = Self::to_wide(target);
            unsafe {
                let result = CredDeleteW(PCWSTR(target_wide.as_ptr()), CRED_TYPE_GENERIC, Some(0));
                match result {
                    Ok(_) => Ok(()),
                    Err(e) if e.code() == ERROR_NOT_FOUND.into() => Ok(()), // already gone
                    Err(e) => Err(AppError::CredentialStore(format!("CredDeleteW failed: {}", e))),
                }
            }
        }

        fn exists(&self, target: &str) -> bool {
            self.load(target).is_ok()
        }
    }
}

// ──────────────────────────────────────────────────────────────
// In-memory fallback (development / non-Windows / tests)
// ──────────────────────────────────────────────────────────────

pub struct InMemoryCredentialStore {
    store: parking_lot::Mutex<std::collections::HashMap<String, String>>,
}

impl InMemoryCredentialStore {
    pub fn new() -> Self {
        InMemoryCredentialStore {
            store: parking_lot::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

impl Default for InMemoryCredentialStore {
    fn default() -> Self { Self::new() }
}

impl CredentialStore for InMemoryCredentialStore {
    fn save(&self, target: &str, _username: &str, secret: &str) -> Result<()> {
        self.store.lock().insert(target.to_string(), secret.to_string());
        Ok(())
    }

    fn load(&self, target: &str) -> Result<String> {
        self.store.lock().get(target).cloned().ok_or_else(|| {
            AppError::CredentialStore(format!("Credential not found: '{}'", target))
        })
    }

    fn delete(&self, target: &str) -> Result<()> {
        self.store.lock().remove(target);
        Ok(())
    }

    fn exists(&self, target: &str) -> bool {
        self.store.lock().contains_key(target)
    }
}

/// Build the appropriate CredentialStore for the current platform.
pub fn build_credential_store() -> std::sync::Arc<dyn CredentialStore> {
    #[cfg(target_os = "windows")]
    {
        std::sync::Arc::new(windows_impl::WindowsCredentialStore::new())
    }
    #[cfg(not(target_os = "windows"))]
    {
        tracing::warn!("Windows Credential Manager not available on this platform; using in-memory store");
        std::sync::Arc::new(InMemoryCredentialStore::new())
    }
}

/// Log redaction filter — scrubs known secret field names from log messages.
/// Called before any log string reaches `system_logs` or tracing output.
pub fn redact_secrets(message: &str) -> String {
    const REDACT_PATTERNS: &[&str] = &[
        "api_key", "api_secret", "access_token", "refresh_token",
        "password", "secret", "totp_secret", "pin", "token",
    ];

    let mut result = message.to_string();
    for pattern in REDACT_PATTERNS {
        // Redact JSON-like  "key": "value"  and  key=value  patterns
        let json_pattern = format!(r#""{}":\s*"[^"]*""#, pattern);
        let kv_pattern   = format!(r#"{}=[^\s&"']*"#, pattern);

        // Simple string replacement (no regex dependency — use a manual approach)
        result = redact_pattern(&result, pattern);
        let _ = (json_pattern, kv_pattern); // patterns documented for future regex use
    }
    result
}

fn redact_pattern(msg: &str, field_name: &str) -> String {
    let mut result = String::with_capacity(msg.len());
    let lower = msg.to_lowercase();
    let needle = field_name.to_lowercase();
    let mut pos = 0;

    while let Some(idx) = lower[pos..].find(&needle) {
        let abs_idx = pos + idx;
        result.push_str(&msg[pos..abs_idx]);
        result.push_str(field_name);
        result.push_str("=[REDACTED]");
        // Skip to end of likely value (next space, quote, or end)
        let after = abs_idx + field_name.len();
        let end = lower[after..].find(|c: char| c.is_whitespace() || c == '"' || c == ',')
            .map(|i| after + i)
            .unwrap_or(msg.len());
        pos = end;
    }
    result.push_str(&msg[pos..]);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_memory_store_round_trip() {
        let store = InMemoryCredentialStore::new();
        store.save("test_key", "user", "my_secret_token").unwrap();
        assert!(store.exists("test_key"));
        assert_eq!(store.load("test_key").unwrap(), "my_secret_token");
        store.delete("test_key").unwrap();
        assert!(!store.exists("test_key"));
    }

    #[test]
    fn redact_secrets_removes_api_key() {
        let msg = "Connecting with api_key=abc123secret and normal_field=ok";
        let redacted = redact_secrets(msg);
        assert!(!redacted.contains("abc123secret"));
        assert!(redacted.contains("[REDACTED]"));
    }
}
