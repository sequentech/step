// SPDX-FileCopyrightText: 2022 David Ruescas <david@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
/// Multiplicative group backend using [malachite](https://www.malachite.rs/).
pub mod malachite;
/// Multiplicative group backend using [num_bigint](https://docs.rs/num-bigint/latest/num_bigint/).
pub mod num_bigint;
/// Elliptic curve backend on top of [ristretto](https://ristretto.group/ristretto.html) using [curve25519_dalek](https://doc.dalek.rs/curve25519_dalek/ristretto/index.html).
pub mod ristretto;
#[cfg(feature = "rug")]
/// Multiplicative group backend using [rug](https://docs.rs/rug/1.16.0/rug/).
pub mod rug;

pub(crate) mod constants {
    pub(crate) const SAFEPRIME_COFACTOR: &str = "2";
    // https://github.com/bfh-evg/unicrypt/blob/2c9b223c1abc6266aa56ace5562200a5050a0c2a/src/main/java/ch/bfh/unicrypt/helper/prime/SafePrime.java
    /* pub(crate) const P_STR_2048: &str = "B7E151628AED2A6ABF7158809CF4F3C762E7160F38B4DA56A784D9045190CFEF324E7738926CFBE5F4BF8D8D8C31D763DA06C80ABB1185EB4F7C7B5757F5958490CFD47D7C19BB42158D9554F7B46BCED55C4D79FD5F24D6613C31C3839A2DDF8A9A276BCFBFA1C877C56284DAB79CD4C2B3293D20E9E5EAF02AC60ACC93ED874422A52ECB238FEEE5AB6ADD835FD1A0753D0A8F78E537D2B95BB79D8DCAEC642C1E9F23B829B5C2780BF38737DF8BB300D01334A0D0BD8645CBFA73A6160FFE393C48CBBBCA060F0FF8EC6D31BEB5CCEED7F2F0BB088017163BC60DF45A0ECB1BCD289B06CBBFEA21AD08E1847F3F7378D56CED94640D6EF0D3D37BE69D0063";
    pub(crate) const Q_STR_2048: &str = "5bf0a8b1457695355fb8ac404e7a79e3b1738b079c5a6d2b53c26c8228c867f799273b9c49367df2fa5fc6c6c618ebb1ed0364055d88c2f5a7be3dababfacac24867ea3ebe0cdda10ac6caaa7bda35e76aae26bcfeaf926b309e18e1c1cd16efc54d13b5e7dfd0e43be2b1426d5bce6a6159949e9074f2f5781563056649f6c3a21152976591c7f772d5b56ec1afe8d03a9e8547bc729be95caddbcec6e57632160f4f91dc14dae13c05f9c39befc5d98068099a50685ec322e5fd39d30b07ff1c9e2465dde5030787fc763698df5ae6776bf9785d84400b8b1de306fa2d07658de6944d8365dff510d68470c23f9fb9bc6ab676ca3206b77869e9bdf34e8031";
    pub(crate) const G_STR_2048: &str = "3";*/

    pub(crate) const P_VERIFICATUM_STR_2048: &str = "49585549017473769285737299189965659293354088286913371933804180900778253856217662802521113040825270214021114944067918826365443480688403488878664971371922806487664111406970012663245195033428706668950006712214428830267861043863002671272535727084730103068500694744742135062909134544770371782327891513041774499809308517270708450370367766144873413397605830861330660620343634294061022593630276805276836395304145517051831281606133359766619313659042006635890778628844508225693978825158392000638704210656475473454575867531351247745913531003971176340768343624926105786111680264179067961026247115541456982560249992525766217307447";
    pub(crate) const Q_VERIFICATUM_STR_2048: &str = "24792774508736884642868649594982829646677044143456685966902090450389126928108831401260556520412635107010557472033959413182721740344201744439332485685961403243832055703485006331622597516714353334475003356107214415133930521931501335636267863542365051534250347372371067531454567272385185891163945756520887249904654258635354225185183883072436706698802915430665330310171817147030511296815138402638418197652072758525915640803066679883309656829521003317945389314422254112846989412579196000319352105328237736727287933765675623872956765501985588170384171812463052893055840132089533980513123557770728491280124996262883108653723";
    pub(crate) const G_VERIFICATUM_STR_2048: &str = "27257469383433468307851821232336029008797963446516266868278476598991619799718416119050669032044861635977216445034054414149795443466616532657735624478207460577590891079795564114912418442396707864995938563067755479563850474870766067031326511471051504594777928264027177308453446787478587442663554203039337902473879502917292403539820877956251471612701203572143972352943753791062696757791667318486190154610777475721752749567975013100844032853600120195534259802017090281900264646220781224136443700521419393245058421718455034330177739612895494553069450438317893406027741045575821283411891535713793639123109933196544017309147";

    // const P_STR_3072: &str =
    // "B7E151628AED2A6ABF7158809CF4F3C762E7160F38B4DA56A784D9045190CFEF324E7738926CFBE5F4BF8D8D8C31D763DA06C80ABB1185EB4F7C7B5757F5958490CFD47D7C19BB42158D9554F7B46BCED55C4D79FD5F24D6613C31C3839A2DDF8A9A276BCFBFA1C877C56284DAB79CD4C2B3293D20E9E5EAF02AC60ACC93ED874422A52ECB238FEEE5AB6ADD835FD1A0753D0A8F78E537D2B95BB79D8DCAEC642C1E9F23B829B5C2780BF38737DF8BB300D01334A0D0BD8645CBFA73A6160FFE393C48CBBBCA060F0FF8EC6D31BEB5CCEED7F2F0BB088017163BC60DF45A0ECB1BCD289B06CBBFEA21AD08E1847F3F7378D56CED94640D6EF0D3D37BE67008E186D1BF275B9B241DEB64749A47DFDFB96632C3EB061B6472BBF84C26144E49C2D04C324EF10DE513D3F5114B8B5D374D93CB8879C7D52FFD72BA0AAE7277DA7BA1B4AF1488D8E836AF14865E6C37AB6876FE690B571121382AF341AFE94F77BCF06C83B8FF5675F0979074AD9A787BC5B9BD4B0C5937D3EDE4C3A79396419CD7"
    // ; const Q_STR_3072: &str =
    // "5bf0a8b1457695355fb8ac404e7a79e3b1738b079c5a6d2b53c26c8228c867f799273b9c49367df2fa5fc6c6c618ebb1ed0364055d88c2f5a7be3dababfacac24867ea3ebe0cdda10ac6caaa7bda35e76aae26bcfeaf926b309e18e1c1cd16efc54d13b5e7dfd0e43be2b1426d5bce6a6159949e9074f2f5781563056649f6c3a21152976591c7f772d5b56ec1afe8d03a9e8547bc729be95caddbcec6e57632160f4f91dc14dae13c05f9c39befc5d98068099a50685ec322e5fd39d30b07ff1c9e2465dde5030787fc763698df5ae6776bf9785d84400b8b1de306fa2d07658de6944d8365dff510d68470c23f9fb9bc6ab676ca3206b77869e9bdf3380470c368df93adcd920ef5b23a4d23efefdcb31961f5830db2395dfc26130a2724e1682619277886f289e9fa88a5c5ae9ba6c9e5c43ce3ea97feb95d0557393bed3dd0da578a446c741b578a432f361bd5b43b7f3485ab88909c1579a0d7f4a7bbde783641dc7fab3af84bc83a56cd3c3de2dcdea5862c9be9f6f261d3c9cb20ce6b"
    // ;
}

#[cfg(any(test, feature = "wasmtest"))]
pub(crate) mod tests {
    use crate::context::Ctx;
    use crate::context::Element;
    use crate::elgamal::*;
    use crate::keymaker::*;
    use crate::serialization::StrandDeserialize;
    use crate::serialization::StrandSerialize;
    use crate::shuffler::{ShuffleProof, Shuffler};

    use crate::util;
    use crate::zkp::{ChaumPedersen, Schnorr};

    pub(crate) fn test_encrypt_exp_generic<C: Ctx>(ctx: &C) {
        let exp = ctx.rnd_exp();
        let sk = PrivateKey::<C>::gen(ctx);

        let encrypted = ctx.encrypt_exp(&exp, sk.get_pk()).unwrap();
        let decrypted = ctx.decrypt_exp(&encrypted, sk);

        assert_eq!(exp, decrypted.unwrap());
    }

    pub(crate) fn test_elgamal_generic<C: Ctx>(ctx: &C, data: C::P) {
        let sk = PrivateKey::gen(ctx);
        let pk = sk.get_pk();

        let plaintext = ctx.encode(&data).unwrap();

        let c = pk.encrypt(&plaintext);
        let d = sk.decrypt(&c);

        let recovered = ctx.decode(&d);
        assert_eq!(data, recovered);
    }

    pub(crate) fn test_elgamal_enc_pok_generic<C: Ctx>(ctx: &C, data: C::P) {
        let sk = PrivateKey::gen(ctx);
        let pk = sk.get_pk();

        let plaintext = ctx.encode(&data).unwrap();
        let label = vec![];

        let (c, proof, _randomness) =
            pk.encrypt_and_pok(&plaintext, &label).unwrap();
        let d = sk.decrypt(&c);
        let zkp = Zkp::new(ctx);
        let proof_ok = zkp
            .encryption_popk_verify(&c.mhr, &c.gr, &proof, &label)
            .unwrap();
        assert!(proof_ok);

        let recovered = ctx.decode(&d);
        assert_eq!(data, recovered);
    }

    pub(crate) fn test_schnorr_generic<C: Ctx>(ctx: &C) {
        let zkp = Zkp::new(ctx);
        let secret = ctx.rnd_exp();
        let public = ctx.gmod_pow(&secret);
        let schnorr = zkp.schnorr_prove(&secret, &public, None, &[]).unwrap();
        let verified = zkp.schnorr_verify(&public, None, &schnorr, &[]);
        assert!(verified);
        let public_false = ctx.gmod_pow(&ctx.rnd_exp());
        let verified_false =
            !zkp.schnorr_verify(&public_false, None, &schnorr, &[]);
        assert!(verified_false);
    }

    use crate::zkp::Zkp;

    pub(crate) fn test_chaumpedersen_generic<C: Ctx>(ctx: &C) {
        let zkp = Zkp::new(ctx);
        let g1 = ctx.generator();
        let g2 = ctx.rnd();
        let secret = ctx.rnd_exp();
        let public1 = ctx.emod_pow(g1, &secret);
        let public2 = ctx.emod_pow(&g2, &secret);
        let proof = zkp
            .cp_prove(&secret, &public1, &public2, None, &g2, &[])
            .unwrap();
        let verified =
            zkp.cp_verify(&public1, &public2, None, &g2, &proof, &[]);

        assert!(verified);
        let public_false = ctx.gmod_pow(&ctx.rnd_exp());
        let verified_false =
            !zkp.cp_verify(&public1, &public_false, None, &g2, &proof, &[]);
        assert!(verified_false);
    }

    pub(crate) fn test_vdecryption_generic<C: Ctx>(ctx: &C, data: C::P) {
        let zkp = Zkp::new(ctx);
        let sk = PrivateKey::gen(ctx);
        let pk = sk.get_pk();

        let plaintext = ctx.encode(&data).unwrap();

        let c = pk.encrypt(&plaintext);
        let (d, proof) = sk.decrypt_and_prove(&c, &[]).unwrap();

        let dec_factor = c.mhr.divp(&d, ctx).modp(ctx);

        let verified = zkp
            .verify_decryption(
                &pk.element,
                &dec_factor,
                &c.mhr,
                &c.gr,
                &proof,
                &[],
            )
            .unwrap();
        let recovered = ctx.decode(&d);
        assert!(verified);
        assert_eq!(data, recovered);
    }

    pub(crate) fn test_distributed_generic<C: Ctx>(ctx: &C, data: C::P) {
        let zkp = Zkp::new(ctx);
        let km1 = Keymaker::gen(ctx);
        let km2 = Keymaker::gen(ctx);
        let (pk1, proof1) = km1.share(&[]).unwrap();
        let (pk2, proof2) = km2.share(&[]).unwrap();

        let verified1 = zkp.schnorr_verify(
            &pk1.element,
            Some(ctx.generator()),
            &proof1,
            &[],
        );
        let verified2 = zkp.schnorr_verify(
            &pk2.element,
            Some(ctx.generator()),
            &proof2,
            &[],
        );
        assert!(verified1);
        assert!(verified2);

        let plaintext = ctx.encode(&data).unwrap();

        let pk1_value = &pk1.element.clone();
        let pk2_value = &pk2.element.clone();
        let pks = vec![pk1, pk2];

        let pk_combined = Keymaker::combine_pks(ctx, pks);
        let c = pk_combined.encrypt(&plaintext);

        let (dec_f1, proof1) = km1.decryption_factor(&c, &[]).unwrap();
        let (dec_f2, proof2) = km2.decryption_factor(&c, &[]).unwrap();

        let verified1 = zkp
            .verify_decryption(pk1_value, &dec_f1, &c.mhr, &c.gr, &proof1, &[])
            .unwrap();
        let verified2 = zkp
            .verify_decryption(pk2_value, &dec_f2, &c.mhr, &c.gr, &proof2, &[])
            .unwrap();
        assert!(verified1);
        assert!(verified2);

        let decs = vec![dec_f1, dec_f2];
        let d = Keymaker::joint_dec(ctx, decs, &c);
        let recovered = ctx.decode(&d);
        assert_eq!(data, recovered);
    }

    pub(crate) fn test_distributed_serialization_generic<C: Ctx>(
        ctx: &C,
        data: Vec<C::P>,
    ) {
        let km1 = Keymaker::gen(ctx);
        let km2 = Keymaker::gen(ctx);
        let (pk1, proof1) = km1.share(&[]).unwrap();
        let (pk2, proof2) = km2.share(&[]).unwrap();

        let share1_pk_b = pk1.strand_serialize().unwrap();
        let share1_proof_b = proof1.strand_serialize().unwrap();

        let share2_pk_b = pk2.strand_serialize().unwrap();
        let share2_proof_b = proof2.strand_serialize().unwrap();

        let share1_pk_d =
            PublicKey::<C>::strand_deserialize(&share1_pk_b).unwrap();
        let share1_proof_d =
            Schnorr::<C>::strand_deserialize(&share1_proof_b).unwrap();

        let share2_pk_d =
            PublicKey::<C>::strand_deserialize(&share2_pk_b).unwrap();
        let share2_proof_d =
            Schnorr::<C>::strand_deserialize(&share2_proof_b).unwrap();

        let verified1 =
            Keymaker::verify_share(ctx, &share1_pk_d, &share1_proof_d, &[]);
        let verified2 =
            Keymaker::verify_share(ctx, &share2_pk_d, &share2_proof_d, &[]);

        assert!(verified1);
        assert!(verified2);

        let pk1_value = &share1_pk_d.element.clone();
        let pk2_value = &share2_pk_d.element.clone();
        let pks = vec![share1_pk_d, share2_pk_d];

        let pk_combined = Keymaker::combine_pks(ctx, pks);
        let mut cs = vec![];

        for plaintext in &data {
            let encoded = ctx.encode(plaintext).unwrap();
            let c = pk_combined.encrypt(&encoded);
            cs.push(c);
        }

        let (decs1, proofs1) = km1.decryption_factor_many(&cs, &[]).unwrap();
        let (decs2, proofs2) = km2.decryption_factor_many(&cs, &[]).unwrap();

        let decs1_b = decs1.strand_serialize().unwrap();
        let proofs1_b = proofs1.strand_serialize().unwrap();

        let decs2_b = decs2.strand_serialize().unwrap();
        let proofs2_b = proofs2.strand_serialize().unwrap();

        let decs1_d = Vec::<C::E>::strand_deserialize(&decs1_b).unwrap();
        let proofs1_d =
            Vec::<ChaumPedersen<C>>::strand_deserialize(&proofs1_b).unwrap();

        let decs2_d = Vec::<C::E>::strand_deserialize(&decs2_b).unwrap();
        let proofs2_d =
            Vec::<ChaumPedersen<C>>::strand_deserialize(&proofs2_b).unwrap();

        let verified1 = Keymaker::verify_decryption_factors(
            ctx,
            pk1_value,
            &cs,
            &decs1_d,
            &proofs1_d,
            &[],
        );
        let verified2 = Keymaker::verify_decryption_factors(
            ctx,
            pk2_value,
            &cs,
            &decs2_d,
            &proofs2_d,
            &[],
        );

        assert!(verified1.unwrap());
        assert!(verified2.unwrap());

        let decs = vec![decs1_d, decs2_d];
        let ds = Keymaker::joint_dec_many(ctx, &decs, &cs);

        let recovered: Vec<C::P> =
            ds.into_iter().map(|d| ctx.decode(&d)).collect();

        assert_eq!(data, recovered);
    }

    pub(crate) fn test_shuffle_generic<C: Ctx>(ctx: &C) {
        let sk = PrivateKey::gen(ctx);
        let pk = sk.get_pk();

        let es = util::random_ciphertexts(10, ctx);
        let seed = vec![];
        let hs = ctx.generators(es.len() + 1, &seed);
        let shuffler = Shuffler {
            pk: &pk,
            generators: &hs,
            ctx: (*ctx).clone(),
        };

        let (e_primes, rs, perm) = shuffler.gen_shuffle(&es);
        let proof =
            shuffler.gen_proof(&es, &e_primes, &rs, &perm, &[]).unwrap();

        let ok = shuffler.check_proof(&proof, &es, &e_primes, &[]).unwrap();

        assert!(ok);
    }

    pub(crate) fn test_shuffle_serialization_generic<C: Ctx>(ctx: &C) {
        let sk = PrivateKey::gen(ctx);
        let pk = sk.get_pk();

        let es = util::random_ciphertexts(10, ctx);
        let seed = vec![];
        let hs = ctx.generators(es.len() + 1, &seed);
        let shuffler = Shuffler {
            pk: &pk,
            generators: &hs,
            ctx: (*ctx).clone(),
        };
        let (e_primes, rs, perm) = shuffler.gen_shuffle(&es);
        let proof =
            shuffler.gen_proof(&es, &e_primes, &rs, &perm, &[]).unwrap();
        // in this test do this only after serialization
        // let ok = shuffler.check_proof(&proof, &es, &e_primes, &[]);
        // assert!(ok);

        let pk_b = pk.strand_serialize().unwrap();
        let es_b = es.strand_serialize().unwrap();
        let eprimes_b = e_primes.strand_serialize().unwrap();
        let proof_b = proof.strand_serialize().unwrap();

        let pk_d = PublicKey::<C>::strand_deserialize(&pk_b).unwrap();
        let es_d = Vec::<Ciphertext<C>>::strand_deserialize(&es_b).unwrap();
        let eprimes_d =
            Vec::<Ciphertext<C>>::strand_deserialize(&eprimes_b).unwrap();
        let proof_d = ShuffleProof::<C>::strand_deserialize(&proof_b).unwrap();

        let shuffler_d = Shuffler {
            pk: &pk_d,
            generators: &hs,
            ctx: (*ctx).clone(),
        };

        let ok_d = shuffler_d
            .check_proof(&proof_d, &es_d, &eprimes_d, &[])
            .unwrap();

        assert!(ok_d);
    }
}
