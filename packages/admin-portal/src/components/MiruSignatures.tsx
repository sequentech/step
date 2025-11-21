// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useMemo} from "react"
import {useTranslation} from "react-i18next"
import {Sequent_Backend_Area} from "@/gql/graphql"
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
    area?: Sequent_Backend_Area | null
    signatures: IMiruSignature[]
}

export const MiruSignatures: React.FC<MiruSignaturesProps> = (props) => {
    const {signatures, area} = props
    const {t} = useTranslation()

    const areaTrustees = useMemo((): Array<string> => {
        let trusteesAnnotation = area?.annotations?.["miru:area-trustee-users"]
        if (!trusteesAnnotation) {
            return []
        }

        try {
            let trustees: Array<string> = JSON.parse(trusteesAnnotation)
            return trustees
        } catch {
            return []
        }
    }, [area?.annotations, area?.annotations?.["miru:area-trustee-users"]])

    const hasSigned = (trusteeName: string) => {
        return !!signatures.find((signature) => signature.sbei_miru_id === trusteeName)
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
                        {areaTrustees.map((trustee) => {
                            return (
                                <TableRow
                                    key={trustee}
                                    sx={{
                                        "&:last-child td, &:last-child th": {border: 0},
                                    }}
                                >
                                    <TableCell sx={{width: "25%"}} component="th" scope="row">
                                        {trustee}
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
