// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {IKeysCeremonyLog as ITaskLog} from "@/services/KeyCeremony"
import {
    Paper,
    Box,
    Typography,
    IconButton,
    Divider,
    List,
    ListItem,
    ListItemText,
} from "@mui/material"
import CloseIcon from "@mui/icons-material/Close"
import CheckCircleIcon from "@mui/icons-material/CheckCircle"
import ErrorIcon from "@mui/icons-material/Error"
import LoaderIcon from "@mui/icons-material/HourglassEmpty"
import NavigateNextIcon from "@mui/icons-material/NavigateNext"
import {ETaskExecutionStatus} from "@sequentech/ui-core"
import {ETasksExecution} from "@/types/tasksExecution"
import {styled} from "@mui/material/styles"

const StyledPaper = styled(Paper)({
    width: 320,
    position: "fixed",
    bottom: 16,
    right: 16,
    padding: 16,
})

const HeaderBox = styled(Box)({
    display: "flex",
    alignItems: "center",
    justifyContent: "space-between",
})

const StatusBox = styled(Box)({
    display: "flex",
    alignItems: "center",
})

const StatusTypography = styled(Typography)({
    fontSize: "14px",
    margin: "0px",
})

const StyledIconButton = styled(IconButton)({
    marginLeft: 8,
})

interface WidgetProps {
    type: ETasksExecution
    status: ETaskExecutionStatus
    logs?: Array<ITaskLog>
}

export const Widget: React.FC<WidgetProps> = ({type, status, logs}) => {
    const getStatusIcon = () => {
        if (status === ETaskExecutionStatus.SUCCESS) return <CheckCircleIcon color="success" />
        if (status === ETaskExecutionStatus.FAILED) return <ErrorIcon color="error" />
        if (status === ETaskExecutionStatus.IN_PROGRESS) return <LoaderIcon color="action" />
        return null
    }

    return (
        <StyledPaper>
            <HeaderBox>
                <StatusTypography>{type}</StatusTypography>
                <StatusBox>
                    {getStatusIcon()}
                    <StyledIconButton size="small">
                        <NavigateNextIcon /> {/* TODO: get the URL */}
                    </StyledIconButton>
                    <StyledIconButton size="small">
                        <CloseIcon /> {/* TODO: manage state */}
                    </StyledIconButton>
                </StatusBox>
            </HeaderBox>
            <Divider sx={{my: 2}} />
            {logs && (
                <List sx={{maxHeight: 200, overflow: "auto"}}>
                    {logs.map((log, index) => (
                        <ListItem key={index}>
                            <ListItemText primary={log.log_text} secondary={log.created_date} />
                        </ListItem>
                    ))}
                </List>
            )}
        </StyledPaper>
    )
}
