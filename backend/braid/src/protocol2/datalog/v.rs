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
    struct PublicKeySignedAll(ConfigurationHash, PublicKeyHash, SharesHashes);
    struct CommitmentsAllSignedAll(ConfigurationHash, CommitmentsHashes);
    struct Ballots(ConfigurationHash, BatchNumber, CiphertextsHash, PublicKeyHash, TrusteePosition, TrusteeSet);
    struct MixComplete(ConfigurationHash, BatchNumber, MixNumber, CiphertextsHash, TrusteePosition);
    struct DecryptionFactors(ConfigurationHash, BatchNumber, DecryptionFactorsHash, CiphertextsHash, SharesHashes, TrusteePosition);
    struct Plaintexts(ConfigurationHash, BatchNumber,PlaintextsHash, DecryptionFactorsHashes, CiphertextsHash, TrusteePosition);
    struct PlaintextsSigned(ConfigurationHash, BatchNumber, PlaintextsHash, DecryptionFactorsHashes, CiphertextsHash, TrusteePosition);
    struct MixSigned(ConfigurationHash, BatchNumber, CiphertextsHash, CiphertextsHash, TrusteePosition);

    ConfigurationSignedAll(config_hash, self_position, num_t, threshold) <- InP(p),
    let Predicate::ConfigurationSignedAll(config_hash, self_position, num_t, threshold) = p;

    PublicKeySignedAll(cfg_h, pk_h, shares_hs) <- InP(p),
    let Predicate::PublicKeySignedAll(cfg_h, pk_h, shares_hs) = p;

    CommitmentsAllSignedAll(cfg_h, commitments_hs) <- InP(p),
    let Predicate::CommitmentsAllSignedAll(cfg_h, commitments_hs) = p;

    Ballots(cfg_h, batch, ballots_h, pk_h, target_t, selected) <- InP(p),
    let Predicate::Ballots(cfg_h, batch, ballots_h, pk_h, target_t, selected) = p;

    MixComplete(cfg_h, batch, mix_number, ciphertexts_h, signer_t) <- InP(p),
    let Predicate::MixComplete(cfg_h, batch, mix_number, ciphertexts_h, signer_t) = p;

    MixSigned(cfg_h, batch, source_h, ciphertexts_h, signer_t) <- InP(p),
    let Predicate::MixSigned(cfg_h, batch, source_h, ciphertexts_h, signer_t) = p;

    DecryptionFactors(cfg_h, batch, dfactors_h, ciphertexts_h, shares_hs, signer_t) <- InP(p),
    let Predicate::DecryptionFactors(cfg_h, batch, dfactors_h, ciphertexts_h, shares_hs, signer_t) = p;

    Plaintexts(ch, batch, plaintexts_h, dfactors_hs, cipher_h, signer_t) <- InP(p),
    let Predicate::Plaintexts(ch, batch, plaintexts_h, dfactors_hs, cipher_h, signer_t) = p;

    PlaintextsSigned(ch, batch, plaintexts_h, df_hs, cipher_h, signer_t) <- InP(p),
    let Predicate::PlaintextsSigned(ch, batch, plaintexts_h, df_hs, cipher_h, signer_t) = p;

    // Intermediate relations

    struct MixVerifiedUpto(ConfigurationHash, BatchNumber, CiphertextsHash, TrusteeCount);

    @output
    #[derive(Debug)]
    pub struct OutP(Predicate);

    MixVerifiedUpto(cfg_h, batch, target_h, 1) <-
    ConfigurationSignedAll(cfg_h, _, _num_t, _),
    PublicKeySignedAll(cfg_h, pk_h, _shares_hs),
    Ballots(cfg_h, batch, ballots_h, pk_h, _, _),
    !Plaintexts(cfg_h, batch, _, _, ballots_h, _),
    MixSigned(cfg_h, batch, ballots_h, target_h, VERIFIER_INDEX);

    MixVerifiedUpto(cfg_h, batch, ciphertexts_h, n + 1) <-
    MixSigned(cfg_h, batch, source_h, ciphertexts_h, VERIFIER_INDEX),
    MixVerifiedUpto(cfg_h, batch, source_h, n),
    !Plaintexts(cfg_h, batch, _, _, source_h, _);

    OutP(Predicate::Z(cfg_h, batch, ballots_h, plaintexts_h)) <-
    ConfigurationSignedAll(cfg_h, _, _num_t, threshold),
    MixVerifiedUpto(cfg_h, batch, ciphertexts_h, threshold),
    MixVerifiedUpto(cfg_h, batch, target_h, 1),
    MixSigned(cfg_h, batch, ballots_h, target_h, VERIFIER_INDEX),
    PublicKeySignedAll(cfg_h, pk_h, _shares_hs),
    Ballots(cfg_h, batch, ballots_h, pk_h, _, selected),
    Plaintexts(cfg_h, batch, plaintexts_h, _dfactors_hs, ciphertexts_h, selected[0] - 1);

}

///////////////////////////////////////////////////////////////////////////
// Running (see datalog::get_phases())
///////////////////////////////////////////////////////////////////////////

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub(crate) struct S;

impl S {
    pub(crate) fn run(&self, predicates: &Vec<Predicate>) -> HashSet<Predicate> {
        trace!(
            "Datalog: state cfg running with {} predicates, {:?}",
            predicates.len(),
            predicates
        );

        let mut runtime = Crepe::new();
        let inputs: Vec<InP> = predicates.iter().map(|p| InP(*p)).collect();
        runtime.extend(&inputs);

        let result: (HashSet<OutP>,) = runtime.run();

        result.0.iter().map(|a| a.0).collect::<HashSet<Predicate>>()
    }
}
