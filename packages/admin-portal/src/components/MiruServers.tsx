// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {useTranslation} from "react-i18next"
import HourglassEmptyIcon from "@mui/icons-material/HourglassEmpty"
import Table from "@mui/material/Table"
import TableBody from "@mui/material/TableBody"
import TableCell from "@mui/material/TableCell"
import TableContainer from "@mui/material/TableContainer"
import TableHead from "@mui/material/TableHead"
import TableRow from "@mui/material/TableRow"
import Paper from "@mui/material/Paper"
import {WizardStyles} from "@/components/styles/WizardStyles"
import CloseIcon from "@mui/icons-material/Close"
import {IMiruCcsServer, IMiruServersSentTo} from "@/types/miru"

interface MiruServersProps {
    servers: IMiruCcsServer[]
    serversSentTo: Array<IMiruServersSentTo>
}

export const MiruServers: React.FC<MiruServersProps> = (props) => {
    const {servers, serversSentTo} = props
    const {t} = useTranslation() //translations to be applied

    const isSentTo = (serverName: string) =>
        serversSentTo.some((server) => server.name === serverName)
    const isSentSuccessfully = (serverName: string) => {
        const server = serversSentTo.find((server) => server.name === serverName)
        return server?.status === "SUCCESS"
    }

    return (
        <>
            <TableContainer sx={{marginTop: 3}} component={Paper}>
                <Table sx={{minWidth: 650}} aria-label="simple table">
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
                    <TableBody>
                        {servers.map((server: any) => {
                            return (
                                <TableRow
                                    key={server.name as any}
                                    sx={{
                                        "&:last-child td, &:last-child th": {border: 0},
                                    }}
                                >
                                    <TableCell sx={{width: "25%"}} component="th" scope="row">
                                        {server.name}
                                    </TableCell>
                                    <TableCell align="center">
                                        {isSentTo(server.name) ? (
                                            isSentSuccessfully(server.name) ? (
                                                <WizardStyles.DoneIcon />
                                            ) : (
                                                <HourglassEmptyIcon />
                                            )
                                        ) : (
                                            <HourglassEmptyIcon />
                                        )}
                                    </TableCell>
                                </TableRow>
                            )
                        }) ?? null}
                    </TableBody>
                </Table>
            </TableContainer>
        </>
    )
}
