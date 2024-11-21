// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {isString} from "@sequentech/ui-core"

export const sortFunction = (a: {name?: string | null}, b: {name?: string | null}) => {
    if (isString(a?.name) && isString(b?.name)) {
        return a.name.localeCompare(b.name)
    }
    return 0
}
