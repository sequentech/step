// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Small known integers expose byte order and arithmetic independently of the
//! randomized end-to-end protocol suite. Each rejection retains a valid control.
#![cfg(any(feature = "num_bigint", feature = "rug", feature = "malachite"))]

use borsh::{BorshDeserialize, BorshSerialize};
use std::io::{self, Write};
use strand::context::{Ctx, Element, Exponent};
use strand::elgamal::PrivateKey;
use strand::serialization::{StrandDeserialize, StrandSerialize};
use strand::zkp::Zkp;

const NOT_QUADRATIC_RESIDUE: &str = "Not a quadratic residue";

struct FailAfter(usize);
impl Write for FailAfter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.0 == 0 {
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        let count = bytes.len().min(self.0);
        self.0 -= count;
        Ok(count)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

macro_rules! backend {
    ($module:ident, $ctx:ty, $exponent:expr, $element:expr, $plaintext:expr, $upper:expr) => {
        mod $module {
            use super::*;
            type C = $ctx;
            type E = <C as Ctx>::E;
            type X = <C as Ctx>::X;
            type P = <C as Ctx>::P;
            // Proof labels bind a key proof to one election.
            const ELECTION_A: &[u8] = b"election-a";
            const ELECTION_B: &[u8] = b"election-b";

            #[test]
            fn known_integers_pin_wire_order_and_arithmetic() {
                let ctx = C::default();
                assert_eq!(
                    ctx.exp_from_u64(258).strand_serialize().unwrap(),
                    $exponent
                );
                let x = X::strand_deserialize(&$exponent).unwrap();
                assert_eq!(x, ctx.exp_from_u64(258));
                // 4^3 = 64 and 258 - 17 = 241; no group reduction is needed.
                let four = E::strand_deserialize(&[1, 0, 0, 0, 4]).unwrap();
                assert_eq!(
                    ctx.emod_pow(&four, &ctx.exp_from_u64(3))
                        .strand_serialize()
                        .unwrap(),
                    [1, 0, 0, 0, 64]
                );
                assert_eq!(
                    ctx.exp_sub_mod(&x, &ctx.exp_from_u64(17)),
                    ctx.exp_from_u64(241)
                );
                let minus_one =
                    ctx.exp_sub_mod(&ctx.exp_from_u64(2), &ctx.exp_from_u64(3));
                // The wire format carries only the magnitude, so a negative
                // representative of q - 1 would come back as 1.
                assert_eq!(
                    X::strand_deserialize(&minus_one.strand_serialize().unwrap())
                        .unwrap(),
                    minus_one,
                    "exp_sub_mod must return the canonical residue q - 1"
                );
                assert_eq!(
                    minus_one.add(&ctx.exp_from_u64(1)).modq(&ctx),
                    ctx.exp_from_u64(0)
                );
            }

            #[test]
            fn group_and_ring_operators_obey_small_known_results() {
                let ctx = C::default();
                let four = ctx.element_from_bytes(&[4]).unwrap();
                let nine = ctx.element_from_bytes(&[9]).unwrap();
                let one = ctx.element_from_bytes(&[1]).unwrap();
                assert_eq!(
                    four.mul(&nine).modp(&ctx),
                    ctx.element_from_bytes(&[36]).unwrap()
                );
                assert_eq!(four.divp(&four, &ctx).modp(&ctx), one);
                assert_eq!(four.invp(&ctx).mul(&four).modp(&ctx), one);
                assert_eq!(ctx.emod_pow(&four, &ctx.exp_from_u64(0)), one);
                let three = ctx.exp_from_u64(3);
                let six = ctx.exp_from_u64(6);
                assert_eq!(three.add(&six).modq(&ctx), ctx.exp_from_u64(9));
                assert_eq!(six.sub(&three).modq(&ctx), three);
                assert_eq!(three.mul(&six).modq(&ctx), ctx.exp_from_u64(18));
                assert_eq!(
                    six.divq(&three, &ctx).modq(&ctx),
                    ctx.exp_from_u64(2)
                );
                assert_eq!(
                    three.invq(&ctx).mul(&three).modq(&ctx),
                    ctx.exp_from_u64(1)
                );
            }

            #[test]
            fn plaintext_wire_is_independent_of_group_encoding() {
                let ctx = C::default();
                // 3 encodes as 4, a quadratic residue in every odd prime field.
                let p = P::strand_deserialize(&$plaintext).unwrap();
                assert_eq!(p.strand_serialize().unwrap(), $plaintext);
                let encoded = ctx.encode(&p).unwrap();
                assert_eq!(
                    encoded.strand_serialize().unwrap(),
                    [1, 0, 0, 0, 4]
                );
                assert_eq!(ctx.decode(&encoded), p);
            }

            #[test]
            fn upper_half_element_decodes_the_known_nonresidue() {
                let ctx = C::default();
                // This fixed prime is 2 modulo 5, making 5 a nonresidue. Thus
                // plaintext 4 encodes to p-5 in the upper half of the group.
                // The fixture is that literal integer, not an encoder output.
                let raw = hex::decode($upper.trim()).unwrap();
                let element = ctx.element_from_bytes(&raw).unwrap();
                let mut plaintext = $plaintext;
                plaintext[4] = 4;
                let expected = P::strand_deserialize(&plaintext).unwrap();
                assert_eq!(ctx.decode(&element), expected);
                assert_eq!(ctx.encode(&expected).unwrap(), element);
            }

            #[test]
            fn stream_decoders_reject_truncation_and_trailing_input() {
                for bytes in [(&$exponent[..], true), (&$element[..], false)] {
                    for end in 0..bytes.0.len() {
                        if bytes.1 {
                            assert!(
                                X::strand_deserialize(&bytes.0[..end]).is_err()
                            );
                        } else {
                            assert!(
                                E::strand_deserialize(&bytes.0[..end]).is_err()
                            );
                        }
                    }
                    let mut extra = bytes.0.to_vec();
                    extra.push(0);
                    if bytes.1 {
                        assert!(X::strand_deserialize(&extra).is_err());
                    } else {
                        assert!(E::strand_deserialize(&extra).is_err());
                    }
                }
                assert!(
                    E::strand_deserialize(&[1, 0, 0, 0, 0]).is_err(),
                    "zero is outside the multiplicative group"
                );
                let ctx = C::default();
                assert_eq!(
                    ctx.exp_from_bytes(&[0]).unwrap(),
                    ctx.exp_from_u64(0)
                );
                assert!(ctx.exp_from_bytes(&[0xff; 257]).is_err());
                assert!(ctx.element_from_bytes(&[0xff; 257]).is_err());
                // 5 is a nonresidue for the fixed prime; the valid square 4
                // above distinguishes membership rejection from parse failure.
                assert!(matches!(
                    ctx.element_from_bytes(&[5]),
                    Err(strand::util::StrandError::Generic(message))
                        if message == NOT_QUADRATIC_RESIDUE
                ));
                assert!(E::strand_deserialize(&$element).is_ok());
                assert!(X::strand_deserialize(&$exponent).is_ok());
            }

            #[test]
            fn scalar_element_and_plaintext_serializers_propagate_sink_failure()
            {
                let x = X::strand_deserialize(&$exponent).unwrap();
                let e = E::strand_deserialize(&$element).unwrap();
                let p = P::strand_deserialize(&$plaintext).unwrap();
                for available in 0..$exponent.len() {
                    assert_eq!(
                        x.serialize(&mut FailAfter(available))
                            .unwrap_err()
                            .kind(),
                        io::ErrorKind::BrokenPipe
                    );
                }
                for available in 0..$element.len() {
                    assert_eq!(
                        e.serialize(&mut FailAfter(available))
                            .unwrap_err()
                            .kind(),
                        io::ErrorKind::BrokenPipe
                    );
                }
                for available in 0..$plaintext.len() {
                    assert_eq!(
                        p.serialize(&mut FailAfter(available))
                            .unwrap_err()
                            .kind(),
                        io::ErrorKind::BrokenPipe
                    );
                }
                assert!(x.serialize(&mut FailAfter($exponent.len())).is_ok());
                assert!(e.serialize(&mut FailAfter($element.len())).is_ok());
                assert!(p.serialize(&mut FailAfter($plaintext.len())).is_ok());
            }

            #[test]
            fn restored_proofs_bind_the_label_and_public_key() {
                let ctx = C::default();
                let sk = PrivateKey::from(&ctx.exp_from_u64(7), &ctx);
                let (pk, proof) = sk.get_pk_and_proof(ELECTION_A).unwrap();
                let proof = strand::zkp::Schnorr::<C>::strand_deserialize(
                    &proof.strand_serialize().unwrap(),
                )
                .unwrap();
                let verifier = Zkp::new(&ctx);
                assert!(verifier.schnorr_verify(
                    pk.element(),
                    None,
                    &proof,
                    ELECTION_A
                ));
                assert!(!verifier.schnorr_verify(
                    pk.element(),
                    None,
                    &proof,
                    ELECTION_B
                ));
                assert!(!verifier.schnorr_verify(
                    &ctx.gmod_pow(&ctx.exp_from_u64(8)),
                    None,
                    &proof,
                    ELECTION_A
                ));
                let four = E::strand_deserialize(&[1, 0, 0, 0, 4]).unwrap();
                let ciphertext =
                    pk.encrypt_with_randomness(&four, &ctx.exp_from_u64(11));
                assert_eq!(
                    sk.decrypt(&ciphertext).strand_serialize().unwrap(),
                    [1, 0, 0, 0, 4]
                );
            }
        }
    };
}

#[cfg(feature = "num_bigint")]
backend!(
    bigint,
    strand::backend::num_bigint::BigintCtx<strand::backend::num_bigint::P2048>,
    [2u8, 0, 0, 0, 2, 1],
    [2u8, 0, 0, 0, 0, 1],
    [1u8, 0, 0, 0, 3],
    include_str!("fixtures/verificatum-minus-five-little.hex")
);
#[cfg(feature = "rug")]
backend!(
    rug,
    strand::backend::rug::RugCtx<strand::backend::rug::P2048>,
    [2u8, 0, 0, 0, 1, 2],
    [2u8, 0, 0, 0, 1, 0],
    [1u8, 0, 0, 0, 3],
    include_str!("fixtures/verificatum-minus-five-big.hex")
);
#[cfg(feature = "malachite")]
backend!(
    malachite,
    strand::backend::malachite::MalachiteCtx<strand::backend::malachite::P2048>,
    [2u8, 0, 0, 0, 1, 2],
    [2u8, 0, 0, 0, 1, 0],
    [1u8, 0, 0, 0, 3, 0],
    include_str!("fixtures/verificatum-minus-five-big.hex")
);

fn context_contract<C: Ctx>() {
    let ctx = C::default();
    let bytes = ctx.strand_serialize().unwrap();
    let restored = C::strand_deserialize(&bytes)
        .expect("a context must deserialize its fixed group parameters");
    assert_eq!(restored.generator(), ctx.generator());
    // Verificatum parameters: generator and q use 256 bytes, p uses 257; cofactor is 2.
    assert_eq!(bytes.len(), 786);
    assert_eq!(&bytes[..4], &[0, 1, 0, 0]);
    assert_eq!(&bytes[781..], &[1, 0, 0, 0, 2]);
    for offset in [4, 264, 525, 785] {
        let mut changed = bytes.clone();
        changed[offset] ^= 1;
        assert!(
            C::strand_deserialize(&changed).is_err(),
            "changed group parameter at {offset}"
        );
    }
    for end in [0, 3, 4, 259, 260, 520, 521, 780, 781, 785] {
        assert!(C::strand_deserialize(&bytes[..end]).is_err());
    }
    let mut extra = bytes.clone();
    extra.push(0);
    assert!(C::strand_deserialize(&extra).is_err());
    for end in [0, 3, 4, 259, 260, 520, 521, 780, 781, 785] {
        assert_eq!(
            ctx.serialize(&mut FailAfter(end)).unwrap_err().kind(),
            io::ErrorKind::BrokenPipe
        );
    }
    assert!(ctx.serialize(&mut FailAfter(bytes.len())).is_ok());
    // Stream APIs consume just the context, leaving the next record untouched.
    let mut input = extra.as_slice();
    let _ = C::deserialize_reader(&mut input).unwrap();
    assert_eq!(input, &[0]);
}
#[cfg(feature = "rug")]
#[test]
fn rug_context_rejects_a_different_group() {
    context_contract::<strand::backend::rug::RugCtx<strand::backend::rug::P2048>>(
    );
}
#[cfg(feature = "malachite")]
#[test]
fn malachite_context_rejects_a_different_group() {
    context_contract::<
        strand::backend::malachite::MalachiteCtx<
            strand::backend::malachite::P2048,
        >,
    >();
}

#[cfg(feature = "num_bigint")]
#[test]
fn bigint_context_preserves_its_fixed_parameters() {
    type C = strand::backend::num_bigint::BigintCtx<
        strand::backend::num_bigint::P2048,
    >;
    let ctx = C::default();
    let wire = ctx.strand_serialize().unwrap();
    assert_eq!(&wire[..4], &[0, 1, 0, 0]);
    assert_eq!(wire.len(), 786);
    assert_eq!(&wire[781..], &[1, 0, 0, 0, 2]);
    let restored = C::strand_deserialize(&wire).expect(
        "fixed modulus must deserialize as a parameter, not a group element",
    );
    assert_eq!(restored, ctx);
    for offset in [4, 264, 525, 785] {
        let mut changed = wire.clone();
        changed[offset] ^= 1;
        assert!(C::strand_deserialize(&changed).is_err());
    }
}
