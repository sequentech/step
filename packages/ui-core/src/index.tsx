// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"

export {
    default as i18n,
    getLanguages,
    initializeLanguages,
    overwriteTranslations,
} from "./services/i18n"
export {useForwardedRef} from "./utils/ref"
export {sleep} from "./services/sleep"
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
export {shuffle, splitList, keyBy, keyByReturn} from "./utils/array"
export {normalizeWriteInText} from "./services/normalizeWriteInText"
export {translate, translateElection} from "./services/translate"
export * from "./types/ElectionEventPresentation"
export * from "./services/percentFormatter"
export * from "./services/wasm"
export * from "./services/sanitizeFilename"
