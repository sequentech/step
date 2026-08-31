// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The @sequentech/ui-core package entry points at its built dist/ bundle,
// which a clean `yarn install` does not produce, so jest can't resolve the
// package on CI. These unit tests only need enums, type guards and the two
// self-contained services below, so they are re-exported straight from
// ui-core's sources instead of loading the whole barrel (which pulls in
// i18next and the WASM context). Mapped in jest.config.cjs.
export * from "../../../ui-core/src/types/AreaPresentation"
export * from "../../../ui-core/src/types/ContestPresentation"
export * from "../../../ui-core/src/types/CoreTypes"
export * from "../../../ui-core/src/types/ElectionPresentation"
export * from "../../../ui-core/src/utils/typechecks"
export * from "../../../ui-core/src/services/translate"
export * from "../../../ui-core/src/services/stringToHtml"

// `applySelection` is a call into sequent-core, where the marker rules live
// and where they are tested. Loading the wasm here would pull in the whole
// ui-core barrel, so it is replaced by a recorder that writes the edit
// through without interpreting it. The reducer tests assert how the reducer
// calls it and what it does with the result; what the rules decide is
// sequent-core's business, not this suite's.
import type {IDecodedVoteChoice, IDecodedVoteContest} from "sequent-core"
import type {IContest} from "../../../ui-core/src/types/CoreTypes"

export const applySelection = jest.fn(
    (
        _contest: IContest,
        selection: IDecodedVoteContest,
        choice: IDecodedVoteChoice | null,
        explicitInvalid: boolean
    ): IDecodedVoteContest => ({
        ...selection,
        is_explicit_invalid: explicitInvalid,
        choices: choice
            ? selection.choices.map((existing) =>
                  existing.id === choice.id ? {...existing, ...choice} : existing
              )
            : selection.choices,
    })
)
