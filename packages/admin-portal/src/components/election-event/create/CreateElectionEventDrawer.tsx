// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Drawer} from "@mui/material"
import {CreateElectionEventScreen} from "./CreateScreen"
import {useCreateElectionEventStore} from "@/providers/CreateElectionEventContextProvider"

export const CreateDataDrawer: React.FC = () => {
    const {createDrawer, closeCreateDrawer} = useCreateElectionEventStore()

    return (
        <>
            <Drawer
                anchor="right"
                open={createDrawer}
                onClose={closeCreateDrawer}
                PaperProps={{
                    sx: {width: "30%"},
                }}
            >
                <CreateElectionEventScreen />
            </Drawer>
        </>
    )
}
