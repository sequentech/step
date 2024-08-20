// SPDX-FileCopyrightText: 2024 Eduardo Robles <dev@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {Typography} from "@mui/material"
import React, {useContext, useState} from "react"
import ElectionHeader from "@/components/ElectionHeader"
import {useTranslation} from "react-i18next"
import {ResourceListStyles} from "@/components/styles/ResourceListStyles"
import {ListTasks} from "../Tasks/ListTasks"
import {Identifier} from "react-admin"

enum ViewMode {
    View,
    List,
}

export const EditElectionEventTasks: React.FC = () => {
    const [viewMode, setViewMode] = useState<ViewMode>(ViewMode.List)
    const [taskExecutionId, setTaskExecutionId] = useState<string | Identifier | null>(null)
    const {t} = useTranslation()

    const onViewTask = (id: Identifier) => {
        setViewMode(ViewMode.View)
        setTaskExecutionId(id)
    }

    return (
        <>
            <ElectionHeader title={t("tasksScreen.title")} subtitle="tasksScreen.subtitle" />
            {viewMode === ViewMode.List ? <ListTasks onViewTask={onViewTask} /> : <>
            
            </>}
        </>
    )
}
