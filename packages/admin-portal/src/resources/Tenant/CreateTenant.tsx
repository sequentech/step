// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {CircularProgress, Drawer, Typography} from "@mui/material"
import React, {useContext, useEffect, useState} from "react"
import {
    SimpleForm,
    TextInput,
    Create,
    useNotify,
    SaveButton,
    Toolbar,
    useRefresh,
    useGetList,
    useGetOne,
    NumberInput,
    required,
    minValue,
    maxValue,
} from "react-admin"
import {useMutation} from "@apollo/client"
import {INSERT_TENANT} from "../../queries/InsertTenant"
import {InsertTenantMutation} from "../../gql/graphql"
import {FieldValues, SubmitHandler} from "react-hook-form"
import {useTranslation} from "react-i18next"
import {useNavigate} from "react-router-dom"
import {isNull} from "@sequentech/ui-core"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useWidgetStore} from "@/providers/WidgetsContextProvider"
import {WidgetProps} from "@/components/Widget"
import {ETasksExecution} from "@/types/tasksExecution"
import {DecryptHelp, PasswordDialog} from "@/components/election-event/export-data/PasswordDialog"
import {decryptionCommand} from "@/resources/Reports/ListReports"

// A keys ceremony needs a threshold of at least 2
const MIN_TRUSTEES = 2

interface CreateTenantProps {
    isDrawerOpen: boolean
    setIsDrawerOpen: (value: boolean) => void
}

export const CreateTenant: React.FC<CreateTenantProps> = ({isDrawerOpen, setIsDrawerOpen}) => {
    const [createTenant] = useMutation<InsertTenantMutation>(INSERT_TENANT)
    const [newId, setNewId] = useState<string | null>(null)
    const [isLoading, setIsLoading] = useState(false)
    const [filePassword, setFilePassword] = useState<string | null>(null)
    const authContext = useContext(AuthContext)
    const [addWidget, setWidgetTaskId, updateWidgetFail] = useWidgetStore()
    const {t} = useTranslation()
    const navigate = useNavigate()
    const refresh = useRefresh()
    const {
        data: newTenant,
        isLoading: isOneLoading,
        error,
    } = useGetOne("sequent_backend_tenant", {
        id: authContext?.tenantId,
    })
    // The new tenant's trustees are copied from these
    const {data: trustees} = useGetList("sequent_backend_trustee", {
        pagination: {page: 1, perPage: 100},
        filter: {tenant_id: authContext?.tenantId},
    })
    const maxTrustees = trustees?.length ?? 0

    // The parent unmounts this component with the drawer, which would take the password
    // dialog with it: the password is only ever shown here
    const closeDrawer = () => {
        if (!filePassword) {
            setIsDrawerOpen(false)
        }
    }

    const closePasswordDialog = () => {
        setFilePassword(null)
        setIsDrawerOpen(false)
    }

    useEffect(() => {
        if (isNull(newId)) {
            return
        }
        if (isLoading && error && !isOneLoading) {
            setIsLoading(false)
            closeDrawer()
            refresh()
            return
        }
        if (isLoading && !error && !isOneLoading && newTenant) {
            setIsLoading(false)
            closeDrawer()
        }
    }, [
        isLoading,
        newTenant,
        isOneLoading,
        error,
        newId,
        filePassword,
        refresh,
        authContext,
        navigate,
    ])

    const onSubmit: SubmitHandler<FieldValues> = async ({slug, num_trustees}) => {
        // The widget downloads the credentials file when the task ends
        const currWidget: WidgetProps = addWidget(ETasksExecution.CREATE_TEMANT, true)
        try {
            let {data, errors} = await createTenant({
                variables: {
                    slug,
                    numTrustees: num_trustees,
                },
            })
            if (errors || data?.insertTenant?.error_msg) {
                setIsLoading(false)
                updateWidgetFail(currWidget.identifier)
                return
            }

            setNewId(data?.insertTenant?.id)
            setFilePassword(data?.insertTenant?.password ?? null)
            setIsLoading(true)
            let taskId = data?.insertTenant?.task_execution?.id
            setWidgetTaskId(currWidget.identifier, taskId)
        } catch (e) {
            setIsLoading(false)
            updateWidgetFail(currWidget.identifier)
        }
    }
    return (
        <>
            <Drawer
                anchor="right"
                open={isDrawerOpen && !filePassword}
                onClose={() => setIsDrawerOpen(false)}
                PaperProps={{
                    sx: {width: "30%"},
                }}
            >
                <SimpleForm
                    onSubmit={onSubmit}
                    toolbar={
                        <Toolbar>
                            <SaveButton disabled={isLoading} />
                        </Toolbar>
                    }
                >
                    <Typography variant="h4">{`${t("tenantScreen.common.title")} ${
                        newTenant?.slug
                    }`}</Typography>
                    <Typography variant="body2">{t("tenantScreen.new.subtitle")}</Typography>
                    <TextInput source="slug" onKeyDown={(event) => event.stopPropagation()} />
                    {trustees ? (
                        <NumberInput
                            source="num_trustees"
                            label={String(t("tenantScreen.new.numTrustees"))}
                            defaultValue={maxTrustees}
                            min={MIN_TRUSTEES}
                            max={maxTrustees}
                            validate={[required(), minValue(MIN_TRUSTEES), maxValue(maxTrustees)]}
                        />
                    ) : null}
                    {isLoading ? <CircularProgress /> : null}
                </SimpleForm>
            </Drawer>
            {filePassword ? (
                <PasswordDialog password={filePassword} onClose={closePasswordDialog}>
                    <DecryptHelp decryptionCommand={decryptionCommand} />
                </PasswordDialog>
            ) : null}
        </>
    )
}
