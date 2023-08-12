// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box} from "@mui/system"
import React, { PropsWithChildren } from "react"
import {List, Resource, Datagrid, TextField} from 'react-admin'

const ElectionList: React.FC<PropsWithChildren> = ({children}) =>
    <List>
        <Datagrid>
            <TextField source="id" />
        </Datagrid>
    </List>

export const TestScreen: React.FC = () => {
    return (
        <Box>
            <b>Hello</b>
            <Resource
                name="sequent_backend_election"
                list={ElectionList}
            />
        </Box>
    )
}