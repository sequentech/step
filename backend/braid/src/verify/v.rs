use crate::protocol2::datalog::*;
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

    CommitmentsAllSignedAll(cfg_h, commitments_hs) <- InP(p),
    let Predicate::CommitmentsAllSignedAll(cfg_h, commitments_hs) = p;

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

    // Intermediate relations

    struct MixVerifiedUpto(ConfigurationHash, BatchNumber, CiphertextsHash, MixingHashes, TrusteeCount);
    struct MixRepeat(ConfigurationHash, BatchNumber);

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
        pub(crate) PublicKeyHash,
        pub(crate) PublicKeyHash,
        pub(crate) CiphertextsHash,
        pub(crate) CiphertextsHash,
        pub(crate) PublicKeyHash,
        pub(crate) PlaintextsHash,
        pub(crate) MixingHashes,
    );

    Target(cfg_h, batch, pk_h, ballots_h, plaintexts_h, ) <-
    ConfigurationSignedAll(cfg_h, _, _num_t, _),
    PublicKeySignedAll(cfg_h, pk_h, _shares_hs),
    Ballots(cfg_h, batch, ballots_h, pk_h, _),
    Plaintexts(cfg_h, batch, plaintexts_h, _, _, _, _);

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

    Verified(cfg_h, batch, pk_h, ballots_pk_h, ballots_h, decrypted_ciphertexts_h, decryption_pk_h, plaintexts_h, new_mixing_hs) <-
    ConfigurationSignedAll(cfg_h, _, _num_t, threshold),
    MixVerifiedUpto(cfg_h, batch, last_ciphertexts_h, mixing_hs, threshold),
    MixVerifiedUpto(cfg_h, batch, _, _, 1),
    MixSigned(cfg_h, batch, ballots_h, _target_h, VERIFIER_INDEX),
    PublicKeySignedAll(cfg_h, pk_h, _shares_hs),
    Ballots(cfg_h, batch, ballots_h, ballots_pk_h, selected),
    Plaintexts(cfg_h, batch, plaintexts_h, _dfactors_hs, decrypted_ciphertexts_h, decryption_pk_h, selected[0] - 1),
    !MixRepeat(cfg_h, batch),
    let new_mixing_hs = MixingHashes(hashes_add(mixing_hs.0, last_ciphertexts_h.0));

}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub(crate) struct S;

impl S {
    pub(crate) fn run(&self, predicates: &Vec<Predicate>) -> (HashSet<Target>, HashSet<Verified>) {
        let mut runtime = Crepe::new();
        let inputs: Vec<InP> = predicates.iter().map(|p| InP(*p)).collect();
        runtime.extend(&inputs);

        runtime.run()
    }
}
