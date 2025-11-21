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
import {styled} from "@mui/material/styles"
import {FormControlLabel, Typography, Box, CircularProgress} from "@mui/material"
import {Accordion, Select, TextField} from "@mui/material"

export const FormStyles = {
    TextInput: styled(TextInput)`
        input {
            padding: 16.5px 14px;
        }

        label:not(.MuiInputLabel-shrink) {
            top: 8px;
        }
    `,
    TextField: styled(TextField)`
        input {
            padding: 16.5px 14px;
        }
    `,
    PasswordInput: styled(PasswordInput)`
        input {
            padding: 16.5px 14px;
        }
    `,
    StatusBox: styled(Box)`
        min-height: 50px;
    `,
    ErrorMessage: styled(Typography)`
        color: ${({theme}) => theme.palette.errorColor};
    `,
    ShowProgress: styled(CircularProgress)`
        text-align: center;
    `,
    ReservedProgressSpace: styled(Box)`
        min-height: 45px;
    `,
    CheckboxControlLabel: styled(FormControlLabel)`
        padding-bottom: 2em !important;
    `,
    AccordionExpanded: styled(Accordion)`
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
    Select: styled(Select)`
        width: 100%;
    `,
    DateInput: styled(DateInput)`
        input {
            padding: 16.5px 14px;
        }
    `,
    AutocompleteInput: styled(AutocompleteInput)`
        input {
            padding: 10.5px 14px !important;
        }
        label:not(.MuiInputLabel-shrink) {
            top: 8px;
        }
    `,
    AutocompleteArrayInput: styled(AutocompleteArrayInput)`
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
