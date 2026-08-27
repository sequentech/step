// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 Free & Fair
// See LICENSE.md for details

//! Serialization module tests

#[cfg(test)]
mod tests {

    use crate::context::Context;
    use crate::context::P256Ctx as PCtx;
    use crate::context::RistrettoCtx as RCtx;
    use crate::cryptosystem::elgamal::{Ciphertext, KeyPair};
    use crate::utils::serialization::LargeVector;
    use crate::utils::serialization::fixed::{FDeserializable, FSerializable};
    use crate::utils::serialization::variable::LENGTH_BYTES;
    use crate::utils::serialization::variable::{VDeserializable, VSerializable};
    use vser_derive::VSerializable as VSer;

    #[test]
    fn test_usize_and_phantomdata() {
        #[derive(Debug, Clone, VSer, PartialEq)]
        struct TestNewLeafTypes<T> {
            size: usize,
            _phantom: std::marker::PhantomData<T>,
        }

        let data = TestNewLeafTypes::<String> {
            size: 12345,
            _phantom: std::marker::PhantomData,
        };

        let serialized = data.ser();
        let deserialized = TestNewLeafTypes::<String>::deser(&serialized).unwrap();

        assert_eq!(data, deserialized);
    }

    #[test]
    fn test_struct_vser_ristretto() {
        test_struct_vser::<RCtx>()
    }

    #[test]
    fn test_struct_vser_p256() {
        test_struct_vser::<PCtx>()
    }

    #[test]
    fn test_elgamal_struct_vser_ristretto() {
        test_elgamal_struct_vser::<RCtx>()
    }

    #[test]
    fn test_elgamal_struct_vser_p256() {
        test_elgamal_struct_vser::<PCtx>()
    }

    #[test]
    fn test_vector_vser_ristretto() {
        test_vector_vser::<RCtx>()
    }

    #[test]
    fn test_vector_vser_p256() {
        test_vector_vser::<PCtx>()
    }

    #[test]
    fn test_4_struct_vser_ristretto() {
        test_4_struct_vser::<RCtx>()
    }

    #[test]
    fn test_4_struct_vser_p256() {
        test_4_struct_vser::<PCtx>()
    }

    fn test_struct_vser<Ctx: Context + PartialEq>() {
        #[derive(Debug, Clone, VSer, PartialEq)]
        struct Test<Ctx: Context> {
            a: String,
            b: Ctx::Element,
            c: String,
        }

        let e1 = Ctx::random_element();
        let d = Test::<Ctx> {
            a: "hello".to_string(),
            b: e1,
            c: "world".to_string(),
        };

        let serialized = d.ser();
        let deserialized = Test::<Ctx>::deser(&serialized).unwrap();

        assert_eq!(d, deserialized);
    }

    fn test_elgamal_struct_vser<Ctx: Context>() {
        #[derive(Debug, VSer, PartialEq)]
        struct EG<Ctx: Context> {
            keypair: KeyPair<Ctx>,
            message: Ctx::Element,
            ciphertext: Ciphertext<Ctx, 1>,
        }

        let keypair = KeyPair::<Ctx>::generate();
        let message = Ctx::random_element();
        let ciphertext: Ciphertext<Ctx, 1> = keypair.encrypt(&[message.clone()]);

        let eg = EG::<Ctx> {
            keypair,
            message: message.clone(),
            ciphertext,
        };

        let serialized = eg.ser();

        let deserialized = EG::<Ctx>::deser(&serialized).unwrap();

        assert_eq!(message, deserialized.message);
        let decrypted = deserialized.keypair.decrypt(&deserialized.ciphertext);
        assert_eq!(decrypted, [message]);
    }

    fn test_vector_vser<Ctx: Context>() {
        #[derive(Debug, VSer, PartialEq)]
        struct EG<Ctx: Context> {
            keypair: KeyPair<Ctx>,
            messages: Vec<Ctx::Element>,
            ciphertexts: Vec<Ciphertext<Ctx, 1>>,
        }

        let count = 10;

        let keypair = KeyPair::<Ctx>::generate();
        let messages: Vec<Ctx::Element> = (0..count).map(|_| Ctx::random_element()).collect();

        let ciphertexts: Vec<Ciphertext<Ctx, 1>> = messages
            .iter()
            .map(|m| keypair.encrypt(&[m.clone()]))
            .collect();

        let eg = EG::<Ctx> {
            keypair,
            messages: messages.clone(),
            ciphertexts: ciphertexts,
        };

        let serialized = eg.ser();

        let deserialized = EG::<Ctx>::deser(&serialized).unwrap();

        for i in 0..count {
            assert_eq!(messages[i], deserialized.messages[i]);
            let decrypted = deserialized.keypair.decrypt(&deserialized.ciphertexts[i]);
            assert_eq!([messages[i].clone()], decrypted);
        }

        // A padded vector encoding must be rejected: `ser` never produces
        // trailing bytes, and accepting them would make distinct byte strings
        // decode to the same value. (This assertion previously checked the
        // opposite — that padded bytes "work" — which is exactly the
        // non-canonical acceptance the serialization audit removed.)
        let items = vec![0u32; 10];
        let mut bytes = items.ser();
        bytes.extend_from_slice(&[0u8; 5]);
        assert!(Vec::<u32>::deser(&bytes).is_err());
    }

    fn test_4_struct_vser<Ctx: Context + PartialEq>() {
        #[derive(Debug, VSer, PartialEq)]
        struct EG<Ctx: Context> {
            keypair: KeyPair<Ctx>,
            messages: Vec<[Ctx::Element; 2]>,
            ciphertexts: Vec<Ciphertext<Ctx, 2>>,
            tag: String,
        }

        let count = 5;

        let keypair = KeyPair::<Ctx>::generate();
        let messages: Vec<[Ctx::Element; 2]> = (0..count)
            .map(|_| [Ctx::random_element(), Ctx::random_element()])
            .collect();

        let ciphertexts: Vec<Ciphertext<Ctx, 2>> =
            messages.iter().map(|m| keypair.encrypt(&m)).collect();

        let tag = "test".to_string();
        let eg = EG {
            keypair,
            messages: messages.clone(),
            ciphertexts: ciphertexts,
            tag: tag.clone(),
        };

        let serialized = eg.ser();

        let back = EG::<Ctx>::deser(&serialized).unwrap();

        assert_eq!(eg, back);

        for i in 0..count {
            let decrypted = back.keypair.decrypt(&back.ciphertexts[i]);
            assert_eq!(messages[i], decrypted);
        }

        assert_eq!(tag, back.tag);
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    #[crate::warning("Miri test fails (Stacked Borrows)")]
    fn test_elgamal_largevector_ristretto() {
        test_elgamal_largevector::<RCtx>();
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    #[crate::warning("Miri test fails (Stacked Borrows)")]
    fn test_elgamal_largevector_p256() {
        test_elgamal_largevector::<PCtx>();
    }

    fn test_elgamal_largevector<Ctx: Context>() {
        let mut lv = LargeVector(vec![]);
        let count = 5;

        for _ in 0..count {
            let gr = [Ctx::random_element()];
            let mhr = [Ctx::random_element()];

            let ciphertext = Ciphertext::<Ctx, 1>::new(gr, mhr);
            lv.push(ciphertext);
        }

        let bytes = lv.ser();
        assert_eq!(
            bytes.len(),
            Ciphertext::<Ctx, 1>::size_bytes() * count + LENGTH_BYTES
        );
        let deserialized = LargeVector::<Ciphertext<Ctx, 1>>::deser(&bytes).unwrap();
        assert_eq!(deserialized.len(), count);
        assert!(!deserialized.is_empty());
        for i in 0..count {
            assert_eq!(lv.0[i], deserialized.0[i]);
        }
    }

    #[test]
    pub fn test_tuple_struct_ristretto() {
        test_tuple_struct_vser::<RCtx>();
    }

    #[test]
    pub fn test_tuple_struct_p256() {
        test_tuple_struct_vser::<PCtx>();
    }

    fn test_tuple_struct_vser<Ctx: Context + PartialEq>() {
        #[derive(Debug, VSer, PartialEq)]
        struct EG<Ctx: Context>(
            KeyPair<Ctx>,
            Vec<[Ctx::Element; 2]>,
            Vec<Ciphertext<Ctx, 2>>,
            String,
            u32,
            u64,
            Option<u16>,
        );

        let count = 5;

        let keypair = KeyPair::<Ctx>::generate();
        let messages: Vec<[Ctx::Element; 2]> = (0..count)
            .map(|_| [Ctx::random_element(), Ctx::random_element()])
            .collect();

        let ciphertexts: Vec<Ciphertext<Ctx, 2>> =
            messages.iter().map(|m| keypair.encrypt(&m)).collect();

        let tag = "test".to_string();
        let eg = EG(
            keypair,
            messages.clone(),
            ciphertexts,
            tag.clone(),
            1,
            1,
            Some(1),
        );

        let serialized = eg.ser();

        let back = EG::<Ctx>::deser(&serialized).unwrap();

        assert_eq!(eg, back);

        for i in 0..count {
            let decrypted = back.0.decrypt(&back.2[i]);
            assert_eq!(messages[i], decrypted);
        }

        assert_eq!(tag, back.3);
        assert_eq!(1, back.6.unwrap());
    }

    #[test]
    pub fn test_tuple_struct_fser_ristretto() {
        test_tuple_struct_fser::<RCtx>();
    }

    #[test]
    pub fn test_tuple_struct_fser_p256() {
        test_tuple_struct_fser::<PCtx>();
    }

    fn test_tuple_struct_fser<Ctx: Context + PartialEq>() {
        #[derive(Debug, VSer, PartialEq)]
        struct EG<Ctx: Context>(KeyPair<Ctx>, [Ctx::Element; 2], Ciphertext<Ctx, 2>, u32);

        let keypair = KeyPair::<Ctx>::generate();
        let message: [Ctx::Element; 2] = [Ctx::random_element(), Ctx::random_element()];
        let ciphertext: Ciphertext<Ctx, 2> = keypair.encrypt(&message);

        let eg = EG(keypair, message.clone(), ciphertext, 1);

        let serialized = eg.ser_f();

        let back = EG::<Ctx>::deser_f(&serialized).unwrap();

        assert_eq!(eg, back);
        let decrypted = back.0.decrypt(&back.2);
        assert_eq!(message, decrypted);
    }

    #[test]
    pub fn test_option_vser_ristretto() {
        test_option_vser::<RCtx>();
    }

    #[test]
    pub fn test_option_vser_p256() {
        test_option_vser::<PCtx>();
    }

    fn test_option_vser<Ctx: Context + PartialEq>() {
        let count = 5;

        let keypair = KeyPair::<Ctx>::generate();
        let messages: Vec<[Ctx::Element; 2]> = (0..count)
            .map(|_| [Ctx::random_element(), Ctx::random_element()])
            .collect();

        let ciphertexts: Vec<Ciphertext<Ctx, 2>> =
            messages.iter().map(|m| keypair.encrypt(&m)).collect();

        // We also test bool, since option uses it as discriminator
        let t = true;
        let serialized = t.ser();
        let back = <bool>::deser(&serialized).unwrap();
        assert_eq!(t, back);

        let t = false;
        let serialized = t.ser();
        let back = <bool>::deser(&serialized).unwrap();
        assert_eq!(t, back);

        let kp = Some(keypair);
        let serialized = kp.ser();
        let back = Option::<KeyPair<Ctx>>::deser(&serialized).unwrap();
        assert_eq!(kp, back);

        let m = Some(messages);
        let serialized = m.ser();
        let back = Option::<Vec<[Ctx::Element; 2]>>::deser(&serialized).unwrap();
        assert_eq!(m, back);

        let c = Some(ciphertexts);
        let serialized = c.ser();
        let back = Option::<Vec<Ciphertext<Ctx, 2>>>::deser(&serialized).unwrap();
        assert_eq!(c, back);

        let n = None;
        let serialized = n.ser();
        let back = Option::<u32>::deser(&serialized).unwrap();

        assert_eq!(n, back);
    }

    /// An array whose elements serialize to *differing* lengths.
    ///
    /// Every other array test in this module uses fixed-width elements
    /// (`[Ctx::Element; 2]` and friends), where all length prefixes are equal
    /// and a deserializer that reads the same prefix every time still succeeds.
    /// `Option<u32>` is 1 byte for `None` and 5 for `Some`, so this shape is the
    /// one that distinguishes them.
    #[test]
    fn test_array_vser_uneven_element_lengths() {
        let values: [Option<u32>; 3] = [Some(1), None, Some(2)];

        let serialized = values.ser();
        let back = <[Option<u32>; 3]>::deser(&serialized).unwrap();

        assert_eq!(values, back);
    }

    /// `bool` accepts only the two encodings it produces.
    ///
    /// This matters beyond `bool` itself, since `Option` uses it as the
    /// discriminator: a tolerated `0x02` would read as `None` and discard the
    /// payload of what was written as `Some`.
    #[test]
    fn test_bool_rejects_non_canonical() {
        assert_eq!(false, bool::deser(&[0]).unwrap());
        assert_eq!(true, bool::deser(&[1]).unwrap());

        assert!(bool::deser(&[2]).is_err());
        assert!(bool::deser(&[0xff]).is_err());

        assert!(Option::<u32>::deser(&[2]).is_err());
    }

    /// `None` is exactly its discriminator; bytes behind it are not accepted
    /// and silently dropped.
    #[test]
    fn test_option_none_rejects_trailing_bytes() {
        let none: Option<u32> = None;
        let serialized = none.ser();
        assert_eq!(1, serialized.len());
        assert_eq!(none, Option::<u32>::deser(&serialized).unwrap());

        assert!(Option::<u32>::deser(&[0, 0xff]).is_err());
    }

    // ------------------------------------------------------------------
    // Strictness pinning: deser must accept exactly ser's image
    // (see SERIALIZATION.md findings S1-S6)
    // ------------------------------------------------------------------

    #[test]
    fn test_string_rejects_trailing_bytes() {
        let s = "hello".to_string();
        let mut bytes = s.ser();
        assert_eq!(s, String::deser(&bytes).unwrap());
        bytes.push(0);
        assert!(String::deser(&bytes).is_err());
    }

    #[test]
    fn test_phantomdata_rejects_any_bytes() {
        use std::marker::PhantomData;
        let p: PhantomData<u32> = PhantomData;
        assert_eq!(0, p.ser().len());
        assert!(PhantomData::<u32>::deser(&[]).is_ok());
        assert!(PhantomData::<u32>::deser(&[0]).is_err());

        // The dangerous position: a struct ENDING in PhantomData receives all
        // remaining bytes there (braid's Configuration has this shape), so
        // trailing junk must fail the whole struct.
        #[derive(Debug, VSer, PartialEq)]
        struct EndsInPhantom<T> {
            value: u64,
            phantom: PhantomData<T>,
        }
        let v = EndsInPhantom::<u32> {
            value: 7,
            phantom: PhantomData,
        };
        let mut bytes = v.ser();
        assert_eq!(v, EndsInPhantom::<u32>::deser(&bytes).unwrap());
        bytes.push(0);
        assert!(EndsInPhantom::<u32>::deser(&bytes).is_err());
    }

    #[test]
    fn test_largevector_rejects_trailing_and_roundtrips_empty() {
        let lv: LargeVector<u64> = LargeVector(vec![1, 2, 3]);
        let mut bytes = lv.ser();
        assert_eq!(lv, LargeVector::<u64>::deser(&bytes).unwrap());
        bytes.push(0);
        assert!(LargeVector::<u64>::deser(&bytes).is_err());

        // The empty vector must round-trip (previously a division by zero).
        let empty: LargeVector<u64> = LargeVector(vec![]);
        let bytes = empty.ser();
        assert_eq!(empty, LargeVector::<u64>::deser(&bytes).unwrap());
    }

    #[test]
    fn test_fixed_tuple_rejects_wrong_total_size() {
        // The macro-generated tuple impl validates the total size up front
        // rather than relying on the last leaf to reject a mis-sized slice.
        let bytes = (&1u32, &2u64, &3u32).ser_f();
        assert_eq!(
            (1u32, 2u64, 3u32),
            <(u32, u64, u32)>::deser_f(&bytes).unwrap()
        );
        assert!(<(u32, u64, u32)>::deser_f(&bytes[..bytes.len() - 1]).is_err());
        let mut longer = bytes.clone();
        longer.push(0);
        assert!(<(u32, u64, u32)>::deser_f(&longer).is_err());
    }

    use crate::utils::serialization::variable::Marker;
    use crate::utils::serialization::variable::TypeId;
    use crate::utils::serialization::variable::ConstMarker;

    #[test]
    pub fn test_marker() {
        struct ExampleType42;
        impl TypeId for ExampleType42 {
            fn type_id() -> u32 {
                42
            }
        }
        struct ExampleType24;
        impl TypeId for ExampleType24 {
            fn type_id() -> u32 {
                24
            }
        }

        #[derive(VSer)]
        struct AnotherType<C: TypeId> {
            marker: Marker<C>,
        }
        impl<C: TypeId> AnotherType<C> {
            fn new() -> Self {
                AnotherType {
                    marker: Marker::<C>::default(),
                }
            }
        }

        let m42 = AnotherType::<ExampleType42>::new();
        let m24 = AnotherType::<ExampleType24>::new();

        let ser42 = m42.ser();
        let ser24 = m24.ser();

        let de42 = AnotherType::<ExampleType42>::deser(&ser42);
        assert!(de42.is_ok());
        let de24 = AnotherType::<ExampleType24>::deser(&ser24);
        assert!(de24.is_ok());
        let de42_from_24 = AnotherType::<ExampleType42>::deser(&ser24);
        assert!(de42_from_24.is_err());
        let de24_from_42 = AnotherType::<ExampleType24>::deser(&ser42);
        assert!(de24_from_42.is_err());

    }

        #[test]
    pub fn test_marker_const() {

        #[derive(VSer)]
        struct AnotherType<const A: u32, const B: u32> {
            marker_a: Marker<ConstMarker<A>>,
            marker_b: Marker<ConstMarker<B>>,
        }
        impl<const A: u32, const B: u32> AnotherType<A, B> {
            fn new() -> Self {
                AnotherType {
                    marker_a: ConstMarker::<A>::new(),
                    marker_b: ConstMarker::<B>::new(),
                }
            }
        }

        let m2_3 = AnotherType::<2, 3>::new();
        let m3_2 = AnotherType::<3, 2>::new();
        let ser2_3 = m2_3.ser();
        let ser3_2 = m3_2.ser();
        let de2_3 = AnotherType::<2, 3>::deser(&ser2_3);
        assert!(de2_3.is_ok());
        let de3_2 = AnotherType::<3, 2>::deser(&ser3_2);
        assert!(de3_2.is_ok());
        let de2_3_from_3_2 = AnotherType::<2, 3>::deser(&ser3_2);
        assert!(de2_3_from_3_2.is_err());
        let de3_2_from_2_3 = AnotherType::<3, 2>::deser(&ser2_3);
        assert!(de3_2_from_2_3.is_err());

    }
}
