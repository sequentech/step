// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export enum ELanguageDetectionPolicy {
    BROWSER_DETECT = "browser-detect",
    FORCE_DEFAULT = "force-default",
}
export interface ILanguageConf {
    enabled_language_codes?: Array<string>
    default_language_code?: string
    language_detection_policy?: ELanguageDetectionPolicy
}
