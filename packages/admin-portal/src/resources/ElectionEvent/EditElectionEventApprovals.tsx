// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect, useState} from "react"
import ElectionHeader from "@/components/ElectionHeader"
import {useTranslation} from "react-i18next"
import {ListApprovals} from "../Approvals/ListApprovals"
import {Identifier, useRecordContext} from "react-admin"
import {ViewApproval} from "../Approvals/ViewApproval"
import {Sequent_Backend_Election_Event} from "@/gql/graphql"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"

enum ViewMode {
    View,
    List,
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
    const {t} = useTranslation()
    const {taskId, setTaskId} = useElectionEventTallyStore()

    const onViewApproval = (id: Identifier) => {
        setViewMode(ViewMode.View)
        setCurrApprovalId(id)
        setTaskId(id)
    }

    const onViewList = () => {
        setViewMode(ViewMode.List)
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
            {/* <ElectionHeader title={t("approvalsScreen.title")} subtitle="approvalsScreen.subtitle" /> */}
            {viewMode === ViewMode.List ? (
                <ListApprovals
                    electionEventId={electionEventId}
                    electionId={electionId}
                    onViewApproval={onViewApproval}
                    electionEventRecord={electionEventRecord}
                />
            ) : (
                <ViewApproval
                    electionEventId={electionEventId}
                    electionId={electionId}
                    currApprovalId={currApprovalId}
                    electionEventRecord={electionEventRecord}
                    goBack={onViewList}
                />
            )}
        </>
    )
}
