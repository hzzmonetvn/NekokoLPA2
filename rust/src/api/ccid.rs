//! 9eSIM CCID USB protocol and ISO 7816-3 ATR functions bridged to Flutter.

use flutter_rust_bridge::frb;
use nlpa_core::ccid::nine_esim::{
    enable_atr_programming_command as core_enable_atr_programming,
    euicc_capability_enabled as core_euicc_capability_enabled, password_hex as core_password_hex,
    set_atr_command as core_set_atr_command, set_euicc_capability as core_set_euicc_capability,
    set_password_command as core_set_password_command,
};

/// Computes or normalizes the 9eSIM USB password hex string.
#[frb]
#[must_use]
pub fn nine_esim_password_hex(value: String) -> String {
    core_password_hex(&value)
}

/// Builds the CCID command to configure the device password.
#[frb]
pub fn nine_esim_set_password_command(
    device_name: String,
    password: String,
) -> Result<Vec<u8>, String> {
    core_set_password_command(&device_name, &password).map_err(|e| e.to_string())
}

/// Builds the CCID command to enable ATR programming mode.
#[frb]
#[must_use]
pub fn nine_esim_enable_atr_programming_command() -> Vec<u8> {
    core_enable_atr_programming().to_vec()
}

/// Builds the CCID command to write a new ATR.
#[frb]
pub fn nine_esim_set_atr_command(atr_bytes: Vec<u8>) -> Result<Vec<u8>, String> {
    core_set_atr_command(&atr_bytes).map_err(|e| e.to_string())
}

/// Checks if eUICC capability is enabled in an ATR.
#[frb]
pub fn atr_euicc_capability_enabled(atr_hex: String) -> Result<bool, String> {
    core_euicc_capability_enabled(&atr_hex).map_err(|e| e.to_string())
}

/// Toggles eUICC capability in an ATR and recalculates TCK checksum.
#[frb]
pub fn atr_set_euicc_capability(atr_hex: String, enabled: bool) -> Result<String, String> {
    core_set_euicc_capability(&atr_hex, enabled).map_err(|e| e.to_string())
}
