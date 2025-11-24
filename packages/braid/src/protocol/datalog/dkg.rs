// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crepe::crepe;

// Distributed key generation.
//
// Each trustee generates a channel with which to
// secretly receive shares from all trustees. They
// also verify and sign all these channels.
//
// Once all trustees have signed all channels they
// compute and post shares.
//
// Once all shares have been posted the trustee
// at position 0 (as defined in the Configuration)
// will combine them to produce the public key.
// The rest of trustees will verify the public key by
// computing it independently, and sign it. Note that
// each trustee will verify their secret shares as part
// of this process.
//
// Once all trustees have signed the public key,
// distributed key generation is complete.
//
// Actions:                 GenChannel
//                          SignChannels
//                          ComputeShares
//                          ComputePublicKey
//                          SignPublicKey
crepe! {

    ///////////////////////////////////////////////////////////////////////////
    // Inference.
    ///////////////////////////////////////////////////////////////////////////

    A(Action::GenChannel(cfg_h)) <-
    ConfigurationSignedAll(cfg_h, self_position, _num_t, _threshold),
    !Channel(cfg_h, _, self_position);

    ChannelsUpTo(cfg_h, new_hashes, n + 1) <-
    ChannelsUpTo(cfg_h, hashes, n),
    Channel(cfg_h, channel_hash, n + 1),
    let new_hashes = ChannelsHashes(super::hashes_set(hashes.0, n + 1, channel_hash.0));

    ChannelsUpTo(cfg_h, ChannelsHashes(hashes), 0) <-
    Channel(cfg_h, hash, 0),
    let hashes = super::hashes_init(hash.0);

    ChannelsAll(cfg_h, ChannelsHashes(hashes.0)) <-
    ConfigurationSignedAll(_config_hash, _self_position, num_t, _threshold),
    // We subtract 1 since trustees positions are 0 based
    ChannelsUpTo(cfg_h, hashes, num_t - 1);

    A(Action::SignChannels(cfg_h, hashes, self_position, num_t)) <-
    ChannelsAll(cfg_h, hashes),
    ConfigurationSignedAll(cfg_h, self_position, num_t, _threshold),
    !ChannelsAllSigned(cfg_h, hashes, self_position);

    ChannelsAllSignedUpTo(cfg_h, channels_hs, n + 1) <-
    ChannelsAllSignedUpTo(cfg_h, channels_hs, n),
    ChannelsAllSigned(cfg_h, channels_hs, n + 1);

    ChannelsAllSignedUpTo(cfg_h, channels_hs, 0) <-
    ChannelsAllSigned(cfg_h, channels_hs, 0);

    ChannelsAllSignedAll(cfg_h, channels_hs) <-
    ConfigurationSignedAll(cfg_h, _self_position, num_t, _threshold),
    ChannelsAllSignedUpTo(cfg_h, channels_hs, num_t - 1);

    A(Action::ComputeShares(cfg_h, channels_hs, num_t, threshold)) <-
    ChannelsAllSignedAll(cfg_h, channels_hs),
    ConfigurationSignedAll(cfg_h, self_position, num_t, threshold),
    !Shares(cfg_h, _, self_position);

    SharesUpTo(cfg_h, new_hashes, n + 1) <-
    SharesUpTo(cfg_h, hashes, n),
    Shares(cfg_h, shares_hash, n + 1),
    let new_hashes = SharesHashes(super::hashes_set(hashes.0, n + 1, shares_hash.0));

    SharesUpTo(cfg_h, SharesHashes(hashes), 0) <-
    Shares(cfg_h, hash, 0),
    let hashes = super::hashes_init(hash.0);

    SharesAll(cfg_h, SharesHashes(hashes.0)) <-
    ConfigurationSignedAll(_config_hash, _self_position, num_t, _threshold),
    // We subtract 1 since trustees positions are 0 based
    SharesUpTo(cfg_h, hashes, num_t - 1);

    A(Action::ComputePublicKey(cfg_h, shares_hs, channels_hs, 0, num_t, threshold)) <-
    SharesAll(cfg_h, shares_hs),
    ChannelsAllSignedAll(cfg_h, channels_hs),
    ConfigurationSignedAll(cfg_h, 0, num_t, threshold),
    !PublicKey(cfg_h, _, shares_hs, channels_hs, 0);

    PublicKeySigned(cfg_h, pk_h, shares_hs, channels_hs, 0) <-
    PublicKey(cfg_h, pk_h, shares_hs, channels_hs, 0);

    A(Action::SignPublicKey(cfg_h, pk_h, shares_hs, channels_hs, self_p, num_t, threshold)) <-
    PublicKey(cfg_h, pk_h, shares_hs, channels_hs, 0),
    ConfigurationSignedAll(cfg_h, self_p, num_t, threshold),
    !PublicKeySigned(cfg_h, _, shares_hs, channels_hs, self_p);

    PublicKeySignedUpTo(cfg_h, pk_h, shares_hs, n + 1) <-
    PublicKeySignedUpTo(cfg_h, pk_h, shares_hs, n),
    PublicKeySigned(cfg_h, pk_h, shares_hs, _channels_hs, n + 1);

    PublicKeySignedUpTo(cfg_h, pk_h, shares_hs, 0) <-
    PublicKeySigned(cfg_h, pk_h, shares_hs, _channels_hs, 0);

    OutP(Predicate::ChannelsAllSignedAll(cfg_h, channels_hs)) <-
    ChannelsAllSignedAll(cfg_h, channels_hs);

    OutP(Predicate::PublicKeySignedAll(cfg_h, pk_h, shares_hs)) <-
    ConfigurationSignedAll(cfg_h, _self_p, num_t, _threshold),
    // we subtract 1 since trustees positions are 0 based
    PublicKeySignedUpTo(cfg_h, pk_h, shares_hs, num_t - 1);

    ///////////////////////////////////////////////////////////////////////////
    // Input relations.
    ///////////////////////////////////////////////////////////////////////////

    struct ConfigurationSignedAll(ConfigurationHash, TrusteePosition, TrusteeCount, Threshold);
    struct Channel(ConfigurationHash, ChannelHash, TrusteePosition);
    struct ChannelsAllSigned(ConfigurationHash, ChannelsHashes, TrusteePosition);
    struct Shares(ConfigurationHash, SharesHash, TrusteePosition);
    struct SharesSigned(ConfigurationHash, SharesHash, TrusteePosition);
    struct PublicKey(ConfigurationHash, PublicKeyHash, SharesHashes, ChannelsHashes, TrusteePosition);
    struct PublicKeySigned(ConfigurationHash, PublicKeyHash, SharesHashes, ChannelsHashes, TrusteePosition);

    ///////////////////////////////////////////////////////////////////////////
    // Convert from InP predicates to crepe relations.
    ///////////////////////////////////////////////////////////////////////////

    ConfigurationSignedAll(config_hash, self_position, num_t, threshold) <- InP(p),
    let Predicate::ConfigurationSignedAll(config_hash, self_position, num_t, threshold) = p;

    Channel(config_hash, hash, signer_position) <- InP(p),
    let Predicate::Channel(config_hash, hash, signer_position) = p;

    ChannelsAllSigned(config_hash, hashes, signer_position) <- InP(p),
    let Predicate::ChannelsSigned(config_hash, hashes, signer_position) = p;

    Shares(config_hash, hash, signer_position) <- InP(p),
    let Predicate::Shares(config_hash, hash, signer_position) = p;

    PublicKey(config_hash, pk_hash, shares_hs, channels_hs, signer_t) <- InP(p),
    let Predicate::PublicKey(config_hash, pk_hash, shares_hs, channels_hs, signer_t) = p;

    PublicKeySigned(config_hash, pk_hash, shares_hs, channels_hs, signer_t) <- InP(p),
    let Predicate::PublicKeySigned(config_hash, pk_hash, shares_hs, channels_hs, signer_t) = p;

    ///////////////////////////////////////////////////////////////////////////
    // Intermediate relations.
    ///////////////////////////////////////////////////////////////////////////

    struct ChannelsUpTo(ConfigurationHash, ChannelsHashes, TrusteePosition);
    struct ChannelsAll(ConfigurationHash, ChannelsHashes);
    struct ChannelsAllSignedUpTo(ConfigurationHash, ChannelsHashes, TrusteePosition);
    struct ChannelsAllSignedAll(ConfigurationHash, ChannelsHashes);
    struct SharesUpTo(ConfigurationHash, SharesHashes, TrusteePosition);
    struct SharesAll(ConfigurationHash, SharesHashes);
    struct PublicKeySignedUpTo(ConfigurationHash, PublicKeyHash, SharesHashes, TrusteePosition);

    @input
    pub struct InP(Predicate);

    @output
    #[derive(Debug)]
    pub struct OutP(Predicate);

    @output
    #[derive(Debug)]
    pub struct A(pub(crate) Action);

    @output
    #[derive(Debug)]
    pub struct DErr(DatalogError);

}

///////////////////////////////////////////////////////////////////////////
// Running (see datalog::get_phases())
///////////////////////////////////////////////////////////////////////////

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub(crate) struct D;

impl D {
    pub(crate) fn run(
        &self,
        predicates: &Vec<Predicate>,
    ) -> (HashSet<Predicate>, HashSet<Action>, HashSet<DatalogError>) {
        trace!(
            "Datalog: state dkg running with {} predicates, {:?}",
            predicates.len(),
            predicates
        );

        let mut runtime = Crepe::new();
        let inputs: Vec<InP> = predicates.iter().map(|p| InP(*p)).collect();
        runtime.extend(&inputs);

        let result: (HashSet<OutP>, HashSet<A>, HashSet<DErr>) = runtime.run();

        (
            result.0.iter().map(|a| a.0).collect::<HashSet<Predicate>>(),
            result.1.iter().map(|i| i.0).collect::<HashSet<Action>>(),
            result
                .2
                .iter()
                .map(|i| i.0)
                .collect::<HashSet<DatalogError>>(),
        )
    }
}
