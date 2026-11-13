// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect, useState} from "react"
import {ListApprovals} from "../Approvals/ListApprovals"
import {Identifier, useRecordContext} from "react-admin"
import {ViewApproval} from "../Approvals/ViewApproval"
import {ApprovalMatrix} from "../Approvals/ApprovalMatrix"
import {Sequent_Backend_Applications, Sequent_Backend_Election_Event} from "@/gql/graphql"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"

enum ViewMode {
    View,
    List,
    Matrix,
}

type TApproval = {
    electionEventId: string
    electionId?: string
    showList?: string
}

export const EditElectionEventApprovals: React.FC<TApproval> = ({
    electionEventId,
    electionId,
    showList,
}) => {
    const electionEventRecord = useRecordContext<Sequent_Backend_Election_Event>()
    const [viewMode, setViewMode] = useState<ViewMode>(ViewMode.List)
    const [tenantId] = useTenantStore()
    // The tenant and event the view, enrollment and matrix source belong to.
    const context = `${tenantId}:${electionEventId}`
    const [viewContext, setViewContext] = useState(context)
    const [currApprovalId, setCurrApprovalId] = useState<string | Identifier | null>(null)
    // The enrollment the matrix was opened from; none when it was opened from the toolbar.
    const [source, setSource] = useState<Sequent_Backend_Applications | null>(null)
    const {taskId, setTaskId} = useElectionEventTallyStore()

    const onViewApproval = (id: Identifier) => {
        setViewContext(context)
        setViewMode(ViewMode.View)
        setCurrApprovalId(id)
        setTaskId(id)
    }

    const onViewList = () => {
        setViewContext(context)
        setViewMode(ViewMode.List)
        // Back in the queue no enrollment is open.
        setCurrApprovalId(null)
        setSource(null)
    }

    const onViewMatrix = (application?: Sequent_Backend_Applications) => {
        setViewContext(context)
        setSource(application ?? null)
        setCurrApprovalId(null)
        setViewMode(ViewMode.Matrix)
    }

    useEffect(() => {
        if (showList) {
            onViewList()
        }
    }, [showList])

    useEffect(() => {
        if (!taskId) {
            onViewList()
        }
    }, [taskId])

    // Another tenant's or event's approvals never show this one's matrix or
    // enrollment, not even for the render before the reset below.
    const shown = viewContext === context ? viewMode : ViewMode.List
    useEffect(() => {
        if (viewContext !== context) {
            onViewList()
        }
    }, [context, viewContext])

    return (
        <>
            {shown === ViewMode.List ? (
                <ListApprovals
                    electionEventId={electionEventId}
                    electionId={electionId}
                    onViewApproval={onViewApproval}
                    onViewMatrix={() => onViewMatrix()}
                    onViewRule={onViewMatrix}
                    electionEventRecord={electionEventRecord}
                />
            ) : shown === ViewMode.Matrix ? (
                <ApprovalMatrix
                    key={`${context}:${source?.id ?? ""}`}
                    electionEventId={electionEventId}
                    goBack={onViewList}
                    source={
                        source
                            ? {application: source, onBack: () => onViewApproval(source.id)}
                            : undefined
                    }
                />
            ) : (
                <ViewApproval
                    electionEventId={electionEventId}
                    electionId={electionId}
                    currApprovalId={currApprovalId}
                    electionEventRecord={electionEventRecord}
                    goBack={onViewList}
                    onViewRule={onViewMatrix}
                />
            )}
        </>
    )
}
