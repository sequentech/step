// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import styled from "@emotion/styled"
import {Paper, Box, Typography, IconButton, Table, TableCell} from "@mui/material"

export const WidgetContainer = styled(Paper)({
    display: "flex",
    flexDirection: "column",
    width: "30vw",
    maxWidth: 450,
    minWidth: 300,
    position: "fixed",
    bottom: 16,
    right: 16,
})

export const HeaderBox = styled(Box)({
    display: "flex",
    flexDirection: "column",
})

export const InfoBox = styled(Box)({
    display: "flex",
    alignItems: "center",
    justifyContent: "space-between",
    width: "100%",
})

export const StatusBox = styled(Box)({
    display: "flex",
    alignItems: "center",
})

export const TypeTypography = styled(Typography)({
    fontSize: "14px",
    margin: "0px",
})

export const StyledIconButton = styled(IconButton)({
    marginLeft: 8,
})

export const StyledProgressBar = styled(Box)({
    width: "100%",
    paddingTop: "5px",
})

export const LogsBox = styled(Box)({
    display: "flex",
    alignItems: "center",
    flexDirection: "column",
})

export const LogTypography = styled(Typography)({
    margin: 0,
    padding: 0,
    fontSize: "14px",
})

export const TransparentTable = styled(Table)({
    "backgroundColor": "transparent",
    "& th, & td": {
        borderBottom: "none",
    },
})

export const TransparentTableCell = styled(TableCell)({
    padding: "8px 16px",
    fontSize: "14px",
})
