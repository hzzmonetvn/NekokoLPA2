//! AMOLED Graphite refined palette for low-level eSIM & modem interface
//! Designed according to unslop craft rules:
//! - Pure black #000000 background for true OLED pixel shutoff
//! - Subdued neutral ramp (#0C0C0C, #161616, #222222) with 1px hairline borders
//! - Exact telecommunications status indicators (emerald signal dot, steel blue accent)

pub struct AmoledPalette {
    pub background: &'static str,
    pub surface: &'static str,
    pub surface_elevated: &'static str,
    pub surface_border: &'static str,
    pub text_primary: &'static str,
    pub text_secondary: &'static str,
    pub text_muted: &'static str,
    pub accent: &'static str,
    pub accent_muted: &'static str,
    pub signal_active: &'static str,
    pub signal_inactive: &'static str,
    pub error: &'static str,
}

pub const AMOLED_PALETTE: AmoledPalette = AmoledPalette {
    background: "#000000",
    surface: "#0C0C0C",
    surface_elevated: "#161616",
    surface_border: "#202020",
    text_primary: "#EEEEEE",
    text_secondary: "#9E9E9E",
    text_muted: "#616161",
    accent: "#388BFD",
    accent_muted: "rgba(56, 139, 253, 0.12)",
    signal_active: "#2EA043",
    signal_inactive: "#484F58",
    error: "#F85149",
};
