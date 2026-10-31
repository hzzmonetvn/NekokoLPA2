//! UI Data Models for Dioxus components

#[derive(Clone, Debug, PartialEq)]
pub enum ProfileState {
    Disabled,
    Enabled,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProfileItem {
    pub iccid: String,
    pub name: String,
    pub provider: String,
    pub state: ProfileState,
    pub icon_base64: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ReaderType {
    UsbCcid,
    Omapi,
    Remote,
    QrtrModem,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReaderInfo {
    pub id: String,
    pub name: String,
    pub kind: ReaderType,
    pub connected: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ApduLogEntry {
    pub is_tx: bool,
    pub data: String,
    pub sw: Option<String>,
}
