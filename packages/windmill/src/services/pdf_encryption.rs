// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{Context, Result};
use lopdf::encryption::crypt_filters::{Aes128CryptFilter, CryptFilter};
use lopdf::{Document, EncryptionState, EncryptionVersion, Permissions};
use std::collections::BTreeMap;
use std::sync::Arc;
use uuid::Uuid;

/// Applies standard PDF AES-128 user-password encryption. The random owner
/// password is intentionally discarded: administrators only need the user
/// password stored in the task-scoped vault entry.
pub fn encrypt_pdf(pdf_bytes: &[u8], user_password: &str) -> Result<Vec<u8>> {
    let mut document = Document::load_mem(pdf_bytes)
        .with_context(|| "Failed to parse the rendered PDF before encryption")?;
    let owner_password = Uuid::new_v4().simple().to_string();
    let filter: Arc<dyn CryptFilter> = Arc::new(Aes128CryptFilter);
    let version = EncryptionVersion::V4 {
        document: &document,
        encrypt_metadata: true,
        crypt_filters: BTreeMap::from([(b"StdCF".to_vec(), filter)]),
        stream_filter: b"StdCF".to_vec(),
        string_filter: b"StdCF".to_vec(),
        owner_password: &owner_password,
        user_password,
        permissions: Permissions::all(),
    };
    let state = EncryptionState::try_from(version)
        .with_context(|| "Failed to prepare PDF password encryption")?;
    document
        .encrypt(&state)
        .with_context(|| "Failed to encrypt the rendered PDF")?;

    let mut encrypted = Vec::new();
    document
        .save_to(&mut encrypted)
        .with_context(|| "Failed to serialize the encrypted PDF")?;
    Ok(encrypted)
}

#[cfg(test)]
mod tests {
    use super::encrypt_pdf;
    use lopdf::{dictionary, Document, Object, StringFormat};

    fn sample_pdf() -> Vec<u8> {
        let mut document = Document::with_version("1.5");
        let pages_id = (1, 0);
        let page_id = (2, 0);

        document.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages",
                "Kids" => vec![Object::Reference(page_id)],
                "Count" => 1,
            }),
        );
        document.objects.insert(
            page_id,
            Object::Dictionary(dictionary! {
                "Type" => "Page",
                "Parent" => Object::Reference(pages_id),
                "MediaBox" => vec![
                    Object::Integer(0),
                    Object::Integer(0),
                    Object::Integer(612),
                    Object::Integer(792),
                ],
            }),
        );
        let catalog_id = document.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => Object::Reference(pages_id),
        });
        document.trailer.set("Root", Object::Reference(catalog_id));
        document.trailer.set(
            "ID",
            Object::Array(vec![
                Object::String(vec![1; 16], StringFormat::Literal),
                Object::String(vec![2; 16], StringFormat::Literal),
            ]),
        );

        let mut bytes = Vec::new();
        document.save_to(&mut bytes).unwrap();
        bytes
    }

    #[test]
    fn requires_the_configured_password_to_open_the_pdf() {
        let encrypted = encrypt_pdf(&sample_pdf(), "document-password").unwrap();

        let document = Document::load_mem(&encrypted).unwrap();
        assert!(document.is_encrypted());

        let mut wrong_password = Document::load_mem(&encrypted).unwrap();
        assert!(wrong_password.decrypt("wrong-password").is_err());

        let mut correct_password = Document::load_mem(&encrypted).unwrap();
        correct_password.decrypt("document-password").unwrap();
        assert!(!correct_password.is_encrypted());
    }
}
