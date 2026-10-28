// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export enum ETranslationPresetTemplate {
    SPANISH_INFORMAL = "es-informal",
    CATALAN_INFORMAL = "cat-informal",
}

/**
 * A ready-made set of translation overrides for one language, from which an
 * admin creates a preset. Keys use the stored override format, `<scope>:<key>`,
 * so applying a template is the same as entering each override by hand in the
 * Localization tab.
 */
export interface ITranslationPresetTemplate {
    id: ETranslationPresetTemplate
    language: string
    overrides: Record<string, string>
}
