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
import {ImportButton, ImportConfig} from "react-admin-import-csv"

interface ListActionsProps {
    withFilter?: boolean
}

export const ListActions: React.FC<ListActionsProps> = (props) => {
    const {withFilter} = props

    const config: ImportConfig = {
        logging: true,
        // Disable the attempt to use "createMany", will instead just use "create" calls
        disableCreateMany: true,
        // Disable the attempt to use "updateMany", will instead just use "update" calls
        disableUpdateMany: true,
    }

    return (
        <TopToolbar>
            <SelectColumnsButton />
            {withFilter ? <FilterButton /> : null}
            <CreateButton />
            <ImportButton  {...props} {...config} />
            <ExportButton />
        </TopToolbar>
    )
}
