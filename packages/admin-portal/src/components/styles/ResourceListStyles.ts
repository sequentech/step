// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import styled from "@emotion/styled"
import {styled as muiStyled} from "@mui/material/styles"
import {Box, Drawer, Button} from "@mui/material"
import {IconButton} from "@sequentech/ui-essentials"
import MailIcon from "@mui/icons-material/Mail"
import DeleteIcon from "@mui/icons-material/Delete"
import FilterAltIcon from "@mui/icons-material/FilterAlt"

export const ResourceListStyles = {
    EmptyBox: styled(Box)`
        display: flex;
        margin: 1em;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        text-align: center;
        width: 100%;
    `,
    EmptyButtonList: styled.div`
        display: flex;
        flex-direction: row;
        gap: 10px;
    `,
    CreateIcon: styled(IconButton)`
        font-size: 24px;
        margin-right: 0.5em;
    `,
    Drawer: muiStyled(Drawer)`
        width: 40%;
        media (max-width: 800px) {
            width: 60%;
        }
    `,
    DeleteIcon: styled(DeleteIcon)`
        margin-right: 10px;
    `,
    MailIcon: styled(MailIcon)`
        margin-right: 10px;
    `,
    FiltersIcon: styled(FilterAltIcon)`
        margin-right: 10px;
    `,
}
