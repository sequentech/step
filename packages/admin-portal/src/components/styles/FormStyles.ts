import { TextInput, PasswordInput } from "react-admin"
import {styled as muiStyled} from "@mui/material/styles"
import {FormControlLabel} from "@mui/material"
import {Accordion, Select} from "@mui/material"

export const FormStyles = {
    TextInput: muiStyled(TextInput)`
        input {
            padding: 16.50px 14px;
        }
    `,
    PasswordInput: muiStyled(PasswordInput)`
        input {
            padding: 16.50px 14px;
        }
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
    `

}