// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The explorers an election's Vitrine policy names: static sites that read the
// public record themselves, run by its monitors and hubs and by the election
// authority (the operator). `@vitrine/verify` lists and links them; this only
// lets screens that may not have the policy yet ask anyway.

import {explorersOf as policyExplorers, type Policy, type PolicyExplorer} from "@vitrine/verify"

export {explorerLink} from "@vitrine/verify"

export type IExplorer = PolicyExplorer

/** The policy's explorers, independent ones first and the operator's last;
 * none while there is no policy. */
export const explorersOf = (policy: Policy | undefined): IExplorer[] =>
    policy ? policyExplorers(policy) : []
