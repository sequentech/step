// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Drawer} from "@mui/material"
import {CreateElectionEventScreen} from "./CreateScreen"

interface ImportVotersTabsProps {
    open: boolean
    closeDrawer: () => void
}

export const CreateDataDrawer: React.FC<ImportVotersTabsProps> = ({open, closeDrawer}) => {
    return (
        <>
            <Drawer
                anchor="right"
                open={open}
                onClose={closeDrawer}
                PaperProps={{
                    sx: {width: "30%"},
                }}
            >
                <CreateElectionEventScreen />
            </Drawer>
        </>
    )
}
