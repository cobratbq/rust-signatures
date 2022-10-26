#![allow(deprecated)]

use digest::Digest;
use dsa::{Components, KeySize, Signature, SigningKey};
use pkcs8::der::{Decode, Encode};
use rand::{CryptoRng, RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;
use sha2::Sha256;
use signature::{DigestVerifier, RandomizedDigestSigner, hazmat::{PrehashSigner, PrehashVerifier}, Signer, Verifier};

/// Seed used for the ChaCha8 RNG
const SEED: u64 = 0x2103_1949;

/// Message to be signed/verified
const MESSAGE: &[u8] = b"test";

/// Message signed by this crate using the keys generated by this CSPRNG
///
/// This signature was generated using the keys generated by this CSPRNG (the per-message `k` component was also generated using the CSPRNG)
const MESSAGE_SIGNATURE_CRATE_ASN1: &[u8] = &[
    0x30, 0x2C, 0x02, 0x14, 0x45, 0x1D, 0xE5, 0x76, 0x21, 0xD8, 0xFD, 0x76, 0xC1, 0x6F, 0x45, 0x4E,
    0xDE, 0x5F, 0x09, 0x79, 0x76, 0x52, 0xF3, 0xA5, 0x02, 0x14, 0x53, 0x60, 0xE6, 0xB7, 0xF0, 0xCF,
    0xAE, 0x49, 0xB1, 0x58, 0x5C, 0xCF, 0x5F, 0x3F, 0x94, 0x49, 0x21, 0xA0, 0xBF, 0xD2,
];

/// Message signed by OpenSSL using the keys generated by this CSPRNG
///
/// This signature was generated using the SHA-256 digest
const MESSAGE_SIGNATURE_OPENSSL_ASN1: &[u8] = &[
    0x30, 0x2C, 0x02, 0x14, 0x6D, 0xB3, 0x8E, 0xAF, 0x97, 0x13, 0x7E, 0x07, 0xFF, 0x24, 0xB8, 0x66,
    0x97, 0x18, 0xE1, 0x6F, 0xD7, 0x9A, 0x28, 0x2D, 0x02, 0x14, 0x47, 0x8C, 0x0B, 0x96, 0x51, 0x08,
    0x08, 0xC8, 0x34, 0x9D, 0x0D, 0x41, 0xC7, 0x73, 0x0F, 0xB5, 0x9C, 0xBB, 0x00, 0x34,
];

/// Get the seeded CSPRNG
fn seeded_csprng() -> impl CryptoRng + RngCore {
    ChaCha8Rng::seed_from_u64(SEED)
}

/// Generate a DSA keypair using a seeded CSPRNG
fn generate_deterministic_keypair() -> SigningKey {
    let mut rng = seeded_csprng();
    let components = Components::generate(&mut rng, KeySize::DSA_1024_160);
    SigningKey::generate(&mut rng, components)
}

#[test]
fn decode_encode_signature() {
    let signature_openssl =
        Signature::from_der(MESSAGE_SIGNATURE_OPENSSL_ASN1).expect("Failed to decode signature");
    let encoded_signature_openssl = signature_openssl
        .to_vec()
        .expect("Failed to encode signature");

    assert_eq!(MESSAGE_SIGNATURE_OPENSSL_ASN1, encoded_signature_openssl);

    let signature_crate =
        Signature::from_der(MESSAGE_SIGNATURE_CRATE_ASN1).expect("Failed to decode signature");
    let encoded_signature_crate = signature_crate
        .to_vec()
        .expect("Failed to encode signature");

    assert_eq!(MESSAGE_SIGNATURE_CRATE_ASN1, encoded_signature_crate);
}

#[test]
fn sign_message() {
    let signing_key = generate_deterministic_keypair();
    let generated_signature =
        signing_key.sign_digest_with_rng(seeded_csprng(), Sha256::new().chain_update(MESSAGE));

    let expected_signature =
        Signature::from_der(MESSAGE_SIGNATURE_CRATE_ASN1).expect("Failed to decode signature");

    assert_eq!(generated_signature, expected_signature);
}

#[test]
fn verify_signature() {
    let signing_key = generate_deterministic_keypair();
    let verifying_key = signing_key.verifying_key();

    let signature = Signature::from_der(MESSAGE_SIGNATURE_OPENSSL_ASN1)
        .expect("Failed to parse ASN.1 representation of the test signature");

    assert!(verifying_key
        .verify_digest(Sha256::new().chain_update(MESSAGE), &signature)
        .is_ok());
}

#[test]
fn signiger_verifier_signature() {
    let signing_key = generate_deterministic_keypair();
    let verifying_key = signing_key.verifying_key();
    let message = b"Hello world! This is the message signed as part of the testing process.";

    // construct signature manually and by `Signer` defaults. Ensure results are identical.
    let manual_digest = Sha256::new_with_prefix(message).finalize();
    let manual_signature = signing_key.sign_prehash(&manual_digest).unwrap();
    let signer_signature = signing_key.sign(message);
    verifying_key.verify(message, &manual_signature).unwrap();
    verifying_key.verify(message, &signer_signature).unwrap();
    assert_eq!(manual_signature, signer_signature);

    // verify signature manually and by `Verifier` defaults. Ensure signatures can be applied interchangeably.
    verifying_key.verify_prehash(&manual_digest, &manual_signature).unwrap();
    verifying_key.verify_prehash(&manual_digest, &signer_signature).unwrap();
    verifying_key.verify(message, &manual_signature).unwrap();
    verifying_key.verify(message, &signer_signature).unwrap();
}