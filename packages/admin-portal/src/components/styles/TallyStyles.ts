// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import styled from "@emotion/styled"
import {styled as muiStyled} from "@mui/material/styles"
import {Box, Toolbar} from "@mui/material"
import Button from "@mui/material/Button"

export const TallyStyles = {
    StyledHeader: styled.div`
        width: 100%;
        display: flex;
        padding: 2rem 0;
    `,
    StyledSpacing: styled.div`
        margin-left: auto;
        margin-right: 10px;
    `,
    StyledFooter: styled.div`
        width: 100%;
        display: flex;
        justify-content: space-between;
        padding: 2rem 0;
    `,
    MiruHeader: styled(Box)`
        width: 100%;
        display: flex;
        flex-direction: row;
        justify-content: space-between;
        margin-bottom: 25px;
    `,
    MiruToolbar: muiStyled(Toolbar)`
        background-color: none;
        padding-right: 0 !important;
    `,
    MiruToolbarButton: styled(Button)`
        border-radius: 0 !important;
        margin-right: auto;
        background-color: ${({theme}) => theme.palette.background.default};
        color: ${({theme}) => theme.palette.brandColor};
    `,
}
