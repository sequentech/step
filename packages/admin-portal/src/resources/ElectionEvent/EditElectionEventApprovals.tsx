// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect, useState} from "react"
import {ListApprovals} from "../Approvals/ListApprovals"
import {Identifier, useDataProvider, useRecordContext} from "react-admin"
import {decisionDetails} from "../Approvals/approvalReview"
import {ViewApproval} from "../Approvals/ViewApproval"
import {ApprovalMatrix} from "../Approvals/ApprovalMatrix"
import {Sequent_Backend_Election_Event} from "@/gql/graphql"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"

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
    const [currApprovalId, setCurrApprovalId] = useState<string | Identifier | null>(null)
    // The rule that decided the enrollment the matrix was opened from.
    const [cameFrom, setCameFrom] = useState<{version: number; rule: number | null} | undefined>()
    const dataProvider = useDataProvider()
    const {taskId, setTaskId} = useElectionEventTallyStore()

    const onViewApproval = (id: Identifier) => {
        setViewMode(ViewMode.View)
        setCurrApprovalId(id)
        setTaskId(id)
    }

    const onViewList = () => {
        setViewMode(ViewMode.List)
        // Back in the queue no enrollment is open: the matrix returns here, not to the last review.
        setCurrApprovalId(null)
    }

    const onViewMatrix = (decided?: {version: number; rule: number | null}) => {
        setCameFrom(decided)
        setViewMode(ViewMode.Matrix)
    }

    const onViewRule = async (id: Identifier) => {
        const {data} = await dataProvider.getOne("sequent_backend_applications", {id})
        const decision = decisionDetails(data)
        onViewMatrix(decision ? {version: decision.matrixVersion, rule: decision.rule} : undefined)
    }

    useEffect(() => {
        if (showList) {
            setViewMode(ViewMode.List)
            setCurrApprovalId(null)
        }
    }, [showList])

    useEffect(() => {
        if (!taskId) {
            setViewMode(ViewMode.List)
            setCurrApprovalId(null)
        }
    }, [taskId])

    return (
        <>
            {viewMode === ViewMode.List ? (
                <ListApprovals
                    electionEventId={electionEventId}
                    electionId={electionId}
                    onViewApproval={onViewApproval}
                    onViewMatrix={() => onViewMatrix()}
                    onViewRule={onViewRule}
                    electionEventRecord={electionEventRecord}
                />
            ) : viewMode === ViewMode.Matrix ? (
                <ApprovalMatrix
                    electionEventId={electionEventId}
                    goBack={
                        currApprovalId && cameFrom ? () => setViewMode(ViewMode.View) : onViewList
                    }
                    cameFrom={cameFrom}
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
