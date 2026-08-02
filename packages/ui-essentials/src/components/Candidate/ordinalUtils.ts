// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {TFunction} from "i18next"

export const getOrdinalSuffix = (num: number, t: TFunction): string => {
    if (num === 1) return `${num}${t("candidate.preferential.ordinals.first")}`
    if (num === 2) return `${num}${t("candidate.preferential.ordinals.second")}`
    if (num === 3) return `${num}${t("candidate.preferential.ordinals.third")}`
    return `${num}${t("candidate.preferential.ordinals.other")}`
}
