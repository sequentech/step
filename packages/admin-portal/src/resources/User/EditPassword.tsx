import React, {MouseEventHandler, useState} from "react"
import Dialog from "@mui/material/Dialog"
import DialogTitle from "@mui/material/DialogTitle"
import {useTranslation} from "react-i18next"
import {
    BooleanInput,
    Edit,
    PasswordInput,
    SaveButton,
    SimpleForm,
    useListContext,
    useNotify,
    useRefresh,
} from "react-admin"
import {Box, DialogContent, InputLabel, Modal, Popover, Tooltip} from "@mui/material"
import {IUser} from "sequent-core"
import {useMutation} from "@apollo/client"
import {EditUsersInput} from "@/gql/graphql"
import {EDIT_USER} from "@/queries/EditUser"
import {FormStyles} from "@/components/styles/FormStyles"
import {faInfoCircle, faTimesCircle} from "@fortawesome/free-solid-svg-icons"
import {Icon, IconButton} from "@sequentech/ui-essentials"
import {styled} from "@mui/material/styles"
import {useTenantStore} from "@/providers/TenantContextProvider"

interface EditPasswordProps {
    open: boolean
    handleClose: () => void
    id: string
    electionEventId?: string
}

const DialogStyle = styled(Dialog)`
    & .MuiPaper-root {
        width: 650px;
        max-width: unset;
        padding-bottom: 12px;
    }
    & .MuiDialogContent-root {
        @media (max-width: ${({theme}) => theme.breakpoints.values.sm}px) {
            padding: 16px 24px 0 24px !important;
        }
    }
`

const InputLabelStyle = styled(InputLabel)<{paddingTop?: boolean}>`
    width: 135px;
    ${({paddingTop = true}) => paddingTop && "padding-top: 15px;"}
`

const InputContainerStyle = styled(Box)`
    display: flex;
    gap: 12px;
    width: 100%;
    align-items: baseline;
    @media (max-width: ${({theme}) => theme.breakpoints.values.sm}px) {
        flex-direction: column;
    }
`

const EditPassword = ({open, handleClose, id, electionEventId}: EditPasswordProps) => {
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()
    const refresh = useRefresh()
    const notify = useNotify()
    const [user, setUser] = useState<IUser>({id})
    const [temporary, setTemportay] = useState<boolean>(true)
    const [edit_user] = useMutation<EditUsersInput>(EDIT_USER)

    const [anchorEl, setAnchorEl] = React.useState<HTMLElement | null>(null)

    const handlePopoverOpen = (event: React.MouseEvent<HTMLElement>) => {
        setAnchorEl(event.currentTarget)
    }

    const handlePopoverClose = () => {
        setAnchorEl(null)
    }

    const openTemporaryInfo = Boolean(anchorEl)

    const equalToPassword = (value: any, allValues: any) => {
        if (!allValues.password || allValues.password.length == 0) {
            return
        }
        if (value !== allValues.password) {
            return t("usersAndRolesScreen.users.fields.passwordMismatch")
        }
    }

    const handleChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
        const {name, value} = e.target
        let newUser = {...user, [name]: value}
        setUser(newUser)
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
        <DialogStyle open={open} onClose={handleClose} className="dialog">
            <DialogTitle className="dialog-title">
                <Box
                    component="span"
                    flexGrow={2}
                    pt="3px"
                    fontWeight="bold"
                    className="dialog-title-text"
                >
                    {t("usersAndRolesScreen.editPassword.label")}
                </Box>
                <IconButton
                    icon={faTimesCircle}
                    variant="primary"
                    onClick={() => handleClose()}
                    className="dialog-icon-close"
                />
            </DialogTitle>
            <DialogContent className="dialog-content">
                <SimpleForm
                    toolbar={<SaveButton fullWidth alwaysEnable />}
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
                            <FormStyles.PasswordInput
                                label={false}
                                source="password"
                                onChange={(e) => {
                                    console.log("Password Input changed", e.target.value)
                                    handleChange(e)
                                }}
                                sx={{flex: 1, margin: "0 auto"}}
                            />
                        </InputContainerStyle>
                        <InputContainerStyle>
                            <InputLabelStyle>
                                {t("usersAndRolesScreen.users.fields.repeatPassword")}:
                            </InputLabelStyle>
                            <FormStyles.PasswordInput
                                label={false}
                                source="confirm_password"
                                validate={equalToPassword}
                                onChange={handleChange}
                                sx={{flex: 1, margin: "0 auto"}}
                            />
                        </InputContainerStyle>
                        <InputContainerStyle sx={{flexDirection: "row !important"}}>
                            <InputLabelStyle paddingTop={false}>
                                <Box sx={{display: "flex", gap: "8px"}}>
                                    {t(`usersAndRolesScreen.editPassword.temporatyLabel`)}
                                    <Box
                                        aria-owns={open ? "mouse-over-popover" : undefined}
                                        onMouseEnter={handlePopoverOpen}
                                        onMouseLeave={handlePopoverClose}
                                    >
                                        <Icon icon={faInfoCircle} />
                                    </Box>
                                    <Popover
                                        id="mouse-over-popover"
                                        sx={{
                                            "pointerEvents": "none",
                                            "& .MuiPopover-paper": {
                                                width: "200px",
                                                padding: "6px",
                                            },
                                        }}
                                        open={openTemporaryInfo}
                                        anchorEl={anchorEl}
                                        anchorOrigin={{
                                            vertical: "bottom",
                                            horizontal: "left",
                                        }}
                                        transformOrigin={{
                                            vertical: "top",
                                            horizontal: "left",
                                        }}
                                        onClose={handlePopoverClose}
                                        disableRestoreFocus
                                    >
                                        <Box
                                            component="span"
                                            sx={{width: "100px", padding: "2px", w: "100px"}}
                                        >
                                            {t(`usersAndRolesScreen.editPassword.temporatyInfo`)}
                                        </Box>
                                    </Popover>
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
            </DialogContent>
        </DialogStyle>
    )
}

export default EditPassword
