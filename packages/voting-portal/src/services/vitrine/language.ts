// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/** The languages the Vitrine UI kit ships. */
const KIT_LANGUAGES = ["en", "ca", "es", "eu", "fr", "gl", "nl", "tl"]

/** The portal's own codes that differ from the kit's. */
const PORTAL_TO_KIT: Record<string, string> = {cat: "ca"}

/** The UI kit's language for one of the portal's: the same code, except the
 * portal's `cat`, which is the kit's `ca`. Anything else reads as English. */
export const vitrineLanguage = (portalLanguage: string | undefined): string => {
    const base = (portalLanguage ?? "").toLowerCase().split(/[-_]/)[0] ?? ""
    const lang = PORTAL_TO_KIT[base] ?? base
    return KIT_LANGUAGES.includes(lang) ? lang : "en"
}
