// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export enum ETranslationScope {
    GLOBAL = "global",
    VOTING_PORTAL = "votingPortal",
    BALLOT_VERIFIER = "ballotVerifier",
    RESULTS_PORTAL = "resultsPortal",
    ADMIN_PORTAL = "adminPortal",
}

export interface IParsedTranslationOverrideKey {
    key: string
    scope?: ETranslationScope
}

export type ITranslationOverrides = Record<string, Record<string, string>>

const translationScopes = new Set<string>(Object.values(ETranslationScope))

export const isTranslationScope = (value: unknown): value is ETranslationScope =>
    typeof value === "string" && translationScopes.has(value)

/**
 * Splits a stored override key only when its prefix is a known scope. Unknown
 * prefixes remain legacy keys so a typo cannot silently target another portal.
 */
export const parseTranslationOverrideKey = (storedKey: string): IParsedTranslationOverrideKey => {
    const separatorIndex = storedKey.indexOf(":")
    if (separatorIndex < 0) {
        return {key: storedKey}
    }

    const candidate = storedKey.slice(0, separatorIndex)
    if (!isTranslationScope(candidate)) {
        return {key: storedKey}
    }

    return {
        key: storedKey.slice(separatorIndex + 1),
        scope: candidate,
    }
}

export const composeTranslationOverrideKey = (key: string, scope: ETranslationScope): string => {
    const parsedKey = parseTranslationOverrideKey(key)
    return `${scope}:${parsedKey.key}`
}

/**
 * Creates or moves one stored override without overwriting another scoped row.
 * Returns undefined when the target key is already occupied.
 */
export const updateTranslationOverride = (
    translations: Record<string, string>,
    key: string,
    scope: ETranslationScope,
    value: string,
    previousStoredKey?: string
): Record<string, string> | undefined => {
    const canonicalKey = parseTranslationOverrideKey(key).key
    if (!canonicalKey.trim()) {
        return undefined
    }

    const storedKey = composeTranslationOverrideKey(canonicalKey, scope)
    if (
        storedKey !== previousStoredKey &&
        Object.prototype.hasOwnProperty.call(translations, storedKey)
    ) {
        return undefined
    }

    const updatedTranslations = {...translations}
    if (previousStoredKey) {
        delete updatedTranslations[previousStoredKey]
    }
    updatedTranslations[storedKey] = value
    return updatedTranslations
}

/**
 * Selects the overrides visible to a portal. Precedence is deterministic:
 * portal-specific keys replace legacy keys, and both replace global keys.
 */
export const filterTranslationOverrides = (
    overrides: ITranslationOverrides | undefined,
    scope: ETranslationScope,
    legacyScope?: ETranslationScope
): ITranslationOverrides | undefined => {
    if (!overrides) {
        return undefined
    }

    const includeLegacyKeys = legacyScope === scope
    const scopedOverrides: ITranslationOverrides = {}

    Object.entries(overrides).forEach(([language, translations]) => {
        const selected: Record<string, string> = {}
        const entries = Object.entries(translations)

        const applyEntries = (entryScope: ETranslationScope | undefined) => {
            entries.forEach(([storedKey, value]) => {
                const parsedKey = parseTranslationOverrideKey(storedKey)
                if (parsedKey.scope !== entryScope || !parsedKey.key) {
                    return
                }
                selected[parsedKey.key] = value
            })
        }

        applyEntries(ETranslationScope.GLOBAL)

        if (includeLegacyKeys) {
            applyEntries(undefined)
        }

        if (scope !== ETranslationScope.GLOBAL) {
            applyEntries(scope)
        }

        if (Object.keys(selected).length > 0) {
            scopedOverrides[language] = selected
        }
    })

    return scopedOverrides
}
