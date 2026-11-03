use nlpa_core::{
    aes_cbc_decrypt, aes_cbc_encrypt, base64_decode, base64_encode, bytes_to_hex, calc_host_mac,
    compute_hash_cc, decrypt_aes_256_cbc_with_key, encrypt_aes_256_cbc_with_key, hex_to_bytes,
    key_from_password, pkcs7_pad, pkcs7_unpad, sha256, AesCipher, CryptoError,
};

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

#[test]
fn test_aes_128_known_answer() {
    // NIST FIPS 197 Appendix B vector:
    // Key = 2b7e151628aed2a6abf7158809cf4f3c
    // Input = 3243f6a8885a308d313198a2e0370734
    // Output = 3925841d02dc09fbdc118597196a0b32
    let key = hex_to_bytes("2b7e151628aed2a6abf7158809cf4f3c").expect("key");
    let input = hex_to_bytes("3243f6a8885a308d313198a2e0370734").expect("input");
    let cipher = AesCipher::new(&key).expect("aes-128 cipher");

    let mut block = [0u8; 16];
    block.copy_from_slice(&input);
    cipher.encrypt_block(&mut block);
    assert_eq!(bytes_to_hex(&block), "3925841d02dc09fbdc118597196a0b32");

    // Decrypt back
    cipher.decrypt_block(&mut block);
    assert_eq!(bytes_to_hex(&block), "3243f6a8885a308d313198a2e0370734");
}

#[test]
fn test_aes_256_known_answer() {
    // NIST FIPS 197 Appendix C vector:
    // Key = 603deb1015ca71be2b73aef0857d77811f352c073b6108d72d9810a30914dff4
    // Input = 6bc1bee22e409f96e93d7e117393172a
    // Output = f3eed1bdb5d2a03c064b5a7e3db181f8
    let key = hex_to_bytes("603deb1015ca71be2b73aef0857d77811f352c073b6108d72d9810a30914dff4")
        .expect("key");
    let input = hex_to_bytes("6bc1bee22e409f96e93d7e117393172a").expect("input");
    let cipher = AesCipher::new(&key).expect("aes-256 cipher");

    let mut block = [0u8; 16];
    block.copy_from_slice(&input);
    cipher.encrypt_block(&mut block);
    assert_eq!(bytes_to_hex(&block), "f3eed1bdb5d2a03c064b5a7e3db181f8");

    // Decrypt back
    cipher.decrypt_block(&mut block);
    assert_eq!(bytes_to_hex(&block), "6bc1bee22e409f96e93d7e117393172a");
}

#[test]
fn test_pkcs7_padding_roundtrip() {
    let empty = b"";
    let padded_empty = pkcs7_pad(empty);
    assert_eq!(padded_empty.len(), 16);
    assert_eq!(padded_empty, vec![16u8; 16]);
    assert_eq!(pkcs7_unpad(&padded_empty).unwrap(), empty);

    let data15 = b"123456789012345"; // 15 bytes
    let padded15 = pkcs7_pad(data15);
    assert_eq!(padded15.len(), 16);
    assert_eq!(padded15[15], 1);
    assert_eq!(pkcs7_unpad(&padded15).unwrap(), data15);

    let data16 = b"1234567890123456"; // 16 bytes
    let padded16 = pkcs7_pad(data16);
    assert_eq!(padded16.len(), 32);
    assert_eq!(&padded16[16..], &[16u8; 16]);
    assert_eq!(pkcs7_unpad(&padded16).unwrap(), data16);

    // Invalid padding
    let invalid = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 0];
    assert_eq!(
        pkcs7_unpad(&invalid).unwrap_err(),
        CryptoError::InvalidPadding
    );
}

#[test]
fn test_aes_cbc_roundtrip() {
    let key = [0x42u8; 32];
    let iv = [0x24u8; 16];
    let plaintext = b"Hello GSMA eSIM remote profile management via Rust!";
    let padded = pkcs7_pad(plaintext);

    let ciphertext = aes_cbc_encrypt(&key, &iv, &padded).expect("cbc encrypt");
    assert_eq!(ciphertext.len(), padded.len());

    let decrypted_padded = aes_cbc_decrypt(&key, &iv, &ciphertext).expect("cbc decrypt");
    let unpadded = pkcs7_unpad(&decrypted_padded).expect("unpad");
    assert_eq!(unpadded, plaintext);
}

#[test]
fn test_encrypt_decrypt_aes_256_cbc_with_key() {
    let password = "SecretCarrierKey";
    let key = key_from_password(password);
    let iv = [0x55u8; 16];
    let payload = b"NekokoLPA2 Pure Rust Secure Payload";

    let encrypted = encrypt_aes_256_cbc_with_key(payload, &key, &iv);
    assert!(encrypted.len() >= 32);
    assert_eq!(&encrypted[..16], &iv);

    let decrypted = decrypt_aes_256_cbc_with_key(&encrypted, &key).expect("decrypt");
    assert_eq!(decrypted, payload);
}

#[test]
fn test_base64_codec() {
    let test_cases = [
        (b"".as_slice(), ""),
        (b"f".as_slice(), "Zg=="),
        (b"fo".as_slice(), "Zm8="),
        (b"foo".as_slice(), "Zm9v"),
        (b"foob".as_slice(), "Zm9vYg=="),
        (b"fooba".as_slice(), "Zm9vYmE="),
        (b"foobar".as_slice(), "Zm9vYmFy"),
    ];

    for (raw, b64) in test_cases {
        assert_eq!(base64_encode(raw), b64);
        assert_eq!(base64_decode(b64).expect("decode"), raw);
    }

    // Invalid base64
    assert!(base64_decode("invalid!").is_err());
    assert!(base64_decode("abc").is_err());
}

#[test]
fn test_calc_host_mac() {
    let session_key = [0x11u8; 16];
    let challenge = [0x22u8; 16];

    let mac = calc_host_mac(&session_key, &challenge).expect("calc mac");
    assert_eq!(mac.len(), 4);
    assert_ne!(mac, [0u8; 4]);
}
