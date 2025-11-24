// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

#[cfg(feature = "wasm")]
pub mod rustcrypto;

#[cfg(feature = "openssl_full")]
pub mod openssl;

#[cfg(not(feature = "openssl_full"))]
pub mod dalek;

#[cfg(all(feature = "openssl_full", feature = "rustcrypto_interop_test"))]
#[cfg(test)]
pub(crate) mod tests {
    use super::openssl::{
        StrandSignature as Ssl_sig, StrandSignaturePk as Ssl_pk,
    };

    use super::rustcrypto::{
        StrandSignature as RCrypto_sig, StrandSignaturePk as RCrypto_pk,
        StrandSignatureSk as RCrypto_sk,
    };
    use crate::serialization::{StrandDeserialize, StrandSerialize};

    #[test]
    pub fn test_signature_interop() {
        let msg = b"ok";
        let msg2 = b"not_ok";

        let (vk_bytes, sig_bytes) = {
            let sk = RCrypto_sk::gen().unwrap();
            let sk_b = sk.strand_serialize().unwrap();
            let sk_d = RCrypto_sk::strand_deserialize(&sk_b).unwrap();

            let sig: RCrypto_sig = sk_d.sign(msg).unwrap();

            let sig_bytes = sig.strand_serialize().unwrap();
            let vk_bytes =
                RCrypto_pk::from(&sk_d).unwrap().strand_serialize().unwrap();

            (vk_bytes, sig_bytes)
        };

        let vk = Ssl_pk::strand_deserialize(&vk_bytes).unwrap();
        vk.check_key().unwrap();
        let sig = Ssl_sig::strand_deserialize(&sig_bytes).unwrap();

        let ok = vk.verify(&sig, msg);
        ok.unwrap();
        // assert!(ok.is_ok());

        let not_ok = vk.verify(&sig, msg2);
        assert!(not_ok.is_err());
    }
}
