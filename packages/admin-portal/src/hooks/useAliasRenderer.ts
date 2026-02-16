// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {translateFromPresentation} from "@sequentech/ui-core"
import {useTranslation} from "react-i18next"

export function useAliasRenderer() {
    const {i18n} = useTranslation()

    const aliasRenderer = (item: any) => {
        if (!item) return "-"

        return (
            translateFromPresentation(item, "alias", i18n.language) ||
            translateFromPresentation(item, "name", i18n.language) ||
            translateFromPresentation(item, "alias", "en") ||
            translateFromPresentation(item, "name", "en") ||
            "-"
        )
    }

    return aliasRenderer
}
