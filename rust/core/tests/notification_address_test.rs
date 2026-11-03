use nlpa_core::is_valid_notification_address;

#[test]
fn test_valid_notification_addresses() {
    assert!(is_valid_notification_address("smdp.example.com"));
    assert!(is_valid_notification_address("esim.carrier.co.uk"));
    assert!(is_valid_notification_address("sub-domain.test.org.")); // with trailing dot
    assert!(is_valid_notification_address("a.bb"));
    assert!(is_valid_notification_address("123.example.com"));
}

#[test]
fn test_invalid_notification_addresses() {
    assert!(!is_valid_notification_address(""));
    assert!(!is_valid_notification_address("   "));
    assert!(!is_valid_notification_address(" smdp.example.com")); // leading space
    assert!(!is_valid_notification_address("smdp.example.com ")); // trailing space
    assert!(!is_valid_notification_address("smdp")); // no dot / single label
    assert!(!is_valid_notification_address("smdp..example.com")); // empty label
    assert!(!is_valid_notification_address("-smdp.example.com")); // leading hyphen
    assert!(!is_valid_notification_address("smdp-.example.com")); // trailing hyphen
    assert!(!is_valid_notification_address("smdp.example.123")); // numeric TLD
    assert!(!is_valid_notification_address("smdp.example.-com")); // TLD leading hyphen
    assert!(!is_valid_notification_address("smdp.ex@mple.com")); // invalid char
    assert!(!is_valid_notification_address("smdp.thư-viện.vn")); // non-ASCII
}
