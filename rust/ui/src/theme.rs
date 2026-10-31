//! AMOLED Pure Black theme tokens for Dioxus components

pub struct AmoledPalette {
    pub background: &'static str,
    pub surface: &'static str,
    pub surface_variant: &'static str,
    pub text_primary: &'static str,
    pub text_secondary: &'static str,
    pub accent: &'static str,
    pub error: &'static str,
    pub success: &'static str,
}

pub const AMOLED_PALETTE: AmoledPalette = AmoledPalette {
    background: "#000000",
    surface: "#101010",
    surface_variant: "#1A1A1A",
    text_primary: "#F5F5F5",
    text_secondary: "#888888",
    accent: "#2196F3",
    error: "#EF5350",
    success: "#4CAF50",
};
