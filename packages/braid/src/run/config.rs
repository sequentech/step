use crate::protocol2::trustee::{ProtocolManager, Trustee};
use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use strand::context::Ctx;
use strand::serialization::{StrandDeserialize, StrandSerialize};
use strand::signature::{StrandSignaturePk, StrandSignatureSk};

#[derive(Serialize, Deserialize)]
pub struct TrusteeConfig {
    // base64 encoding of a StrandSignatureSk serialization
    pub signing_key_sk: String,
    // base64 encoding of a StrandSignaturePk serialization
    pub signing_key_pk: String,
    // base64 encoding of a GenericArray<u8, U32>,
    pub encryption_key: String,
}
impl TrusteeConfig {
    pub fn from<C: Ctx>(trustee: &Trustee<C>) -> TrusteeConfig {
        let sk_bytes = trustee.signing_key.strand_serialize().unwrap();
        // FIXME unwrap
        let pk_bytes = StrandSignaturePk::from(&trustee.signing_key).unwrap()
            .strand_serialize()
            .unwrap();
        let ek_bytes = trustee.encryption_key.as_slice();

        let sk_string: String = general_purpose::STANDARD_NO_PAD.encode(sk_bytes);
        let pk_string: String = general_purpose::STANDARD_NO_PAD.encode(pk_bytes);
        let ek_string: String = general_purpose::STANDARD_NO_PAD.encode(ek_bytes);

        TrusteeConfig {
            signing_key_sk: sk_string,
            signing_key_pk: pk_string,
            encryption_key: ek_string,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct ProtocolManagerConfig {
    // base64 encoding of a StrandSignatureSk serialization
    pub signing_key: String,
}
impl ProtocolManagerConfig {
    pub fn from<C: Ctx>(pm: &ProtocolManager<C>) -> ProtocolManagerConfig {
        let sk_bytes = pm.signing_key.strand_serialize().unwrap();

        let sk_string: String = general_purpose::STANDARD_NO_PAD.encode(sk_bytes);

        ProtocolManagerConfig {
            signing_key: sk_string,
        }
    }
    pub fn get_signing_key(&self) -> anyhow::Result<StrandSignatureSk> {
        let bytes = general_purpose::STANDARD_NO_PAD.decode(&self.signing_key)?;

        Ok(StrandSignatureSk::strand_deserialize(&bytes)?)
    }
}
