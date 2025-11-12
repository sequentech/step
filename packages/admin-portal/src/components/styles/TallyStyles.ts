// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {styled} from "@mui/material/styles"
import {Box, Toolbar} from "@mui/material"
import Button from "@mui/material/Button"

export const TallyStyles = {
    WizardContainer: styled("div")`
        display: flex;
        align-items: center;
        flex-direction: column;
        min-height: 100%;
    `,
    ContentWrapper: styled("div")`
        width: 100%;
        flex-grow: 1;
        overflow-y: auto;
        padding-bottom: 1rem; // Add some padding at the bottom to prevent content from being hidden behind the footer
    `,
    StyledHeader: styled("div")`
        width: 100%;
        display: flex;
        padding: 2rem 0;
    `,
    StyledSpacing: styled("div")`
        margin-left: auto;
        margin-right: 10px;
    `,
    FooterContainer: styled("div")`
        width: 100%;
        max-width: 1280px;
        position: sticky;
        bottom: 0;
        background-color: ${({theme}) => theme.palette.background.default};
        box-shadow: 0 -2px 4px rgba(0, 0, 0, 0.1);
    `,
    StyledFooter: styled("div")`
        max-width: 1280px;
        margin: 0 auto;
        display: flex;
        justify-content: space-between;
        padding: 1rem;
    `,
    MiruHeader: styled(Box)`
        width: 100%;
        display: flex;
        flex-direction: row;
        justify-content: space-between;
        margin-bottom: 25px;
    `,
    MiruToolbar: styled(Toolbar)`
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
