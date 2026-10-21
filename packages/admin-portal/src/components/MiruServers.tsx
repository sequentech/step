// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {useTranslation} from "react-i18next"
import HourglassEmptyIcon from "@mui/icons-material/HourglassEmpty"
import Box from "@mui/material/Box"
import Table from "@mui/material/Table"
import TableBody from "@mui/material/TableBody"
import TableCell from "@mui/material/TableCell"
import TableContainer from "@mui/material/TableContainer"
import TableHead from "@mui/material/TableHead"
import TableRow from "@mui/material/TableRow"
import Paper from "@mui/material/Paper"
import useMediaQuery from "@mui/material/useMediaQuery"
import {useTheme} from "@mui/material/styles"
import {WizardStyles} from "@/components/styles/WizardStyles"
import {IMiruCcsServer, IMiruServersSentTo} from "@/types/miru"
import {isManualReceipt, publicKeySha256, shortKeyFingerprint} from "@/services/transmissionPackage"

interface MiruServersProps {
    servers: IMiruCcsServer[]
    serversSentTo: Array<IMiruServersSentTo>
}

/** The start of the SHA-256 of each destination's key, by destination name. */
const useKeyFingerprints = (servers: IMiruCcsServer[]): Record<string, string> => {
    const [fingerprints, setFingerprints] = useState<Record<string, string>>({})
    const keys = servers.map((server) => `${server.name}\n${server.public_key_pem}`).join("\n\n")

    useEffect(() => {
        let current = true
        const compute = async () => {
            const entries = await Promise.all(
                servers.map(async (server): Promise<[string, string] | null> => {
                    try {
                        const sha256 = await publicKeySha256(server.public_key_pem)
                        return [server.name, shortKeyFingerprint(sha256)]
                    } catch {
                        return null
                    }
                })
            )
            if (current) {
                setFingerprints(
                    Object.fromEntries(
                        entries.filter((entry): entry is [string, string] => !!entry)
                    )
                )
            }
        }
        compute()
        return () => {
            current = false
        }
    }, [keys])

    return fingerprints
}

export const MiruServers: React.FC<MiruServersProps> = (props) => {
    const {servers, serversSentTo} = props
    const {t} = useTranslation()
    const theme = useTheme()
    const phone = useMediaQuery(theme.breakpoints.down("sm"))
    const fingerprints = useKeyFingerprints(servers)

    const isSentTo = (serverName: string) =>
        serversSentTo.some((server) => server.name === serverName)
    const isSentSuccessfully = (serverName: string) => {
        const server = serversSentTo.find((server) => server.name === serverName)
        return server?.status === "SUCCESS"
    }

    const nameOf = (server: IMiruCcsServer) => (
        <>
            <Box>{server.name}</Box>
            {isManualReceipt(server) && fingerprints[server.name] ? (
                <Box sx={{color: "text.secondary"}} data-testid="destination-key">
                    {t("tally.transmissionPackage.destinationServers.sealedFor", {
                        key: fingerprints[server.name],
                    })}
                </Box>
            ) : null}
        </>
    )

    const statusOf = (server: IMiruCcsServer) =>
        isManualReceipt(server) ? (
            <Box>{t("tally.transmissionPackage.destinationServers.handOver")}</Box>
        ) : isSentTo(server.name) && isSentSuccessfully(server.name) ? (
            <WizardStyles.DoneIcon />
        ) : (
            <HourglassEmptyIcon />
        )

    return (
        <TableContainer sx={{marginTop: 3}} component={Paper}>
            <Table aria-label={t("tally.transmissionPackage.destinationServers.title")}>
                {phone ? null : (
                    <TableHead>
                        <TableRow>
                            <TableCell sx={{width: "25%"}}>
                                {t("tally.transmissionPackage.destinationServers.table.serverName")}
                            </TableCell>
                            <TableCell align="center">
                                {t("tally.transmissionPackage.destinationServers.table.sendStatus")}
                            </TableCell>
                        </TableRow>
                    </TableHead>
                )}
                <TableBody>
                    {servers.map((server) => (
                        <TableRow
                            key={server.name}
                            sx={{"&:last-child td, &:last-child th": {border: 0}}}
                        >
                            {phone ? (
                                <TableCell component="th" scope="row">
                                    {nameOf(server)}
                                    <Box sx={{marginTop: 1}}>{statusOf(server)}</Box>
                                </TableCell>
                            ) : (
                                <>
                                    <TableCell sx={{width: "25%"}} component="th" scope="row">
                                        {nameOf(server)}
                                    </TableCell>
                                    <TableCell align="center">{statusOf(server)}</TableCell>
                                </>
                            )}
                        </TableRow>
                    ))}
                </TableBody>
            </Table>
        </TableContainer>
    )
}
