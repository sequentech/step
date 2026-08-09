// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"

export {
    default as i18n,
    getLanguages,
    initializeLanguages,
    overwriteTranslations,
    applyLanguagePolicy,
    applyConfigurationLanguagePolicy,
    USER_LANGUAGE_COOKIE_NAME,
    toBCP47,
} from "./services/i18n"
export {useForwardedRef} from "./utils/ref"
export {sleep} from "./services/sleep"
export * from "./services/WasmContext"
export {stringToHtml} from "./services/stringToHtml"
export * from "./types/LanguageConf"
export * from "./types/TenantSettings"
export * from "./types/TenantTheme"
export * from "./types/CandidatePresentation"
export * from "./types/ContestPresentation"
export * from "./types/ElectionPresentation"
export * from "./types/CoreTypes"
export {isNumber, isString, isArray, isNull, isUndefined} from "./utils/typechecks"
export {downloadBlob, downloadUrl} from "./services/downloadBlob"
export {shuffle, splitList, keyBy} from "./utils/array"
export {normalizeWriteInText} from "./services/normalizeWriteInText"
export {
    isTranslatablePresentation,
    translate,
    translateFromPresentation,
} from "./services/translate"
export * from "./services/votingPortalDateTime"
export * from "./types/ElectionEventPresentation"
export * from "./services/percentFormatter"
export * from "./services/cssClassNameFormatter"
export * from "./services/wasm"
export * from "./services/sanitizeFilename"
export * from "./types/AreaPresentation"
export * from "./services/candidatePresentation"
export * from "./services/categoryService"
export * from "./utils/cookies"
export * from "./constants/keycloak"
