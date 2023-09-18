// SPDX-FileCopyrightText: 2021 David Ruescas <david@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
pub mod dalek;
pub mod rustcrypto;
pub mod zcash;

#[cfg(feature = "openssl")]
pub mod openssl;

#[cfg(feature = "openssl")]
#[cfg(test)]
pub(crate) mod tests {
    use super::openssl::{
        StrandSignature as Ssl_sig, StrandSignaturePk as Ssl_pk,
    };

    use super::rustcrypto::{
        StrandSignature as RCrypto_sig, StrandSignaturePk as RCrypto_pk,
        StrandSignatureSk as RCrypto_sk,
    };
    use crate::rng::StrandRng;
    use crate::serialization::{StrandDeserialize, StrandSerialize};

    #[test]
    pub fn test_signature_interop() {
        let msg = b"ok";
        let msg2 = b"not_ok";
        let mut rng = StrandRng;

        let (vk_bytes, sig_bytes) = {
            let sk = RCrypto_sk::new(&mut rng).unwrap();
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
