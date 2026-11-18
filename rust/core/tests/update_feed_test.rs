use nlpa_core::{check_update_feed, select_android_update, COMMUNITY_CERT};

const VENDOR_CERT: &str = "d1c0f48b370e74d4ea4770ed4c3cd70a3198d31f";

fn make_release_json(build: u64, sigs: &[&str]) -> String {
    let sig_json = sigs
        .iter()
        .map(|s| format!(r#""{s}""#))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        r#"{{
        "version": "2.5.1",
        "build": {build},
        "url": "https://github.com/hzzmonetvn/NekokoLPA2/releases/tag/v2.5.1%2B{build}",
        "download_url": "https://example.invalid/{build}.apk",
        "changelog": "Fix",
        "signatures": [{sig_json}]
    }}"#
    )
}

#[test]
fn test_check_update_feed_channel_isolation_and_cert_pinning() {
    let feed = format!(
        r#"{{
            "android": {{
                "open-rooted-{COMMUNITY_CERT}": {},
                "std-{COMMUNITY_CERT}": {},
                "open-{COMMUNITY_CERT}": {},
                "community-{COMMUNITY_CERT}": {}
            }}
        }}"#,
        make_release_json(700, &[COMMUNITY_CERT]),
        make_release_json(800, &[COMMUNITY_CERT]),
        make_release_json(601, &[COMMUNITY_CERT]),
        make_release_json(607, &[COMMUNITY_CERT]),
    );

    // Normal community selects highest matching non-rooted community/open release (607)
    let candidate = check_update_feed(&feed, false, 606, &COMMUNITY_CERT.to_ascii_uppercase())
        .expect("community update");
    assert_eq!(candidate.build, 607);
    assert_eq!(candidate.version, "2.5.1");

    // Privileged selects rooted channel only (700)
    let rooted = check_update_feed(&feed, true, 606, COMMUNITY_CERT).expect("rooted update");
    assert_eq!(rooted.build, 700);

    // Does not offer same or older build
    assert_eq!(check_update_feed(&feed, false, 607, COMMUNITY_CERT), None);
    assert_eq!(check_update_feed(&feed, false, 700, COMMUNITY_CERT), None);

    // Rejects wrong / non-community certificate
    assert_eq!(check_update_feed(&feed, false, 600, VENDOR_CERT), None);
    assert_eq!(check_update_feed(&feed, false, 600, ""), None);
}

#[test]
fn test_check_update_feed_rejects_unrestricted_malformed_and_wrong_sig_entries() {
    let feed = format!(
        r#"{{
            "open": {},
            "open-unverified": {},
            "open-{COMMUNITY_CERT}": [{}, {}]
        }}"#,
        make_release_json(900, &[]),
        make_release_json(901, &[COMMUNITY_CERT]),
        make_release_json(608, &[COMMUNITY_CERT]),
        make_release_json(999, &[VENDOR_CERT]),
    );

    let candidate = check_update_feed(&feed, false, 606, COMMUNITY_CERT).expect("valid entry");
    assert_eq!(candidate.build, 608);
}

#[test]
fn test_select_android_update_supports_legacy_open_and_multi_channels() {
    let legacy_feed = format!(
        r#"{{"community-{COMMUNITY_CERT}": {}, "community-rooted-{COMMUNITY_CERT}": {}}}"#,
        make_release_json(610, &[COMMUNITY_CERT]),
        make_release_json(611, &[COMMUNITY_CERT]),
    );
    assert_eq!(
        select_android_update(&legacy_feed, "open", &[COMMUNITY_CERT], 600).map(|u| u.build),
        Some(610)
    );
    assert_eq!(
        select_android_update(&legacy_feed, "open-rooted", &[COMMUNITY_CERT], 600)
            .map(|u| u.build),
        Some(611)
    );

    let multi_feed = format!(
        r#"{{"multi-{COMMUNITY_CERT}-{VENDOR_CERT}": {}}}"#,
        make_release_json(615, &[COMMUNITY_CERT, VENDOR_CERT]),
    );
    assert_eq!(
        select_android_update(&multi_feed, "multi", &[VENDOR_CERT], 600).map(|u| u.build),
        Some(615)
    );
    assert_eq!(
        select_android_update(&multi_feed, "lineage", &[COMMUNITY_CERT, VENDOR_CERT], 600),
        None
    );
}
