import { TextInput, PasswordInput } from "react-admin"
import {styled as muiStyled} from "@mui/material/styles"

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
}