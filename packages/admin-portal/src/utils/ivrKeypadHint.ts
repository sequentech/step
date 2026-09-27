// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {keypadHint, type IvrExpectedInput} from "@sequentech/ui-essentials"
import type {TFunction} from "i18next"

/**
 * The emulator keypad's hint, in the admin's language: which keys the prompt accepts,
 * or that any digits will do when the IVR lists none (a PIN).
 */
export const ivrKeypadHint = (
    t: TFunction,
    expected: Pick<IvrExpectedInput, "valid_inputs" | "max_digits" | "timeout">
): string =>
    keypadHint(expected, {
        listed: (values) => t("electionEventScreen.ivr.emulator.inputPlaceholder", values),
        anyKeys: (values) => t("electionEventScreen.ivr.emulator.inputPlaceholderAnyKeys", values),
    })
