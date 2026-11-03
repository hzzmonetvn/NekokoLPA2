use nlpa_core::{bytes_to_hex, compute_hash_cc, hex_to_bytes, sha256};

#[test]
fn test_sha256_standard_vectors() {
    // Vector 1: Empty string
    let empty_hash = sha256(b"");
    assert_eq!(
        bytes_to_hex(&empty_hash),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );

    // Vector 2: "abc"
    let abc_hash = sha256(b"abc");
    assert_eq!(
        bytes_to_hex(&abc_hash),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );

    // Vector 3: "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq" (56 bytes, 2-block transition)
    let long_input = b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq";
    let long_hash = sha256(long_input);
    assert_eq!(
        bytes_to_hex(&long_hash),
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
    );
}

#[test]
fn test_compute_hash_cc_sgp22() {
    let confirmation_code = "1234";
    let transaction_id = hex_to_bytes("0102030405060708090A0B0C0D0E0F10").expect("hex bytes");

    let hash_cc = compute_hash_cc(confirmation_code, &transaction_id);

    // Manual step verification:
    // 1. sha256("1234") = 03ac674216f3e15c761ee1a5e255f067953623c8b388b4459e13f978d7c846f4
    let step1 = sha256(b"1234");
    assert_eq!(
        bytes_to_hex(&step1),
        "03ac674216f3e15c761ee1a5e255f067953623c8b388b4459e13f978d7c846f4"
    );

    // 2. sha256(step1 || transaction_id)
    let mut combined = Vec::new();
    combined.extend_from_slice(&step1);
    combined.extend_from_slice(&transaction_id);
    let step2 = sha256(&combined);

    assert_eq!(hash_cc, step2);
    assert_eq!(hash_cc.len(), 32);
}
