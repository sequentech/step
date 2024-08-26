// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {IKeysCeremonyLog as ITaskLog} from "@/services/KeyCeremony"
import {Paper, Box, Typography, IconButton, Divider} from "@mui/material"
import CloseIcon from "@mui/icons-material/Close"
import {Visibility} from "@mui/icons-material"
import CheckCircleIcon from "@mui/icons-material/CheckCircle"
import ErrorIcon from "@mui/icons-material/Error"
import LoaderIcon from "@mui/icons-material/HourglassEmpty"
import {ETaskExecutionStatus} from "@sequentech/ui-core"
import {ETasksExecution} from "@/types/tasksExecution"
import {styled} from "@mui/material/styles"
import {useLocation, useNavigate} from "react-router-dom"

const StyledPaper = styled(Paper)({
    width: 320,
    position: "fixed",
    bottom: 16,
    right: 16,
    padding: 16,
    zIndex: 1300,
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
    onClose: (val: {}) => void
    logs?: Array<ITaskLog>
}

export const Widget: React.FC<WidgetProps> = ({type, status, onClose, logs}) => {
    const navigate = useNavigate()
    const location = useLocation()

    const getStatusIcon = () => {
        if (status === ETaskExecutionStatus.SUCCESS) return <CheckCircleIcon color="success" />
        if (status === ETaskExecutionStatus.FAILED) return <ErrorIcon color="error" />
        if (status === ETaskExecutionStatus.IN_PROGRESS) return <LoaderIcon color="action" />
        return null
    }

    const handleNavigateNext = () => {
        const baseUrl = location.pathname.split("/").slice(0, 3).join("/")
        const newUrl = `${baseUrl}/8`
        navigate(newUrl)
    }

    return (
        <StyledPaper>
            <HeaderBox>
                <StatusTypography>{type}</StatusTypography>
                <StatusBox>
                    {getStatusIcon()}
                    <StyledIconButton size="small">
                        <Visibility onClick={handleNavigateNext} />
                    </StyledIconButton>
                    <StyledIconButton size="small">
                        <CloseIcon onClick={onClose} />
                    </StyledIconButton>
                </StatusBox>
            </HeaderBox>
        </StyledPaper>
    )
}
