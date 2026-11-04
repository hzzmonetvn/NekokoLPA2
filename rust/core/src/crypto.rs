//! Cryptographic primitives, AES-CBC, AES-CMAC, SHA-1, SHA-256, PKCS#7, Base64, and GSMA SGP.22 confirmation code hash calculations.
//!
//! Provides a zero-dependency, safe Rust implementation of:
//! - Standard SHA-256 (RFC 6234)
//! - Standard SHA-1 (RFC 3174) for PKIX SubjectKeyIdentifier derivation
//! - GSMA SGP.22 confirmation code hash computation: `HashCc = SHA256(SHA256(CC) || TxID)`
//! - FIPS 197 AES-128 and AES-256 block cipher
//! - Cipher Block Chaining (CBC) mode with PKCS#7 padding
//! - NIST SP 800-38B / RFC 4493 AES-CMAC message authentication
//! - Standard Base64 encoding and decoding
//! - Telecom GM authentication host MAC calculation

use std::fmt;

/// Errors arising during cryptographic operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CryptoError {
    InvalidKeyLength(usize),
    InvalidIvLength(usize),
    InvalidCiphertextLength(usize),
    InvalidPadding,
    InvalidBase64,
    InvalidUtf8,
}

impl fmt::Display for CryptoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidKeyLength(len) => {
                write!(f, "Invalid AES key length: {len} (expected 16 or 32 bytes)")
            }
            Self::InvalidIvLength(len) => write!(f, "Invalid IV length: {len} (expected 16 bytes)"),
            Self::InvalidCiphertextLength(len) => write!(
                f,
                "Invalid ciphertext length: {len} (must be multiple of 16)"
            ),
            Self::InvalidPadding => write!(f, "Invalid PKCS#7 padding"),
            Self::InvalidBase64 => write!(f, "Invalid base64 payload"),
            Self::InvalidUtf8 => write!(f, "Decrypted plaintext is not valid UTF-8"),
        }
    }
}

impl std::error::Error for CryptoError {}

// ============================================================================
// SHA-256 (RFC 6234) & GSMA SGP.22 HashCc
// ============================================================================

/// Computes the SHA-256 digest of `data`.
#[must_use]
pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut state: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut buffer = [0u8; 64];
    let mut chunk_len = 0;

    for &byte in data {
        buffer[chunk_len] = byte;
        chunk_len += 1;
        if chunk_len == 64 {
            compress_sha256_block(&mut state, &buffer);
            chunk_len = 0;
        }
    }

    buffer[chunk_len] = 0x80;
    chunk_len += 1;

    if chunk_len > 56 {
        for b in &mut buffer[chunk_len..64] {
            *b = 0;
        }
        compress_sha256_block(&mut state, &buffer);
        chunk_len = 0;
    }

    for b in &mut buffer[chunk_len..56] {
        *b = 0;
    }
    buffer[56..64].copy_from_slice(&bit_len.to_be_bytes());
    compress_sha256_block(&mut state, &buffer);

    let mut output = [0u8; 32];
    for (i, word) in state.iter().enumerate() {
        output[i * 4..(i + 1) * 4].copy_from_slice(&word.to_be_bytes());
    }
    output
}

/// SGP.22 Confirmation Code hash computation:
/// `HashCc = SHA256(SHA256(ConfirmationCode) || TransactionID)`
#[must_use]
pub fn compute_hash_cc(confirmation_code: &str, transaction_id: &[u8]) -> [u8; 32] {
    let first_hash = sha256(confirmation_code.as_bytes());
    let mut combined = Vec::with_capacity(32 + transaction_id.len());
    combined.extend_from_slice(&first_hash);
    combined.extend_from_slice(transaction_id);
    sha256(&combined)
}

/// Generates a 32-byte AES key from a password via SHA-256.
#[must_use]
pub fn key_from_password(password: &str) -> [u8; 32] {
    sha256(password.as_bytes())
}

const SHA256_K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

#[inline]
fn ch(x: u32, y: u32, z: u32) -> u32 {
    (x & y) ^ (!x & z)
}

#[inline]
fn maj(x: u32, y: u32, z: u32) -> u32 {
    (x & y) ^ (x & z) ^ (y & z)
}

#[inline]
fn big_sigma0(x: u32) -> u32 {
    x.rotate_right(2) ^ x.rotate_right(13) ^ x.rotate_right(22)
}

#[inline]
fn big_sigma1(x: u32) -> u32 {
    x.rotate_right(6) ^ x.rotate_right(11) ^ x.rotate_right(25)
}

#[inline]
fn small_sigma0(x: u32) -> u32 {
    x.rotate_right(7) ^ x.rotate_right(18) ^ (x >> 3)
}

#[inline]
fn small_sigma1(x: u32) -> u32 {
    x.rotate_right(17) ^ x.rotate_right(19) ^ (x >> 10)
}

fn compress_sha256_block(state: &mut [u32; 8], block: &[u8; 64]) {
    let mut w = [0u32; 64];
    for (i, chunk) in block.as_chunks::<4>().0.iter().enumerate() {
        w[i] = u32::from_be_bytes(*chunk);
    }
    for i in 16..64 {
        w[i] = small_sigma1(w[i - 2])
            .wrapping_add(w[i - 7])
            .wrapping_add(small_sigma0(w[i - 15]))
            .wrapping_add(w[i - 16]);
    }

    let mut a = state[0];
    let mut b = state[1];
    let mut c = state[2];
    let mut d = state[3];
    let mut e = state[4];
    let mut f = state[5];
    let mut g = state[6];
    let mut h = state[7];

    for i in 0..64 {
        let t1 = h
            .wrapping_add(big_sigma1(e))
            .wrapping_add(ch(e, f, g))
            .wrapping_add(SHA256_K[i])
            .wrapping_add(w[i]);
        let t2 = big_sigma0(a).wrapping_add(maj(a, b, c));

        h = g;
        g = f;
        f = e;
        e = d.wrapping_add(t1);
        d = c;
        c = b;
        b = a;
        a = t1.wrapping_add(t2);
    }

    state[0] = state[0].wrapping_add(a);
    state[1] = state[1].wrapping_add(b);
    state[2] = state[2].wrapping_add(c);
    state[3] = state[3].wrapping_add(d);
    state[4] = state[4].wrapping_add(e);
    state[5] = state[5].wrapping_add(f);
    state[6] = state[6].wrapping_add(g);
    state[7] = state[7].wrapping_add(h);
}

// ============================================================================
// SHA-1 (RFC 3174) for PKIX SubjectKeyIdentifier
// ============================================================================

/// Computes the 160-bit SHA-1 digest of `data`.
///
/// Implemented per RFC 3174 for standard X.509 Subject Key Identifier (SKI) derivation.
#[must_use]
pub fn sha1(data: &[u8]) -> [u8; 20] {
    let mut state: [u32; 5] = [
        0x6745_2301,
        0xEFCD_AB89,
        0x98BA_DCFE,
        0x1032_5476,
        0xC3D2_E1F0,
    ];

    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut buffer = [0u8; 64];
    let mut chunk_len = 0;

    for &byte in data {
        buffer[chunk_len] = byte;
        chunk_len += 1;
        if chunk_len == 64 {
            compress_sha1_block(&mut state, &buffer);
            chunk_len = 0;
        }
    }

    // Padding: append 0x80 bit
    buffer[chunk_len] = 0x80;
    chunk_len += 1;

    if chunk_len > 56 {
        for b in &mut buffer[chunk_len..64] {
            *b = 0;
        }
        compress_sha1_block(&mut state, &buffer);
        chunk_len = 0;
    }

    for b in &mut buffer[chunk_len..56] {
        *b = 0;
    }
    buffer[56..64].copy_from_slice(&bit_len.to_be_bytes());
    compress_sha1_block(&mut state, &buffer);

    let mut digest = [0u8; 20];
    for (i, word) in state.iter().enumerate() {
        digest[i * 4..(i + 1) * 4].copy_from_slice(&word.to_be_bytes());
    }
    digest
}

fn compress_sha1_block(state: &mut [u32; 5], block: &[u8; 64]) {
    let mut w = [0u32; 80];
    for (i, chunk) in block.as_chunks::<4>().0.iter().enumerate() {
        w[i] = u32::from_be_bytes(*chunk);
    }
    for i in 16..80 {
        w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
    }

    let mut a = state[0];
    let mut b = state[1];
    let mut c = state[2];
    let mut d = state[3];
    let mut e = state[4];

    for (i, &w_i) in w.iter().enumerate() {
        let (f, k) = match i {
            0..=19 => ((b & c) | (!b & d), 0x5A82_7999),
            20..=39 => (b ^ c ^ d, 0x6ED9_EBA1),
            40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1B_BCDC),
            _ => (b ^ c ^ d, 0xCA62_C1D6),
        };

        let temp = a
            .rotate_left(5)
            .wrapping_add(f)
            .wrapping_add(e)
            .wrapping_add(k)
            .wrapping_add(w_i);
        e = d;
        d = c;
        c = b.rotate_left(30);
        b = a;
        a = temp;
    }

    state[0] = state[0].wrapping_add(a);
    state[1] = state[1].wrapping_add(b);
    state[2] = state[2].wrapping_add(c);
    state[3] = state[3].wrapping_add(d);
    state[4] = state[4].wrapping_add(e);
}

/// Computes the RFC 2104 / RFC 2202 HMAC-SHA1 message authentication code.
#[must_use]
pub fn hmac_sha1(key: &[u8], data: &[u8]) -> [u8; 20] {
    let mut k = [0u8; 64];
    if key.len() > 64 {
        let digest = sha1(key);
        k[..20].copy_from_slice(&digest);
    } else {
        k[..key.len()].copy_from_slice(key);
    }

    let mut ipad = [0u8; 64];
    let mut opad = [0u8; 64];
    for i in 0..64 {
        ipad[i] = k[i] ^ 0x36;
        opad[i] = k[i] ^ 0x5c;
    }

    let mut inner_input = Vec::with_capacity(64 + data.len());
    inner_input.extend_from_slice(&ipad);
    inner_input.extend_from_slice(data);
    let inner_hash = sha1(&inner_input);

    let mut outer_input = [0u8; 64 + 20];
    outer_input[..64].copy_from_slice(&opad);
    outer_input[64..].copy_from_slice(&inner_hash);
    sha1(&outer_input)
}

// ============================================================================
// FIPS 197 AES-128 & AES-256 Engine
// ============================================================================

const S_BOX: [u8; 256] = [
    0x63, 0x7c, 0x77, 0x7b, 0xf2, 0x6b, 0x6f, 0xc5, 0x30, 0x01, 0x67, 0x2b, 0xfe, 0xd7, 0xab, 0x76,
    0xca, 0x82, 0xc9, 0x7d, 0xfa, 0x59, 0x47, 0xf0, 0xad, 0xd4, 0xa2, 0xaf, 0x9c, 0xa4, 0x72, 0xc0,
    0xb7, 0xfd, 0x93, 0x26, 0x36, 0x3f, 0xf7, 0xcc, 0x34, 0xa5, 0xe5, 0xf1, 0x71, 0xd8, 0x31, 0x15,
    0x04, 0xc7, 0x23, 0xc3, 0x18, 0x96, 0x05, 0x9a, 0x07, 0x12, 0x80, 0xe2, 0xeb, 0x27, 0xb2, 0x75,
    0x09, 0x83, 0x2c, 0x1a, 0x1b, 0x6e, 0x5a, 0xa0, 0x52, 0x3b, 0xd6, 0xb3, 0x29, 0xe3, 0x2f, 0x84,
    0x53, 0xd1, 0x00, 0xed, 0x20, 0xfc, 0xb1, 0x5b, 0x6a, 0xcb, 0xbe, 0x39, 0x4a, 0x4c, 0x58, 0xcf,
    0xd0, 0xef, 0xaa, 0xfb, 0x43, 0x4d, 0x33, 0x85, 0x45, 0xf9, 0x02, 0x7f, 0x50, 0x3c, 0x9f, 0xa8,
    0x51, 0xa3, 0x40, 0x8f, 0x92, 0x9d, 0x38, 0xf5, 0xbc, 0xb6, 0xda, 0x21, 0x10, 0xff, 0xf3, 0xd2,
    0xcd, 0x0c, 0x13, 0xec, 0x5f, 0x97, 0x44, 0x17, 0xc4, 0xa7, 0x7e, 0x3d, 0x64, 0x5d, 0x19, 0x73,
    0x60, 0x81, 0x4f, 0xdc, 0x22, 0x2a, 0x90, 0x88, 0x46, 0xee, 0xb8, 0x14, 0xde, 0x5e, 0x0b, 0xdb,
    0xe0, 0x32, 0x3a, 0x0a, 0x49, 0x06, 0x24, 0x5c, 0xc2, 0xd3, 0xac, 0x62, 0x91, 0x95, 0xe4, 0x79,
    0xe7, 0xc8, 0x37, 0x6d, 0x8d, 0xd5, 0x4e, 0xa9, 0x6c, 0x56, 0xf4, 0xea, 0x65, 0x7a, 0xae, 0x08,
    0xba, 0x78, 0x25, 0x2e, 0x1c, 0xa6, 0xb4, 0xc6, 0xe8, 0xdd, 0x74, 0x1f, 0x4b, 0xbd, 0x8b, 0x8a,
    0x70, 0x3e, 0xb5, 0x66, 0x48, 0x03, 0xf6, 0x0e, 0x61, 0x35, 0x57, 0xb9, 0x86, 0xc1, 0x1d, 0x9e,
    0xe1, 0xf8, 0x98, 0x11, 0x69, 0xd9, 0x8e, 0x94, 0x9b, 0x1e, 0x87, 0xe9, 0xce, 0x55, 0x28, 0xdf,
    0x8c, 0xa1, 0x89, 0x0d, 0xbf, 0xe6, 0x42, 0x68, 0x41, 0x99, 0x2d, 0x0f, 0xb0, 0x54, 0xbb, 0x16,
];

const INV_S_BOX: [u8; 256] = [
    0x52, 0x09, 0x6a, 0xd5, 0x30, 0x36, 0xa5, 0x38, 0xbf, 0x40, 0xa3, 0x9e, 0x81, 0xf3, 0xd7, 0xfb,
    0x7c, 0xe3, 0x39, 0x82, 0x9b, 0x2f, 0xff, 0x87, 0x34, 0x8e, 0x43, 0x44, 0xc4, 0xde, 0xe9, 0xcb,
    0x54, 0x7b, 0x94, 0x32, 0xa6, 0xc2, 0x23, 0x3d, 0xee, 0x4c, 0x95, 0x0b, 0x42, 0xfa, 0xc3, 0x4e,
    0x08, 0x2e, 0xa1, 0x66, 0x28, 0xd9, 0x24, 0xb2, 0x76, 0x5b, 0xa2, 0x49, 0x6d, 0x8b, 0xd1, 0x25,
    0x72, 0xf8, 0xf6, 0x64, 0x86, 0x68, 0x98, 0x16, 0xd4, 0xa4, 0x5c, 0xcc, 0x5d, 0x65, 0xb6, 0x92,
    0x6c, 0x70, 0x48, 0x50, 0xfd, 0xed, 0xb9, 0xda, 0x5e, 0x15, 0x46, 0x57, 0xa7, 0x8d, 0x9d, 0x84,
    0x90, 0xd8, 0xab, 0x00, 0x8c, 0xbc, 0xd3, 0x0a, 0xf7, 0xe4, 0x58, 0x05, 0xb8, 0xb3, 0x45, 0x06,
    0xd0, 0x2c, 0x1e, 0x8f, 0xca, 0x3f, 0x0f, 0x02, 0xc1, 0xaf, 0xbd, 0x03, 0x01, 0x13, 0x8a, 0x6b,
    0x3a, 0x91, 0x11, 0x41, 0x4f, 0x67, 0xdc, 0xea, 0x97, 0xf2, 0xcf, 0xce, 0xf0, 0xb4, 0xe6, 0x73,
    0x96, 0xac, 0x74, 0x22, 0xe7, 0xad, 0x35, 0x85, 0xe2, 0xf9, 0x37, 0xe8, 0x1c, 0x75, 0xdf, 0x6e,
    0x47, 0xf1, 0x1a, 0x71, 0x1d, 0x29, 0xc5, 0x89, 0x6f, 0xb7, 0x62, 0x0e, 0xaa, 0x18, 0xbe, 0x1b,
    0xfc, 0x56, 0x3e, 0x4b, 0xc6, 0xd2, 0x79, 0x20, 0x9a, 0xdb, 0xc0, 0xfe, 0x78, 0xcd, 0x5a, 0xf4,
    0x1f, 0xdd, 0xa8, 0x33, 0x88, 0x07, 0xc7, 0x31, 0xb1, 0x12, 0x10, 0x59, 0x27, 0x80, 0xec, 0x5f,
    0x60, 0x51, 0x7f, 0xa9, 0x19, 0xb5, 0x4a, 0x0d, 0x2d, 0xe5, 0x7a, 0x9f, 0x93, 0xc9, 0x9c, 0xef,
    0xa0, 0xe0, 0x3b, 0x4d, 0xae, 0x2a, 0xf5, 0xb0, 0xc8, 0xeb, 0xbb, 0x3c, 0x83, 0x53, 0x99, 0x61,
    0x17, 0x2b, 0x04, 0x7e, 0xba, 0x77, 0xd6, 0x26, 0xe1, 0x69, 0x14, 0x63, 0x55, 0x21, 0x0c, 0x7d,
];

const RCON: [u8; 10] = [0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x1b, 0x36];

#[inline]
fn xtime(x: u8) -> u8 {
    (x << 1) ^ (if (x & 0x80) != 0 { 0x1b } else { 0 })
}

#[inline]
fn gmul(mut a: u8, mut b: u8) -> u8 {
    let mut p = 0u8;
    for _ in 0..8 {
        if (b & 1) != 0 {
            p ^= a;
        }
        a = xtime(a);
        b >>= 1;
    }
    p
}

/// AES key scheduler and single-block transformer.
pub struct AesCipher {
    rounds: usize,
    round_keys: Vec<[u8; 16]>,
}

impl AesCipher {
    /// Creates a new AES cipher instance with a 16-byte (AES-128) or 32-byte (AES-256) key.
    pub fn new(key: &[u8]) -> Result<Self, CryptoError> {
        let (nk, rounds) = match key.len() {
            16 => (4, 10),
            32 => (8, 14),
            other => return Err(CryptoError::InvalidKeyLength(other)),
        };

        let total_words = (rounds + 1) * 4;
        let mut w = vec![[0u8; 4]; total_words];

        for i in 0..nk {
            w[i].copy_from_slice(&key[i * 4..(i + 1) * 4]);
        }

        for i in nk..total_words {
            let mut temp = w[i - 1];
            if i % nk == 0 {
                temp = [
                    S_BOX[usize::from(temp[1])] ^ RCON[(i / nk) - 1],
                    S_BOX[usize::from(temp[2])],
                    S_BOX[usize::from(temp[3])],
                    S_BOX[usize::from(temp[0])],
                ];
            } else if nk > 6 && (i % nk) == 4 {
                temp = [
                    S_BOX[usize::from(temp[0])],
                    S_BOX[usize::from(temp[1])],
                    S_BOX[usize::from(temp[2])],
                    S_BOX[usize::from(temp[3])],
                ];
            }
            for j in 0..4 {
                w[i][j] = w[i - nk][j] ^ temp[j];
            }
        }

        let mut round_keys = Vec::with_capacity(rounds + 1);
        for r in 0..=rounds {
            let mut rk = [0u8; 16];
            for c in 0..4 {
                rk[c * 4..(c + 1) * 4].copy_from_slice(&w[r * 4 + c]);
            }
            round_keys.push(rk);
        }

        Ok(Self { rounds, round_keys })
    }

    /// Encrypts one 16-byte block in place.
    pub fn encrypt_block(&self, block: &mut [u8; 16]) {
        self.add_round_key(block, 0);

        for r in 1..self.rounds {
            Self::sub_bytes(block);
            Self::shift_rows(block);
            Self::mix_columns(block);
            self.add_round_key(block, r);
        }

        Self::sub_bytes(block);
        Self::shift_rows(block);
        self.add_round_key(block, self.rounds);
    }

    /// Decrypts one 16-byte block in place.
    pub fn decrypt_block(&self, block: &mut [u8; 16]) {
        self.add_round_key(block, self.rounds);

        for r in (1..self.rounds).rev() {
            Self::inv_shift_rows(block);
            Self::inv_sub_bytes(block);
            self.add_round_key(block, r);
            Self::inv_mix_columns(block);
        }

        Self::inv_shift_rows(block);
        Self::inv_sub_bytes(block);
        self.add_round_key(block, 0);
    }

    #[inline]
    fn add_round_key(&self, state: &mut [u8; 16], round: usize) {
        let rk = &self.round_keys[round];
        for i in 0..16 {
            state[i] ^= rk[i];
        }
    }

    #[inline]
    fn sub_bytes(state: &mut [u8; 16]) {
        for b in state.iter_mut() {
            *b = S_BOX[usize::from(*b)];
        }
    }

    #[inline]
    fn inv_sub_bytes(state: &mut [u8; 16]) {
        for b in state.iter_mut() {
            *b = INV_S_BOX[usize::from(*b)];
        }
    }

    #[inline]
    fn shift_rows(state: &mut [u8; 16]) {
        let s = *state;
        state[1] = s[5];
        state[5] = s[9];
        state[9] = s[13];
        state[13] = s[1];

        state[2] = s[10];
        state[6] = s[14];
        state[10] = s[2];
        state[14] = s[6];

        state[3] = s[15];
        state[7] = s[3];
        state[11] = s[7];
        state[15] = s[11];
    }

    #[inline]
    fn inv_shift_rows(state: &mut [u8; 16]) {
        let s = *state;
        state[1] = s[13];
        state[5] = s[1];
        state[9] = s[5];
        state[13] = s[9];

        state[2] = s[10];
        state[6] = s[14];
        state[10] = s[2];
        state[14] = s[6];

        state[3] = s[7];
        state[7] = s[11];
        state[11] = s[15];
        state[15] = s[3];
    }

    #[inline]
    fn mix_columns(state: &mut [u8; 16]) {
        for c in 0..4 {
            let idx = c * 4;
            let s0 = state[idx];
            let s1 = state[idx + 1];
            let s2 = state[idx + 2];
            let s3 = state[idx + 3];

            state[idx] = xtime(s0) ^ (xtime(s1) ^ s1) ^ s2 ^ s3;
            state[idx + 1] = s0 ^ xtime(s1) ^ (xtime(s2) ^ s2) ^ s3;
            state[idx + 2] = s0 ^ s1 ^ xtime(s2) ^ (xtime(s3) ^ s3);
            state[idx + 3] = (xtime(s0) ^ s0) ^ s1 ^ s2 ^ xtime(s3);
        }
    }

    #[inline]
    fn inv_mix_columns(state: &mut [u8; 16]) {
        for c in 0..4 {
            let idx = c * 4;
            let s0 = state[idx];
            let s1 = state[idx + 1];
            let s2 = state[idx + 2];
            let s3 = state[idx + 3];

            state[idx] = gmul(s0, 0x0e) ^ gmul(s1, 0x0b) ^ gmul(s2, 0x0d) ^ gmul(s3, 0x09);
            state[idx + 1] = gmul(s0, 0x09) ^ gmul(s1, 0x0e) ^ gmul(s2, 0x0b) ^ gmul(s3, 0x0d);
            state[idx + 2] = gmul(s0, 0x0d) ^ gmul(s1, 0x09) ^ gmul(s2, 0x0e) ^ gmul(s3, 0x0b);
            state[idx + 3] = gmul(s0, 0x0b) ^ gmul(s1, 0x0d) ^ gmul(s2, 0x09) ^ gmul(s3, 0x0e);
        }
    }
}

// ============================================================================
// PKCS#7 & AES-CBC
// ============================================================================

/// Appends PKCS#7 padding to align `data` to a 16-byte block boundary.
#[must_use]
pub fn pkcs7_pad(data: &[u8]) -> Vec<u8> {
    let pad_len = 16 - (data.len() % 16);
    let mut out = Vec::with_capacity(data.len() + pad_len);
    out.extend_from_slice(data);
    out.resize(out.len() + pad_len, pad_len as u8);
    out
}

/// Validates and strips PKCS#7 padding from `data`.
pub fn pkcs7_unpad(data: &[u8]) -> Result<&[u8], CryptoError> {
    if data.is_empty() || !data.len().is_multiple_of(16) {
        return Err(CryptoError::InvalidPadding);
    }
    let pad_len = usize::from(data[data.len() - 1]);
    if pad_len == 0 || pad_len > 16 || pad_len > data.len() {
        return Err(CryptoError::InvalidPadding);
    }
    let split_pos = data.len() - pad_len;
    for &b in &data[split_pos..] {
        if usize::from(b) != pad_len {
            return Err(CryptoError::InvalidPadding);
        }
    }
    Ok(&data[..split_pos])
}

/// Encrypts raw data using AES in CBC mode with a given key and IV.
///
/// `data.len()` must be a multiple of 16.
pub fn aes_cbc_encrypt(key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>, CryptoError> {
    if iv.len() != 16 {
        return Err(CryptoError::InvalidIvLength(iv.len()));
    }
    if !data.len().is_multiple_of(16) {
        return Err(CryptoError::InvalidCiphertextLength(data.len()));
    }

    let cipher = AesCipher::new(key)?;
    let mut out = Vec::with_capacity(data.len());
    let mut prev_block = [0u8; 16];
    prev_block.copy_from_slice(iv);

    for chunk in data.as_chunks::<16>().0 {
        let mut block = [0u8; 16];
        for i in 0..16 {
            block[i] = chunk[i] ^ prev_block[i];
        }
        cipher.encrypt_block(&mut block);
        out.extend_from_slice(&block);
        prev_block = block;
    }

    Ok(out)
}

/// Decrypts raw ciphertext using AES in CBC mode with a given key and IV.
///
/// `ciphertext.len()` must be a multiple of 16.
pub fn aes_cbc_decrypt(key: &[u8], iv: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>, CryptoError> {
    if iv.len() != 16 {
        return Err(CryptoError::InvalidIvLength(iv.len()));
    }
    if !ciphertext.len().is_multiple_of(16) {
        return Err(CryptoError::InvalidCiphertextLength(ciphertext.len()));
    }

    let cipher = AesCipher::new(key)?;
    let mut out = Vec::with_capacity(ciphertext.len());
    let mut prev_block = [0u8; 16];
    prev_block.copy_from_slice(iv);

    for chunk in ciphertext.as_chunks::<16>().0 {
        let mut block = [0u8; 16];
        block.copy_from_slice(chunk);
        cipher.decrypt_block(&mut block);
        for i in 0..16 {
            block[i] ^= prev_block[i];
        }
        out.extend_from_slice(&block);
        prev_block.copy_from_slice(chunk);
    }

    Ok(out)
}

/// High-level AES-256-CBC encryption with PKCS#7 padding:
/// Prepend 16-byte IV, then encrypt padded plaintext.
#[must_use]
pub fn encrypt_aes_256_cbc_with_key(plaintext: &[u8], key: &[u8; 32], iv: &[u8; 16]) -> Vec<u8> {
    let padded = pkcs7_pad(plaintext);
    let ciphertext = aes_cbc_encrypt(key, iv, &padded).expect("valid padded block");
    let mut combined = Vec::with_capacity(16 + ciphertext.len());
    combined.extend_from_slice(iv);
    combined.extend_from_slice(&ciphertext);
    combined
}

/// High-level AES-256-CBC decryption with PKCS#7 padding:
/// First 16 bytes are the IV; remaining bytes are the ciphertext.
pub fn decrypt_aes_256_cbc_with_key(
    payload: &[u8],
    key: &[u8; 32],
) -> Result<Vec<u8>, CryptoError> {
    if payload.len() < 32 || !(payload.len() - 16).is_multiple_of(16) {
        return Err(CryptoError::InvalidCiphertextLength(payload.len()));
    }
    let iv = &payload[..16];
    let ciphertext = &payload[16..];
    let decrypted = aes_cbc_decrypt(key, iv, ciphertext)?;
    let unpadded = pkcs7_unpad(&decrypted)?;
    Ok(unpadded.to_vec())
}

/// Calculates Telecom GM Host MAC:
/// Encrypt challenge with session key (zero IV), XOR first and second 8 bytes, take 4 bytes.
pub fn calc_host_mac(session_key: &[u8], challenge: &[u8]) -> Result<[u8; 4], CryptoError> {
    let ct = aes_cbc_encrypt(session_key, &[0u8; 16], challenge)?;
    if ct.len() < 16 {
        return Err(CryptoError::InvalidCiphertextLength(ct.len()));
    }
    let mut res = [0u8; 4];
    for i in 0..4 {
        res[i] = ct[i] ^ ct[i + 8];
    }
    Ok(res)
}

// ============================================================================
// AES-CMAC (NIST SP 800-38B / RFC 4493)
// ============================================================================

#[inline]
fn shift_left_128(input: &[u8; 16]) -> [u8; 16] {
    let mut out = [0u8; 16];
    let mut carry = 0u8;
    for i in (0..16).rev() {
        let b = input[i];
        out[i] = (b << 1) | carry;
        carry = (b & 0x80) >> 7;
    }
    out
}

/// Generates AES-CMAC subkeys $K_1$ and $K_2$ per NIST SP 800-38B.
fn generate_cmac_subkeys(cipher: &AesCipher) -> ([u8; 16], [u8; 16]) {
    let mut l = [0u8; 16];
    cipher.encrypt_block(&mut l);

    let msb_l = (l[0] & 0x80) != 0;
    let mut k1 = shift_left_128(&l);
    if msb_l {
        k1[15] ^= 0x87;
    }

    let msb_k1 = (k1[0] & 0x80) != 0;
    let mut k2 = shift_left_128(&k1);
    if msb_k1 {
        k2[15] ^= 0x87;
    }

    (k1, k2)
}

/// Computes the 128-bit (16 bytes) AES-CMAC message authentication code.
///
/// Implemented per NIST SP 800-38B and RFC 4493 for GlobalPlatform SCP03 mutual authentication.
pub fn aes_cmac(key: &[u8], data: &[u8]) -> Result<[u8; 16], CryptoError> {
    let cipher = AesCipher::new(key)?;
    let (k1, k2) = generate_cmac_subkeys(&cipher);

    let n = data.len().div_ceil(16);
    let num_blocks = if n == 0 { 1 } else { n };
    let is_complete = !data.is_empty() && data.len().is_multiple_of(16);

    let mut state = [0u8; 16];

    for i in 0..(num_blocks - 1) {
        let chunk = &data[i * 16..(i + 1) * 16];
        for j in 0..16 {
            state[j] ^= chunk[j];
        }
        cipher.encrypt_block(&mut state);
    }

    let mut last_block = [0u8; 16];
    if is_complete {
        let chunk = &data[(num_blocks - 1) * 16..];
        last_block.copy_from_slice(chunk);
        for j in 0..16 {
            last_block[j] ^= k1[j];
        }
    } else {
        let offset = (num_blocks - 1) * 16;
        let rem = &data[offset..];
        last_block[..rem.len()].copy_from_slice(rem);
        last_block[rem.len()] = 0x80;
        for j in 0..16 {
            last_block[j] ^= k2[j];
        }
    }

    for j in 0..16 {
        state[j] ^= last_block[j];
    }
    cipher.encrypt_block(&mut state);

    Ok(state)
}

/// Computes the 8-byte (64-bit) truncated AES-CMAC used in GlobalPlatform SCP03 C-MAC and cryptograms.
pub fn aes_cmac_8(key: &[u8], data: &[u8]) -> Result<[u8; 8], CryptoError> {
    let full = aes_cmac(key, data)?;
    let mut out = [0u8; 8];
    out.copy_from_slice(&full[..8]);
    Ok(out)
}

// ============================================================================
// Standard Base64 Codec
// ============================================================================

const BASE64_CHARS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Encodes `data` into standard base64 string.
#[must_use]
pub fn base64_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0];
        let b1 = if chunk.len() > 1 { chunk[1] } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] } else { 0 };

        let idx0 = usize::from(b0 >> 2);
        let idx1 = usize::from(((b0 & 0x03) << 4) | (b1 >> 4));
        let idx2 = usize::from(((b1 & 0x0F) << 2) | (b2 >> 6));
        let idx3 = usize::from(b2 & 0x3F);

        out.push(char::from(BASE64_CHARS[idx0]));
        out.push(char::from(BASE64_CHARS[idx1]));
        if chunk.len() > 1 {
            out.push(char::from(BASE64_CHARS[idx2]));
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(char::from(BASE64_CHARS[idx3]));
        } else {
            out.push('=');
        }
    }
    out
}

/// Decodes standard base64 string into raw bytes.
pub fn base64_decode(input: &str) -> Result<Vec<u8>, CryptoError> {
    let clean: Vec<u8> = input
        .bytes()
        .filter(|&b| !b.is_ascii_whitespace())
        .collect();
    if clean.is_empty() {
        return Ok(Vec::new());
    }
    if !clean.len().is_multiple_of(4) {
        return Err(CryptoError::InvalidBase64);
    }

    let mut out = Vec::with_capacity((clean.len() / 4) * 3);
    for chunk in clean.as_chunks::<4>().0 {
        let mut val = 0u32;
        let mut pad_count = 0;
        for (i, &b) in chunk.iter().enumerate() {
            if b == b'=' {
                pad_count += 1;
                val <<= 6;
            } else {
                let sextet = match b {
                    b'A'..=b'Z' => b - b'A',
                    b'a'..=b'z' => b - b'a' + 26,
                    b'0'..=b'9' => b - b'0' + 52,
                    b'+' => 62,
                    b'/' => 63,
                    _ => return Err(CryptoError::InvalidBase64),
                };
                val = (val << 6) | u32::from(sextet);
            }
            if pad_count > 0 && b != b'=' && i > 0 {
                return Err(CryptoError::InvalidBase64);
            }
        }

        match pad_count {
            0 => {
                out.push((val >> 16) as u8);
                out.push((val >> 8) as u8);
                out.push(val as u8);
            }
            1 => {
                out.push((val >> 16) as u8);
                out.push((val >> 8) as u8);
            }
            2 => {
                out.push((val >> 16) as u8);
            }
            _ => return Err(CryptoError::InvalidBase64),
        }
    }

    Ok(out)
}
