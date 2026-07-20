// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useCallback, useEffect, useMemo, useState} from "react"
import {
    Alert,
    Button,
    CircularProgress,
    Divider,
    Drawer,
    IconButton,
    Stack,
    Tooltip,
    Typography,
} from "@mui/material"
import DoneIcon from "@mui/icons-material/Done"
import CloseIcon from "@mui/icons-material/Close"
import VisibilityIcon from "@mui/icons-material/Visibility"
import {DataGrid, GridColDef, GridRenderCellParams} from "@mui/x-data-grid"
import ElectionHeader from "@/components/ElectionHeader"
import {ElectoralLogFilters, ElectoralLogList} from "@/components/ElectoralLogList"
import {ECastVoteResolution, IndeterminateCastVote} from "./types"
import {mockFetchIndeterminateCastVotes, mockResolveCastVote} from "./mockIndeterminateEngine"

interface IndeterminateVotesTabProps {
    electionEventId?: string
    onPendingCountChange: (count: number) => void
}

const formatTimestamp = (iso: string): string => new Date(iso).toLocaleString()

const MetadataLine: React.FC<{label: string; value: string}> = ({label, value}) => (
    <Typography variant="body2">
        <Typography component="span" variant="body2" color="text.secondary">
            {label}:{" "}
        </Typography>
        {value}
    </Typography>
)

/**
 * "Indeterminate Ballot Resolution" from DatafixPossibleImplementation.md:
 * lists indeterminate cast votes, each opening a review drawer (same
 * "Actions column with a Review button -> detail drawer" pattern as
 * TallySheetImports.tsx) with the ballot's details and that voter's
 * electoral log entries (same ElectoralLogList used by ListUsers.tsx's
 * "User's Logs" action, filtered by username), and resolves it to
 * valid/discarded - a direct Postgres write, never a VoterView call. The
 * cast-vote listing/resolution is mocked in mockIndeterminateEngine.ts.
 */
export const IndeterminateVotesTab: React.FC<IndeterminateVotesTabProps> = ({
    electionEventId,
    onPendingCountChange,
}) => {
    const [rows, setRows] = useState<IndeterminateCastVote[] | null>(null)
    const [reviewVote, setReviewVote] = useState<IndeterminateCastVote | null>(null)
    const [resolving, setResolving] = useState(false)

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

    const handleResolve = useCallback(
        async (resolution: ECastVoteResolution) => {
            if (!reviewVote) {
                return
            }
            setResolving(true)
            // MOCK: writes cast_vote.status directly, no VoterView call.
            await mockResolveCastVote(reviewVote.id, resolution)
            setRows((previous) => previous?.filter((vote) => vote.id !== reviewVote.id) ?? previous)
            setResolving(false)
            setReviewVote(null)
        },
        [reviewVote]
    )

    const columns: GridColDef<IndeterminateCastVote>[] = useMemo(
        () => [
            {field: "userId", headerName: "User ID", width: 160},
            {field: "voterIdString", headerName: "Username", width: 140},
            {field: "enabled", headerName: "Enabled", type: "boolean", width: 100},
            {field: "voted", headerName: "Voted", type: "boolean", width: 100},
            {field: "ballotId", headerName: "Ballot ID", flex: 1, minWidth: 220},
            {
                field: "createdAt",
                headerName: "Cast at",
                width: 180,
                renderCell: (params: GridRenderCellParams<IndeterminateCastVote>) =>
                    formatTimestamp(params.row.createdAt),
            },
            {
                field: "actions",
                headerName: "Actions",
                width: 100,
                sortable: false,
                filterable: false,
                renderCell: (params: GridRenderCellParams<IndeterminateCastVote>) => (
                    <Tooltip title="Review">
                        <IconButton size="small" onClick={() => setReviewVote(params.row)}>
                            <VisibilityIcon fontSize="small" />
                        </IconButton>
                    </Tooltip>
                ),
            },
        ],
        []
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
                Ballots left indeterminate by an ambiguous SetVoted outcome. Review each one to see
                its electoral log entries and resolve it to "valid" or "discarded" - this writes the
                vote status directly and never calls VoterView. Do this daily, before importing that
                day's reconciliation file.
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
                    initialState={{pagination: {paginationModel: {pageSize: 10}}}}
                    pageSizeOptions={[10, 25, 50]}
                    disableRowSelectionOnClick
                />
            )}

            <Drawer
                anchor="right"
                open={!!reviewVote}
                onClose={() => setReviewVote(null)}
                PaperProps={{sx: {width: {xs: "100vw", md: 700}, maxWidth: "100vw"}}}
            >
                {reviewVote ? (
                    <Stack gap={2} sx={{p: 3}}>
                        <Stack direction="row" justifyContent="space-between" alignItems="center">
                            <ElectionHeader
                                title="Cast Vote Review"
                                subtitle={reviewVote.ballotId}
                            />
                            <Stack direction="row" gap={1}>
                                <Button
                                    color="success"
                                    variant="contained"
                                    startIcon={<DoneIcon />}
                                    disabled={resolving}
                                    onClick={() => handleResolve("valid")}
                                >
                                    Valid
                                </Button>
                                <Button
                                    color="error"
                                    variant="outlined"
                                    startIcon={<CloseIcon />}
                                    disabled={resolving}
                                    onClick={() => handleResolve("discarded")}
                                >
                                    Discard
                                </Button>
                                <Button disabled={resolving} onClick={() => setReviewVote(null)}>
                                    Close
                                </Button>
                            </Stack>
                        </Stack>

                        <Stack gap={0.5}>
                            <MetadataLine label="User ID" value={reviewVote.userId} />
                            <MetadataLine label="Username" value={reviewVote.voterIdString} />
                            <MetadataLine
                                label="Cast at"
                                value={formatTimestamp(reviewVote.createdAt)}
                            />
                            <MetadataLine
                                label="Last updated"
                                value={formatTimestamp(reviewVote.lastUpdatedAt)}
                            />
                        </Stack>

                        <Divider />

                        <Typography variant="h6">Electoral log</Typography>
                        {/* Same getter ListUsers.tsx uses for its "User's Logs" action -
                            see ElectoralLogList/getElectoralLog. */}
                        <ElectoralLogList
                            electionEventId={electionEventId}
                            showActions={false}
                            filterToShow={ElectoralLogFilters.USERNAME}
                            filterValue={reviewVote.voterIdString}
                        />
                    </Stack>
                ) : null}
            </Drawer>
        </Stack>
    )
}

export default IndeterminateVotesTab
