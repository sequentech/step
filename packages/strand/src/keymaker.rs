// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
#[cfg(feature = "rayon")]
use rayon::prelude::*;

use crate::context::{Ctx, Element};
use crate::elgamal::{Ciphertext, PrivateKey, PublicKey};
use crate::util::{Par, StrandError};
use crate::zkp::{ChaumPedersen, Schnorr, Zkp};

/// Distributed key generation (non threshold) as a simple sum
pub(crate) struct Keymaker<C: Ctx> {
    sk: PrivateKey<C>,
    pk: PublicKey<C>,
    ctx: C,
}

impl<C: Ctx> Keymaker<C> {
    pub(crate) fn gen(ctx: &C) -> Keymaker<C> {
        let sk = PrivateKey::gen(ctx);
        let pk = PublicKey::from_element(&sk.pk_element, ctx);

        Keymaker {
            sk,
            pk,
            ctx: (*ctx).clone(),
        }
    }

    pub(crate) fn from_sk(sk: PrivateKey<C>, ctx: &C) -> Keymaker<C> {
        let pk = PublicKey::from_element(&sk.pk_element, ctx);

        Keymaker {
            sk,
            pk,
            ctx: (*ctx).clone(),
        }
    }

    pub(crate) fn share(
        &self,
        label: &[u8],
    ) -> Result<(PublicKey<C>, Schnorr<C>), StrandError> {
        let zkp = Zkp::new(&self.ctx);
        let pk = PublicKey::from_element(&self.pk.element, &self.ctx);
        let proof = zkp.schnorr_prove(&self.sk.value, &pk.element, None, label);

        Ok((pk, proof?))
    }

    pub(crate) fn verify_share(
        ctx: &C,
        pk: &PublicKey<C>,
        proof: &Schnorr<C>,
        label: &[u8],
    ) -> bool {
        let zkp = Zkp::new(ctx);
        zkp.schnorr_verify(&pk.element, None, proof, label)
    }

    pub(crate) fn combine_pks(ctx: &C, pks: Vec<PublicKey<C>>) -> PublicKey<C> {
        let mut acc: C::E = pks[0].element.clone();

        for pk in pks.iter().skip(1) {
            acc = acc.mul(&pk.element).modp(ctx);
        }

        PublicKey::from_element(&acc, ctx)
    }

    pub(crate) fn decryption_factor(
        &self,
        c: &Ciphertext<C>,
        label: &[u8],
    ) -> Result<(C::E, ChaumPedersen<C>), StrandError> {
        let dec_factor = self.sk.decryption_factor(c);
        let zkp = Zkp::new(&self.ctx);
        let proof = zkp.decryption_proof(
            &self.sk.value,
            &self.pk.element,
            &dec_factor,
            &c.mhr,
            &c.gr,
            label,
        );

        Ok((dec_factor, proof?))
    }

    #[allow(clippy::type_complexity)]
    pub(crate) fn decryption_factor_many(
        &self,
        cs: &[Ciphertext<C>],
        label: &[u8],
    ) -> Result<(Vec<C::E>, Vec<ChaumPedersen<C>>), StrandError> {
        let decs_proofs: Result<Vec<(C::E, ChaumPedersen<C>)>, StrandError> =
            cs.par().map(|c| self.decryption_factor(c, label)).collect();

        let d = decs_proofs?.into_iter().unzip();

        Ok(d)
    }

    pub(crate) fn joint_dec(
        ctx: &C,
        decs: Vec<C::E>,
        c: &Ciphertext<C>,
    ) -> C::E {
        let mut acc: C::E = decs[0].clone();
        for dec in decs.iter().skip(1) {
            acc = acc.mul(dec).modp(ctx);
        }

        c.mhr.divp(&acc, ctx).modp(ctx)
    }

    pub(crate) fn joint_dec_many(
        ctx: &C,
        decs: &[Vec<C::E>],
        cs: &[Ciphertext<C>],
    ) -> Vec<C::E> {
        let decrypted: Vec<C::E> = cs
            .par()
            .enumerate()
            .map(|(i, c)| {
                let mut acc: C::E = decs[0][i].clone();

                for dec in decs.iter().skip(1) {
                    acc = acc.mul(&dec[i]).modp(ctx);
                }
                c.mhr.divp(&acc, ctx).modp(ctx)
            })
            .collect();

        decrypted
    }

    pub(crate) fn verify_decryption_factors(
        ctx: &C,
        pk_value: &C::E,
        ciphertexts: &[Ciphertext<C>],
        decs: &[C::E],
        proofs: &[ChaumPedersen<C>],
        label: &[u8],
    ) -> Result<bool, StrandError> {
        assert_eq!(decs.len(), proofs.len());
        assert_eq!(decs.len(), ciphertexts.len());
        let zkp = Zkp::new(ctx);

        let results: Result<Vec<bool>, StrandError> = (0..decs.len())
            .par()
            .map(|i| {
                zkp.verify_decryption(
                    pk_value,
                    &decs[i],
                    &ciphertexts[i].mhr,
                    &ciphertexts[i].gr,
                    &proofs[i],
                    label,
                )
            })
            .collect();

        let notok = results?.iter().any(|x| !x);

        Ok(!notok)
    }
}

#[cfg(any(test, feature = "wasmtest"))]
pub(crate) mod tests {
    use crate::context::Ctx;
    use crate::elgamal::*;
    use crate::keymaker::*;
    use crate::serialization::StrandDeserialize;
    use crate::serialization::StrandSerialize;

    use crate::zkp::{ChaumPedersen, Schnorr, Zkp};

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
}
