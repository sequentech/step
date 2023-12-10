import {TextInput, PasswordInput} from "react-admin"
import {styled as muiStyled} from "@mui/material/styles"
import {FormControlLabel} from "@mui/material"

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
}
