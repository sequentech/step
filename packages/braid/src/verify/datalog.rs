// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::protocol::datalog::{hashes_add, hashes_init};
use crate::protocol::predicate::Predicate;
use b3::messages::newtypes::*;
use crepe::crepe;
use std::collections::HashSet;

///////////////////////////////////////////////////////////////////////////
// Logic
///////////////////////////////////////////////////////////////////////////
crepe! {

    @input
    pub struct InP(Predicate);

    // Input relations, used to convert from InP predicates to crepe relations
    struct Configuration(ConfigurationHash, TrusteePosition, TrusteeCount, Threshold);
    struct ConfigurationSigned(ConfigurationHash, TrusteePosition);
    struct ConfigurationSignedAll(ConfigurationHash, TrusteePosition, TrusteeCount, Threshold);
    struct PublicKeySignedAll(ConfigurationHash, PublicKeyHash, SharesHashes);
    struct PublicKey(ConfigurationHash, PublicKeyHash, SharesHashes, ChannelsHashes, TrusteePosition);
    struct PublicKeySigned(ConfigurationHash, PublicKeyHash, SharesHashes, ChannelsHashes, TrusteePosition);
    struct Ballots(ConfigurationHash, BatchNumber, CiphertextsHash, PublicKeyHash, TrusteeSet);
    struct MixComplete(ConfigurationHash, BatchNumber, MixNumber, CiphertextsHash, TrusteePosition);
    struct DecryptionFactors(ConfigurationHash, BatchNumber, DecryptionFactorsHash, CiphertextsHash, SharesHashes, TrusteePosition);
    struct Plaintexts(ConfigurationHash, BatchNumber,PlaintextsHash, DecryptionFactorsHashes, CiphertextsHash, PublicKeyHash, TrusteePosition);
    struct PlaintextsSigned(ConfigurationHash, BatchNumber, PlaintextsHash, DecryptionFactorsHashes, CiphertextsHash, PublicKeyHash, TrusteePosition);
    struct Mix(ConfigurationHash, BatchNumber, CiphertextsHash, CiphertextsHash, MixNumber, TrusteePosition);
    struct MixSigned(ConfigurationHash, BatchNumber, CiphertextsHash, CiphertextsHash, TrusteePosition);

    ConfigurationSignedAll(config_hash, self_position, num_t, threshold) <- InP(p),
    let Predicate::ConfigurationSignedAll(config_hash, self_position, num_t, threshold) = p;

    PublicKeySignedAll(cfg_h, pk_h, shares_hs) <- InP(p),
    let Predicate::PublicKeySignedAll(cfg_h, pk_h, shares_hs) = p;

    Ballots(cfg_h, batch, ballots_h, pk_h, selected) <- InP(p),
    let Predicate::Ballots(cfg_h, batch, ballots_h, pk_h, selected) = p;

    MixComplete(cfg_h, batch, mix_number, ciphertexts_h, signer_t) <- InP(p),
    let Predicate::MixComplete(cfg_h, batch, mix_number, ciphertexts_h, signer_t) = p;

    Mix(cfg_h, batch, source_h, mix_h, mix_number, signer_t) <- InP(p),
    let Predicate::Mix(cfg_h, batch, source_h, mix_h, mix_number, signer_t) = p;

    MixSigned(cfg_h, batch, source_h, ciphertexts_h, signer_t) <- InP(p),
    let Predicate::MixSigned(cfg_h, batch, source_h, ciphertexts_h, signer_t) = p;

    DecryptionFactors(cfg_h, batch, dfactors_h, ciphertexts_h, shares_hs, signer_t) <- InP(p),
    let Predicate::DecryptionFactors(cfg_h, batch, dfactors_h, ciphertexts_h, shares_hs, signer_t) = p;

    Plaintexts(cfg_h, batch, plaintexts_h, dfactors_hs, cipher_h, pk_h, signer_t) <- InP(p),
    let Predicate::Plaintexts(cfg_h, batch, plaintexts_h, dfactors_hs, cipher_h, pk_h, signer_t) = p;

    PlaintextsSigned(cfg_h, batch, plaintexts_h, df_hs, cipher_h, pk_h, signer_t) <- InP(p),
    let Predicate::PlaintextsSigned(cfg_h, batch, plaintexts_h, df_hs, cipher_h, pk_h, signer_t) = p;

    ConfigurationSigned(cfg_h, signer_t) <- InP(p),
    let Predicate::ConfigurationSigned(cfg_h, signer_t) = p;

    PublicKey(config_hash, pk_hash, shares_hs, channels_hs, signer_t) <- InP(p),
    let Predicate::PublicKey(config_hash, pk_hash, shares_hs, channels_hs, signer_t) = p;

    PublicKeySigned(config_hash, pk_hash, shares_hs, channels_hs, signer_t) <- InP(p),
    let Predicate::PublicKeySigned(config_hash, pk_hash, shares_hs, channels_hs, signer_t) = p;

    Configuration(cfg_h, self_position, num_t, threshold) <- InP(p),
    let Predicate::Configuration(cfg_h, self_position, num_t, threshold) = p;

    // Intermediate relations

    struct ConfigurationSignedUpTo(ConfigurationHash, TrusteePosition);
    struct PublicKeySignedUpTo(ConfigurationHash, PublicKeyHash, SharesHashes, TrusteePosition);
    struct MixVerifiedUpto(ConfigurationHash, BatchNumber, CiphertextsHash, MixingHashes, TrusteeCount);
    struct MixRepeat(ConfigurationHash, BatchNumber);

    @output
    pub struct RootVerified(
        pub(crate) ConfigurationHash,
        pub(crate) PublicKeyHash,
    );

    @output
    pub struct Target(
        pub(crate) ConfigurationHash,
        pub(crate) BatchNumber,
        pub(crate) PublicKeyHash,
        pub(crate) CiphertextsHash,
        pub(crate) PlaintextsHash,
    );

    @output
    pub struct Verified(
        pub(crate) ConfigurationHash,
        pub(crate) BatchNumber,
        pub(crate) CiphertextsHash,
        pub(crate) CiphertextsHash,
        pub(crate) PublicKeyHash,
        pub(crate) PlaintextsHash,
        pub(crate) MixingHashes,
    );

    RootVerified(cfg_h, pk_h) <-
    ConfigurationSignedAll(cfg_h, _, _num_t, _),
    PublicKeySignedAll(cfg_h, pk_h, _shares_hs),
    PublicKeySigned(cfg_h, pk_h, _, _, VERIFIER_INDEX);

    Target(cfg_h, batch, pk_h, ballots_h, plaintexts_h, ) <-
    ConfigurationSignedAll(cfg_h, _, _num_t, _),
    Ballots(cfg_h, batch, ballots_h, pk_h, _),
    Plaintexts(cfg_h, batch, plaintexts_h, _, _, _, _);

    ConfigurationSignedUpTo(cfg_h, n + 1) <-
    ConfigurationSignedUpTo(cfg_h, n),
    ConfigurationSigned(cfg_h, n + 1);

    ConfigurationSignedUpTo(cfg_h, 0) <-
    ConfigurationSigned(cfg_h, 0);

    PublicKeySigned(cfg_h, pk_h, shares_hs, channels_hs, 0) <-
    PublicKey(cfg_h, pk_h, shares_hs, channels_hs, 0);

    PublicKeySignedUpTo(cfg_h, pk_h, shares_hs, n + 1) <-
    PublicKeySignedUpTo(cfg_h, pk_h, shares_hs, n),
    PublicKeySigned(cfg_h, pk_h, shares_hs, _channels_hs, n + 1);

    PublicKeySignedUpTo(cfg_h, pk_h, shares_hs, 0) <-
    PublicKeySigned(cfg_h, pk_h, shares_hs, _channels_hs, 0);

    PublicKeySignedAll(cfg_h, pk_h, shares_hs) <-
    ConfigurationSignedAll(cfg_h, _self_p, num_t, _threshold),
    PublicKeySignedUpTo(cfg_h, pk_h, shares_hs, num_t - 1);

    ConfigurationSignedAll(cfg_h, self_position, num_t, threshold) <-
    Configuration(cfg_h, self_position, num_t, threshold),
    // We subtract 1 since trustees positions are 0 based
    ConfigurationSignedUpTo(cfg_h, num_t - 1);

    MixVerifiedUpto(cfg_h, batch, target_h, mixing_hs, 1) <-
    ConfigurationSignedAll(cfg_h, _, _num_t, _),
    PublicKeySignedAll(cfg_h, pk_h, _shares_hs),
    Ballots(cfg_h, batch, ballots_h, pk_h, _),
    !Plaintexts(cfg_h, batch, _, _, ballots_h, _, _),
    MixSigned(cfg_h, batch, ballots_h, target_h, VERIFIER_INDEX),
    let mixing_hs = MixingHashes(hashes_init(ballots_h.0));

    MixVerifiedUpto(cfg_h, batch, ciphertexts_h, new_mixing_hs, n + 1) <-
    MixSigned(cfg_h, batch, source_h, ciphertexts_h, VERIFIER_INDEX),
    MixVerifiedUpto(cfg_h, batch, source_h, mixing_hs, n),
    !Plaintexts(cfg_h, batch, _, _, source_h, _, _),
    let new_mixing_hs = MixingHashes(hashes_add(mixing_hs.0, source_h.0));

    // Extra checks to ensure that all mixing trustees are unique
    MixRepeat(cfg_h, batch) <-
    ConfigurationSignedAll(cfg_h, _, _num_t, _threshold),
    Mix(cfg_h, batch, _, _, mix_number1, signer_t1),
    Mix(cfg_h, batch, _, _, mix_number2, signer_t2),
    (mix_number1 != mix_number2),
    (signer_t1 == signer_t2);

    Verified(cfg_h, batch, ballots_h, decrypted_ciphertexts_h, decryption_pk_h, plaintexts_h, new_mixing_hs) <-
    ConfigurationSignedAll(cfg_h, _, _num_t, threshold),
    MixVerifiedUpto(cfg_h, batch, last_ciphertexts_h, mixing_hs, threshold),
    MixVerifiedUpto(cfg_h, batch, _, _, 1),
    MixSigned(cfg_h, batch, ballots_h, _target_h, VERIFIER_INDEX),
    Ballots(cfg_h, batch, ballots_h, _, selected),
    Plaintexts(cfg_h, batch, plaintexts_h, _dfactors_hs, decrypted_ciphertexts_h, decryption_pk_h, selected[0] - 1),
    !MixRepeat(cfg_h, batch),
    let new_mixing_hs = MixingHashes(hashes_add(mixing_hs.0, last_ciphertexts_h.0));

}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub(crate) struct S;

impl S {
    pub(crate) fn run(
        &self,
        predicates: &Vec<Predicate>,
    ) -> (HashSet<RootVerified>, HashSet<Target>, HashSet<Verified>) {
        let mut runtime = Crepe::new();
        let inputs: Vec<InP> = predicates.iter().map(|p| InP(*p)).collect();
        runtime.extend(&inputs);

        runtime.run()
    }
}
