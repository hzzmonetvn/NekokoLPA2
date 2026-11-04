//! SIM file-control parameter and application-record parsing.

use super::der::{Asn1Error, Asn1Reader};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FcpInfo {
    pub file_structure: u8,
    pub record_size: usize,
    pub record_count: usize,
    pub file_size: usize,
}

impl FcpInfo {
    #[must_use]
    pub fn is_linear_fixed(&self) -> bool {
        self.file_structure == 0x42
    }
}

/// Parses an FCP template and extracts supported file metadata.
pub fn parse_fcp(data: &[u8]) -> Result<FcpInfo, Asn1Error> {
    let mut reader = Asn1Reader::new(data);
    let root = reader.read_next()?.ok_or(Asn1Error::UnexpectedEof {
        offset: 0,
        needed: 1,
    })?;
    if reader.has_more() || root.tag != 0x62 {
        return Err(Asn1Error::UnexpectedTag {
            expected: 0x62,
            actual: root.tag,
        });
    }
    let children = root.children()?;
    let mut info = FcpInfo {
        file_structure: 0,
        record_size: 0,
        record_count: 0,
        file_size: 0,
    };
    for child in children {
        match child.tag {
            0x82 => {
                if child.value.len() != 2 && child.value.len() != 5 {
                    return Err(Asn1Error::InvalidInteger);
                }
                info.file_structure = child.value[0];
                if child.value.len() == 5 {
                    info.record_size =
                        usize::from(u16::from_be_bytes([child.value[2], child.value[3]]));
                    info.record_count = usize::from(child.value[4]);
                }
            }
            0x80 => {
                if child.value.len() != 2 {
                    return Err(Asn1Error::InvalidInteger);
                }
                info.file_size = usize::from(u16::from_be_bytes([child.value[0], child.value[1]]));
            }
            _ => {}
        }
    }
    if info.file_structure == 0 {
        return Err(Asn1Error::MissingField {
            tag: 0x82,
            name: "file descriptor",
        });
    }
    Ok(info)
}

/// Extracts AID and label from a single EF_DIR application record.
pub fn parse_application_record(data: &[u8]) -> Result<Option<(Vec<u8>, String)>, Asn1Error> {
    let end = data
        .iter()
        .rposition(|byte| *byte != 0xff)
        .map_or(0, |i| i + 1);
    if end == 0 {
        return Ok(None);
    }
    let mut reader = Asn1Reader::new(&data[..end]);
    let record = reader.read_next()?.ok_or(Asn1Error::UnexpectedEof {
        offset: 0,
        needed: 1,
    })?;
    if reader.has_more() || record.tag != 0x61 {
        return Err(Asn1Error::UnexpectedTag {
            expected: 0x61,
            actual: record.tag,
        });
    }
    let mut aid = None;
    let mut label = String::new();
    for child in record.children()? {
        match child.tag {
            0x4f => aid = Some(child.value),
            0x50 => label = String::from_utf8_lossy(&child.value).into_owned(),
            _ => {}
        }
    }
    let aid = aid
        .filter(|value: &Vec<u8>| !value.is_empty())
        .ok_or(Asn1Error::MissingField {
            tag: 0x4f,
            name: "application AID",
        })?;
    Ok(Some((aid, label)))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_file_control_fields() {
        let data = [
            0x62, 0x0b, 0x82, 0x05, 0x42, 0, 0, 0x20, 4, 0x80, 0x02, 0x01, 0x00,
        ];
        let info = parse_fcp(&data).expect("valid FCP");
        assert!(info.is_linear_fixed());
        assert_eq!(info.record_size, 32);
        assert_eq!(info.record_count, 4);
        assert_eq!(info.file_size, 256);
    }
}
