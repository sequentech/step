// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The election's policy ID looks like a Tracking ID: 13 groups of four from
// the same alphabet. A voter who types it where a Tracking ID is expected is
// told what it is (design board PolicyId), and nothing is looked up for it.

import {policyIdMatches} from "@vitrine/verify"

/** Whether what was typed is the start of the policy's ID (`sha256:` and its
 * hex): its first four groups or more, read as a Tracking ID is. Shorter
 * input is not, so a ballot's short ID is never taken for it. */
export const isPolicyIdTyped = (typed: string, policyId: string | undefined): boolean =>
    policyId !== undefined && policyIdMatches(typed, policyId)
