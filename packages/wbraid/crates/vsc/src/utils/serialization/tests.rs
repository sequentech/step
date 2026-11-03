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
    use crate::utils::serialization::{Deserializable, PAR_MIN_ELEMENTS, Serializable};
    use canonical_derive::Canonical;

    #[test]
    fn test_usize_and_phantomdata() {
        #[derive(Debug, Clone, Canonical, PartialEq)]
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
        #[derive(Debug, Clone, Canonical, PartialEq)]
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
        #[derive(Debug, Canonical, PartialEq)]
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
        #[derive(Debug, Canonical, PartialEq)]
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
        #[derive(Debug, Canonical, PartialEq)]
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
    pub fn test_tuple_struct_ristretto() {
        test_tuple_struct_vser::<RCtx>();
    }

    #[test]
    pub fn test_tuple_struct_p256() {
        test_tuple_struct_vser::<PCtx>();
    }

    fn test_tuple_struct_vser<Ctx: Context + PartialEq>() {
        #[derive(Debug, Canonical, PartialEq)]
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
        #[derive(Debug, Canonical, PartialEq)]
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

    // -----------------------------------------------------------------------
    // Lists: parallel encoding and decoding behind the unchanged encoding
    // -----------------------------------------------------------------------

    /// The list encoding by definition: a big-endian u64 count, then each
    /// element's encoding in order. The oracle for `Vec<T>::ser`.
    fn reference_list_ser<T: Serializable>(items: &[T]) -> Vec<u8> {
        let mut out = u64::try_from(items.len()).unwrap().to_be_bytes().to_vec();
        for item in items {
            out.extend(item.ser());
        }
        out
    }

    /// The list decoding by definition: the count, then that many sequential
    /// element reads from the cursor, then `deser`'s exhaustion check. The
    /// oracle for `Vec<T>::deser`, including which error it reports.
    fn reference_list_deser<T: Deserializable>(
        bytes: &[u8],
    ) -> Result<Vec<T>, crate::utils::error::Error> {
        let mut input = bytes;
        let count = u64::read(&mut input)?;
        let mut items = Vec::new();
        for _ in 0..count {
            items.push(T::read(&mut input)?);
        }
        if !input.is_empty() {
            return Err(crate::utils::error::Error::DeserializationError(
                "Trailing bytes after value".to_string(),
            ));
        }
        Ok(items)
    }

    #[test]
    fn test_fixed_width_hints() {
        assert_eq!(<u8 as Deserializable>::FIXED_WIDTH, Some(1));
        assert_eq!(<u64 as Deserializable>::FIXED_WIDTH, Some(8));
        assert_eq!(<u128 as Deserializable>::FIXED_WIDTH, Some(16));
        assert_eq!(<usize as Deserializable>::FIXED_WIDTH, Some(8));
        assert_eq!(<bool as Deserializable>::FIXED_WIDTH, Some(1));
        assert_eq!(
            <<RCtx as Context>::Element as Deserializable>::FIXED_WIDTH,
            Some(32)
        );
        assert_eq!(
            <<RCtx as Context>::Scalar as Deserializable>::FIXED_WIDTH,
            Some(32)
        );
        assert_eq!(
            <<PCtx as Context>::Element as Deserializable>::FIXED_WIDTH,
            Some(33)
        );
        assert_eq!(
            <<PCtx as Context>::Scalar as Deserializable>::FIXED_WIDTH,
            Some(32)
        );
        assert_eq!(
            <[<RCtx as Context>::Element; 2] as Deserializable>::FIXED_WIDTH,
            Some(64)
        );
        assert_eq!(
            <Ciphertext<RCtx, 2> as Deserializable>::FIXED_WIDTH,
            Some(128)
        );
        assert_eq!(
            <Ciphertext<PCtx, 3> as Deserializable>::FIXED_WIDTH,
            Some(198)
        );
        assert_eq!(<Vec<u8> as Deserializable>::FIXED_WIDTH, None);
        assert_eq!(<String as Deserializable>::FIXED_WIDTH, None);
        assert_eq!(<Option<u64> as Deserializable>::FIXED_WIDTH, None);
        assert_eq!(
            <std::marker::PhantomData<u8> as Deserializable>::FIXED_WIDTH,
            Some(0)
        );

        #[derive(Canonical)]
        struct Mixed {
            a: u64,
            s: String,
        }
        assert_eq!(<Mixed as Deserializable>::FIXED_WIDTH, None);

        #[derive(Canonical)]
        struct Fixed {
            a: u64,
            b: [u32; 3],
            c: bool,
        }
        assert_eq!(<Fixed as Deserializable>::FIXED_WIDTH, Some(8 + 12 + 1));

        #[derive(Canonical)]
        struct Unit;
        assert_eq!(<Unit as Deserializable>::FIXED_WIDTH, Some(0));
    }

    #[test]
    fn test_vec_ser_matches_reference_ristretto() {
        test_vec_ser_matches_reference::<RCtx>();
    }

    #[test]
    fn test_vec_ser_matches_reference_p256() {
        test_vec_ser_matches_reference::<PCtx>();
    }

    /// `Vec<T>::ser` is byte-identical to the definition on both sides of the
    /// parallel threshold, for fixed- and variable-width elements.
    fn test_vec_ser_matches_reference<Ctx: Context>() {
        let big = PAR_MIN_ELEMENTS + 777;
        for n in [0usize, 1, PAR_MIN_ELEMENTS - 1, PAR_MIN_ELEMENTS, big] {
            let elems: Vec<Ctx::Element> = (0..n).map(|_| Ctx::random_element()).collect();
            assert_eq!(elems.ser(), reference_list_ser(&elems), "elements, n = {n}");

            let arrays: Vec<[Ctx::Scalar; 3]> = (0..n)
                .map(|_| std::array::from_fn(|_| Ctx::random_scalar()))
                .collect();
            assert_eq!(
                arrays.ser(),
                reference_list_ser(&arrays),
                "scalar arrays, n = {n}"
            );

            let strings: Vec<String> = (0..n).map(|i| "x".repeat(i % 7)).collect();
            assert_eq!(
                strings.ser(),
                reference_list_ser(&strings),
                "strings, n = {n}"
            );
        }
        let keypair = KeyPair::<Ctx>::generate();
        let cts: Vec<Ciphertext<Ctx, 2>> = (0..big)
            .map(|_| {
                keypair
                    .pkey
                    .encrypt(&[Ctx::random_element(), Ctx::random_element()])
            })
            .collect();
        assert_eq!(cts.ser(), reference_list_ser(&cts), "ciphertexts");
    }

    #[test]
    fn test_vec_deser_matches_reference_ristretto() {
        test_vec_deser_matches_reference::<RCtx>();
    }

    #[test]
    fn test_vec_deser_matches_reference_p256() {
        test_vec_deser_matches_reference::<PCtx>();
    }

    /// `Vec<T>::deser` accepts exactly what the definition accepts and reports
    /// the same error otherwise, on both sides of the parallel threshold.
    fn test_vec_deser_matches_reference<Ctx: Context>() {
        for n in [17usize, PAR_MIN_ELEMENTS + 5] {
            let elems: Vec<Ctx::Element> = (0..n).map(|_| Ctx::random_element()).collect();
            check_list_deser_agreement(&elems);

            let arrays: Vec<[Ctx::Scalar; 3]> = (0..n)
                .map(|_| std::array::from_fn(|_| Ctx::random_scalar()))
                .collect();
            check_list_deser_agreement(&arrays);

            let keypair = KeyPair::<Ctx>::generate();
            let cts: Vec<Ciphertext<Ctx, 2>> = (0..n)
                .map(|_| {
                    keypair
                        .pkey
                        .encrypt(&[Ctx::random_element(), Ctx::random_element()])
                })
                .collect();
            check_list_deser_agreement(&cts);
        }
    }

    /// Valid, corrupted (first, middle, last element), truncated, extended
    /// and mis-counted encodings of `items` decode identically — same
    /// acceptance, same value, same error text — through `Vec<T>::deser` and
    /// the sequential definition.
    fn check_list_deser_agreement<T>(items: &[T])
    where
        T: Serializable + Deserializable + PartialEq + std::fmt::Debug + Send + Sync,
    {
        let width = T::FIXED_WIDTH.expect("fixed-width element");
        let n = items.len();
        let valid = reference_list_ser(items);
        let mut cases: Vec<(String, Vec<u8>)> = vec![("valid".to_string(), valid.clone())];
        for index in [0, n / 2, n - 1] {
            let mut bytes = valid.clone();
            let start = 8 + index * width;
            for byte in &mut bytes[start..start + width] {
                *byte = 0xFF;
            }
            cases.push((format!("element {index} corrupted"), bytes));
        }
        cases.push((
            "truncated by one byte".to_string(),
            valid[..valid.len() - 1].to_vec(),
        ));
        cases.push((
            "truncated by one element".to_string(),
            valid[..valid.len() - width].to_vec(),
        ));
        let mut extended = valid.clone();
        extended.push(0);
        cases.push(("one trailing byte".to_string(), extended));
        for (label, count) in [("count one too many", n + 1), ("count one too few", n - 1)] {
            let mut bytes = valid.clone();
            bytes[..8].copy_from_slice(&u64::try_from(count).unwrap().to_be_bytes());
            cases.push((label.to_string(), bytes));
        }
        let mut huge = valid.clone();
        huge[..8].copy_from_slice(&u64::MAX.to_be_bytes());
        cases.push(("count u64::MAX".to_string(), huge));

        for (name, bytes) in cases {
            let reference = reference_list_deser::<T>(&bytes);
            let actual = Vec::<T>::deser(&bytes);
            match (reference, actual) {
                (Ok(r), Ok(a)) => {
                    assert_eq!(r, a, "{name} (n = {n}): values differ");
                    assert_eq!(
                        a.as_slice(),
                        items,
                        "{name} (n = {n}): not the encoded items"
                    );
                }
                (Err(r), Err(a)) => {
                    assert_eq!(
                        r.to_string(),
                        a.to_string(),
                        "{name} (n = {n}): errors differ"
                    );
                }
                (r, a) => panic!("{name} (n = {n}): reference {r:?} vs actual {a:?}"),
            }
        }
    }
}
