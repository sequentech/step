// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {
    getEffectiveSupportMaterialsPolicy,
    type IElectionEventPresentation,
} from "@sequentech/ui-core"

/** Resolve one published policy for both the chooser gate and acknowledgement screen. */
export const getPublishedSupportMaterialsPolicy = (
    ballotPresentation: IElectionEventPresentation | undefined,
    eventPresentation: IElectionEventPresentation | null | undefined
) => getEffectiveSupportMaterialsPolicy((ballotPresentation ?? eventPresentation)?.materials)
