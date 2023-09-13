// SPDX-FileCopyrightText: 2021 David Ruescas <david@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
#[cfg(feature = "rayon")]
use rayon::prelude::*;

use crate::context::{Ctx, Element};
use crate::elgamal::{Ciphertext, PrivateKey, PublicKey};
use crate::util::{Par, StrandError};
use crate::zkp::{ChaumPedersen, Schnorr, Zkp};

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
