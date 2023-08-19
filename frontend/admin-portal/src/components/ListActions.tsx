// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {
    CreateButton,
    ExportButton,
    FilterButton,
    SelectColumnsButton,
    TopToolbar,
} from "react-admin"
import { ImportButton } from "react-admin-import-csv"

interface ListActionsProps {
    withFilter?: boolean
}

export const ListActions: React.FC<ListActionsProps> = (props) => {
    const {withFilter} = props

    return <TopToolbar>
        <SelectColumnsButton />
        {withFilter ? <FilterButton /> : null}
        <CreateButton />
        <ImportButton {...props} />
        <ExportButton />
    </TopToolbar>
}
