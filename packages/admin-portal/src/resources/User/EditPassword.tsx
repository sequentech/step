// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import Dialog from "@mui/material/Dialog"
import DialogTitle from "@mui/material/DialogTitle"
import {useTranslation} from "react-i18next"
import {BooleanInput, SaveButton, SimpleForm, useNotify, useRefresh} from "react-admin"
import {Box, DialogContent, InputLabel} from "@mui/material"
import {IUser} from "sequent-core"
import {useMutation} from "@apollo/client"
import {EditUsersInput} from "@/gql/graphql"
import {EDIT_USER} from "@/queries/EditUser"
import {FormStyles} from "@/components/styles/FormStyles"
import {faInfoCircle, faTimesCircle} from "@fortawesome/free-solid-svg-icons"
import {IconButton} from "@sequentech/ui-essentials"
import {styled} from "@mui/material/styles"
import {useTenantStore} from "@/providers/TenantContextProvider"
import IconTooltip from "@/components/IconTooltip"
import FormDialog from "@/components/FormDialog"
interface EditPasswordProps {
    open: boolean
    handleClose: () => void
    id: string
    electionEventId?: string
}

export const InputLabelStyle = styled(InputLabel, {
    shouldForwardProp: (prop) => prop !== "paddingTop",
})<{paddingTop?: boolean}>`
    width: 135px;
    ${({paddingTop = true}) => (paddingTop ? "padding-top: 15px;" : "padding-top: 0;")}
`

export const InputContainerStyle = styled(Box)`
    display: flex;
    gap: 12px;
    width: 100%;
    align-items: baseline;
    @media (max-width: ${({theme}) => theme.breakpoints.values.sm}px) {
        flex-direction: column;
    }
`

export const PasswordInputStyle = styled(FormStyles.PasswordInput)(({theme, error}) => {
    return {
        "flex": "1",
        "margin": "0 auto",

        "& .MuiFormHelperText-root.MuiFormHelperText-sizeSmall.MuiFormHelperText-contained": {
            ...(error && {
                borderColor: theme.palette.error.main,
                color: theme.palette.error.main,
            }),
        },
        "& .MuiOutlinedInput-root": {
            "& fieldset": {
                ...(error && {borderColor: theme.palette.error.main}),
            },
        },
        "& .MuiInputBase-root.MuiOutlinedInput-root.MuiInputBase-colorPrimary.MuiInputBase-formControl.MuiInputBase-sizeSmall.MuiInputBase-adornedEnd":
            {
                marginBlockEnd: "0px",
            },
    }
})

const EditPassword = ({open, handleClose, id, electionEventId}: EditPasswordProps) => {
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()
    const refresh = useRefresh()
    const notify = useNotify()
    const [user, setUser] = useState<IUser>({id})
    const [temporary, setTemportay] = useState<boolean>(true)
    const [edit_user] = useMutation<EditUsersInput>(EDIT_USER)
    const [errorText, setErrorText] = useState("")

    const equalToPassword = (allValues: any) => {
        if (!allValues.password || allValues.password.length == 0) {
            return
        }
        if (allValues.confirm_password !== allValues.password) {
            setErrorText(t("usersAndRolesScreen.users.fields.passwordMismatch"))
        }

        if (errorText && allValues.confirm_password === allValues.password) {
            setErrorText("")
        }
    }

    const handleChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
        const {name, value} = e.target

        const updatedUser = {
            ...user,
            [name]: value,
        }

        //only run on password update
        if (name === "confirm_password" || name === "password") {
            equalToPassword(updatedUser)
        }

        setUser(updatedUser)
    }

    const validatePassword = (value: any) => {
        /*TODO: we should validate only to the extent that these policies are 
		in place in keycloak
		if (!value || value.length == 0) {
			return
		}
	 
		const hasEnoughChars = value.length < 8
		const hasUpperCase = /[A-Z]/.test(value)
		const hasLowerCase = /[a-z]/.test(value)
		const hasDigit = /\d/.test(value)
		const hasSpecialChar = /[^a-zA-Z\d]/.test(value)
	 
		if (hasEnoughChars) {
			return t("usersAndRolesScreen.users.fields.passwordLengthValidate")
		}
	 
		if (!hasUpperCase) {
			return t("usersAndRolesScreen.users.fields.passwordUppercaseValidate")
		}
	 
		if (!hasLowerCase) {
			return t("usersAndRolesScreen.users.fields.passwordLowercaseValidate")
		}
	 
		if (!hasDigit) {
			return t("usersAndRolesScreen.users.fields.passwordDigitValidate")
		}
	 
		if (!hasSpecialChar) {
			return t("usersAndRolesScreen.users.fields.passwordSpecialCharValidate")
		}*/
    }

    const onSubmit = async () => {
        try {
            await edit_user({
                variables: {
                    body: {
                        user_id: user?.id,
                        tenant_id: tenantId,
                        election_event_id: electionEventId,
                        password:
                            user?.password && user?.password.length > 0 ? user.password : undefined,
                        temporary: temporary,
                    },
                },
            })
            notify(t("usersAndRolesScreen.voters.errors.editSuccess"), {type: "success"})
            refresh()
            handleClose?.()
        } catch (error) {
            notify(t("usersAndRolesScreen.voters.errors.editError"), {type: "error"})
            handleClose?.()
        }
    }

    return (
        <FormDialog
            open={open}
            onClose={handleClose}
            title={t("usersAndRolesScreen.editPassword.label")}
        >
            <>
                <SimpleForm
                    toolbar={<SaveButton fullWidth alwaysEnable={!errorText} />}
                    record={user}
                    onSubmit={onSubmit}
                    sanitizeEmptyValues
                    sx={{padding: 0}}
                >
                    <>
                        <InputContainerStyle>
                            <InputLabelStyle>
                                {t("usersAndRolesScreen.users.fields.password")}:
                            </InputLabelStyle>
                            <PasswordInputStyle
                                label={false}
                                source="password"
                                onChange={handleChange}
                                error={!!errorText}
                            />
                        </InputContainerStyle>
                        <InputContainerStyle>
                            <InputLabelStyle>
                                {t("usersAndRolesScreen.users.fields.repeatPassword")}:
                            </InputLabelStyle>
                            <PasswordInputStyle
                                label={false}
                                source="confirm_password"
                                helperText={errorText}
                                error={!!errorText}
                                onChange={handleChange}
                            />
                        </InputContainerStyle>
                        <InputContainerStyle sx={{flexDirection: "row !important"}}>
                            <InputLabelStyle paddingTop={false}>
                                <Box sx={{display: "flex", gap: "8px"}}>
                                    {t(`usersAndRolesScreen.editPassword.temporatyLabel`)}
                                    <IconTooltip
                                        icon={faInfoCircle}
                                        info={t(`usersAndRolesScreen.editPassword.temporatyInfo`)}
                                    />
                                </Box>
                            </InputLabelStyle>
                            <BooleanInput
                                source=""
                                label={false}
                                onChange={(e) => setTemportay(!temporary)}
                                checked={temporary}
                            />
                        </InputContainerStyle>
                    </>
                </SimpleForm>
            </>
        </FormDialog>
    )
}

export default EditPassword
