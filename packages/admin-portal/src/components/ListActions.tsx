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
import {styled} from "@mui/material/styles"

const StyledSelectColumnsButton = styled(SelectColumnsButton)`
    min-width: unset;
`

const StyledFilterButton = styled(FilterButton)`
    .MuiButton-root {
        min-width: unset;
    }
`

const StyledImportButton = styled(ImportButton)`
    .MuiButton-root {
        min-width: unset;
    }
`

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
            <StyledSelectColumnsButton />
            {withFilter ? <StyledFilterButton /> : null}
            <CreateButton sx={{ minWidth: "unset"}}/>
            <StyledImportButton  {...props} {...config} />
            <ExportButton sx={{ minWidth: "unset"}}/>
        </TopToolbar>
    )
}
