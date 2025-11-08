// SPDX-FileCopyrightText: 2024 Eduardo Robles <dev@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect, useState} from "react"
import ElectionHeader from "@/components/ElectionHeader"
import {useTranslation} from "react-i18next"
import {ListTasks} from "../Tasks/ListTasks"
import {Identifier, useRecordContext} from "react-admin"
import {ViewTask} from "../Tasks/ViewTask"
import {Sequent_Backend_Election_Event} from "@/gql/graphql"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"

enum ViewMode {
    View,
    List,
}

type TTask = {
    showList?: string
}

export const EditElectionEventTasks: React.FC<TTask> = ({showList}) => {
    const electionEventRecord = useRecordContext<Sequent_Backend_Election_Event>()
    const [viewMode, setViewMode] = useState<ViewMode>(ViewMode.List)
    const [currTaskId, setCurrTaskId] = useState<string | Identifier | null>(null)
    const {t} = useTranslation()
    const {taskId, setTaskId} = useElectionEventTallyStore()

    const onViewTask = (id: Identifier) => {
        setViewMode(ViewMode.View)
        setCurrTaskId(id)
        setTaskId(id)
    }

    const onViewList = () => {
        setViewMode(ViewMode.List)
    }

    useEffect(() => {
        if (showList) {
            setViewMode(ViewMode.List)
            setCurrTaskId(null)
        }
    }, [showList])

    useEffect(() => {
        if (!taskId) {
            setViewMode(ViewMode.List)
            setCurrTaskId(null)
        }
    }, [taskId])

    return (
        <>
            <ElectionHeader title={String(t("tasksScreen.title"))} subtitle="tasksScreen.subtitle" />
            {viewMode === ViewMode.List ? (
                <ListTasks onViewTask={onViewTask} electionEventRecord={electionEventRecord} />
            ) : (
                <ViewTask
                    currTaskId={currTaskId}
                    electionEventRecord={electionEventRecord}
                    goBack={onViewList}
                />
            )}
        </>
    )
}
