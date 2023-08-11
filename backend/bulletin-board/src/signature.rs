// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use strand::signature::{
    StrandSignature as Signature, StrandSignaturePk as PublicKey,
    StrandSignatureSk as SecretKey,
};

use crate::{Board, CreateBoardRequest, Error, ModifyBoardRequest};

/// Trait representing objects whose signature is verifiable
pub trait Signable: Sized {
    fn signable_data(&self) -> Result<Vec<u8>, Error>;
    fn signature(&self) -> Result<Signature, Error>;
    fn signer_public_key(&self) -> Result<PublicKey, Error>;
    fn sign(&self, secret_key: &SecretKey) -> Result<Self, Error>;
}

/// Trait used to verify signature of a Signed object.
///
/// You normally shouldn't implement it directly, but instead implement the
/// `Signed` trait and use the automatic blanked implementation of `Verify`.
pub trait Verify {
    fn verify_signature(&self) -> Result<(), Error>;
}

/// Blanket implementation of `Verify` for all types implementing `Signed`.
impl<T: Signable> Verify for T {
    fn verify_signature(&self) -> Result<(), Error> {
        let signature = self.signature()?;
        let public_key = self.signer_public_key()?;
        public_key
            .verify(&signature, &self.signable_data()?)
            .map_err(|error_message| {
                Error::SignatureVerificationError(error_message.into())
            })
    }
}

impl From<&CreateBoardRequest> for Board {
    fn from(request: &CreateBoardRequest) -> Self {
        Board {
            uuid: request.board_uuid.clone(),
            name: request.board_name.clone(),
            description: request.board_description.clone(),
            metadata: request.board_metadata.clone(),
            is_archived: false,
            permissions: request.permissions.clone(),
            public_key: "".to_string(),
        }
    }
}

impl From<&ModifyBoardRequest> for Board {
    fn from(request: &ModifyBoardRequest) -> Self {
        Board {
            uuid: request.board_uuid.clone(),
            name: request.board_name.clone(),
            description: request.board_description.clone(),
            metadata: request.board_metadata.clone(),
            is_archived: request.is_archived,
            permissions: request.permissions.clone(),
            public_key: "".to_string(),
        }
    }
}
