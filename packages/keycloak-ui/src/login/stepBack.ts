// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {TemplateBack, TemplateLabel} from "./Template"

// The form parameter the authenticators read as Back (StepBack.PARAM).
export const STEP_BACK_PARAM = "back"

/** Back to the previous step of the flow, when the page's authenticator offers it. */
export function stepBack(
    kcContext: {url: {loginAction: string}; stepBack?: boolean},
    label: TemplateLabel
): TemplateBack | undefined {
    if (!kcContext.stepBack) {
        return undefined
    }
    return {label, action: kcContext.url.loginAction, fields: {[STEP_BACK_PARAM]: "true"}}
}
