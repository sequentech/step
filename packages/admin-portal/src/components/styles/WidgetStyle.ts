// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import styled from "@emotion/styled"
import {Paper, Box, Typography, IconButton, Table, TableCell, AccordionSummary} from "@mui/material"

export const CustomAccordionSummary = styled(AccordionSummary)(({theme}) => ({
    "backgroundColor": "#0F054C",
    "color": theme.palette.common.white,
    "borderTopLeftRadius": "6px",
    "borderTopRightRadius": "6px",
    "& .MuiAccordionSummary-expandIconWrapper": {
        color: theme.palette.common.white,
    },
    "& .MuiAccordionSummary-content": {
        margin: "0",
    },
}))

export const WidgetContainer = styled(Paper)({
    display: "flex",
    flexDirection: "column",
    width: 450,
    position: "fixed",
    bottom: 16,
    right: 16,
})

export const HeaderBox = styled(Box)({
    display: "flex",
    flexDirection: "column",
    width: "100%",
})

export const InfoBox = styled(Box)({
    display: "flex",
    alignItems: "center",
    justifyContent: "space-between",
    width: "100%",
})

export const IconsBox = styled(Box)({
    display: "flex",
    alignItems: "center",
})

export const TypeTypography = styled(Typography)({
    fontSize: "14px",
    margin: "0px",
    color: "white",
    fontWeight: "500",
})

export const StyledIconButton = styled(IconButton)({
    "marginLeft": 3,
    "color": "white",
    ":hover": {
        backgroundColor: "rgba(255, 255, 255, 0.8)",
        borderRadius: "50%",
        color: "rgba(15, 5, 76, 0.8)",
    },
})

export const StyledProgressBar = styled(Box)({
    width: "100%",
    paddingTop: "5px",
})

export const LogsBox = styled(Box)({
    display: "flex",
    flexDirection: "column",
    maxHeight: "101px",
    overflowY: "auto",
})

export const LogTypography = styled(Typography)({
    margin: 0,
    padding: "3px 14px",
    fontSize: "14px",
    fontWeight: "500",
})

export const ViewTaskTypography = styled(TypeTypography)({
    "margin": 0,
    "padding": "3px",
    "color": "#0F054C",
    "cursor": "pointer",
    "width": "max-content",
    "alignSelf": "flex-end",
    "fontWeight": 400,
    ":hover": {
        backgroundColor: "rgba(15, 5, 76, 0.8)",
        borderRadius: "10%",
        color: "rgba(255, 255, 255, 0.8)",
    },
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
