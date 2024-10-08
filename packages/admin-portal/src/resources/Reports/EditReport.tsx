
import {useTenantStore} from "@/providers/TenantContextProvider"
import React from "react"
import {EditBase, Identifier} from "react-admin"
import {EditReportForm} from "./EditReportForm"

interface CreateReportProps {
    close?: () => void
    electionEventId?: string
    reportId?: Identifier | null
}

export const EditReport: React.FC<CreateReportProps> = ({close, electionEventId, reportId}) => {
    const [tenantId] = useTenantStore()
    return (
        <EditBase>
            <EditReportForm close={close} electionEventId={electionEventId} tenantId={tenantId} isEditReport={true} reportId={reportId}/>
        </EditBase>
    )
}