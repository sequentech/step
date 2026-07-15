// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {styled} from "@mui/material/styles"
import Table from "@mui/material/Table"
import TableBody from "@mui/material/TableBody"
import TableCell from "@mui/material/TableCell"
import TableContainer from "@mui/material/TableContainer"
import TableHead from "@mui/material/TableHead"
import TableRow from "@mui/material/TableRow"
import Typography from "@mui/material/Typography"

export interface ReviewChangesRow {
    field: string
    label: string
    currentValue: string
    newValue: string
}

export interface ReviewChangesTableProps {
    title: string
    subtitle?: string
    fieldLabel: string
    currentValueLabel: string
    newValueLabel: string
    rows: ReviewChangesRow[]
    headingRef?: React.RefObject<HTMLDivElement | null>
    className?: string
}

const ReviewChangesTableStyles = {
    TableContainer: styled(TableContainer)`
        width: 100%;
        margin-top: 0.5rem;
        box-shadow: none;
    `,
    HeaderCell: styled(TableCell)`
        background-color: ${({theme}) => `${theme.palette.brandColor}14`};
        color: ${({theme}) => theme.palette.brandColor};
        font-weight: ${({theme}) => theme.typography.fontWeightBold};
    `,
    OldValueCell: styled(TableCell)`
        opacity: 0.6;
    `,
}

/**
 * Generic before/after review table: shows only the rows the caller passes
 * in (typically a diff of changed fields), with the current value struck
 * through (via a semantic <del>) next to the new value shown plain. Callers
 * own translation, diffing, and when to compute `rows` - this component is
 * purely presentational so it can be reused by any review-before-commit
 * flow in the admin portal.
 */
export const ReviewChangesTable: React.FC<ReviewChangesTableProps> = ({
    title,
    subtitle,
    fieldLabel,
    currentValueLabel,
    newValueLabel,
    rows,
    headingRef,
    className,
}) => {
    return (
        <div className={className}>
            <Typography
                variant="h5"
                component="h2"
                ref={headingRef}
                tabIndex={-1}
                sx={{
                    color: (theme) => theme.palette.brandColor,
                    fontWeight: (theme) => theme.typography.fontWeightBold,
                    marginTop: "1rem",
                    overflowWrap: "break-word",
                }}
            >
                {title}
            </Typography>
            {subtitle && (
                <Typography variant="body2" component="p" color="text.secondary">
                    {subtitle}
                </Typography>
            )}
            <ReviewChangesTableStyles.TableContainer>
                <Table aria-label={title}>
                    <TableHead>
                        <TableRow>
                            <ReviewChangesTableStyles.HeaderCell>
                                {fieldLabel}
                            </ReviewChangesTableStyles.HeaderCell>
                            <ReviewChangesTableStyles.HeaderCell>
                                {currentValueLabel}
                            </ReviewChangesTableStyles.HeaderCell>
                            <ReviewChangesTableStyles.HeaderCell>
                                {newValueLabel}
                            </ReviewChangesTableStyles.HeaderCell>
                        </TableRow>
                    </TableHead>
                    <TableBody>
                        {rows.map((row) => (
                            <TableRow key={row.field}>
                                <TableCell
                                    sx={{fontWeight: (theme) => theme.typography.fontWeightMedium}}
                                >
                                    {row.label}
                                </TableCell>
                                <ReviewChangesTableStyles.OldValueCell>
                                    <del>{row.currentValue}</del>
                                </ReviewChangesTableStyles.OldValueCell>
                                <TableCell>{row.newValue}</TableCell>
                            </TableRow>
                        ))}
                    </TableBody>
                </Table>
            </ReviewChangesTableStyles.TableContainer>
        </div>
    )
}

export default ReviewChangesTable
