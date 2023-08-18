use super::*;
use crepe::crepe;

///////////////////////////////////////////////////////////////////////////
// Logic
///////////////////////////////////////////////////////////////////////////
crepe! {

    @input
    pub struct InP(Predicate);

    // Input relations, used to convert from InP predicates to crepe relations

    struct ConfigurationSignedAll(ConfigurationHash, TrusteePosition, TrusteeCount, Threshold);
    struct Commitments(ConfigurationHash, CommitmentsHash, TrusteePosition);
    struct CommitmentsAllSigned(ConfigurationHash, CommitmentsHashes, TrusteePosition);
    struct Shares(ConfigurationHash, SharesHash, TrusteePosition);
    struct SharesSigned(ConfigurationHash, SharesHash, TrusteePosition);
    struct PublicKey(ConfigurationHash, PublicKeyHash, SharesHashes, CommitmentsHashes, TrusteePosition);
    struct PublicKeySigned(ConfigurationHash, PublicKeyHash, SharesHashes, CommitmentsHashes, TrusteePosition);

    ConfigurationSignedAll(config_hash, self_position, num_t, threshold) <- InP(p),
    let Predicate::ConfigurationSignedAll(config_hash, self_position, num_t, threshold) = p;

    Commitments(config_hash, hash, signer_position) <- InP(p),
    let Predicate::Commitments(config_hash, hash, signer_position) = p;

    CommitmentsAllSigned(config_hash, hash, signer_position) <- InP(p),
    let Predicate::CommitmentsSigned(config_hash, hash, signer_position) = p;

    Shares(config_hash, hash, signer_position) <- InP(p),
    let Predicate::Shares(config_hash, hash, signer_position) = p;

    PublicKey(config_hash, pk_hash, shares_hs, commitments_hs, signer_t) <- InP(p),
    let Predicate::PublicKey(config_hash, pk_hash, shares_hs, commitments_hs, signer_t) = p;

    PublicKeySigned(config_hash, pk_hash, shares_hs, commitments_hs, signer_t) <- InP(p),
    let Predicate::PublicKeySigned(config_hash, pk_hash, shares_hs, commitments_hs, signer_t) = p;

    // Intermediate relations

    struct CommitmentsUpTo(ConfigurationHash, CommitmentsHashes, TrusteePosition);
    struct CommitmentsAll(ConfigurationHash, CommitmentsHashes);
    struct CommitmentsAllSignedUpTo(ConfigurationHash, CommitmentsHashes, TrusteePosition);
    struct CommitmentsAllSignedAll(ConfigurationHash, CommitmentsHashes);
    struct SharesUpTo(ConfigurationHash, SharesHashes, TrusteePosition);
    struct SharesAll(ConfigurationHash, SharesHashes);
    struct PublicKeySignedUpTo(ConfigurationHash, PublicKeyHash, SharesHashes, TrusteePosition);

    @output
    #[derive(Debug)]
    pub struct OutP(Predicate);

    @output
    #[derive(Debug)]
    pub struct A(pub(crate) Action);

    A(Action::GenCommitments(cfg_h, num_t, threshold)) <-
    ConfigurationSignedAll(cfg_h, self_position, num_t, threshold),
    !Commitments(cfg_h, _, self_position);

    CommitmentsUpTo(cfg_h, new_hashes, n + 1) <-
    CommitmentsUpTo(cfg_h, hashes, n),
    Commitments(cfg_h, commitments_hash, n + 1),
    let new_hashes = CommitmentsHashes(super::hashes_set(hashes.0, n + 1, commitments_hash.0));

    CommitmentsUpTo(cfg_h, CommitmentsHashes(hashes), 0) <-
    Commitments(cfg_h, hash, 0),
    let hashes = super::hashes_init(hash.0);

    CommitmentsAll(cfg_h, CommitmentsHashes(hashes.0)) <-
    ConfigurationSignedAll(_config_hash, _self_position, num_t, _threshold),
    // We subtract 1 since trustees positions are 0 based
    CommitmentsUpTo(cfg_h, hashes, num_t - 1);

    A(Action::SignCommitments(cfg_h, hashes)) <-
    CommitmentsAll(cfg_h, hashes),
    ConfigurationSignedAll(cfg_h, self_position, _num_t, _threshold),
    !CommitmentsAllSigned(cfg_h, hashes, self_position);

    CommitmentsAllSignedUpTo(cfg_h, commitments_hs, n + 1) <-
    CommitmentsAllSignedUpTo(cfg_h, commitments_hs, n),
    CommitmentsAllSigned(cfg_h, commitments_hs, n + 1);

    CommitmentsAllSignedUpTo(cfg_h, commitments_hs, 0) <-
    CommitmentsAllSigned(cfg_h, commitments_hs, 0);

    CommitmentsAllSignedAll(cfg_h, commitments_hs) <-
    ConfigurationSignedAll(cfg_h, _self_position, num_t, _threshold),
    CommitmentsAllSignedUpTo(cfg_h, commitments_hs, num_t - 1);

    A(Action::ComputeShares(cfg_h, commitments_hs, self_position, num_t, threshold)) <-
    CommitmentsAllSignedAll(cfg_h, commitments_hs),
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

    A(Action::ComputePublicKey(cfg_h, shares_hs, commitments_hs, 0, num_t, threshold)) <-
    SharesAll(cfg_h, shares_hs),
    CommitmentsAllSignedAll(cfg_h, commitments_hs),
    ConfigurationSignedAll(cfg_h, 0, num_t, threshold),
    !PublicKey(cfg_h, _, shares_hs, commitments_hs, 0);

    PublicKeySigned(cfg_h, pk_h, shares_hs, commitments_hs, 0) <-
    PublicKey(cfg_h, pk_h, shares_hs, commitments_hs, 0);

    A(Action::SignPublicKey(cfg_h, pk_h, shares_hs, commitments_hs, self_p, num_t, threshold)) <-
    PublicKey(cfg_h, pk_h, shares_hs, commitments_hs, 0),
    ConfigurationSignedAll(cfg_h, self_p, num_t, threshold),
    !PublicKeySigned(cfg_h, _, shares_hs, commitments_hs, self_p);

    PublicKeySignedUpTo(cfg_h, pk_h, shares_hs, n + 1) <-
    PublicKeySignedUpTo(cfg_h, pk_h, shares_hs, n),
    PublicKeySigned(cfg_h, pk_h, shares_hs, _commitments_hs, n + 1);

    PublicKeySignedUpTo(cfg_h, pk_h, shares_hs, 0) <-
    PublicKeySigned(cfg_h, pk_h, shares_hs, _commitments_hs, 0);

    OutP(Predicate::CommitmentsAllSignedAll(cfg_h, commitments_hs)) <-
    CommitmentsAllSignedAll(cfg_h, commitments_hs);

    OutP(Predicate::PublicKeySignedAll(cfg_h, pk_h, shares_hs)) <-
    ConfigurationSignedAll(cfg_h, _self_p, num_t, _threshold),
    // we subtract 1 since trustees positions are 0 based
    PublicKeySignedUpTo(cfg_h, pk_h, shares_hs, num_t - 1);

}

///////////////////////////////////////////////////////////////////////////
// Running (see datalog::get_phases())
///////////////////////////////////////////////////////////////////////////

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub(crate) struct S;

impl S {
    pub(crate) fn run(&self, predicates: &Vec<Predicate>) -> (HashSet<Predicate>, HashSet<Action>) {
        trace!(
            "Datalog: state dkg running with {} predicates, {:?}",
            predicates.len(),
            predicates
        );

        let mut runtime = Crepe::new();
        let inputs: Vec<InP> = predicates.iter().map(|p| InP(*p)).collect();
        runtime.extend(&inputs);

        let result: (HashSet<OutP>, HashSet<A>) = runtime.run();

        (
            result.0.iter().map(|a| a.0).collect::<HashSet<Predicate>>(),
            result.1.iter().map(|i| i.0).collect::<HashSet<Action>>(),
        )
    }
}
