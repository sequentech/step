// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
/// Multiplicative group backend using [malachite](https://www.malachite.rs/).
#[cfg(feature = "malachite")]
pub mod malachite;
/// Multiplicative group backend using [num_bigint](https://docs.rs/num-bigint/latest/num_bigint/).
#[cfg(feature = "num_bigint")]
pub mod num_bigint;
/// Elliptic curve backend on top of [ristretto](https://ristretto.group/ristretto.html) using [curve25519_dalek](https://doc.dalek.rs/curve25519_dalek/ristretto/index.html).
pub mod ristretto;
#[cfg(feature = "rug")]
/// Multiplicative group backend using [rug](https://docs.rs/rug/1.16.0/rug/).
pub mod rug;

#[allow(dead_code)]
pub(crate) mod constants {
    pub(crate) const SAFEPRIME_COFACTOR: &str = "2";

    // Unicrypt 2048 primes, faster with small generator
    // https://github.com/bfh-evg/unicrypt/blob/2c9b223c1abc6266aa56ace5562200a5050a0c2a/src/main/java/ch/bfh/unicrypt/helper/prime/SafePrime.java
    pub(crate) const P_STR_2048: &str = "B7E151628AED2A6ABF7158809CF4F3C762E7160F38B4DA56A784D9045190CFEF324E7738926CFBE5F4BF8D8D8C31D763DA06C80ABB1185EB4F7C7B5757F5958490CFD47D7C19BB42158D9554F7B46BCED55C4D79FD5F24D6613C31C3839A2DDF8A9A276BCFBFA1C877C56284DAB79CD4C2B3293D20E9E5EAF02AC60ACC93ED874422A52ECB238FEEE5AB6ADD835FD1A0753D0A8F78E537D2B95BB79D8DCAEC642C1E9F23B829B5C2780BF38737DF8BB300D01334A0D0BD8645CBFA73A6160FFE393C48CBBBCA060F0FF8EC6D31BEB5CCEED7F2F0BB088017163BC60DF45A0ECB1BCD289B06CBBFEA21AD08E1847F3F7378D56CED94640D6EF0D3D37BE69D0063";
    pub(crate) const Q_STR_2048: &str = "5bf0a8b1457695355fb8ac404e7a79e3b1738b079c5a6d2b53c26c8228c867f799273b9c49367df2fa5fc6c6c618ebb1ed0364055d88c2f5a7be3dababfacac24867ea3ebe0cdda10ac6caaa7bda35e76aae26bcfeaf926b309e18e1c1cd16efc54d13b5e7dfd0e43be2b1426d5bce6a6159949e9074f2f5781563056649f6c3a21152976591c7f772d5b56ec1afe8d03a9e8547bc729be95caddbcec6e57632160f4f91dc14dae13c05f9c39befc5d98068099a50685ec322e5fd39d30b07ff1c9e2465dde5030787fc763698df5ae6776bf9785d84400b8b1de306fa2d07658de6944d8365dff510d68470c23f9fb9bc6ab676ca3206b77869e9bdf34e8031";
    pub(crate) const G_STR_2048: &str = "3";

    // VERIFICATUM 2048 primes, slower with larger generator
    pub(crate) const P_VERIFICATUM_STR_2048: &str = "49585549017473769285737299189965659293354088286913371933804180900778253856217662802521113040825270214021114944067918826365443480688403488878664971371922806487664111406970012663245195033428706668950006712214428830267861043863002671272535727084730103068500694744742135062909134544770371782327891513041774499809308517270708450370367766144873413397605830861330660620343634294061022593630276805276836395304145517051831281606133359766619313659042006635890778628844508225693978825158392000638704210656475473454575867531351247745913531003971176340768343624926105786111680264179067961026247115541456982560249992525766217307447";
    pub(crate) const Q_VERIFICATUM_STR_2048: &str = "24792774508736884642868649594982829646677044143456685966902090450389126928108831401260556520412635107010557472033959413182721740344201744439332485685961403243832055703485006331622597516714353334475003356107214415133930521931501335636267863542365051534250347372371067531454567272385185891163945756520887249904654258635354225185183883072436706698802915430665330310171817147030511296815138402638418197652072758525915640803066679883309656829521003317945389314422254112846989412579196000319352105328237736727287933765675623872956765501985588170384171812463052893055840132089533980513123557770728491280124996262883108653723";
    pub(crate) const G_VERIFICATUM_STR_2048: &str = "27257469383433468307851821232336029008797963446516266868278476598991619799718416119050669032044861635977216445034054414149795443466616532657735624478207460577590891079795564114912418442396707864995938563067755479563850474870766067031326511471051504594777928264027177308453446787478587442663554203039337902473879502917292403539820877956251471612701203572143972352943753791062696757791667318486190154610777475721752749567975013100844032853600120195534259802017090281900264646220781224136443700521419393245058421718455034330177739612895494553069450438317893406027741045575821283411891535713793639123109933196544017309147";
}

#[cfg(any(test, feature = "wasmtest"))]
pub(crate) mod tests {
    use crate::context::Ctx;
    use crate::context::Element;
    use crate::elgamal::*;
    use crate::serialization::StrandDeserialize;
    use crate::serialization::StrandSerialize;
    use std::time::Instant;

    use crate::util;
    use crate::zkp::Zkp;

    pub(crate) fn test_encrypt_exp_generic<C: Ctx>(ctx: &C) {
        let mut rng = ctx.get_rng();
        let exp = ctx.rnd_exp(&mut rng);
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
        let mut rng = ctx.get_rng();
        let zkp = Zkp::new(ctx);
        let secret = ctx.rnd_exp(&mut rng);
        let public = ctx.gmod_pow(&secret);
        let schnorr = zkp.schnorr_prove(&secret, &public, None, &[]).unwrap();
        let verified = zkp.schnorr_verify(&public, None, &schnorr, &[]);
        assert!(verified);
        let public_false = ctx.gmod_pow(&ctx.rnd_exp(&mut rng));
        let verified_false =
            !zkp.schnorr_verify(&public_false, None, &schnorr, &[]);
        assert!(verified_false);
    }

    pub(crate) fn test_chaumpedersen_generic<C: Ctx>(ctx: &C) {
        let mut rng = ctx.get_rng();
        let zkp = Zkp::new(ctx);
        let g1 = ctx.generator();
        let g2 = ctx.rnd(&mut rng);
        let secret = ctx.rnd_exp(&mut rng);
        let public1 = ctx.emod_pow(g1, &secret);
        let public2 = ctx.emod_pow(&g2, &secret);
        let proof = zkp
            .cp_prove(&secret, &public1, &public2, None, &g2, &[])
            .unwrap();
        let verified =
            zkp.cp_verify(&public1, &public2, None, &g2, &proof, &[]);

        assert!(verified);
        let public_false = ctx.gmod_pow(&ctx.rnd_exp(&mut rng));
        let verified_false =
            !zkp.cp_verify(&public1, &public_false, None, &g2, &proof, &[]);
        assert!(verified_false);
    }

    pub(crate) fn test_rerand_generic<C: Ctx>(ctx: &C) {
        let mut rng = ctx.get_rng();
        let zkp = Zkp::new(ctx);
        let sk = PrivateKey::gen(ctx);
        let pk = sk.get_pk();

        let plaintext = ctx.rnd_plaintext(&mut rng);
        let p = ctx.encode(&plaintext).unwrap();

        let c1 = pk.encrypt(&p);

        // re randomize
        let x = ctx.rnd_exp(&mut rng);
        let one = pk.one(&x);
        let c_ = c1.mul(&one);

        // ensure sure that the rerandomized ciphertext has the right value
        let decrypted = sk.decrypt(&c_);
        let decoded = ctx.decode(&decrypted);
        assert_eq!(plaintext, decoded);

        // prove
        let big_x = c_.gr.divp(&c1.gr, &ctx).modp(&ctx);
        let big_y = c_.mhr.divp(&c1.mhr, &ctx).modp(&ctx);

        let k = zkp.icp_prover_1();
        let (c, e, r) = zkp.icp_verifier_2(&k);
        let (big_a, big_b, a) = zkp.icp_prover_3(&pk.element);
        let z = zkp.icp_prover_5(&a, &x, &c, &k, &e, &r).unwrap();
        let ok = zkp.icp_verifier_6(
            &big_a,
            &big_b,
            &z,
            &e,
            &pk.element,
            &big_x,
            &big_y,
        );

        assert!(ok);

        // create a different ciphertext
        let c_different = Ciphertext::<C> {
            mhr: ctx.rnd(&mut rng),
            gr: ctx.rnd(&mut rng),
        };

        // re-randomize the different ciphertext (we reuse the same randomizing
        // ciphertext as above)
        let c_ = c_different.mul(&one);

        // proof with a different ciphertext re-randomization should fail
        let big_x = c_.gr.divp(&c1.gr, &ctx).modp(&ctx);
        let big_y = c_.mhr.divp(&c1.mhr, &ctx).modp(&ctx);

        let k = zkp.icp_prover_1();
        let (c, e, r) = zkp.icp_verifier_2(&k);
        let (big_a, big_b, a) = zkp.icp_prover_3(&pk.element);
        let z = zkp.icp_prover_5(&a, &x, &c, &k, &e, &r).unwrap();
        let ok = zkp.icp_verifier_6(
            &big_a,
            &big_b,
            &z,
            &e,
            &pk.element,
            &big_x,
            &big_y,
        );

        assert!(!ok);

        // non re-randomization should fail
        // does not encrypt the value '1'
        let not_one = Ciphertext::<C> {
            mhr: ctx
                .rnd(&mut rng)
                .mul(&ctx.emod_pow(&pk.element, &x))
                .modp(&ctx),
            gr: ctx.gmod_pow(&x),
        };
        let c_ = c1.mul(&not_one);

        // proof with a non-rerandomization (not multiplying by one) should fail
        let big_x = c_.gr.divp(&c1.gr, &ctx).modp(&ctx);
        let big_y = c_.mhr.divp(&c1.mhr, &ctx).modp(&ctx);

        let k = zkp.icp_prover_1();
        let (c, e, r) = zkp.icp_verifier_2(&k);
        let (big_a, big_b, a) = zkp.icp_prover_3(&pk.element);
        let z = zkp.icp_prover_5(&a, &x, &c, &k, &e, &r).unwrap();
        let ok = zkp.icp_verifier_6(
            &big_a,
            &big_b,
            &z,
            &e,
            &pk.element,
            &big_x,
            &big_y,
        );

        assert!(!ok);
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

    cfg_if::cfg_if! {
        if #[cfg(not(feature = "wasm"))] {
        use crate::shuffler::{ShuffleProof, Shuffler};
        use crate::shuffler_product::StrandRectangle;

    pub(crate) fn test_shuffle_generic<C: Ctx>(ctx: &C) {
        let sk = PrivateKey::gen(ctx);
        let pk = sk.get_pk();
        println!("Computing ciphertexts..");
        let es = util::random_ciphertexts(1000, ctx);
        let seed = vec![];
        let now = Instant::now(); println!("* generators..");
        let hs = ctx.generators(es.len() + 1, &seed).unwrap();
        println!("* generators {}", now.elapsed().as_millis());
        let shuffler = Shuffler {
            pk: &pk,
            ctx: (*ctx).clone(),
        };

        let beg = Instant::now();

        let now = Instant::now(); println!("* gen shuffle..");
        let (e_primes, rs, perm) = shuffler.gen_shuffle(&es);
        println!("* gen shuffle {}", now.elapsed().as_millis());
        let now = Instant::now();println!("* gen proof..");
        let proof = shuffler.gen_proof(es.clone(), &e_primes, rs, hs.clone(), perm, &[]).unwrap();
        println!("* gen proof {}", now.elapsed().as_millis());
        let now = Instant::now(); println!("* check proof..");
        let ok = shuffler.check_proof(&proof, es, e_primes, hs, &[]).unwrap();
        println!("* check proof {}", now.elapsed().as_millis());

        println!("All shuffle {}", beg.elapsed().as_millis());

        assert!(ok);
    }

    pub(crate) fn test_product_shuffle_generic<C: Ctx>(ctx: &C) {
        let sk = PrivateKey::gen(ctx);
        let pk = sk.get_pk();

        let es = util::random_product_ciphertexts(100, 3, ctx);
        let seed = vec![];
        let hs = ctx.generators(es.rows().len() + 1, &seed).unwrap();
        let shuffler = crate::shuffler_product::Shuffler {
            pk: &pk,
            generators: &hs,
            ctx: (*ctx).clone(),
        };

        let (e_primes, rs, perm) = shuffler.gen_shuffle(&es);
        let proof = shuffler.gen_proof(&es, &e_primes, rs, &perm, &[]).unwrap();

        let ok = shuffler.check_proof(&proof, &es, &e_primes, &[]).unwrap();

        assert!(ok);

        let e_primes_bad = util::random_product_ciphertexts(100, 3, ctx);
        let ok = shuffler.check_proof(&proof, &es, &e_primes_bad, &[]).unwrap();
        assert!(!ok);
    }

    pub(crate) fn test_shuffle_serialization_generic<C: Ctx>(ctx: &C) {
        let sk = PrivateKey::gen(ctx);
        let pk = sk.get_pk();

        let es = util::random_ciphertexts(10, ctx);
        let seed = vec![];
        let hs = ctx.generators(es.len() + 1, &seed).unwrap();
        let shuffler = Shuffler {
            pk: &pk,
            ctx: (*ctx).clone(),
        };
        let (e_primes, rs, perm) = shuffler.gen_shuffle(&es);
        let proof = shuffler.gen_proof(es.clone(), &e_primes, rs, hs.clone(), perm, &[]).unwrap();
        // in this test do this only after serialization
        // let ok = shuffler.check_proof(&proof, &es, &e_primes, &[]);
        // assert!(ok);

        let pk_b = pk.strand_serialize().unwrap();
        let es_b = es.strand_serialize().unwrap();
        let eprimes_b = e_primes.strand_serialize().unwrap();
        let proof_b = proof.strand_serialize().unwrap();
        // let pr = crate::shuffler::ShuffleProof2 { a: es[0] };
        // let proof_b = borsh::to_vec(&proof).unwrap();

        let pk_d = PublicKey::<C>::strand_deserialize(&pk_b).unwrap();
        let es_d = Vec::<Ciphertext<C>>::strand_deserialize(&es_b).unwrap();
        let eprimes_d =
            Vec::<Ciphertext<C>>::strand_deserialize(&eprimes_b).unwrap();
        let proof_d = ShuffleProof::<C>::strand_deserialize(&proof_b).unwrap();

        let shuffler_d = Shuffler {
            pk: &pk_d,
            ctx: (*ctx).clone(),
        };

        let ok_d = shuffler_d
            .check_proof(&proof_d, es_d, eprimes_d, hs, &[])
            .unwrap();

        assert!(ok_d);
    }

    pub(crate) fn test_product_shuffle_serialization_generic<C: Ctx>(ctx: &C) {
        let sk = PrivateKey::gen(ctx);
        let pk = sk.get_pk();

        let es = util::random_product_ciphertexts(100, 3, ctx);
        let seed = vec![];
        let hs = ctx.generators(es.rows().len() + 1, &seed).unwrap();
        let shuffler = crate::shuffler_product::Shuffler {
            pk: &pk,
            generators: &hs,
            ctx: (*ctx).clone(),
        };
        let (e_primes, rs, perm) = shuffler.gen_shuffle(&es);
        let proof = shuffler.gen_proof(&es, &e_primes, rs, &perm, &[]).unwrap();
        // in this test do this only after serialization
        // let ok = shuffler.check_proof(&proof, &es, &e_primes, &[]);
        // assert!(ok);

        let pk_b = pk.strand_serialize().unwrap();
        let es_b = es.strand_serialize().unwrap();
        let eprimes_b = e_primes.strand_serialize().unwrap();
        let proof_b = proof.strand_serialize().unwrap();

        let pk_d = PublicKey::<C>::strand_deserialize(&pk_b).unwrap();
        let es_d = StrandRectangle::<Ciphertext<C>>::strand_deserialize(&es_b).unwrap();
        let eprimes_d =
            StrandRectangle::<Ciphertext<C>>::strand_deserialize(&eprimes_b).unwrap();
        let proof_d = crate::shuffler_product::ShuffleProof::<C>::strand_deserialize(&proof_b).unwrap();

        let shuffler_d = crate::shuffler_product::Shuffler {
            pk: &pk_d,
            generators: &hs,
            ctx: (*ctx).clone(),
        };

        let ok_d = shuffler_d
            .check_proof(&proof_d, &es_d, &eprimes_d, &[])
            .unwrap();

        assert!(ok_d);
    }

    }}
}
