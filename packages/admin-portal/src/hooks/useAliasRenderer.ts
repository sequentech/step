// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {translateElection} from "@sequentech/ui-core"
import {useTranslation} from "react-i18next"

export function useAliasRenderer() {
    const {i18n} = useTranslation()

    const aliasRenderer = (item: any) => {
        if (!item) return "-"

        return (
            translateElection(item, "alias", i18n.language) ||
            translateElection(item, "name", i18n.language) ||
            item.alias ||
            item.name ||
            "-"
        )
    }

    return aliasRenderer
}
