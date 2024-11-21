// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {ShowBase} from "react-admin"
import {ContestTabs} from "./ContestTabs"

export const ContestBaseTabs: React.FC = () => {
    return (
        <ShowBase>
            <ContestTabs />
        </ShowBase>
    )
}
