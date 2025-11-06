// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use wasm_bindgen::prelude::*;

use crate::backend::ristretto::RistrettoCtx;
use crate::backend::ristretto::RistrettoPointS;
use crate::context::Ctx;
use crate::elgamal::PrivateKey;
use crate::shuffler::Shuffler;
use crate::util::Par;
use rayon::iter::ParallelIterator;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
    #[wasm_bindgen]
    fn postMessage(s: &str);
    #[no_mangle]
    #[used]
    static performance: web_sys::Performance;
}

pub fn message(s: &str) {
    postMessage(s);
}

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct CiphertextS {
    pub gr: String,
    pub mhr: String,
}

#[derive(Serialize, Deserialize)]
pub struct PlaintextS {
    pub value: String,
}

fn pretty_print_int(number: isize) -> String {
    let mut pretty_number = String::new();
    let number_str = number.to_string();
    let chars = number_str.chars().rev().enumerate();
    for (index, character) in chars {
        if index != 0 && index % 3 == 0 {
            pretty_number.insert(0, ',');
        }
        pretty_number.insert(0, character);
    }
    return pretty_number;
}

use crate::elgamal::Ciphertext;
pub fn to_ciphertext_s(ciphertext: &Ciphertext<RistrettoCtx>) -> CiphertextS {
    let gr = hex::encode(ciphertext.gr.0.compress().to_bytes());
    let mhr = hex::encode(ciphertext.mhr.0.compress().to_bytes());

    CiphertextS { gr, mhr }
}

fn from_ciphertext_s(ciphertext: &CiphertextS) -> Ciphertext<RistrettoCtx> {
    let ctx = RistrettoCtx;
    let gr = ctx
        .element_from_bytes(&hex::decode(&ciphertext.gr).unwrap())
        .unwrap();
    let mhr = ctx
        .element_from_bytes(&hex::decode(&ciphertext.mhr).unwrap())
        .unwrap();

    Ciphertext { gr, mhr }
}

fn to_plaintext_s(plaintext: &RistrettoPointS) -> PlaintextS {
    let ctx = RistrettoCtx;
    // let value = hex::encode(ctx.decode(&plaintext));
    let decoded = ctx.decode(&plaintext);
    let value = if decoded[0] == 1u8 {
        "yes".to_string()
    } else if decoded[0] == 0u8 {
        "no".to_string()
    } else {
        "error".to_string()
    };

    PlaintextS { value }
}

fn secret_key() -> PrivateKey<RistrettoCtx> {
    let ctx = RistrettoCtx;
    let sk = ctx.exp_from_u64(1u64);
    PrivateKey::from(&sk, &ctx)
}

#[wasm_bindgen]
pub fn encrypt(nyes: u32, nno: u32) -> JsValue {
    let ctx = RistrettoCtx;
    let sk = secret_key();
    let pk = sk.get_pk();

    let yes = [1u8; 30];
    let no = [0u8; 30];

    let pyes = ctx.encode(&yes).unwrap();
    let pno = ctx.encode(&no).unwrap();

    let now = performance.now();
    let mut ys: Vec<CiphertextS> = (0..nyes)
        .par()
        .map(|_| {
            let c = pk.encrypt(&pyes);
            to_ciphertext_s(&c)
        })
        .collect();

    let mut ns: Vec<CiphertextS> = (0..nno)
        .par()
        .map(|_| {
            let c = pk.encrypt(&pno);
            to_ciphertext_s(&c)
        })
        .collect();

    let per_sec =
        ((nyes + nno) as f64 / ((performance.now() - now) / 1000.0)) as isize;
    let per_hour = per_sec * 3_600;

    message(&format!(
        "Encrypt: {} ciphertexts/second - {} ciphertexts/hour",
        pretty_print_int(per_sec),
        pretty_print_int(per_hour)
    ));

    message(&format!(
        "Encrypted {} values",
        pretty_print_int((nyes + nno) as isize)
    ));

    let _ = &ys.append(&mut ns);
    serde_wasm_bindgen::to_value(&ys).unwrap()
}

#[wasm_bindgen]
pub fn shuffle(value: JsValue) -> JsValue {
    let ctx = RistrettoCtx;
    let sk = secret_key();
    let pk = sk.get_pk();

    let values: Vec<CiphertextS> =
        serde_wasm_bindgen::from_value(value).unwrap();

    let es: Vec<Ciphertext<RistrettoCtx>> =
        values.iter().map(|v| from_ciphertext_s(v)).collect();
    let seed = vec![];
    let hs = ctx.generators(es.len() + 1, &seed);
    let shuffler = Shuffler {
        pk: &pk,
        generators: &hs,
        ctx: ctx.clone(),
    };
    let now = performance.now();

    message("Gen shuffle..");
    let (e_primes, rs, perm) = shuffler.gen_shuffle(&es);
    message("Gen proof..");
    let proof = shuffler
        .gen_proof(&es, &e_primes, &rs, &perm, &vec![])
        .unwrap();

    message("Verify proof..");
    let ok = shuffler
        .check_proof(&proof, &es, &e_primes, &vec![])
        .unwrap();
    message(&format!("Proof ok: {}", ok));

    let per_sec =
        (es.len() as f64 / ((performance.now() - now) / 1000.0)) as isize;
    let per_hour = per_sec * 3_600;
    message(&format!(
        "Prove + Verify: {} ciphertexts/second - {} ciphertexts/hour",
        pretty_print_int(per_sec),
        pretty_print_int(per_hour)
    ));
    message(&format!(
        "Shuffled {} values",
        pretty_print_int(es.len() as isize)
    ));

    let cs: Vec<CiphertextS> =
        e_primes.iter().map(|e| to_ciphertext_s(e)).collect();

    serde_wasm_bindgen::to_value(&cs).unwrap()
}

#[wasm_bindgen]
pub fn decrypt(value: JsValue) -> JsValue {
    let sk = secret_key();

    let values: Vec<CiphertextS> =
        serde_wasm_bindgen::from_value(value).unwrap();

    let now = performance.now();
    let ps: Vec<PlaintextS> = values
        .par()
        .map(|v| {
            let c = from_ciphertext_s(&v);
            to_plaintext_s(&sk.decrypt(&c))
        })
        .collect();

    let per_sec =
        (ps.len() as f64 / ((performance.now() - now) / 1000.0)) as isize;
    let per_hour = per_sec * 3_600;
    message(&format!(
        "Decrypt: {} ciphertexts/second - {} ciphertexts/hour",
        pretty_print_int(per_sec),
        pretty_print_int(per_hour)
    ));
    message(&format!(
        "Decrypted {} values",
        pretty_print_int(ps.len() as isize)
    ));

    serde_wasm_bindgen::to_value(&ps).unwrap()
}
