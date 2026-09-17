// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use b3::messages::{
    artifact::Configuration,
    message::{Message, Signer},
    newtypes::*,
    protocol_manager::{ProtocolManager, ProtocolManagerConfig},
    statement::Statement,
};
use std::marker::PhantomData;
use strand::{
    backend::ristretto::RistrettoCtx,
    serialization::StrandSerialize,
    signature::{StrandSignaturePk, StrandSignatureSk},
};

type Manager = ProtocolManager<RistrettoCtx>;
fn signer(seed: u8) -> Manager {
    // Synthetic, fixed PKCS#8 Ed25519 seeds; never production keys.
    let mut der = vec![
        0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x04, 0x22, 0x04,
        0x20,
    ];
    der.extend([seed; 32]);
    Manager::new(StrandSignatureSk::from_der(&der).unwrap())
}
fn configuration() -> (Manager, Manager, Configuration<RistrettoCtx>) {
    let manager = signer(1);
    let trustee = signer(2);
    let cfg = Configuration::new(
        0x0102,
        StrandSignaturePk::from_sk(&manager.signing_key).unwrap(),
        vec![
            StrandSignaturePk::from_sk(&trustee.signing_key).unwrap(),
            StrandSignaturePk::from_sk(&signer(3).signing_key).unwrap(),
        ],
        2,
        PhantomData,
    );
    (manager, trustee, cfg)
}

#[test]
fn configuration_thresholds_unique_trustees_and_positions_are_checked() {
    let (manager, trustee, mut cfg) = configuration();
    assert!(cfg.is_valid());
    assert_eq!(
        cfg.get_trustee_position(&StrandSignaturePk::from_sk(&manager.signing_key).unwrap()),
        Some(1000)
    );
    assert_eq!(
        cfg.get_trustee_position(&StrandSignaturePk::from_sk(&trustee.signing_key).unwrap()),
        Some(0)
    );
    assert_eq!(cfg.get_trustee_position(&cfg.trustees[1]), Some(1));
    assert_eq!(
        cfg.get_trustee_position(&StrandSignaturePk::from_sk(&signer(4).signing_key).unwrap()),
        None
    );
    for threshold in [0, 1, 3] {
        cfg.threshold = threshold;
        assert!(!cfg.is_valid());
    }
    cfg.threshold = 2;
    let second = cfg.trustees[1].clone();
    cfg.trustees[1] = cfg.trustees[0].clone();
    assert!(!cfg.is_valid());
    cfg.trustees[1] = second;
    assert!(cfg.is_valid());
    cfg.trustees.truncate(1);
    assert!(!cfg.is_valid());
    cfg.trustees = (2..14)
        .map(|seed| StrandSignaturePk::from_sk(&signer(seed).signing_key).unwrap())
        .collect();
    cfg.threshold = 12;
    assert!(cfg.is_valid());
    cfg.trustees
        .push(StrandSignaturePk::from_sk(&signer(14).signing_key).unwrap());
    assert!(!cfg.is_valid());
}

#[test]
fn labels_use_little_endian_numbers_and_utf8_byte_lengths() {
    let (_, _, cfg) = configuration();
    let expected =
        hex::decode("0201000000000000000000000000000004030000000000000200000000000000c3a9")
            .unwrap();
    assert_eq!(cfg.label(0x0304, "é".into()), expected);
    assert_ne!(cfg.label(0x0305, "é".into()), expected);
    assert_ne!(cfg.label(0x0304, "e".into()), expected);
}

#[test]
fn valid_bootstrap_and_trustee_acknowledgement_report_distinct_signer_positions() {
    let (manager, trustee, cfg) = configuration();
    let bootstrap = Message::bootstrap_msg(&cfg, &manager).unwrap();
    let verified = bootstrap.verify(&cfg).unwrap();
    assert_eq!(verified.signer_position, 1000);
    assert_eq!(verified.artifact.unwrap(), cfg.strand_serialize().unwrap());
    let acknowledgement = Message::configuration_msg(&cfg, &trustee).unwrap();
    let verified = acknowledgement.verify(&cfg).unwrap();
    assert_eq!(verified.signer_position, 0);
    assert!(verified.artifact.is_none());
}

#[test]
fn unknown_sender_modified_signature_and_wrong_configuration_are_rejected() {
    let (manager, trustee, cfg) = configuration();
    let good = Message::configuration_msg(&cfg, &trustee).unwrap();
    assert!(good.verify(&cfg).is_ok());
    let stranger = Message::configuration_msg(&cfg, &signer(9)).unwrap();
    assert!(stranger
        .verify(&cfg)
        .unwrap_err()
        .to_string()
        .contains("not part of the configuration"));
    let mut altered = good.try_clone().unwrap();
    altered.signature = manager.signing_key.sign(b"different signed bytes").unwrap();
    assert!(altered
        .verify(&cfg)
        .unwrap_err()
        .to_string()
        .contains("Signature verification failed"));
    let mut other = cfg.clone();
    other.id += 1;
    let message = Message::configuration_msg(&other, &trustee).unwrap();
    assert!(message
        .verify(&cfg)
        .unwrap_err()
        .to_string()
        .contains("mismatched configuration hash"));
}

#[test]
fn mix_signature_number_is_bounded_by_the_configuration() {
    let (_, trustee, cfg) = configuration();
    let cfg_hash = ConfigurationHash::from_configuration(&cfg).unwrap();
    let statement = |mix| {
        Statement::MixSigned(
            17,
            cfg_hash,
            5,
            mix,
            CiphertextsHash([4; 64]),
            CiphertextsHash([5; 64]),
        )
    };
    assert!(trustee
        .sign(statement(2), None)
        .unwrap()
        .verify(&cfg)
        .is_ok());
    assert!(trustee
        .sign(statement(3), None)
        .unwrap()
        .verify(&cfg)
        .unwrap_err()
        .to_string()
        .contains("mix signature number is out of range"));
}

#[test]
fn a_configuration_artifact_requires_the_protocol_manager() {
    let (manager, trustee, cfg) = configuration();
    assert!(Message::bootstrap_msg(&cfg, &manager)
        .unwrap()
        .verify(&cfg)
        .is_ok());
    assert!(Message::bootstrap_msg(&cfg, &trustee)
        .unwrap()
        .verify(&cfg)
        .unwrap_err()
        .to_string()
        .contains("Configuration must be signed by protocol manager"));
}

#[test]
fn malformed_configuration_artifacts_return_errors_instead_of_panicking() {
    let (manager, _, cfg) = configuration();
    let mut message = Message::bootstrap_msg(&cfg, &manager).unwrap();
    assert!(message.verify(&cfg).is_ok());
    message.artifact.as_mut().unwrap()[0] ^= 1;
    assert!(message
        .verify(&cfg)
        .unwrap_err()
        .to_string()
        .contains("configuration artifact hash"));
}

#[test]
fn a_configuration_artifact_cannot_be_attached_to_an_acknowledgement() {
    let (_, trustee, cfg) = configuration();
    let mut acknowledgement = Message::configuration_msg(&cfg, &trustee).unwrap();
    assert!(acknowledgement.verify(&cfg).is_ok());
    acknowledgement.artifact = Some(cfg.strand_serialize().unwrap());
    assert!(acknowledgement
        .verify(&cfg)
        .unwrap_err()
        .to_string()
        .contains("configuration artifact requires a Configuration statement"));
}

#[test]
fn exported_manager_keys_sign_with_the_same_identity_and_reject_invalid_imports() {
    let (manager, _, _) = configuration();
    let config = ProtocolManagerConfig::from(&manager);
    let restored = config.get_signing_key().unwrap();
    let pk = StrandSignaturePk::from_sk(&manager.signing_key).unwrap();
    pk.verify(
        &restored.sign(b"synthetic challenge").unwrap(),
        b"synthetic challenge",
    )
    .unwrap();
    for value in ["!", "AA==", ""] {
        assert!(ProtocolManagerConfig {
            signing_key: value.into()
        }
        .get_signing_key()
        .is_err());
    }
}

#[cfg(feature = "client")]
#[test]
fn postgres_message_mapping_preserves_statement_fields_and_rejects_integer_overflow() {
    use b3::client::pgsql::B3MessageRow;
    let (_, trustee, cfg) = configuration();
    let cfg_hash = ConfigurationHash::from_configuration(&cfg).unwrap();
    let message = |batch, mix| {
        trustee
            .sign(
                Statement::MixSigned(
                    123,
                    cfg_hash,
                    batch,
                    mix,
                    CiphertextsHash([4; 64]),
                    CiphertextsHash([5; 64]),
                ),
                None,
            )
            .unwrap()
    };
    let control = message(i32::MAX as usize, i32::MAX as usize);
    let wire = control.strand_serialize().unwrap();
    let row = B3MessageRow::try_from(control).unwrap();
    assert_eq!(row.batch, i32::MAX);
    assert_eq!(row.mix_number, i32::MAX);
    assert_eq!(row.statement_timestamp, 123);
    assert_eq!(row.statement_kind, "MixSigned");
    assert_eq!(row.message, wire);
    assert_eq!(row.version, "1");
    assert!(B3MessageRow::try_from(message(i32::MAX as usize + 1, 0)).is_err());
    assert!(B3MessageRow::try_from(message(0, i32::MAX as usize + 1)).is_err());
}
