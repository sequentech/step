// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ETranslationPresetTemplate, ITranslationPresetTemplate} from "@/types/translationPresets"
import {catalanInformalTemplate} from "./catalanInformal"
import {spanishInformalTemplate} from "./spanishInformal"

export const TRANSLATION_PRESET_TEMPLATES: Record<
    ETranslationPresetTemplate,
    ITranslationPresetTemplate
> = {
    [ETranslationPresetTemplate.SPANISH_INFORMAL]: spanishInformalTemplate,
    [ETranslationPresetTemplate.CATALAN_INFORMAL]: catalanInformalTemplate,
}
