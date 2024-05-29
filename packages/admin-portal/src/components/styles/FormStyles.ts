// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {TextInput, PasswordInput} from "react-admin"
import {styled as muiStyled} from "@mui/material/styles"
import {FormControlLabel, Typography, Box, CircularProgress} from "@mui/material"
import {Accordion, Select, TextField} from "@mui/material"

export const FormStyles = {
    TextInput: muiStyled(TextInput)`
        input {
            padding: 16.50px 14px;
        }
    `,
    TextField: muiStyled(TextField)`
        input {
            padding: 16.50px 14px;
        }
    `,
    PasswordInput: muiStyled(PasswordInput)`
        input {
            padding: 16.50px 14px;
        }
    `,
    StatusBox: muiStyled(Box)`
        min-height: 50px;
    `,
    ErrorMessage: muiStyled(Typography)`
        color: ${({theme}) => theme.palette.errorColor};
    `,
    ShowProgress: muiStyled(CircularProgress)`
        text-align: center;
    `,
    ReservedProgressSpace: muiStyled(Box)`
        min-height: 45px;
    `,
    CheckboxControlLabel: muiStyled(FormControlLabel)`
        padding-bottom: 2em !important;
    `,
    AccordionExpanded: muiStyled(Accordion)`
        margin-top: 1em;
        width: 100%;

        .MuiAccordionSummary-content {
            margin-bottom: 0;

            > div {
                padding-bottom: 0;
            }
        }

        .MuiAccordionSummary-expandIconWrapper {
            display: none;
        }
    `,
    Select: muiStyled(Select)`
        width: 100%;
    `,
}
