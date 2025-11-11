// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ApplicationChangeStatusBody, Sequent_Backend_Applications} from "@/gql/graphql"
import React from "react"
import {useTranslation} from "react-i18next"
import {theme} from "@sequentech/ui-essentials"
import {Box, styled} from "@mui/material"
import {
    Button,
    SelectInput,
    TextInput,
    required,
    SimpleForm,
    Toolbar,
    SaveButton,
    useNotify,
} from "react-admin"
import {useMutation} from "@apollo/client"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {IApplicationsStatus, RejectReason} from "@/types/applications"
import FormDialog from "@/components/FormDialog"
import {CHANGE_APPLICATION_STATUS} from "@/queries/ChangeApplicationStatus"
import CancelOutlined from "@mui/icons-material/CancelOutlined"

const RejectBox = styled(Box)(() => ({
    width: "100%",
    display: "flex",
    justifyContent: "flex-end",
}))
const RejectButton = styled(Button)(({theme}) => ({
    "borderColor": theme.palette.errorColor,
    "color": theme.palette.errorColor,
    "backgroundColor": theme.palette.white,
    "width": "max-content",
    "margin": "1rem 0",
    "&:hover": {
        backgroundColor: theme.palette.errorColor,
        color: theme.palette.white,
    },
}))

export interface RejectApplicationButtonProps {
    label: string
    onClick: (val: boolean) => void
}

export const RejectApplicationButton: React.FC<RejectApplicationButtonProps> = ({
    label,
    onClick,
}) => (
    <RejectBox>
        <RejectButton onClick={() => onClick(true)}>
            <Box style={{display: "flex", gap: "10px"}}>
                <CancelOutlined sx={{width: "20px", marginLeft: "10px", paddingTop: "4px"}} />
                {label}
            </Box>
        </RejectButton>
    </RejectBox>
)

interface IRejectData {
    rejection_reason?: string
    rejection_message?: string
}

export interface RejectApplicationDialogProps {
    electionEventId: string
    task: Sequent_Backend_Applications
    goBack: () => void
    rejectDialogOpen: boolean
    setRejectDialogOpen: (val: boolean) => void
}

export const RejectApplicationDialog: React.FC<RejectApplicationDialogProps> = ({
    electionEventId,
    task,
    goBack,
    rejectDialogOpen,
    setRejectDialogOpen,
}) => {
    const notify = useNotify()
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()
    const [rejectVoter] = useMutation<ApplicationChangeStatusBody>(CHANGE_APPLICATION_STATUS)

    const handleReject = async (data?: IRejectData) => {
        if (data) {
            const {errors} = await rejectVoter({
                variables: {
                    tenant_id: tenantId,
                    id: task?.id,
                    user_id: "", // user_id is not available!!
                    area_id: task?.area_id,
                    election_event_id: electionEventId,
                    rejection_reason: data?.rejection_reason,
                    rejection_message: data?.rejection_message,
                },
            })
            if (errors) {
                notify(t(`approvalsScreen.notifications.rejectError`), {type: "error"})
                return
            }
            notify(t(`approvalsScreen.notifications.rejectSuccess`), {type: "success"})
            goBack()
        }
        setRejectDialogOpen(false)
    }

    const rejectionChoices = () => {
        return (Object.values(RejectReason) as RejectReason[]).map((value) => ({
            id: value,
            name: t(`approvalsScreen.reject.reasons.${value.toLowerCase()}`),
        }))
    }

    return (
        <FormDialog
            open={rejectDialogOpen && task.status === IApplicationsStatus.PENDING}
            title={String(t("approvalsScreen.reject.label"))}
            onClose={() => handleReject()}
        >
            <SimpleForm
                defaultValues={{
                    rejection_reason: "",
                    rejection_message: "",
                }}
                onSubmit={(data: IRejectData) => {
                    handleReject(data)
                }}
                sanitizeEmptyValues
                toolbar={
                    <Toolbar
                        style={{
                            backgroundColor: "inherit",
                            display: "flex",
                            justifyContent: "flex-end",
                        }}
                    >
                        <SaveButton
                            className="election-event-save-button"
                            icon={<CancelOutlined />}
                            label={String(t("approvalsScreen.reject.label"))}
                            color="error"
                            style={{backgroundColor: theme.palette.errorColor}}
                        />
                    </Toolbar>
                }
            >
                <Box>
                    {t("approvalsScreen.reject.confirm")}
                    <SelectInput
                        source="rejection_reason"
                        label={String(t("approvalsScreen.reject.rejectReason"))}
                        choices={rejectionChoices()}
                        validate={required()}
                    />
                    <TextInput
                        source="rejection_message"
                        label={String(t("approvalsScreen.reject.message"))}
                        fullWidth
                        validate={[
                            (value, allValues) => {
                                if (allValues.rejection_reason === RejectReason.OTHER && !value) {
                                    return t("approvalsScreen.reject.messageRequired")
                                }
                                return undefined
                            },
                        ]}
                    />
                </Box>
            </SimpleForm>
        </FormDialog>
    )
}
