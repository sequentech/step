// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {CircularProgress, Drawer, Typography} from "@mui/material"
import React, {useContext, useEffect, useState} from "react"
import {SimpleForm, TextInput, Create, useNotify, useRefresh, useGetOne} from "react-admin"
import {useMutation} from "@apollo/client"
import {INSERT_TENANT} from "../../queries/InsertTenant"
import {InsertTenantMutation} from "../../gql/graphql"
import {FieldValues, SubmitHandler} from "react-hook-form"
import {useTranslation} from "react-i18next"
import {useNavigate} from "react-router"
import {isNull} from "@sequentech/ui-core"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useWidgetStore} from "@/providers/WidgetsContextProvider"
import {WidgetProps} from "@/components/Widget"
import {ETasksExecution} from "@/types/tasksExecution"

interface CreateTenantProps {
    isDrawerOpen: boolean
    setIsDrawerOpen: (value: boolean) => void
}

export const CreateTenant: React.FC<CreateTenantProps> = ({isDrawerOpen, setIsDrawerOpen}) => {
    const [createTenant] = useMutation<InsertTenantMutation>(INSERT_TENANT)
    const [newId, setNewId] = useState<string | null>(null)
    const [isLoading, setIsLoading] = useState(false)
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

    useEffect(() => {
        if (isNull(newId)) {
            return
        }
        if (isLoading && error && !isOneLoading) {
            setIsLoading(false)
            setIsDrawerOpen(false)
            refresh()
            return
        }
        if (isLoading && !error && !isOneLoading && newTenant) {
            setIsLoading(false)
            setIsDrawerOpen(false)
        }
    }, [isLoading, newTenant, isOneLoading, error, newId, refresh, authContext, navigate])

    const onSubmit: SubmitHandler<FieldValues> = async ({slug}) => {
        const currWidget: WidgetProps = addWidget(ETasksExecution.CREATE_TEMANT)
        try {
            let {data, errors} = await createTenant({
                variables: {
                    slug,
                },
            })
            if (errors || !data?.insertTenant?.id) {
                setIsLoading(false)
                updateWidgetFail(currWidget.identifier)
                return
            }

            setNewId(data?.insertTenant?.id)
            setIsLoading(true)
            let taskId = data?.insertTenant?.task_execution?.id
            setWidgetTaskId(currWidget.identifier, taskId)
        } catch (e) {
            setIsLoading(false)
            updateWidgetFail(currWidget.identifier)
        }
    }
    return (
        <Drawer
            anchor="right"
            open={isDrawerOpen}
            onClose={() => setIsDrawerOpen(false)}
            PaperProps={{
                sx: {width: "30%"},
            }}
        >
            <SimpleForm onSubmit={onSubmit}>
                <Typography variant="h4">{`${t("tenantScreen.common.title")} ${
                    newTenant?.slug
                }`}</Typography>
                <Typography variant="body2">{t("tenantScreen.new.subtitle")}</Typography>
                <TextInput source="slug" onKeyDown={(event) => event.stopPropagation()} />
                {isLoading ? <CircularProgress /> : null}
            </SimpleForm>
        </Drawer>
    )
}
