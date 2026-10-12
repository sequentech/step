// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The @sequentech/ui-core package entry points at its built dist/ bundle,
// which a clean `yarn install` does not produce, so jest can't resolve the
// package on CI. These unit tests only need enums, type guards and the
// self-contained services below, so they are re-exported straight from
// ui-core's sources instead of loading the whole barrel (which pulls in
// i18next and the WASM context). Mapped in jest.config.cjs.
import {
    apply_slate_js,
    get_ballot_style_slates_coverage_js,
    get_ballot_style_slates_js,
} from "sequent-core"
import type {IDecodedVoteContest} from "sequent-core"
import type {IBallotStyle, IContest} from "../../../ui-core/src/types/CoreTypes"
import {SLATES_ANNOTATION} from "../../../ui-core/src/types/Slates"
import type {ISlate, ISlateChoices, ISlatesConfig} from "../../../ui-core/src/types/Slates"
import type {ISlateCoverage} from "../../../ui-core/src/types/SlateCoverage"

export * from "../../../ui-core/src/types/AreaPresentation"
export * from "../../../ui-core/src/types/ContestPresentation"
export * from "../../../ui-core/src/types/CoreTypes"
export * from "../../../ui-core/src/types/ElectionPresentation"
export * from "../../../ui-core/src/types/ElectionEventPresentation"
export * from "../../../ui-core/src/utils/typechecks"
export * from "../../../ui-core/src/services/translate"
export * from "../../../ui-core/src/services/ballotErrorMessages"
export * from "../../../ui-core/src/services/stringToHtml"
export * from "../../../ui-core/src/services/candidatePresentation"
export * from "../../../ui-core/src/services/cssClassNameFormatter"
export * from "../../../ui-core/src/types/Slates"
export * from "../../../ui-core/src/types/SlateCoverage"
export * from "../../../ui-core/src/services/slates"
export * from "../../../ui-core/src/services/acclamation"

// services/wasm.ts pulls in the whole ui-core barrel, so its slate calls are
// repeated here. sequent-core underneath is src/__mocks__/sequentCoreStub.ts.
const hasSlates = (ballotStyle: IBallotStyle): boolean =>
    SLATES_ANNOTATION in (ballotStyle.election_annotations ?? {})

export const getBallotStyleSlates = (ballotStyle: IBallotStyle): ISlatesConfig | null =>
    hasSlates(ballotStyle) ? (get_ballot_style_slates_js(ballotStyle) ?? null) : null

export const getBallotStyleSlatesCoverage = (
    ballotStyle: IBallotStyle
): Array<ISlateCoverage> | null =>
    hasSlates(ballotStyle) ? (get_ballot_style_slates_coverage_js(ballotStyle) ?? null) : null

export const applySlate = (
    slate: ISlate,
    contests: Array<IContest>,
    current: Array<IDecodedVoteContest>
): ISlateChoices => apply_slate_js(slate, contests, current)
