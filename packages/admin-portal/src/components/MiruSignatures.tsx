// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {useTranslation} from "react-i18next"
import {Sequent_Backend_Tally_Session_Execution} from "@/gql/graphql"
import HourglassEmptyIcon from "@mui/icons-material/HourglassEmpty"
import Table from "@mui/material/Table"
import TableBody from "@mui/material/TableBody"
import TableCell from "@mui/material/TableCell"
import TableContainer from "@mui/material/TableContainer"
import TableHead from "@mui/material/TableHead"
import TableRow from "@mui/material/TableRow"
import Paper from "@mui/material/Paper"
import {WizardStyles} from "@/components/styles/WizardStyles"
import {IMiruSignature} from "@/types/miru"

interface MiruSignaturesProps {
    tallySessionExecution?: Sequent_Backend_Tally_Session_Execution | null
    signatures: IMiruSignature[]
}

export const MiruSignatures: React.FC<MiruSignaturesProps> = (props) => {
    const {signatures, tallySessionExecution} = props
    const {t} = useTranslation() //translations to be applied

    const hasSigned = (trusteeName: string) => {
        return !!signatures.find((signature) => signature.trustee_name === trusteeName)
    }

    return (
        <>
            <TableContainer sx={{marginTop: 3}} component={Paper}>
                <Table sx={{minWidth: 650}} aria-label="simple table">
                    <TableHead>
                        <TableRow>
                            <TableCell sx={{width: "25%"}}>
                                {t("tally.transmissionPackage.signatures.table.trusteeName")}
                            </TableCell>
                            <TableCell align="center">
                                {t("tally.transmissionPackage.signatures.table.signed")}
                            </TableCell>
                        </TableRow>
                    </TableHead>
                    <TableBody>
                        {tallySessionExecution?.status?.trustees.map((trustee: any) => {
                            return (
                                <TableRow
                                    key={trustee.name as any}
                                    sx={{
                                        "&:last-child td, &:last-child th": {border: 0},
                                    }}
                                >
                                    <TableCell sx={{width: "25%"}} component="th" scope="row">
                                        {trustee.name}
                                    </TableCell>
                                    <TableCell align="center">
                                        {hasSigned(trustee) ? (
                                            <WizardStyles.DoneIcon />
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
