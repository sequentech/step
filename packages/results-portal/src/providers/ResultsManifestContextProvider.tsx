// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {createContext, useCallback, useContext, useMemo, useState} from "react"
import {ResultsManifest} from "@/types/results"

interface ResultsManifestContextValue {
    availableLanguages: string[]
    defaultLocale: string
    setManifestLanguageConfig: (
        manifest: Pick<ResultsManifest, "available_languages" | "default_locale"> | undefined
    ) => void
    resetManifestLanguageConfig: () => void
}

const DEFAULT_LANGUAGE_CONFIG = {
    availableLanguages: ["en"],
    defaultLocale: "en",
}

const normalizeLanguageConfig = (
    availableLanguages: string[] | undefined,
    defaultLocale: string | undefined
) => {
    const normalizedDefault = defaultLocale?.trim() || "en"
    const languages = Array.from(
        new Set(
            (availableLanguages ?? [])
                .map((language) => language.trim())
                .filter((language) => language.length > 0)
        )
    )

    if (languages.length === 0) {
        languages.push(normalizedDefault)
    }

    return {
        availableLanguages: languages,
        defaultLocale: normalizedDefault,
    }
}

const ResultsManifestContext = createContext<ResultsManifestContextValue>({
    ...DEFAULT_LANGUAGE_CONFIG,
    setManifestLanguageConfig: () => undefined,
    resetManifestLanguageConfig: () => undefined,
})

export const ResultsManifestContextProvider: React.FC<{children: React.ReactNode}> = ({
    children,
}) => {
    const [languageConfig, setLanguageConfig] = useState(DEFAULT_LANGUAGE_CONFIG)

    const setManifestLanguageConfig = useCallback(
        (manifest: Pick<ResultsManifest, "available_languages" | "default_locale"> | undefined) =>
            setLanguageConfig(
                normalizeLanguageConfig(manifest?.available_languages, manifest?.default_locale)
            ),
        []
    )
    const resetManifestLanguageConfig = useCallback(
        () => setLanguageConfig(DEFAULT_LANGUAGE_CONFIG),
        []
    )
    const value = useMemo(
        () => ({
            ...languageConfig,
            setManifestLanguageConfig,
            resetManifestLanguageConfig,
        }),
        [languageConfig, setManifestLanguageConfig, resetManifestLanguageConfig]
    )

    return (
        <ResultsManifestContext.Provider value={value}>{children}</ResultsManifestContext.Provider>
    )
}

export const useResultsManifest = () => useContext(ResultsManifestContext)
