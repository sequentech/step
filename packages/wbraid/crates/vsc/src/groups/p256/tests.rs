// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 Free & Fair
// See LICENSE.md for details

//! P-256 module tests

use super::*;
use crate::context::Context;
use crate::context::P256Ctx as Ctx;
use crate::traits::groups::CryptographicGroup;
use crate::traits::groups::GroupElement;
use crate::traits::groups::GroupScalar;
use crate::utils::serialization::{VDeserializable, VSerializable};

#[test]
fn test_p256_scalar_from_u32() {
    use rand::RngExt;
    let u: u32 = Ctx::get_rng().random();

    let _scalar: P256Scalar = u.into();
    let one: P256Scalar = 1u32.into();
    assert_eq!(one, P256Scalar::one());
}

#[test]
fn test_p256_scalar_negation() {
    let s1 = Ctx::random_scalar();
    let s_neg = s1.neg();

    assert_eq!(
        s_neg.add(&s1),
        P256Scalar::zero(),
        "Negation property s + (-s) = 0 failed"
    );
}

#[test]
fn test_p256_scalar_inversion() {
    let s = Ctx::random_scalar();

    if s != P256Scalar::zero() {
        let s_inv = s.inv().unwrap();
        let product = s.mul(&s_inv);
        assert_eq!(product, P256Scalar::one(), "s * s_inv = 1 property failed");
    }

    let zero = P256Scalar::zero();
    assert!(zero.inv().is_none(), "Inversion of zero must be None");
}

#[test]
fn test_p256_element_power_product() {
    let s1 = Ctx::random_scalar();
    let s2 = Ctx::random_scalar();
    let g = Ctx::generator();

    let e1 = g.exp(&s1);
    let e2 = g.exp(&s2);
    let e3_sum = e1.mul(&e2);

    let s_sum = s1.add(&s2);
    let e3_expected = g.exp(&s_sum);

    assert_eq!(
        e3_sum, e3_expected,
        "Element addition failed: e1+e2 != (s1+s2)*G"
    );
}

#[test]
fn test_p256_element_inv() {
    let s = Ctx::random_scalar();
    let g = Ctx::generator();
    let e = g.exp(&s);

    let e_neg = e.inv();
    let e_plus_e_neg = e.mul(&e_neg);

    assert_eq!(
        e_plus_e_neg,
        P256Element::one(),
        "Element negation failed: e + (-e) != Id"
    );

    let s_neg = s.neg();
    let e_neg_expected = g.exp(&s_neg);
    assert_eq!(
        e_neg, e_neg_expected,
        "Element negation failed: (-s)*G != -(s*G)"
    );
}

#[test]
fn test_p256_element_power_power() {
    let s1 = Ctx::random_scalar();
    let s2 = Ctx::random_scalar();
    let g = Ctx::generator();

    let e1 = g.exp(&s1);
    let e2 = e1.exp(&s2);

    let s_prod = s1.mul(&s2);
    let e_expected = g.exp(&s_prod);

    assert_eq!(e2, e_expected, "Element scalar multiplication failed");
}

#[test]
fn test_p256_element_identity_properties() {
    let s = Ctx::random_scalar();
    let g = Ctx::generator();
    let e = g.exp(&s);
    let id = P256Element::one();

    assert_eq!(e.mul(&id), e, "e + Id != e");
    assert_eq!(id.mul(&e), e, "Id + e != e");

    let zero_scalar = P256Scalar::zero();
    assert_eq!(g.exp(&zero_scalar), id, "G^0 != Id");
}

#[test]
fn test_p256_element_mul_commutativity() {
    let e1 = Ctx::random_element();
    let e2 = Ctx::random_element();

    let sum1 = e1.mul(&e2);
    let sum2 = e2.mul(&e1);

    assert_eq!(sum1, sum2, "Element multiplication is not commutative");
}

#[test]
fn test_p256_element_mul_associativity() {
    let e1 = Ctx::random_element();
    let e2 = Ctx::random_element();
    let e3 = Ctx::random_element();

    let sum_left_assoc = (e1.mul(&e2)).mul(&e3);
    let sum_right_assoc = e1.mul(&(e2.mul(&e3)));

    assert_eq!(
        sum_left_assoc, sum_right_assoc,
        "Element multiplication is not associative"
    );
}

#[test]
fn test_p256_scalar_element_addition_distributivity() {
    let s_op = Ctx::random_scalar();

    let e1 = Ctx::random_element();
    let e2 = Ctx::random_element();

    // (e1 * e2)^s
    let sum_elements = e1.mul(&e2);
    let lhs = sum_elements.exp(&s_op);

    // (e1^s) * (e2^s)
    let term1 = e1.exp(&s_op);
    let term2 = e2.exp(&s_op);
    let rhs = term1.mul(&term2);

    assert_eq!(lhs, rhs, "Distributivity (e1*e2)^s = e1^s * e2^s failed");
}

#[test]
fn test_p256_scalar_element_mul_distributivity() {
    let s1 = Ctx::random_scalar();
    let s2 = Ctx::random_scalar();

    let e = Ctx::random_element();

    let sum_scalars = s1.add(&s2);
    let lhs = e.exp(&sum_scalars);

    let term1 = e.exp(&s1);
    let term2 = e.exp(&s2);
    let rhs = term1.mul(&term2);

    assert_eq!(lhs, rhs, "Distributivity e^(s1+s2) = e^s1 + e^s2 failed");
}

#[test]
fn test_p256_group_hash_to_scalar() {
    let input1 = b"some input data";
    let input2 = b"other input data";
    let ds_tag = b"ds tag";

    let s1 = P256Group::hash_to_scalar(&[input1], &[ds_tag]).unwrap();
    // Same input, same output
    let s2 = P256Group::hash_to_scalar(&[input1], &[ds_tag]).unwrap();
    // Different input, different output
    let s3 = P256Group::hash_to_scalar(&[input2], &[ds_tag]).unwrap();

    assert_eq!(s1, s2, "Hash to scalar not equal for equal input");
    assert_ne!(
        s1, s3,
        "Hash to scalar produces same output for different inputs"
    );
}

#[test]
fn test_p256_element_serialization() {
    let s = Ctx::random_scalar();
    let g = Ctx::generator();
    let e_orig = g.exp(&s);

    let serialized_e = e_orig.ser();
    assert_eq!(serialized_e.len(), 33, "Serialized element length mismatch");

    let e_deserialized = P256Element::deser(&serialized_e).unwrap();
    assert_eq!(
        e_orig, e_deserialized,
        "Original and deserialized elements do not match"
    );

    let e_id = P256Element::one();
    let ser_id = e_id.ser();
    let des_id = P256Element::deser(&ser_id).unwrap();
    assert_eq!(e_id, des_id);

    // test wrong length
    let bytes = [0u8; 40];
    let result = P256Element::deser(&bytes);
    assert!(result.is_err());

    // test bad data
    // this array of bytes does not correspond to a point
    let bytes = [1u8; 33];
    let result = P256Element::deser(&bytes);
    assert!(result.is_err());
}

#[test]
fn test_p256_scalar_serialization() {
    let s_orig = Ctx::random_scalar();

    let serialized_s = s_orig.ser();
    assert_eq!(serialized_s.len(), 32, "Serialized scalar length mismatch");

    let s_deserialized = P256Scalar::deser(&serialized_s).unwrap();
    assert_eq!(
        s_orig, s_deserialized,
        "Original and deserialized scalars do not match"
    );

    let s_zero = P256Scalar::zero();
    let ser_zero = s_zero.ser();
    let des_zero = P256Scalar::deser(&ser_zero).unwrap();
    assert_eq!(s_zero, des_zero);

    let s_one = P256Scalar::one();
    let ser_one = s_one.ser();
    let des_one = P256Scalar::deser(&ser_one).unwrap();
    assert_eq!(s_one, des_one);

    // test wrong length
    let bytes = [0u8; 40];
    let result = P256Scalar::deser(&bytes);
    assert!(result.is_err());
}

#[test]
fn test_p256_hash_to_element_different_inputs() {
    let input1 = b"test input 1";
    let input2 = b"test input 2";
    let tag = b"domain_sep";

    let elem1 = P256Group::hash_to_element(&[input1], &[tag]).unwrap();
    let elem2 = P256Group::hash_to_element(&[input2], &[tag]).unwrap();
    let elem3 = P256Group::hash_to_element(&[input1], &[tag]).unwrap();

    // Different inputs should yield different elements
    assert_ne!(elem1, elem2);
    // Same input should yield the same element
    assert_eq!(elem1, elem3);
}

#[test]
fn test_p256_hash_to_element_different_tags() {
    let input = b"test input";
    let tag1 = b"tag1";
    let tag2 = b"tag2";

    let elem1 = P256Group::hash_to_element(&[input], &[tag1]).unwrap();
    let elem2 = P256Group::hash_to_element(&[input], &[tag2]).unwrap();

    assert_ne!(elem1, elem2);
}

#[test]
fn test_p256_hash_to_element_empty_input() {
    let h = P256Group::hash_to_element(&[], &[]);
    assert!(h.is_err())
}

#[test]
fn test_p256_hash_to_scalar_empty_input() {
    let h = P256Group::hash_to_scalar(&[], &[]);
    assert!(h.is_err())
}

///////////////////////////////////////////////////////////////////////////
// Byte/scalar encoding (the P-256 counterpart of the Ristretto codec)
///////////////////////////////////////////////////////////////////////////

mod encoding {
    use crate::context::Context;
    use crate::groups::p256::group::P256Group;
    use crate::traits::groups::{CryptographicGroup, GroupElement};

    #[test]
    fn thirty_bytes_round_trip() {
        for pattern in [0x00u8, 0x01, 0x7f, 0x80, 0xff] {
            let input = [pattern; 30];
            let element = P256Group::encode_30_bytes(&input).expect("encode");
            let output = P256Group::decode_30_bytes(&element).expect("decode");
            assert_eq!(input, output, "payload must survive the embedding");
        }
    }

    #[test]
    fn encoding_is_deterministic() {
        let input = [0x5au8; 30];
        let a = P256Group::encode_30_bytes(&input).unwrap();
        let b = P256Group::encode_30_bytes(&input).unwrap();
        assert!(a.equals(&b), "the same payload must give the same point");
    }

    #[test]
    fn distinct_payloads_give_distinct_points() {
        let a = P256Group::encode_30_bytes(&[0u8; 30]).unwrap();
        let mut other = [0u8; 30];
        other[29] = 1;
        let b = P256Group::encode_30_bytes(&other).unwrap();
        assert!(!a.equals(&b));
    }

    #[test]
    fn random_payloads_round_trip() {
        use rand::Rng;
        let mut rng = rand::rng();
        for _ in 0..100 {
            let mut input = [0u8; 30];
            rng.fill_bytes(&mut input);
            let element = P256Group::encode_30_bytes(&input).unwrap();
            assert_eq!(input, P256Group::decode_30_bytes(&element).unwrap());
        }
    }

    #[test]
    fn scalars_round_trip_through_two_elements() {
        for _ in 0..50 {
            let scalar = crate::context::P256Ctx::random_scalar();
            let elements = P256Group::encode_scalar(&scalar).expect("encode scalar");
            let back = P256Group::decode_scalar(&elements).expect("decode scalar");
            assert_eq!(scalar, back);
        }
    }

    /// The DKG share path: encrypt a scalar to a public key and recover it.
    #[test]
    fn encrypted_scalars_round_trip() {
        use crate::cryptosystem::elgamal::KeyPair;
        for _ in 0..20 {
            let keypair = KeyPair::<crate::context::P256Ctx>::generate();
            let scalar = crate::context::P256Ctx::random_scalar();

            let ciphertext =
                P256Group::encrypt_scalar(&scalar, &keypair.pkey.y).expect("encrypt");
            let recovered =
                P256Group::decrypt_scalar(&ciphertext, &keypair.skey).expect("decrypt");

            assert_eq!(scalar, recovered, "DKG share must survive encryption");
        }
    }

    #[test]
    fn byte_arrays_round_trip_at_both_supported_sizes() {
        let single = [0x33u8; 30];
        let encoded = P256Group::encode_bytes::<30, 1>(&single).unwrap();
        assert_eq!(P256Group::decode_bytes::<1, 30>(&encoded).unwrap(), single);

        let double = [0x77u8; 32];
        let encoded = P256Group::encode_bytes::<32, 2>(&double).unwrap();
        assert_eq!(P256Group::decode_bytes::<2, 32>(&encoded).unwrap(), double);
    }
}
