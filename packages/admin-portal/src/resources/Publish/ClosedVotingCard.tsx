// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Alert, Box, Table, TableBody, TableCell, TableRow, Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import {useSigningFormat} from "@/components/signing/format"
import type {TFunction} from "i18next"
import type {ISigningClosingBallotBox, ISigningPanelData} from "@/lib/signing/api"
import {SigningRequestStatus} from "@/lib/signing/types"

/** A closing signature in the seal record. */
interface IClosingSignature {
    display_name?: string
    username?: string
    signed_at?: string
}

/**
 * What the seal record shows of one seal the close made, one per country;
 * the seal itself is VOTE-FREEZE's.
 */
interface ISealSummary {
    area_id?: string | null
    area_name?: string | null
    hash_algorithm?: string
    hash?: string
    ballots?: number | null
    signed_by?: string | null
}

/**
 * What closing voting keeps as its result: the seal record. Results stored
 * before `seals` existed carry `seal: null` instead, which is ignored.
 */
interface ISealRecord {
    closed_at?: string
    code?: string
    signatures?: IClosingSignature[]
    seals?: ISealSummary[] | null
}

/** A ballot box's status, as the Post's Dashboard words it. */
const ballotBoxStatus = (
    t: TFunction,
    box: ISigningClosingBallotBox,
    time: (value: string) => string
): string => {
    switch (box.status) {
        case "pending":
            // Past its deadline, the sealer takes it on its next run.
            return new Date(box.grace_deadline).getTime() <= Date.now()
                ? t("dashboard.ballotBoxes.status.due")
                : t("dashboard.ballotBoxes.status.sealing", {time: time(box.grace_deadline)})
        case "sealed":
            return t("dashboard.ballotBoxes.status.publishing")
        case "published":
            return t("dashboard.ballotBoxes.status.sealed")
        default:
            return t("dashboard.ballotBoxes.status.failed")
    }
}

const shortHash = (hash: string) =>
    hash.length > 16 ? `${hash.slice(0, 8)}…${hash.slice(-8)}` : hash

type Rows = Array<[string, string]>

/** A table of label and value rows. */
const RowsTable: React.FC<{label: string; rows: Rows}> = ({label, rows}) => (
    <Table size="small" aria-label={label}>
        <TableBody>
            {rows.map(([name, value]) => (
                <TableRow key={name}>
                    <TableCell
                        component="th"
                        scope="row"
                        sx={{width: "45%", color: "text.secondary"}}
                    >
                        {name}
                    </TableCell>
                    <TableCell sx={{overflowWrap: "anywhere"}}>{value}</TableCell>
                </TableRow>
            ))}
        </TableBody>
    </Table>
)

/**
 * The closed-voting card of a completed close voting request: when voting
 * closed, one block per country ballot box (as it stands now, or as the
 * stored result shows it), then the closing signatures and the signing code.
 */
export const ClosedVotingCard: React.FC<{data: ISigningPanelData}> = ({data}) => {
    const {t} = useTranslation()
    const format = useSigningFormat(data.time_zone)
    const record = (data.request.execution_result ?? null) as ISealRecord | null
    if (data.request.status !== SigningRequestStatus.Executed || !record?.closed_at) {
        return (
            <Alert severity="info" sx={{width: "100%"}} data-testid="closed-voting-pending">
                {t("signing.closed.pending")}
            </Alert>
        )
    }
    const signatures = record.signatures ?? []
    const seals = record.seals ?? []
    // The boxes are sealed after the close (VOTE-FREEZE), so the panel's
    // current ballot boxes, when it has them, stand in for the stored seals.
    const ballotBoxes = data.ballot_boxes ?? []
    const boxBlocks = ballotBoxes.map((box) => {
        const rows: Rows = [
            [t("dashboard.ballotBoxes.column.status"), ballotBoxStatus(t, box, format.time)],
        ]
        if (box.ballots !== undefined && box.ballots !== null) {
            rows.push([t("signing.closed.ballots"), box.ballots.toLocaleString()])
        }
        if (box.seal_hash) {
            rows.push([
                t("signing.closed.sealHash", {algorithm: box.hash_algorithm}),
                shortHash(box.seal_hash),
            ])
        }
        return {key: box.area_id, country: box.area_name || box.area_id, rows}
    })
    const allSealed =
        ballotBoxes.length > 0
            ? ballotBoxes.every((box) => box.status === "sealed" || box.status === "published")
            : seals.length > 0
    const storedBlocks = seals.map((seal, index) => {
        const rows: Rows = []
        if (seal.ballots !== undefined && seal.ballots !== null) {
            rows.push([t("signing.closed.ballots"), seal.ballots.toLocaleString()])
        }
        if (seal.hash) {
            rows.push([
                t("signing.closed.sealHash", {algorithm: seal.hash_algorithm ?? ""}),
                shortHash(seal.hash),
            ])
        }
        if (seal.signed_by) {
            rows.push([t("signing.closed.signedBy"), seal.signed_by])
        }
        const country = seal.area_name || seal.area_id || ""
        return {key: `${seal.area_id ?? ""}-${index}`, country, rows}
    })
    const sealBlocks = ballotBoxes.length > 0 ? boxBlocks : storedBlocks
    const rows: Rows = []
    rows.push([
        t("signing.closed.signatures"),
        t("signing.closed.signaturesValue", {
            count: signatures.length,
            code: record.code ?? data.request.code,
        }),
    ])
    rows.push([
        t("signing.closed.signers"),
        signatures
            .map((signature) => signature.display_name ?? signature.username ?? "")
            .filter(Boolean)
            .join(", "),
    ])
    return (
        <Box
            data-testid="closed-voting"
            sx={{
                width: "100%",
                border: 1,
                borderColor: "success.light",
                borderRadius: 1,
                bgcolor: "rgba(46, 125, 50, 0.06)",
                p: 2,
            }}
        >
            <Typography component="h3" sx={{fontWeight: 600, mb: 1}}>
                {allSealed
                    ? t("signing.closed.titleSealed", {time: format.time(record.closed_at)})
                    : t("signing.closed.title", {time: format.time(record.closed_at)})}
            </Typography>
            {sealBlocks.map(({key, country, rows: sealRows}) => (
                <Box key={key} sx={{mb: 1}}>
                    <Typography component="h4" variant="body2" sx={{fontWeight: 600}}>
                        {country}
                    </Typography>
                    <RowsTable label={country} rows={sealRows} />
                </Box>
            ))}
            <RowsTable label={t("signing.closed.record")} rows={rows} />
        </Box>
    )
}
