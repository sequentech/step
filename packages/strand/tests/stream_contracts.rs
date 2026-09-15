// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Wire layout and fallible streams are separate contracts. Small containers
//! pin literal bytes; generated cryptographic records retain valid-use controls.

use borsh::{BorshDeserialize, BorshSerialize};
use std::io::{self, Read, Write};
use strand::backend::ristretto::RistrettoCtx;
use strand::context::{Ctx, Element};
use strand::elgamal::{Ciphertext, PrivateKey, PublicKey};
use strand::serialization::{StrandDeserialize, StrandSerialize, StrandVector};
use strand::shuffler_product::StrandRectangle;
#[cfg(not(feature = "openssl_full"))]
use strand::signature::{
    StrandSignature, StrandSignaturePk, StrandSignatureSk,
};
use strand::util::StrandError;
use strand::zkp::{ChaumPedersen, Schnorr, Zkp};

const SINK_ERROR: &str = "synthetic sink failure";

struct FailingWriter(usize);

impl Write for FailingWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.is_empty() {
            return Ok(0);
        }
        if self.0 == 0 {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                SINK_ERROR,
            ));
        }
        let written = self.0.min(bytes.len());
        self.0 -= written;
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn stream_contract<T: BorshSerialize + BorshDeserialize>(
    value: &T,
    bytes: &[u8],
) -> T {
    assert!(!bytes.is_empty());
    assert_eq!(value.strand_serialize().unwrap(), bytes);
    for boundary in 0..bytes.len() {
        let error = value.serialize(&mut FailingWriter(boundary)).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
        assert_eq!(error.to_string(), SINK_ERROR);
        assert!(
            T::strand_deserialize(&bytes[..boundary]).is_err(),
            "accepted prefix {boundary}"
        );
    }
    // The failing sink must also accept a complete record when capacity permits.
    value.serialize(&mut FailingWriter(bytes.len())).unwrap();
    let mut trailing = bytes.to_vec();
    trailing.push(0);
    assert!(T::strand_deserialize(&trailing).is_err());
    T::strand_deserialize(bytes).unwrap()
}

#[test]
fn parallel_vector_wire_format_preserves_element_boundaries() {
    // Outer count, then a byte-length and little-endian u16 for each element.
    let expected = [2, 0, 0, 0, 2, 0, 0, 0, 0x34, 0x12, 2, 0, 0, 0, 0xcd, 0xab];
    let value = StrandVector(vec![0x1234_u16, 0xabcd]);
    assert_eq!(stream_contract(&value, &expected).0, [0x1234, 0xabcd]);
    assert!(stream_contract(&StrandVector::<u16>(vec![]), &[0, 0, 0, 0])
        .0
        .is_empty());
}

#[test]
fn rectangle_wire_format_preserves_row_lengths_and_order() {
    // Two separately encoded Vec<u8> rows, each containing two cells.
    let expected = [
        2, 0, 0, 0, 6, 0, 0, 0, 2, 0, 0, 0, 11, 12, 6, 0, 0, 0, 2, 0, 0, 0, 21,
        22,
    ];
    let value =
        StrandRectangle::new(vec![vec![11_u8, 12], vec![21, 22]]).unwrap();
    assert_eq!(
        stream_contract(&value, &expected).rows(),
        &vec![vec![11, 12], vec![21, 22]]
    );
}

/// A fallible element exercises the generic container's inner error path,
/// independently of failures while writing the completed outer buffer.
struct FallibleByte(u8);

impl BorshSerialize for FallibleByte {
    fn serialize<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        if self.0 == 255 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "refused element",
            ));
        }
        self.0.serialize(writer)
    }
}

impl BorshDeserialize for FallibleByte {
    fn deserialize_reader<R: Read>(reader: &mut R) -> io::Result<Self> {
        let value = u8::deserialize_reader(reader)?;
        if value == 255 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "refused element",
            ));
        }
        Ok(Self(value))
    }
}

fn assert_element_error(error: StrandError, kind: io::ErrorKind) {
    match error {
        StrandError::SerializationError(source) => {
            assert_eq!(source.kind(), kind);
            assert_eq!(source.to_string(), "refused element");
        }
        other => panic!("lost the element's IO error: {other}"),
    }
}

#[test]
fn vector_propagates_inner_element_errors_without_accepting_a_partial_result() {
    let expected = [3, 0, 0, 0, 1, 0, 0, 0, 10, 1, 0, 0, 0, 20, 1, 0, 0, 0, 30];
    let values = || {
        StrandVector(vec![FallibleByte(10), FallibleByte(20), FallibleByte(30)])
    };
    assert_eq!(
        stream_contract(&values(), &expected)
            .0
            .iter()
            .map(|v| v.0)
            .collect::<Vec<_>>(),
        [10, 20, 30]
    );
    for (element, offset) in [(0, 8), (1, 13), (2, 18)] {
        let mut invalid = values();
        invalid.0[element].0 = 255;
        assert_element_error(
            invalid.strand_serialize().unwrap_err(),
            io::ErrorKind::InvalidInput,
        );
        let mut invalid = expected;
        invalid[offset] = 255;
        let error = StrandVector::<FallibleByte>::strand_deserialize(&invalid)
            .err()
            .unwrap();
        assert_element_error(error, io::ErrorKind::InvalidData);
    }
}

#[test]
fn rectangle_propagates_inner_row_errors_without_accepting_a_partial_result() {
    let expected = [
        2, 0, 0, 0, 5, 0, 0, 0, 1, 0, 0, 0, 10, 5, 0, 0, 0, 1, 0, 0, 0, 20,
    ];
    let values = || {
        StrandRectangle::new(vec![
            vec![FallibleByte(10)],
            vec![FallibleByte(20)],
        ])
        .unwrap()
    };
    let restored = stream_contract(&values(), &expected);
    assert_eq!(
        restored.rows().iter().map(|r| r[0].0).collect::<Vec<_>>(),
        [10, 20]
    );
    for (row, offset) in [(0, 12), (1, 21)] {
        let rows = (0..2)
            .map(|i| vec![FallibleByte(if i == row { 255 } else { 10 })])
            .collect();
        let invalid = StrandRectangle::new(rows).unwrap();
        assert_element_error(
            invalid.strand_serialize().unwrap_err(),
            io::ErrorKind::InvalidInput,
        );
        let mut invalid = expected;
        invalid[offset] = 255;
        let error =
            StrandRectangle::<FallibleByte>::strand_deserialize(&invalid)
                .err()
                .unwrap();
        assert_element_error(error, io::ErrorKind::InvalidData);
    }
}

#[test]
fn scalar_and_neutral_ciphertext_have_fixed_width_encodings() {
    let ctx = RistrettoCtx;
    let scalar = ctx.exp_from_u64(11);
    let mut scalar_bytes = [0; 32];
    scalar_bytes[0] = 11;
    assert_eq!(stream_contract(&scalar, &scalar_bytes), scalar);
    let identity = <RistrettoCtx as Ctx>::E::mul_identity();
    let ciphertext = Ciphertext::<RistrettoCtx> {
        mhr: identity.clone(),
        gr: identity.clone(),
    };
    let restored = stream_contract(&ciphertext, &[0; 64]);
    let key = PrivateKey::from(&ctx.exp_from_u64(7), &ctx);
    assert_eq!(key.decrypt(&restored), identity);
}

#[test]
fn serialized_elgamal_keys_retain_their_decryption_contract() {
    let ctx = RistrettoCtx;
    let secret = PrivateKey::from(&ctx.exp_from_u64(7), &ctx);
    let public = secret.get_pk();
    let message = ctx.gmod_pow(&ctx.exp_from_u64(13));
    let ciphertext = public.encrypt(&message);
    assert_eq!(secret.decrypt(&ciphertext), message);
    let restored_secret: PrivateKey<RistrettoCtx> =
        stream_contract(&secret, &secret.strand_serialize().unwrap());
    let restored_public: PublicKey<RistrettoCtx> =
        stream_contract(&public, &public.strand_serialize().unwrap());
    assert_eq!(restored_public.element(), public.element());
    assert_eq!(restored_secret.decrypt(&ciphertext), message);
    assert_eq!(secret.decrypt(&restored_public.encrypt(&message)), message);
}

#[test]
#[cfg(not(feature = "openssl_full"))]
fn serialized_signatures_and_public_keys_still_verify_the_original_message() {
    let secret = StrandSignatureSk::r#gen().unwrap();
    let public = StrandSignaturePk::from_sk(&secret).unwrap();
    let signature = secret.sign(b"stream contract").unwrap();
    public.verify(&signature, b"stream contract").unwrap();
    let restored_public: StrandSignaturePk =
        stream_contract(&public, &public.strand_serialize().unwrap());
    let restored_signature: StrandSignature =
        stream_contract(&signature, &signature.to_bytes());
    restored_public
        .verify(&restored_signature, b"stream contract")
        .unwrap();
    assert!(restored_public
        .verify(&restored_signature, b"changed")
        .is_err());
}

#[test]
fn serialized_proofs_keep_statement_and_context_binding() {
    let ctx = RistrettoCtx;
    let zkp = Zkp::new(&ctx);
    let secret = ctx.exp_from_u64(7);
    let public = ctx.gmod_pow(&secret);
    let second_base = ctx.gmod_pow(&ctx.exp_from_u64(11));
    let second_public = ctx.emod_pow(&second_base, &secret);
    let schnorr = zkp
        .schnorr_prove(&secret, &public, None, b"context")
        .unwrap();
    assert!(zkp.schnorr_verify(&public, None, &schnorr, b"context"));
    let restored: Schnorr<RistrettoCtx> =
        stream_contract(&schnorr, &schnorr.strand_serialize().unwrap());
    assert!(zkp.schnorr_verify(&public, None, &restored, b"context"));
    assert!(!zkp.schnorr_verify(&public, None, &restored, b"other"));
    let equality = zkp
        .cp_prove(
            &secret,
            &public,
            &second_public,
            None,
            &second_base,
            b"context",
        )
        .unwrap();
    assert!(zkp.cp_verify(
        &public,
        &second_public,
        None,
        &second_base,
        &equality,
        b"context"
    ));
    let restored: ChaumPedersen<RistrettoCtx> =
        stream_contract(&equality, &equality.strand_serialize().unwrap());
    assert!(zkp.cp_verify(
        &public,
        &second_public,
        None,
        &second_base,
        &restored,
        b"context"
    ));
    assert!(!zkp.cp_verify(
        &public,
        &second_public,
        None,
        &second_base,
        &restored,
        b"other"
    ));
}

#[test]
#[cfg(not(any(feature = "openssl_core", feature = "openssl_full")))]
fn encrypted_data_streams_preserve_authenticated_plaintext() {
    let key = strand::symm::sk_from_bytes(&[42; 32]).unwrap();
    let value = strand::symm::encrypt(key, b"synthetic trustee share").unwrap();
    assert_eq!(
        strand::symm::decrypt(&key, &value).unwrap(),
        b"synthetic trustee share"
    );
    let restored = stream_contract(&value, &value.strand_serialize().unwrap());
    assert_eq!(
        strand::symm::decrypt(&key, &restored).unwrap(),
        b"synthetic trustee share"
    );
}

#[test]
#[cfg(not(any(feature = "openssl_core", feature = "openssl_full")))]
fn hash_wrapper_streams_preserve_all_sixty_four_bytes() {
    let expected = std::array::from_fn::<_, 64, _>(|i| i as u8);
    let value = strand::hash::HashWrapper::new(expected);
    assert_eq!(stream_contract(&value, &expected).into_inner(), expected);
}

#[test]
fn single_shuffle_proof_streams_preserve_a_valid_proof() {
    let ctx = RistrettoCtx;
    let public = PrivateKey::from(&ctx.exp_from_u64(7), &ctx).get_pk();
    let original = strand::util::random_ciphertexts(2, &ctx);
    let generators = ctx.generators(3, b"stream generators").unwrap();
    let shuffler = strand::shuffler::Shuffler::new(&public, &ctx);
    let (shuffled, randomness, permutation) = shuffler.gen_shuffle(&original);
    let proof = shuffler
        .gen_proof(
            original.clone(),
            &shuffled,
            randomness,
            generators.clone(),
            permutation,
            b"context",
        )
        .unwrap();
    assert!(shuffler
        .check_proof(
            &proof,
            original.clone(),
            shuffled.clone(),
            generators.clone(),
            b"context"
        )
        .unwrap());
    let restored = stream_contract(&proof, &proof.strand_serialize().unwrap());
    assert!(shuffler
        .check_proof(
            &restored,
            original.clone(),
            shuffled.clone(),
            generators.clone(),
            b"context"
        )
        .unwrap());
    assert!(!shuffler
        .check_proof(&restored, original, shuffled, generators, b"other")
        .unwrap());
}

#[test]
fn product_shuffle_proof_streams_preserve_a_valid_proof() {
    let ctx = RistrettoCtx;
    let public = PrivateKey::from(&ctx.exp_from_u64(7), &ctx).get_pk();
    let original = strand::util::random_product_ciphertexts(2, 2, &ctx);
    let generators = ctx.generators(3, b"stream generators").unwrap();
    let shuffler =
        strand::shuffler_product::Shuffler::new(&public, &generators, &ctx);
    let (shuffled, randomness, permutation) = shuffler.gen_shuffle(&original);
    let proof = shuffler
        .gen_proof(&original, &shuffled, randomness, &permutation, b"context")
        .unwrap();
    assert!(shuffler
        .check_proof(&proof, &original, &shuffled, b"context")
        .unwrap());
    let restored = stream_contract(&proof, &proof.strand_serialize().unwrap());
    assert!(shuffler
        .check_proof(&restored, &original, &shuffled, b"context")
        .unwrap());
    assert!(!shuffler
        .check_proof(&restored, &original, &shuffled, b"other")
        .unwrap());
}

#[cfg(any(feature = "openssl_core", feature = "openssl_full"))]
#[test]
fn aes_streams_preserve_the_tag_iv_and_authenticated_context() {
    let key = strand::symm::sk_from_bytes(&[42; 32]).unwrap();
    let value =
        strand::symm::encrypt(key, b"trustee share", b"context").unwrap();
    let restored = stream_contract(&value, &value.strand_serialize().unwrap());
    assert_eq!(
        strand::symm::decrypt(&key, &restored, b"context").unwrap(),
        b"trustee share"
    );
    assert!(strand::symm::decrypt(&key, &restored, b"other context").is_err());
}

#[cfg(feature = "openssl_full")]
#[test]
fn p384_streams_preserve_signing_keys_and_reject_partial_der_records() {
    use strand::signature::{StrandSignaturePk, StrandSignatureSk};
    let secret = StrandSignatureSk::r#gen().unwrap();
    let public = StrandSignaturePk::from(&secret).unwrap();
    let signature = secret.sign(b"trustee message").unwrap();
    // DER payloads have a Borsh u32 byte-length prefix, independently of the
    // opaque key material. Every prefix and failing writer must be rejected.
    let frame = |der: Vec<u8>| {
        let mut bytes =
            u32::try_from(der.len()).unwrap().to_le_bytes().to_vec();
        bytes.extend(der);
        bytes
    };
    let secret = stream_contract(&secret, &frame(secret.to_der().unwrap()));
    let public = stream_contract(&public, &frame(public.to_der().unwrap()));
    let signature =
        stream_contract(&signature, &frame(signature.to_der().unwrap()));
    public.verify(&signature, b"trustee message").unwrap();
    public
        .verify(&secret.sign(b"restored key").unwrap(), b"restored key")
        .unwrap();
    assert!(public.verify(&signature, b"changed message").is_err());
}
