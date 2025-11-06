// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Election Styles

import styled from "@emotion/styled"
import {styled as muiStyled} from "@mui/material/styles"

import {SaveButton, Toolbar} from "react-admin"
import {AccordionDetails, Box, Chip, Typography, CircularProgress} from "@mui/material"
import Button from "@mui/material/Button"
import DoneOutlineIcon from "@mui/icons-material/DoneOutline"
import {ElectionHeaderStyles} from "@/components/styles/ElectionHeaderStyles"

export const WizardStyles = {
    WizardWrapper: styled(Box)`
        max-width: 1280px;
        display: flex;
        flex-direction: column;
        align-items: left;
        justify-content: left;
        text-align: left;
        width: 100%;
        margin: auto;
    `,
    Toolbar: styled(Toolbar)`
        bottom: 0;
        position: sticky;
        flex-direction: row;
        justify-content: space-between;
    `,
    BackButton: styled(Button)`
        margin-right: auto;
        background-color: ${({theme}) => theme.palette.grey[100]};
        color: ${({theme}) => theme.palette.brandColor};
    `,
    NextButton: styled(Button)`
        margin-left: auto;
    `,
    DownloadButton: styled(Button)`
        width: fit-content;
        padding: 0 2em;
        margin: 1em 2em 2em 0;
    `,
    CreateButton: muiStyled(SaveButton)`
        margin-left: auto;
        flex-direction: row-reverse;
    `,
    DownloadProgress: muiStyled(CircularProgress)`
    `,
    StatusBox: styled(Box)`
        min-height: 50px;
    `,
    ContentBox: styled(Box)`
        margin-top: 30px;
        margin-bottom: 30px;
    `,
    DoneIcon: styled(DoneOutlineIcon)`
        color: ${({theme}) => theme.palette.brandSuccess};
    `,

    AccordionTitle: styled(ElectionHeaderStyles.Title)`
        margin-bottom: 0 !important;
    `,
    AccordionSubTitle: styled.div`
        color: rgba(0, 0, 0, 0.6);
        font-size: 14px;
        font-family: Roboto;
        font-weight: 400;
        line-height: 20.02px;
        letter-spacing: 0.17px;
        word-wrap: break-word;
    `,

    AccordionDetails: styled(AccordionDetails)`
        padding-top: 0;
        margin-top: -10px;
    `,
    CeremonyStatus: styled(Chip)`
        margin-top: 6px;
        margin-left: auto;
        margin-right: 10px;
        font-weight: bold;
    `,
    ErrorMessage: styled(Typography)`
        color: ${({theme}) => theme.palette.errorColor};
    `,
    SucessMessage: styled(Typography)`
        color: ${({theme}) => theme.palette.brandSuccess};
        font-weight: bold;
    `,
    StepHeader: styled(Typography)`
        margin: 25px 0;
    `,
    MainContent: styled(Box)`
        max-width: 1024px;
        display: flex;
        flex-direction: column;
        align-items: left;
        justify-content: left;
        text-align: left;
        width: 100%;
        margin: auto;
    `,
    OrderedList: styled.ol`
        padding: 1em;
        margin-top: 1em;
        margin-left: 2em;
        display: block;
        list-style-type: decimal;
    `,
    ListItem: styled.li`
        padding: 1em;
        display: list-item;
    `,
    WizardContainer: styled.div`
        display: flex;
        flex-direction: column;
        min-height: 100%;
    `,
    ContentWrapper: styled.div`
        flex-grow: 1;
        overflow-y: auto;
        padding-bottom: 1rem; // Add some padding at the bottom to prevent content from being hidden behind the footer
    `,
    FooterContainer: styled.div`
        width: 100%;
        position: sticky;
        bottom: 0;
        background-color: ${({theme}) => theme.palette.background.default};
        box-shadow: 0 -2px 4px rgba(0, 0, 0, 0.1);
    `,
    StyledFooter: styled.div`
        margin: 0 auto;
        display: flex;
        justify-content: space-between;
        padding: 1rem;
    `,
}
