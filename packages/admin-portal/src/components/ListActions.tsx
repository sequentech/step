import {
    Button,
    CreateButton,
    ExportButton,
    FilterButton,
    SelectColumnsButton,
    TopToolbar,
} from "react-admin"
import {ImportButton, ImportConfig} from "react-admin-import-csv"
// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, { useState } from "react"

import {Add} from "@mui/icons-material"
import { Drawer } from '@mui/material'

interface ListActionsProps {
    withFilter?: boolean
    Component?: React.ReactNode
}

export const ListActions: React.FC<ListActionsProps> = (props) => {
    const {withFilter, Component} = props

    const [open, setOpen] = useState<boolean>(false)

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
            {Component && (
                <>
                    <Button label="Create" onClick={() => setOpen(true)}><Add /></Button>
                    <Drawer
                        anchor="right"
                        open={open}
                        onClose={() => setOpen(false)}
                        PaperProps={{
                            sx: {width: "40%"},
                        }}
                    >
                        {Component}
                    </Drawer>
                </>
            )}
            <ImportButton {...props} {...config} />
            <ExportButton />
        </TopToolbar>
    )
}
