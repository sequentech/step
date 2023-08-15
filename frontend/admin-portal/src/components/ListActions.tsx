// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {CreateButton, ExportButton, SelectColumnsButton, TopToolbar} from "react-admin"

export const ListActions: React.FC = () => (
    <TopToolbar>
        <SelectColumnsButton />
        {/*<FilterButton/>*/}
        <CreateButton />
        <ExportButton />
    </TopToolbar>
)
