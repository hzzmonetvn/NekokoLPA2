use nlpa_core::profile_size_predictor::{ProfileSizeCatalog, ReferenceSizeEntry};
use std::collections::HashMap;

#[test]
fn test_predict_size_exact_match_with_eum_override() {
    let mut eum_sizes = HashMap::new();
    eum_sizes.insert("89043051".to_string(), 42000);
    eum_sizes.insert("89043052".to_string(), 45000);

    let entries = vec![
        ReferenceSizeEntry {
            rsp: "smdp.example.com".to_string(),
            plmn: Some("20404".to_string()),
            service_provider_name: Some("CarrierX".to_string()),
            reference_size: 40000,
            eum_sizes,
        },
        ReferenceSizeEntry {
            rsp: "smdp.other.com".to_string(),
            plmn: Some("310260".to_string()),
            service_provider_name: Some("T-Mobile".to_string()),
            reference_size: 50000,
            eum_sizes: HashMap::new(),
        },
    ];

    let catalog = ProfileSizeCatalog::new(Some("89043051".to_string()), entries);

    // EUM match
    let size = catalog.predict_size(
        Some("890430520000000000000001"),
        Some("smdp.example.com"),
        Some("20404"),
        Some("CarrierX"),
    );
    assert_eq!(size, Some(45000));

    // Fallback to reference_size when EUM is unknown
    let size_unknown_eum = catalog.predict_size(
        Some("999999990000000000000001"),
        Some("smdp.example.com"),
        Some("20404"),
        Some("CarrierX"),
    );
    assert_eq!(size_unknown_eum, Some(40000));
}

#[test]
fn test_predict_size_spn_plmn_priority_scoring() {
    let entries = vec![
        ReferenceSizeEntry {
            rsp: "smdp.test.com".to_string(),
            plmn: Some("12345".to_string()),
            service_provider_name: Some("Generic".to_string()),
            reference_size: 30000,
            eum_sizes: HashMap::new(),
        },
        ReferenceSizeEntry {
            rsp: "smdp.test.com".to_string(),
            plmn: Some("12345".to_string()),
            service_provider_name: Some("Specific Carrier".to_string()),
            reference_size: 35000,
            eum_sizes: HashMap::new(),
        },
    ];

    let catalog = ProfileSizeCatalog::new(None, entries);

    // Should choose entry 2 because SPN matches (+3) + PLMN (+2) + RSP (+1) = score 6
    let size = catalog.predict_size(
        None,
        Some("smdp.test.com"),
        Some("12345"),
        Some("Specific Carrier Premium"),
    );
    assert_eq!(size, Some(35000));
}

#[test]
fn test_predict_size_empty_catalog() {
    let catalog = ProfileSizeCatalog::new(None, Vec::new());
    assert_eq!(catalog.predict_size(None, None, None, None), None);
}

#[test]
fn test_profile_size_catalog_from_json() {
    let json_data = r#"{
        "reference_eum": "89044045",
        "results": [
            {
                "plmn": "20201",
                "serviceProviderName": "COSMOTE",
                "rsp": "rsp-0003.oberthur.net",
                "reference_size": 39008,
                "eum_sizes": {
                    "89044045": 39008,
                    "89043051": 41798
                }
            }
        ]
    }"#;

    let catalog = ProfileSizeCatalog::from_json(json_data).expect("valid catalog json");
    assert_eq!(catalog.reference_eum.as_deref(), Some("89044045"));
    assert_eq!(catalog.entries.len(), 1);

    // Matches EUM
    let size = catalog.predict_size(
        Some("8904305112345678"),
        Some("rsp-0003.oberthur.net"),
        Some("20201"),
        Some("COSMOTE"),
    );
    assert_eq!(size, Some(41798));

    // Fallback to reference_size
    let fallback = catalog.predict_size(
        Some("9999999912345678"),
        Some("rsp-0003.oberthur.net"),
        Some("20201"),
        Some("COSMOTE"),
    );
    assert_eq!(fallback, Some(39008));
}
