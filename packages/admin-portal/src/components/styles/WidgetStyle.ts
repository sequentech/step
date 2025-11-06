// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import styled from "@emotion/styled"
import {Paper, Box, Typography, IconButton, Table, TableCell, AccordionSummary} from "@mui/material"
import {Button} from "react-admin"

import {styled as muiStyled} from "@mui/material/styles"

export const CustomAccordionSummary = styled(AccordionSummary)<{isLoading: boolean}>(
    ({theme, isLoading}) => ({
        "backgroundColor": theme.palette.customGrey.main,
        "color": theme.palette.common.white,
        "borderTopLeftRadius": "6px",
        "borderTopRightRadius": "6px",
        "minHeight": "45px !important",
        "& .MuiAccordionSummary-expandIconWrapper": {
            color: theme.palette.common.white,
            height: isLoading ? "17x" : "100%",
            rotate: "180deg",
        },
        "& .MuiAccordionSummary-content": {
            margin: "0",
        },
    })
)

export const WidgetContainer = styled(Paper)(({theme}) => ({
    display: "flex",
    flexDirection: "column",
    width: 450,
    borderTopLeftRadius: "7px",
    borderTopRightRadius: "7px",
    boxShadow: "0px 5px 5px -3px rgba(0,0,0,0.1)",
}))

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
    padding: "5px",
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
    maxWidth: "180px",
    overflow: "hidden",
    color: theme.palette.common.white,
    fontWeight: "500",
    textOverflow: "ellipsis",
    whiteSpace: "nowrap",
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
    "marginRight": 10,
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
    padding: "auto 0",
})

export const LogTypography = styled(Typography)({
    margin: 0,
    padding: "3px 0",
    fontSize: "14px",
    fontWeight: "500",
})

export const ViewTaskTypography = muiStyled(Button)(({theme}) => ({
    "margin": "5px",
    "paddingLeft": "25px",
    "backgroundColor": theme.palette.white,
    "color": theme.palette.brandColor,
    "borderRadius": "6px",
    "border": `1px solid ${theme.palette.brandColor}`,
    "cursor": "pointer",
    "width": "max-content",
    "alignSelf": "flex-end",
    "fontWeight": 400,
    ":hover": {
        backgrkoundColor: theme.palette.brandColor,
        color: theme.palette.white,
    },
}))

export const DownloaButton = muiStyled(Button)(({theme}) => ({
    "margin": "5px",
    "background": theme.palette.brandColor,
    "color": theme.palette.white,
    "borderRadius": "6px",
    "border": `1px solid ${theme.palette.brandColor}`,
    "cursor": "pointer",
    "width": "max-content",
    "alignSelf": "flex-end",
    "fontWeight": 400,
    ":hover": {
        color: theme.palette.brandColor,
        backgroundColor: theme.palette.white,
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
        padding: "8px 0",
        fontSize: "14px",
        color: isFailed ? "darkred" : "inherit",
    })
)
