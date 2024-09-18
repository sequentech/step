// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import styled from "@emotion/styled"
import {Paper, Box, Typography, IconButton, Table, TableCell, AccordionSummary} from "@mui/material"

export const CustomAccordionSummary = styled(AccordionSummary)(({theme}) => ({
    "backgroundColor": theme.palette.customGrey.main,
    "color": theme.palette.common.white,
    "borderTopLeftRadius": "6px",
    "borderTopRightRadius": "6px",
    "minHeight": "45px !important",
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
})

export const StackContainer = styled(Box)({
    display: "flex",
    flexDirection: "column",
    position: "fixed",
    bottom: 16,
    right: 16,
    zIndex: 1000,
    gap: "10px",
    maxHeight: "80vh",
    overflowY: "auto",
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

export const TypeTypography = styled(Typography)(({theme}) => ({
    fontSize: "14px",
    margin: "0px",
    color: theme.palette.common.white,
    fontWeight: "500",
}))

export const StatusIconsBox = styled(Box)({
    display: "flex",
    justifyContent: "space-between",
    width: "50%",
})

export const IconsBox = styled(Box)({
    display: "flex",
    alignItems: "center",
})

export const StyledIconButton = styled(IconButton)(({theme}) => ({
    "marginLeft": 3,
    "color": theme.palette.common.white,
    ":hover": {
        backgroundColor: "rgba(255, 255, 255, 0.8)",
        borderRadius: "50%",
        color: "rgba(15, 5, 76, 0.8)",
    },
}))

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

export const ViewTaskTypography = styled(TypeTypography)(({theme}) => ({
    "margin": 0,
    "padding": "3px",
    "color": theme.palette.brandColor,
    "cursor": "pointer",
    "width": "max-content",
    "alignSelf": "flex-end",
    "fontWeight": 400,
    ":hover": {
        backgroundColor: theme.palette.customGrey.light,
        borderRadius: "10%",
    },
}))

export const TransparentTable = styled(Table)({
    "backgroundColor": "transparent",
    "& th, & td": {
        borderBottom: "none",
    },
})

interface TransparentTableCellProps {
    isFailed?: boolean
}

export const TransparentTableCell = styled(TableCell)<TransparentTableCellProps>(
    ({isFailed, theme}) => ({
        padding: "8px 16px",
        fontSize: "14px",
        color: isFailed ? "darkred" : "inherit",
    })
)
