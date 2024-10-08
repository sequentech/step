import {PageHeaderStyles} from "@/components/styles/PageHeaderStyles"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import React, {useContext} from "react"
import {CreateBase, RecordContext, useNotify} from "react-admin"
import {useTranslation} from "react-i18next"
import {EditReportForm} from "./EditReportForm"

interface CreateReportProps {
    close?: () => void
    electionEventId?: string
}

export const CreateReport: React.FC<CreateReportProps> = ({close, electionEventId}) => {
    const [tenantId] = useTenantStore()
    console.log("im in the create report also")
    return (
        <CreateBase>
            <EditReportForm close={close} electionEventId={electionEventId} tenantId={tenantId} isEditReport={false} />
        </CreateBase>
    )
}
