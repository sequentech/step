// SPDX-FileCopyrightText: 2024 Félix Robles <dev@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactElement, useState} from "react"
import {List, TextInput, useRecordContext, useListContext} from "react-admin"
import {useTranslation} from "react-i18next"
import {Sequent_Backend_Election_Event} from "@/gql/graphql"
import {Accordion, AccordionDetails, AccordionSummary, Box, Typography} from "@mui/material"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"
import {ITaskExecuted} from "@sequentech/ui-core"

interface TaskAccordionProps {
    index: number
    record: ITaskExecuted
    expanded: string | false
    handleChange: (id: string) => void
}
const TaskAccordion: React.FC<TaskAccordionProps> = ({index, record, expanded, handleChange}) => {
    const {t} = useTranslation()
    console.log({record})

    const formatDateToRFC1123 = (date: Date): string => {
        return date.toUTCString()
    }

    return (
        <Accordion expanded={expanded === record.id} onChange={() => handleChange(record.id)}>
            <AccordionSummary expandIcon={<ExpandMoreIcon />}>
                <Typography variant="subtitle1">
                    <p>
                        <strong>{t("tasksScreen.column.id")}</strong> {index}
                    </p>
                    <p>
                        <strong>{t("tasksScreen.column.start_at")}</strong>{" "}
                        {formatDateToRFC1123(new Date(record.start_at))}
                    </p>
                    <p>
                        <strong>{t("tasksScreen.column.name")}</strong> {record.name}
                    </p>
                </Typography>
            </AccordionSummary>
            <AccordionDetails>
                <Box>
                    <Typography>
                        <strong>{t("tasksScreen.column.execution_status")}</strong>{" "}
                        {record.execution_status}
                    </Typography>
                    {record.logs && (
                        <Typography>
                            <strong>{t("tasksScreen.column.logs")}</strong>{" "}
                            <pre>{JSON.stringify(record.logs, null, 2)}</pre>
                        </Typography>
                    )}
                    {record.end_at && (
                        <Typography>
                            <strong>{t("tasksScreen.column.end_at")}</strong>{" "}
                            {new Date(record.end_at).toUTCString()}
                        </Typography>
                    )}
                    <Typography>
                        <strong>{t("tasksScreen.column.executed_by_user_id")}</strong>{" "}
                        {record.executed_by_user_id}
                    </Typography>
                </Box>
            </AccordionDetails>
        </Accordion>
    )
}

interface TaskAccordionListProps {
    expanded: string | false
    handleAccordionChange: (id: string) => void
}

const TaskAccordionList: React.FC<TaskAccordionListProps> = ({expanded, handleAccordionChange}) => {
    const {data, isLoading} = useListContext<ITaskExecuted>()

    if (isLoading) {
        return <Typography>Loading...</Typography>
    }

    if (!data || data.length === 0) {
        //TODO: use logs design for empty list
        return <Typography>No tasks found.</Typography>
    }

    return (
        <Box>
            {data.map((record, index) => (
                <TaskAccordion
                    index={index + 1}
                    key={record.id}
                    record={record}
                    expanded={expanded}
                    handleChange={handleAccordionChange}
                />
            ))}
        </Box>
    )
}

export interface ListTasksProps {
    aside?: ReactElement
}
export const ListTasks: React.FC<ListTasksProps> = ({aside}) => {
    const {t} = useTranslation()
    const [expanded, setExpanded] = useState<string | false>(false)
    const electionEventRecord = useRecordContext<Sequent_Backend_Election_Event>()

    const filters: Array<ReactElement> = [
        <TextInput source="id" key="id_filter" label={t("filters.id")} />,
        <TextInput
            source="statement_kind"
            key="statement_kind_filter"
            label={t("filters.statementKind")}
        />,
    ]

    const handleAccordionChange = (taskId: string) => {
        setExpanded((prevExpanded) => (prevExpanded === taskId ? false : taskId))
    }

    return (
        <List
            resource="sequent_backend_tasks_execution"
            filters={filters}
            filter={{election_event_id: electionEventRecord?.id || undefined}}
            sort={{field: "id", order: "DESC"}}
            aside={aside}
            perPage={10}
        >
            <TaskAccordionList expanded={expanded} handleAccordionChange={handleAccordionChange} />
        </List>
    )
}
