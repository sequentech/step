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

interface ListActionsProps {
    withFilter?: boolean
}

export const ListActions: React.FC<ListActionsProps> = ({withFilter}) => (
    <TopToolbar>
        <SelectColumnsButton />
        {withFilter ? <FilterButton /> : null}
        <CreateButton />
        <ExportButton />
    </TopToolbar>
)
