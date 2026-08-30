// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {createContext, useCallback, useContext, useEffect, useMemo, useState} from "react"
import {i18n} from "@sequentech/ui-core"
import {getScene, getVariant, SCENES} from "./catalog"
import {
    applyOverride,
    applyOverrides,
    getBundleForLanguage,
    getOriginalBundle,
    getOriginalValue,
    LOC_STUDIO_LANGUAGES,
    OverridesByLanguage,
} from "./i18n"
import {
    isContentKey,
    parseUploadedElectionEvent,
    resetAllContentFields,
    UploadedElectionEvent,
} from "./uploadedElection"

const STORAGE_KEY = "sequent-loc-studio-overrides"

const loadStoredOverrides = (): OverridesByLanguage => {
    try {
        const raw = window.localStorage.getItem(STORAGE_KEY)
        if (!raw) {
            return {}
        }
        const parsed = JSON.parse(raw) as OverridesByLanguage
        return parsed && typeof parsed === "object" ? parsed : {}
    } catch {
        return {}
    }
}

interface LocStudioContextValue {
    sceneId: string
    variantId: string
    language: string
    languageOptions: string[]
    selectedKey: string | null
    hoveredKey: string | null
    onScreenKeys: string[]
    overrides: OverridesByLanguage
    previewRevision: number
    defaults: Record<string, string>
    currentBundle: Record<string, string>
    uploadedEvent: UploadedElectionEvent | null
    uploadError: string | null
    importDialogOpen: boolean
    setSceneId: (sceneId: string) => void
    setVariantId: (variantId: string) => void
    setLanguage: (language: string) => void
    setSelectedKey: (key: string | null) => void
    setHoveredKey: (key: string | null) => void
    setOnScreenKeys: (keys: string[]) => void
    setOverride: (key: string, value: string) => void
    resetOverride: (key: string) => void
    resetAllOverrides: () => void
    importOverrides: (incoming: OverridesByLanguage) => void
    isKeyEdited: (key: string) => boolean
    getOriginalForKey: (key: string) => string | undefined
    loadUploadedEvent: (file: File) => Promise<void>
    clearUploadedEvent: () => void
    openImportDialog: () => void
    closeImportDialog: () => void
}

const LocStudioContext = createContext<LocStudioContextValue | undefined>(undefined)

export interface LocStudioProviderProps extends React.PropsWithChildren {
    initialUploadedEvent?: UploadedElectionEvent | null
    initialOverrides?: OverridesByLanguage
}

export const LocStudioProvider: React.FC<LocStudioProviderProps> = ({
    children,
    initialUploadedEvent = null,
    initialOverrides,
}) => {
    const [sceneId, setSceneIdState] = useState(SCENES[0].id)
    const [variantId, setVariantIdState] = useState(SCENES[0].variants[0].id)
    const [language, setLanguageState] = useState<string>("en")
    const [selectedKey, setSelectedKey] = useState<string | null>(null)
    const [hoveredKey, setHoveredKey] = useState<string | null>(null)
    const [onScreenKeys, setOnScreenKeys] = useState<string[]>([])
    const [overrides, setOverrides] = useState<OverridesByLanguage>(() => ({
        ...loadStoredOverrides(),
        ...(initialOverrides || {}),
    }))
    const [previewRevision, setPreviewRevision] = useState(0)
    const [defaults] = useState(() => getOriginalBundle("en"))
    const [uploadedEvent, setUploadedEvent] = useState<UploadedElectionEvent | null>(
        initialUploadedEvent
    )
    const [uploadError, setUploadError] = useState<string | null>(null)
    const [importDialogOpen, setImportDialogOpen] = useState(false)

    useEffect(() => {
        const stored = loadStoredOverrides()
        Object.entries(stored).forEach(([lang, values]) => {
            applyOverrides(lang, values)
        })
        setPreviewRevision((value) => value + 1)
    }, [])

    const persist = (next: OverridesByLanguage) => {
        setOverrides(next)
        window.localStorage.setItem(STORAGE_KEY, JSON.stringify(next))
        setPreviewRevision((value) => value + 1)
    }

    const setSceneId = (nextSceneId: string) => {
        setSceneIdState(nextSceneId)
        const scene = getScene(nextSceneId)
        setVariantIdState(scene.variants[0].id)
        setSelectedKey(null)
        setPreviewRevision((value) => value + 1)
    }

    const setVariantId = (nextVariantId: string) => {
        setVariantIdState(nextVariantId)
        setSelectedKey(null)
        setPreviewRevision((value) => value + 1)
    }

    const setLanguage = useCallback((nextLanguage: string) => {
        setLanguageState(nextLanguage)
        void i18n.changeLanguage(nextLanguage)
        const languageOverrides = loadStoredOverrides()[nextLanguage] || {}
        applyOverrides(nextLanguage, languageOverrides)
        setPreviewRevision((value) => value + 1)
    }, [])

    const setOverride = (key: string, value: string) => {
        if (isContentKey(key)) {
            uploadedEvent?.fieldRefs.get(key)?.setValue(language, value)
            setPreviewRevision((revision) => revision + 1)
            return
        }
        applyOverride(language, key, value)
        const next = {
            ...overrides,
            [language]: {
                ...(overrides[language] || {}),
                [key]: value,
            },
        }
        persist(next)
    }

    const resetOverride = (key: string) => {
        if (isContentKey(key)) {
            const ref = uploadedEvent?.fieldRefs.get(key)
            if (ref) {
                ref.setValue(language, ref.getOriginal(language))
                setPreviewRevision((revision) => revision + 1)
            }
            return
        }
        const languageOverrides = {...(overrides[language] || {})}
        delete languageOverrides[key]
        const original = getOriginalValue(language, key)
        if (original !== undefined) {
            applyOverride(language, key, original)
        }
        persist({
            ...overrides,
            [language]: languageOverrides,
        })
    }

    const resetAllOverrides = () => {
        Object.entries(overrides).forEach(([lang, values]) => {
            Object.keys(values).forEach((key) => {
                const original = getOriginalValue(lang, key)
                if (original !== undefined) {
                    applyOverride(lang, key, original)
                }
            })
        })
        if (uploadedEvent) {
            resetAllContentFields(uploadedEvent)
        }
        persist({})
    }

    const isKeyEdited = (key: string): boolean => {
        if (isContentKey(key)) {
            const ref = uploadedEvent?.fieldRefs.get(key)
            return Boolean(ref && ref.getCurrent(language) !== ref.getOriginal(language))
        }
        return overrides[language]?.[key] !== undefined
    }

    const getOriginalForKey = (key: string): string | undefined => {
        if (isContentKey(key)) {
            return uploadedEvent?.fieldRefs.get(key)?.getOriginal(language)
        }
        return getOriginalValue(language, key)
    }

    const loadUploadedEvent = async (file: File): Promise<void> => {
        try {
            const text = await file.text()
            const parsed = JSON.parse(text) as unknown
            const uploaded = parseUploadedElectionEvent(parsed, file.name)
            setUploadedEvent(uploaded)
            setUploadError(null)
            const defaultLang =
                (
                    uploaded.electionEventPresentations[0] as
                        | {language_conf?: {default_language_code?: string}}
                        | undefined
                )?.language_conf?.default_language_code || uploaded.languages[0]
            if (defaultLang) {
                setLanguage(defaultLang)
            }
            setSceneId("election-list")
            setImportDialogOpen(false)
            setPreviewRevision((revision) => revision + 1)
        } catch (error) {
            setUploadError(error instanceof Error ? error.message : "Could not read that file.")
        }
    }

    const openImportDialog = () => {
        setUploadError(null)
        setImportDialogOpen(true)
    }

    const closeImportDialog = () => {
        setImportDialogOpen(false)
    }

    const clearUploadedEvent = () => {
        setUploadedEvent(null)
        setUploadError(null)
        setPreviewRevision((revision) => revision + 1)
    }

    const importOverrides = (incoming: OverridesByLanguage) => {
        const merged: OverridesByLanguage = {...overrides}
        Object.entries(incoming).forEach(([lang, values]) => {
            merged[lang] = {
                ...(merged[lang] || {}),
                ...values,
            }
            if (lang === language) {
                applyOverrides(lang, values)
            }
        })
        persist(merged)
    }

    const currentBundle = useMemo(() => {
        const bundle = getBundleForLanguage(language)
        if (uploadedEvent) {
            uploadedEvent.fieldRefs.forEach((ref, key) => {
                bundle[key] = ref.getCurrent(language)
            })
        }
        return bundle
    }, [language, previewRevision, uploadedEvent])

    const languageOptions = useMemo(() => {
        const codes = new Set<string>(LOC_STUDIO_LANGUAGES)
        uploadedEvent?.languages.forEach((code) => codes.add(code))
        return Array.from(codes)
    }, [uploadedEvent])

    const value: LocStudioContextValue = {
        sceneId,
        variantId,
        language,
        languageOptions,
        selectedKey,
        hoveredKey,
        onScreenKeys,
        overrides,
        previewRevision,
        defaults,
        currentBundle,
        uploadedEvent,
        uploadError,
        importDialogOpen,
        setSceneId,
        setVariantId,
        setLanguage,
        setSelectedKey,
        setHoveredKey,
        setOnScreenKeys,
        setOverride,
        resetOverride,
        resetAllOverrides,
        importOverrides,
        isKeyEdited,
        getOriginalForKey,
        loadUploadedEvent,
        clearUploadedEvent,
        openImportDialog,
        closeImportDialog,
    }

    return <LocStudioContext.Provider value={value}>{children}</LocStudioContext.Provider>
}

export const useLocStudio = (): LocStudioContextValue => {
    const context = useContext(LocStudioContext)
    if (!context) {
        throw new Error("useLocStudio must be used within LocStudioProvider")
    }
    return context
}

export const useCurrentScene = () => {
    const {sceneId, variantId} = useLocStudio()
    const scene = getScene(sceneId)
    const variant = getVariant(scene, variantId)
    return {scene, variant}
}
