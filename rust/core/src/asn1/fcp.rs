//! SIM file-control parameter and application-record parsing.

use super::der::{Asn1Error, Asn1Node, Asn1Reader};

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

fn sim_children(node: &Asn1Node) -> Result<Vec<Asn1Node>, Asn1Error> {
    let mut reader = Asn1Reader::new(&node.value);
    let mut children = Vec::new();
    while reader.has_more() {
        let offset = reader.offset();
        let first = node.value[offset];
        if first & 0x1f == 0x1f {
            let mut next = offset + 1;
            while next < node.value.len() {
                let byte = node.value[next];
                next += 1;
                if byte & 0x80 == 0 {
                    break;
                }
            }
            if next == node.value.len() && node.value[next - 1] & 0x80 != 0 {
                return Err(Asn1Error::UnexpectedEof {
                    offset: node.value.len(),
                    needed: 1,
                });
            }
        }
        let mut child = reader
            .read_next()?
            .ok_or(Asn1Error::UnexpectedEof { offset, needed: 1 })?;
        if first & 0x1f == 0x1f {
            child.tag = first as u32;
        }
        children.push(child);
    }
    Ok(children)
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
    let children = sim_children(&root)?;
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
    for child in sim_children(&record)? {
        match child.tag {
            0x4f => aid = Some(child.value),
            0x50 => {
                label = child.value.iter().map(|byte| char::from(*byte)).collect();
            }
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

    #[test]
    fn parses_descriptor_without_record_fields_and_ignores_extended_tags() {
        let data = [0x62, 0x07, 0x82, 0x02, 0x42, 0x00, 0x9f, 0x01, 0x00];
        let info = parse_fcp(&data).expect("valid descriptor-only FCP");
        assert_eq!(info.file_structure, 0x42);
        assert_eq!(info.record_size, 0);
        assert_eq!(info.record_count, 0);
        assert_eq!(info.file_size, 0);
    }

    #[test]
    fn rejects_missing_descriptor_and_bad_field_lengths() {
        assert!(matches!(
            parse_fcp(&[0x62, 0x00]),
            Err(Asn1Error::MissingField { tag: 0x82, .. })
        ));
        assert!(parse_fcp(&[0x62, 0x03, 0x82, 0x01, 0x42]).is_err());
        assert!(parse_fcp(&[0x62, 0x05, 0x82, 0x02, 0x42, 0x00, 0x80, 0x00]).is_err());
    }

    #[test]
    fn parses_application_record_and_trailing_padding() {
        let data = [
            0x61, 0x09, 0x4f, 0x04, 0xa0, 0x00, 0x00, 0x01, 0x50, 0x01, b'U', 0xff, 0xff,
        ];
        let (aid, label) = parse_application_record(&data)
            .expect("valid EF_DIR record")
            .expect("application present");
        assert_eq!(aid, [0xa0, 0x00, 0x00, 0x01]);
        assert_eq!(label, "U");
    }

    #[test]
    fn handles_padding_missing_aid_and_dart_label_code_units() {
        assert_eq!(
            parse_application_record(&[0xff, 0xff]).expect("padding"),
            None
        );
        assert!(parse_application_record(&[0x61, 0x03, 0x4f, 0x00]).is_err());
        let data = [0x61, 0x08, 0x4f, 0x01, 0xaa, 0x50, 0x03, 0x41, 0xff, 0x42];
        let (_, label) = parse_application_record(&data)
            .expect("valid non-UTF8 label")
            .expect("application present");
        assert_eq!(label, "AÿB");
    }
}
