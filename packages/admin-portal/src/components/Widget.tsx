// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect, useState} from "react"
import {
    Accordion,
    AccordionDetails,
    Divider,
    LinearProgress,
    TableBody,
    TableRow,
} from "@mui/material"
import {
    TransparentTable,
    TransparentTableCell,
    WidgetContainer,
    HeaderBox,
    InfoBox,
    TypeTypography,
    IconsBox,
    StyledIconButton,
    StyledProgressBar,
    LogTypography,
    LogsBox,
    CustomAccordionSummary,
    ViewTaskTypography,
} from "./styles/WidgetStyle"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"
import CloseIcon from "@mui/icons-material/Close"
import {Visibility} from "@mui/icons-material"
import {ETaskExecutionStatus} from "@sequentech/ui-core"
import {ETasksExecution} from "@/types/tasksExecution"
import {StatusChip} from "./StatusChip"
import {IKeysCeremonyLog as ITaskLog} from "@/services/KeyCeremony"
import {useTranslation} from "react-i18next"
import {ViewTask} from "@/resources/Tasks/ViewTask"

interface LogTableProps {
    logs: ITaskLog[]
}

export const LogTable: React.FC<LogTableProps> = ({logs}) => {
    return (
        <TransparentTable>
            <TableBody>
                {logs.map((log, index) => (
                    <TableRow key={index}>
                        <TransparentTableCell>
                            {new Date(log.created_date).toLocaleString()}
                        </TransparentTableCell>
                        <TransparentTableCell>{log.log_text}</TransparentTableCell>
                    </TableRow>
                ))}
            </TableBody>
        </TransparentTable>
    )
}

export interface WidgetStateProps {
    type: ETasksExecution
    status: ETaskExecutionStatus
    logs?: Array<ITaskLog>
    id?: String
}

interface WidgetProps {
    type: ETasksExecution
    status: ETaskExecutionStatus
    onClose: (val: {}) => void
    onSuccess?: () => void
    onFailure?: () => void
    logs?: Array<ITaskLog>
    id?: String
}

export const Widget: React.FC<WidgetProps> = ({
    type,
    status,
    onClose,
    onSuccess,
    onFailure,
    logs,
    id,
}) => {
    const {t} = useTranslation()
    const [expanded, setExpanded] = useState(false)
    const [openTaskModal, setOpenTaskModal] = useState(false)

    const initialLog: ITaskLog[] = [
        {created_date: new Date().toLocaleString(), log_text: "Task started"},
    ]

    useEffect(() => {
        if (status === ETaskExecutionStatus.FAILED) {
            setExpanded(true)
            onFailure && onFailure()
        } else if (status === ETaskExecutionStatus.SUCCESS) {
            onSuccess && onSuccess()
        }
    }, [status])

    const onSetViewTask = (event: React.ChangeEvent<{}>) => {
        event.stopPropagation()
        setOpenTaskModal(!openTaskModal)
    }

    return (
        <>
            <WidgetContainer>
                <Accordion expanded={expanded} onChange={() => setExpanded(!expanded)}>
                    <CustomAccordionSummary
                        expandIcon={<ExpandMoreIcon />}
                        sx={{backgroundColor: "#0F054C"}}
                    >
                        <HeaderBox>
                            <InfoBox>
                                <TypeTypography>
                                    <b>Task: </b>
                                    {t(`tasksScreen.tasksExecution.${type}`)}
                                </TypeTypography>
                                <StatusChip status={status} />
                                <IconsBox>
                                    <StyledIconButton size="small">
                                        <Visibility onClick={(event) => onSetViewTask(event)} />
                                    </StyledIconButton>
                                    <StyledIconButton size="small">
                                        <CloseIcon onClick={onClose} />
                                    </StyledIconButton>
                                </IconsBox>
                            </InfoBox>
                            {status === ETaskExecutionStatus.IN_PROGRESS && (
                                <StyledProgressBar>
                                    <LinearProgress />
                                </StyledProgressBar>
                            )}
                        </HeaderBox>
                    </CustomAccordionSummary>
                    <AccordionDetails
                        sx={{display: "flex", flexDirection: "column", padding: "8px 16px"}}
                    >
                        <LogsBox>
                            <LogTypography>{t("widget.logs")}</LogTypography>
                            <Divider />
                            <LogTable logs={logs || initialLog} />
                        </LogsBox>
                        <ViewTaskTypography onClick={onSetViewTask}>View Task</ViewTaskTypography>
                    </AccordionDetails>
                </Accordion>
            </WidgetContainer>
            {openTaskModal && id && (
                <ViewTask currTaskId={id} goBack={() => setOpenTaskModal(false)} isModal={true} />
            )}
        </>
    )
}
