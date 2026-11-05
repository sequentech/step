// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {TFunction} from "i18next"

const CODE = /^[A-Z0-9]+(?:_[A-Z0-9]+)*$/

/** `SOMETHING_NEW` as "Something new"; text that is not a code as it is. */
export function humaniseCode(code: string): string {
    if (!CODE.test(code)) return code
    const words = code.toLowerCase().replace(/_/g, " ")
    return words.charAt(0).toUpperCase() + words.slice(1)
}

/** A notice the server sent, in the viewer's language; an unknown one as words. */
export function noticeText(t: TFunction, code: string): string {
    return t(`monitoring.notices.${code}`, {defaultValue: humaniseCode(code)})
}
