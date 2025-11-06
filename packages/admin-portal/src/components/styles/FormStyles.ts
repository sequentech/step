// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    TextInput,
    PasswordInput,
    DateInput,
    AutocompleteInput,
    AutocompleteArrayInput,
} from "react-admin"
import {styled as muiStyled} from "@mui/material/styles"
import {FormControlLabel, Typography, Box, CircularProgress} from "@mui/material"
import {Accordion, Select, TextField} from "@mui/material"

export const FormStyles = {
    TextInput: muiStyled(TextInput)`
        input {
            padding: 16.50px 14px;
        }
        
        label:not(.MuiInputLabel-shrink) {
            top: 8px;
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
    DateInput: muiStyled(DateInput)`
        input {
            padding: 16.50px 14px;
        }
    `,
    AutocompleteInput: muiStyled(AutocompleteInput)`
        input {
            padding: 10.5px 14px !important;
        }
        label:not(.MuiInputLabel-shrink) {
            top: 8px;
        }
    `,
    AutocompleteArrayInput: muiStyled(AutocompleteArrayInput)`
        fieldset {
            border-color: ${({theme}) => theme.palette.grey[400]} !important;
        }

        .Mui-focused > fieldset {
            border-color: ${({theme}) => theme.palette.primary.main} !important;
        }

        label {
            color: ${({theme}) => theme.palette.grey[700]} !important;
        }

        label.Mui-focused {
            color: ${({theme}) => theme.palette.primary.main} !important;
        }

        .MuiFormHelperText-root {
            display: none;
        }
    `,
}
