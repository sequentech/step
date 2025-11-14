// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {styled} from "@mui/material/styles"
import {Box, Drawer, Button} from "@mui/material"
import {CustomTabPanel} from "../CustomTabPanel"
import Tabs from "@mui/material/Tabs"

export const SidebarScreenStyles = {
    CustomTabPanel: styled(CustomTabPanel)`
        padding: 10px;
    `,
    Tabs: styled(Tabs)`
        border-bottom: 1px solid rgba(0, 0, 0, 0.12);
        margin-bottom: 10px;
    `,
}
