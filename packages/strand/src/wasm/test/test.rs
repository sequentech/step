// SPDX-FileCopyrightText: 2022 David Ruescas <david@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use rand::RngCore;
use wasm_bindgen::prelude::*;

// use crate::backend::malachite::{MalachiteCtx, P2048 as MP2048};
use crate::backend::num_bigint::{BigintCtx, P2048};
use crate::backend::ristretto::RistrettoCtx;
use crate::backend::tests::*;
use crate::context::Ctx;
use crate::rng::StrandRng;
// use crate::threshold::tests::test_threshold_generic;
// use crate::keymaker::tests::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
    #[wasm_bindgen]
    fn postMessage(s: &str);
}

pub fn message(s: &str) {
    log(s);
    postMessage(s);
}

#[wasm_bindgen]
pub fn test() {
    message("--- wasm::test.rs");
    test_elgamal();
    test_schnorr();
    test_chaumpedersen();
    test_rerand();
    test_vdecryption();
    test_encrypt_exp();
    test_encrypt_pok();

    // test_distributed();
    // test_distributed_serialization();
    // test_threshold();
    // test_shuffle_serialization();
}

#[wasm_bindgen]
pub fn test_chaumpedersen() {
    message("* Ristretto chaumpedersen..");
    let ctx = RistrettoCtx;
    test_chaumpedersen_generic(&ctx);

    message("* BigInt chaumpedersen..");
    let ctx: BigintCtx<P2048> = Default::default();
    test_chaumpedersen_generic(&ctx);
    /*
        message("* Malachite chaumpedersen..");
        let ctx: MalachiteCtx<MP2048> = Default::default();
        test_chaumpedersen_generic(&ctx);
    */
}

#[wasm_bindgen]
pub fn test_rerand() {
    message("* Ristretto rerand..");
    let ctx = RistrettoCtx;
    test_rerand_generic(&ctx);

    message("* BigInt rerand..");
    let ctx: BigintCtx<P2048> = Default::default();
    test_rerand_generic(&ctx);
    /*
        message("* Malachite rerand..");
        let ctx: MalachiteCtx<MP2048> = Default::default();
        test_rerand_generic(&ctx);
    */
}

#[wasm_bindgen]
pub fn test_elgamal() {
    message("* Ristretto encrypt..");
    let ctx = RistrettoCtx;
    let mut csprng = StrandRng;
    let mut fill = [0u8; 30];
    csprng.fill_bytes(&mut fill);
    let plaintext = to_plaintext_array(fill.as_ref());
    test_elgamal_generic(&ctx, plaintext);

    message("* BigInt encrypt..");
    let ctx: BigintCtx<P2048> = Default::default();
    let mut rng = ctx.get_rng();
    let plaintext = ctx.rnd_plaintext(&mut rng);
    test_elgamal_generic(&ctx, plaintext);
    /*
        message("* Malachite encrypt..");
        let ctx: MalachiteCtx<MP2048> = Default::default();
        let mut rng = ctx.get_rng();
        let plaintext = ctx.rnd_plaintext(&mut rng);
        test_elgamal_generic(&ctx, plaintext);
    */
}

#[wasm_bindgen]
pub fn test_schnorr() {
    message("* Ristretto schnorr..");
    let ctx = RistrettoCtx;
    test_schnorr_generic(&ctx);

    message("* BigInt schnorr..");
    let ctx: BigintCtx<P2048> = Default::default();
    test_schnorr_generic(&ctx);
    /*
        message("* Malachite schnorr..");
        let ctx: MalachiteCtx<MP2048> = Default::default();
        test_schnorr_generic(&ctx);
    */
}

#[wasm_bindgen]
pub fn test_vdecryption() {
    message("* Ristretto vdecryption..");
    let mut csprng = StrandRng;
    let ctx = RistrettoCtx;
    let mut fill = [0u8; 30];
    csprng.fill_bytes(&mut fill);
    let plaintext = to_plaintext_array(fill.as_ref());
    test_vdecryption_generic(&ctx, plaintext);

    message("* BigInt vdecryption..");
    let ctx: BigintCtx<P2048> = Default::default();
    let mut rng = ctx.get_rng();
    let plaintext = ctx.rnd_plaintext(&mut rng);
    test_vdecryption_generic(&ctx, plaintext);
    /*
        message("* Malachite vdecryption..");
        let ctx: MalachiteCtx<MP2048> = Default::default();
        let mut rng = ctx.get_rng();
        let plaintext = ctx.rnd_plaintext(&mut rng);
        test_vdecryption_generic(&ctx, plaintext);
    */
}

pub fn test_encrypt_exp() {
    message("* Ristretto encrypt exp..");
    let ctx = RistrettoCtx;
    test_encrypt_exp_generic(&ctx);

    message("* BigInt encrypt exp..");
    let ctx: BigintCtx<P2048> = Default::default();
    test_encrypt_exp_generic(&ctx);
    /*
        message("* Malachite encrypt exp..");
        let ctx: MalachiteCtx<MP2048> = Default::default();
        test_encrypt_exp_generic(&ctx);
    */
}

pub fn test_encrypt_pok() {
    message("* Ristretto encrypt_pok..");
    let mut csprng = StrandRng;
    let ctx = RistrettoCtx;
    let mut fill = [0u8; 30];
    csprng.fill_bytes(&mut fill);
    let plaintext = to_plaintext_array(fill.as_ref());
    test_elgamal_enc_pok_generic(&ctx, plaintext);

    message("* BigInt encrypt_pok..");
    let ctx: BigintCtx<P2048> = Default::default();
    let plaintext = ctx.rnd_plaintext(&mut csprng);
    test_elgamal_enc_pok_generic(&ctx, plaintext);
    /*
        message("* Malachite encrypt_pok..");
        let ctx: MalachiteCtx<MP2048> = Default::default();
        let plaintext = ctx.rnd_plaintext(&mut csprng);
        test_elgamal_enc_pok_generic(&ctx, plaintext);
    */
}

/*
#[wasm_bindgen]
pub fn test_shuffle() {
    message("* Ristretto shuffle..");
    let ctx = RistrettoCtx;
    test_shuffle_generic(&ctx);
    message("* BigInt shuffle..");
    let ctx: BigintCtx<P2048> = Default::default();
    test_shuffle_generic(&ctx);
    message("* Malachite shuffle..");
    let ctx: MalachiteCtx<MP2048> = Default::default();
    test_shuffle_generic(&ctx);
}

#[wasm_bindgen]
pub fn test_shuffle_serialization() {
    message("* Ristretto shuffle + serialization..");
    let ctx = RistrettoCtx;
    test_shuffle_serialization_generic(&ctx);

    message("* BigInt shuffle + serialization..");
    let ctx: BigintCtx<P2048> = Default::default();
    test_shuffle_serialization_generic(&ctx);

    message("* Malachite shuffle + serialization..");
    let ctx: MalachiteCtx<MP2048> = Default::default();
    test_shuffle_serialization_generic(&ctx);
}

pub fn test_distributed() {
    message("* Ristretto distributed..");
    let mut csprng = StrandRng;
    let ctx = RistrettoCtx;
    let mut fill = [0u8; 30];
    csprng.fill_bytes(&mut fill);
    let plaintext = to_plaintext_array(fill.as_ref());
    test_distributed_generic(&ctx, plaintext);

    message("* BigInt distributed..");
    let ctx: BigintCtx<P2048> = Default::default();
    let mut rng = ctx.get_rng();
    let plaintext = ctx.rnd_plaintext(&mut rng);
    test_distributed_generic(&ctx, plaintext);

    message("* Malachite distributed..");
    let ctx: MalachiteCtx<MP2048> = Default::default();
    let mut rng = ctx.get_rng();
    let plaintext = ctx.rnd_plaintext(&mut rng);
    test_distributed_generic(&ctx, plaintext);
}

#[wasm_bindgen]
pub fn test_distributed_serialization() {
    message("* Ristretto distributed + serialization..");
    let mut csprng = StrandRng;
    let ctx = RistrettoCtx;
    let mut ps = vec![];
    for _ in 0..1 {
        let mut fill = [0u8; 30];
        csprng.fill_bytes(&mut fill);
        let p = to_plaintext_array(fill.as_ref());
        ps.push(p);
    }
    test_distributed_serialization_generic(&ctx, ps);

    message("* BigInt distributed + serialization..");
    let ctx: BigintCtx<P2048> = Default::default();
    let mut rng = ctx.get_rng();
    let mut ps = vec![];
    for _ in 0..1 {
        let p = ctx.rnd_plaintext(&mut rng);
        ps.push(p);
    }
    test_distributed_serialization_generic(&ctx, ps);

    message("* Malachite distributed + serialization..");
    let ctx: MalachiteCtx<MP2048> = Default::default();
    let mut rng = ctx.get_rng();
    let mut ps = vec![];
    for _ in 0..1 {
        let p = ctx.rnd_plaintext(&mut rng);
        ps.push(p);
    }
    test_distributed_serialization_generic(&ctx, ps);
}

pub fn test_threshold() {
    message("* Ristretto threshold..");
    let mut csprng = StrandRng;
    let ctx = RistrettoCtx;
    let mut fill = [0u8; 30];
    csprng.fill_bytes(&mut fill);
    let plaintext = to_plaintext_array(fill.as_ref());
    let trustees = 5usize;
    let threshold = 3usize;
    test_threshold_generic(&ctx, trustees, threshold, plaintext);

    message("* BigInt threshold..");
    let trustees = 5usize;
    let threshold = 3usize;
    let ctx: BigintCtx<P2048> = Default::default();
    let mut rng = ctx.get_rng();
    let plaintext = ctx.rnd_plaintext(&mut rng);
    test_threshold_generic(&ctx, trustees, threshold, plaintext);

    message("* Malachite threshold..");
    let trustees = 5usize;
    let threshold = 3usize;
    let ctx: MalachiteCtx<MP2048> = Default::default();
    let mut rng = ctx.get_rng();
    let plaintext = ctx.rnd_plaintext(&mut rng);
    test_threshold_generic(&ctx, trustees, threshold, plaintext);
}*/

fn to_plaintext_array(input: &[u8]) -> [u8; 30] {
    crate::backend::ristretto::to_ristretto_plaintext_array(input).unwrap()
}
