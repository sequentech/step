// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Alert, Table, TableBody, TableCell, TableRow, Typography} from "@mui/material"

export function OfflineCustodyInstructions({
    requestFilename,
    proofFilename,
    tallySessionId = null,
}: {
    requestFilename: string
    proofFilename: string
    tallySessionId?: string | null
}) {
    return (
        <Alert severity="info" sx={{mb: 2}}>
            <Typography paragraph>
                Copy the public custody request to your own encrypted working directory on the
                offline trustee workstation. In trustee.json, set these public fields. Keep your
                existing working directory and private-key filename.
            </Typography>
            <Table size="small" aria-label="Offline trustee configuration" sx={{mb: 2}}>
                <TableBody>
                    {[
                        ["request_file", JSON.stringify(requestFilename)],
                        ["proof_file", JSON.stringify(proofFilename)],
                        ["tally_session_id", JSON.stringify(tallySessionId)],
                    ].map(([name, value]) => (
                        <TableRow key={name}>
                            <TableCell component="th" scope="row">
                                <code>{name}</code>
                            </TableCell>
                            <TableCell sx={{overflowWrap: "anywhere"}}>
                                <code>{value}</code>
                            </TableCell>
                        </TableRow>
                    ))}
                </TableBody>
            </Table>
            <Typography>From the accepted tool folder, run:</Typography>
            <Typography component="pre" sx={{my: 1, whiteSpace: "pre-wrap"}}>
                <code>./trustee attest trustee.json</code>
            </Typography>
            <Typography>
                Expect “CUSTODY PROOF CREATED: {proofFilename}”, then upload that public proof here.
                If the command stops, correct the reported problem before continuing. Keep your
                private trustee keys on the encrypted medium; never upload them.
            </Typography>
        </Alert>
    )
}
