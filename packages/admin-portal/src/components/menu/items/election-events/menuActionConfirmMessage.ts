// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {TFunction} from "react-i18next"
import {MISSING_ALIAS} from "@/hooks/useAliasRenderer"

export type MenuActionConfirm = "archive" | "unarchive" | "delete"

// Names the item by the alias the tree shows, or keeps the generic message
// when the item has neither an alias nor a name.
export const getMenuActionConfirmMessage = (
    t: TFunction,
    action: MenuActionConfirm,
    name?: string | null
): string => {
    const target = name?.trim()

    return String(
        target && target !== MISSING_ALIAS
            ? t(`sideMenu.menuActions.messages.confirm.${action}Named`, {name: target})
            : t(`sideMenu.menuActions.messages.confirm.${action}`)
    )
}
