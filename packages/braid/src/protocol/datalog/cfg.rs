// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crepe::crepe;

///////////////////////////////////////////////////////////////////////////
// Logic
///////////////////////////////////////////////////////////////////////////
crepe! {

    @input
    pub struct InP(Predicate);

    // Input relations, used to convert from InP predicates to crepe relations

    struct Configuration(ConfigurationHash, TrusteePosition, TrusteeCount, Threshold);
    struct ConfigurationSigned(ConfigurationHash, TrusteePosition);

    Configuration(cfg_h, self_position, num_t, threshold) <- InP(p),
    let Predicate::Configuration(cfg_h, self_position, num_t, threshold) = p;

    ConfigurationSigned(cfg_h, signer_t) <- InP(p),
    let Predicate::ConfigurationSigned(cfg_h, signer_t) = p;

    // Intermediate relations

    struct ConfigurationSignedUpTo(ConfigurationHash, TrusteePosition);

    @output
    #[derive(Debug)]
    pub struct OutP(Predicate);

    @output
    #[derive(Debug)]
    pub struct A(pub(crate) Action);

    @output
    #[derive(Debug)]
    pub struct DErr(DatalogError);

    A(Action::SignConfiguration(cfg_h)) <-
    Configuration(cfg_h, self_position, _, _),
    !ConfigurationSigned(cfg_h, self_position);

    ConfigurationSignedUpTo(cfg_h, n + 1) <-
    ConfigurationSignedUpTo(cfg_h, n),
    ConfigurationSigned(cfg_h, n + 1);

    ConfigurationSignedUpTo(cfg_h, 0) <-
    ConfigurationSigned(cfg_h, 0);

    OutP(Predicate::ConfigurationSignedAll(cfg_h, self_position, num_t, threshold)) <-
    Configuration(cfg_h, self_position, num_t, threshold),
    // We subtract 1 since trustees positions are 0 based
    ConfigurationSignedUpTo(cfg_h, num_t - 1);
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
            "Datalog: state cfg running with {} predicates, {:?}",
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
