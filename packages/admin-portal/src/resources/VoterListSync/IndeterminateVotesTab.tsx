// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useCallback, useEffect, useMemo, useState} from "react"
import {Alert, Button, Chip, CircularProgress, Stack, Typography} from "@mui/material"
import {DataGrid, GridColDef, GridRenderCellParams} from "@mui/x-data-grid"
import {ECastVoteResolution, IndeterminateCastVote} from "./types"
import {mockFetchIndeterminateCastVotes, mockResolveCastVote} from "./mockIndeterminateEngine"

interface IndeterminateVotesTabProps {
    onPendingCountChange: (count: number) => void
}

const formatTimestamp = (iso: string): string => new Date(iso).toLocaleString()

/**
 * "Indeterminate Ballot Resolution" from DatafixPossibleImplementation.md:
 * lists indeterminate cast votes next to their electoral log entries
 * (joined by ballot_id) and resolves each to valid/discarded - a direct
 * Postgres write, never a VoterView call. Backend interaction is mocked in
 * mockIndeterminateEngine.ts.
 */
export const IndeterminateVotesTab: React.FC<IndeterminateVotesTabProps> = ({
    onPendingCountChange,
}) => {
    const [rows, setRows] = useState<IndeterminateCastVote[] | null>(null)
    const [resolvingId, setResolvingId] = useState<string | null>(null)

    useEffect(() => {
        let cancelled = false
        // MOCK: see mockFetchIndeterminateCastVotes.
        mockFetchIndeterminateCastVotes().then((votes) => {
            if (!cancelled) {
                setRows(votes)
            }
        })
        return () => {
            cancelled = true
        }
    }, [])

    useEffect(() => {
        onPendingCountChange(rows?.length ?? 0)
    }, [rows, onPendingCountChange])

    const handleResolve = useCallback(async (voteId: string, resolution: ECastVoteResolution) => {
        setResolvingId(voteId)
        // MOCK: writes cast_vote.status directly, no VoterView call.
        await mockResolveCastVote(voteId, resolution)
        setRows((previous) => previous?.filter((vote) => vote.id !== voteId) ?? previous)
        setResolvingId(null)
    }, [])

    const columns: GridColDef<IndeterminateCastVote>[] = useMemo(
        () => [
            {field: "voterIdString", headerName: "Voter ID", width: 120},
            {field: "ballotId", headerName: "Ballot ID", width: 160},
            {
                field: "createdAt",
                headerName: "Cast at",
                width: 170,
                renderCell: (params: GridRenderCellParams<IndeterminateCastVote>) =>
                    formatTimestamp(params.row.createdAt),
            },
            {
                field: "electoralLogEntries",
                headerName: "Electoral log (joined by ballot ID)",
                flex: 1,
                minWidth: 340,
                renderCell: (params: GridRenderCellParams<IndeterminateCastVote>) => (
                    <Stack spacing={0.5} sx={{py: 1}}>
                        {params.row.electoralLogEntries.map((entry) => (
                            <Stack key={entry.id} direction="row" spacing={1} alignItems="center">
                                <Chip
                                    size="small"
                                    label={entry.statementKind}
                                    color={
                                        entry.statementKind === "CastVoteError"
                                            ? "warning"
                                            : "default"
                                    }
                                />
                                <Typography variant="caption" color="text.secondary">
                                    {formatTimestamp(entry.createdAt)} - {entry.description}
                                </Typography>
                            </Stack>
                        ))}
                    </Stack>
                ),
            },
            {
                field: "actions",
                headerName: "Resolve",
                width: 230,
                sortable: false,
                filterable: false,
                renderCell: (params: GridRenderCellParams<IndeterminateCastVote>) => (
                    <Stack direction="row" spacing={1}>
                        <Button
                            size="small"
                            variant="outlined"
                            color="success"
                            disabled={resolvingId === params.row.id}
                            onClick={() => handleResolve(params.row.id, "valid")}
                        >
                            Mark valid
                        </Button>
                        <Button
                            size="small"
                            variant="outlined"
                            color="error"
                            disabled={resolvingId === params.row.id}
                            onClick={() => handleResolve(params.row.id, "discarded")}
                        >
                            Mark discarded
                        </Button>
                    </Stack>
                ),
            },
        ],
        [resolvingId, handleResolve]
    )

    if (rows === null) {
        return (
            <Stack direction="row" spacing={2} alignItems="center">
                <CircularProgress size={20} />
                <Typography>Loading indeterminate ballots...</Typography>
            </Stack>
        )
    }

    return (
        <Stack spacing={2}>
            <Typography color="text.secondary">
                Ballots left indeterminate by an ambiguous SetVoted outcome. Resolve each to "valid"
                or "discarded" using the electoral log entries below - this writes the vote status
                directly and never calls VoterView. Do this daily, before importing that day's
                reconciliation file.
            </Typography>
            {rows.length === 0 ? (
                <Alert severity="success">No indeterminate ballots for this election event.</Alert>
            ) : (
                <DataGrid
                    autoHeight
                    rows={rows}
                    columns={columns}
                    density="compact"
                    getRowId={(row) => row.id}
                    getRowHeight={() => "auto"}
                    initialState={{pagination: {paginationModel: {pageSize: 10}}}}
                    pageSizeOptions={[10, 25, 50]}
                    disableRowSelectionOnClick
                />
            )}
        </Stack>
    )
}

export default IndeterminateVotesTab
