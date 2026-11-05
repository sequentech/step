// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {Language} from "@vitrine/ui"

/** The languages the Vitrine UI kit ships. */
const KIT_LANGUAGES: readonly Language[] = ["en", "ca", "es", "eu", "fr", "gl", "nl", "tl"]

/** The verifier's own codes that differ from the kit's. */
const APP_TO_KIT: Record<string, Language> = {cat: "ca"}

const isKitLanguage = (lang: string): lang is Language =>
    (KIT_LANGUAGES as readonly string[]).includes(lang)

/** The UI kit's language for one of the verifier's: the same code, except the
 * verifier's `cat`, which is the kit's `ca`. Anything else reads as English. */
export const vitrineLanguage = (appLanguage: string | undefined): Language => {
    const base = (appLanguage ?? "").toLowerCase().split(/[-_]/)[0] ?? ""
    const lang = APP_TO_KIT[base] ?? base
    return isKitLanguage(lang) ? lang : "en"
}
