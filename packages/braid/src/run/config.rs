use crate::protocol2::trustee::Trustee;
use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use strand::context::Ctx;
use strand::signature::StrandSignaturePk;

#[derive(Serialize, Deserialize)]
pub struct TrusteeConfig {
    // base64 encoding of a der encoded pkcs#8 v1
    pub signing_key_sk: String,
    // base64 encoding of a der encoded spki
    pub signing_key_pk: String,
    // base64 encoding of a sign::SymmetricKey
    pub encryption_key: String,
}
impl TrusteeConfig {
    pub fn from<C: Ctx>(trustee: &Trustee<C>) -> TrusteeConfig {
        let sk_string = trustee.signing_key.to_der_b64_string().unwrap();

        /*let pk_bytes = StrandSignaturePk::from(&trustee.signing_key)
        .unwrap()
        .strand_serialize()
        .unwrap();*/
        let pk_string = StrandSignaturePk::from_sk(&trustee.signing_key)
            .unwrap()
            .to_der_b64_string()
            .unwrap();

        // Compatible with both aes and chacha20poly backends
        let ek_bytes = trustee.encryption_key.as_slice();

        // let pk_string: String = general_purpose::STANDARD_NO_PAD.encode(pk_bytes);
        let ek_string: String = general_purpose::STANDARD_NO_PAD.encode(ek_bytes);

        TrusteeConfig {
            signing_key_sk: sk_string,
            signing_key_pk: pk_string,
            encryption_key: ek_string,
        }
    }
}
