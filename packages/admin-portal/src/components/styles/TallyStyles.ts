// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
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
        z-index: 10; // Renders the footer on top of other content, avoids transparency issues
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
        flex-wrap: wrap;
        justify-content: space-between;
        gap: 8px;
        margin-bottom: 25px;

        & > :first-of-type {
            flex: 1 1 360px;
            min-width: 0;
        }

        ${({theme}) => theme.breakpoints.down("sm")} {
            flex-direction: column;

            & > :first-of-type {
                flex: 0 0 auto;
            }
        }
    `,
    MiruToolbar: styled(Toolbar)`
        background-color: none;
        padding-right: 0 !important;
        flex-wrap: wrap;
        gap: 8px;
        align-self: flex-start;

        ${({theme}) => theme.breakpoints.down("md")} {
            padding-left: 0 !important;
        }
    `,
    MiruHash: styled("div")`
        margin-top: 8px;
        font-size: 14px;
        overflow-wrap: break-word;

        & code {
            font-family: "Roboto Mono", monospace;
            user-select: all;
            word-spacing: 0.25em;
        }
    `,
    MiruToolbarButton: styled(Button)`
        border-radius: 0 !important;
        margin-right: auto;
        background-color: ${({theme}) => theme.palette.background.default};
        color: ${({theme}) => theme.palette.brandColor};
    `,
}
