// SPDX-FileCopyrightText: 2024 Eduardo Robles <dev@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {Typography} from "@mui/material"
import React, {useState} from "react"
import ElectionHeader from "@/components/ElectionHeader"
import {useTranslation} from "react-i18next"
import {ListTasks} from "../Tasks/ListTasks"
import {Identifier, useRecordContext} from "react-admin"
import {ViewTask} from "../Tasks/ViewTask"
import {Sequent_Backend_Election_Event} from "@/gql/graphql"

enum ViewMode {
    View,
    List,
}

export const EditElectionEventTasks: React.FC = () => {
    const electionEventRecord = useRecordContext<Sequent_Backend_Election_Event>()
    const [viewMode, setViewMode] = useState<ViewMode>(ViewMode.List)
    const [currTaskId, setCurrTaskId] = useState<string | Identifier | null>(null)
    const {t} = useTranslation()

    const onViewTask = (id: Identifier) => {
        setViewMode(ViewMode.View)
        setCurrTaskId(id)
    }

    const onViewList = () => {
        setViewMode(ViewMode.List)
    }

    return (
        <>
            <ElectionHeader title={t("tasksScreen.title")} subtitle="tasksScreen.subtitle" />
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
