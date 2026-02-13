// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState, useRef, useEffect} from "react"
import {Accordion, AccordionSummary, Box, Typography} from "@mui/material"
import Table from "@mui/material/Table"
import TableBody from "@mui/material/TableBody"
import TableCell from "@mui/material/TableCell"
import TableContainer from "@mui/material/TableContainer"
import TableHead from "@mui/material/TableHead"
import TableRow from "@mui/material/TableRow"
import Paper from "@mui/material/Paper"
import {useTranslation} from "react-i18next"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"
import {WizardStyles} from "@/components/styles/WizardStyles"
import {IKeysCeremonyLog} from "@/services/KeyCeremony"
import {styled} from "@mui/material/styles"

export const AccordionDetails = styled(WizardStyles.AccordionDetails)`
    max-height: 400px;
    overflow-y: scroll;
`

interface LogsProps {
    logs?: Array<IKeysCeremonyLog>
}
function usePreviousValue<T>(value: T): T {
    const ref = useRef<T>(value)
    useEffect(() => {
        ref.current = value
    })
    return ref.current
}

export const Logs: React.FC<LogsProps> = ({logs}) => {
    const [logsExpanded, setLogsExpanded] = useState(true)
    const {t} = useTranslation()
    const myDivRef = useRef<HTMLDivElement>(null)
    const prevLogs = usePreviousValue(logs)
    useEffect(() => {
        if (!logsExpanded) {
            return
        }
        if (!myDivRef.current) {
            return
        }
        myDivRef.current.scroll({
            top: myDivRef.current.scrollHeight,
            behavior: "smooth",
        })
    }, [logsExpanded, myDivRef.current])
    useEffect(() => {
        if (!logsExpanded) {
            return
        }
        if (!myDivRef.current) {
            return
        }
        const {scrollTop, scrollHeight, clientHeight} = myDivRef.current
        const isNearBottom = scrollTop + clientHeight >= scrollHeight
        if (isNearBottom || (prevLogs && logs && prevLogs.length < logs.length)) {
            myDivRef.current.scroll({
                top: myDivRef.current.scrollHeight,
                behavior: "smooth",
            })
        }
    }, [logsExpanded, myDivRef.current, logs])

    return (
        <Accordion
            sx={{width: "100%"}}
            expanded={logsExpanded}
            onChange={() => setLogsExpanded(!logsExpanded)}
        >
            <AccordionSummary expandIcon={<ExpandMoreIcon />}>
                <WizardStyles.AccordionTitle>
                    {t("keysGeneration.ceremonyStep.logsHeader.title")}
                </WizardStyles.AccordionTitle>
            </AccordionSummary>
            <AccordionDetails ref={myDivRef}>
                {!!logs && logs.length > 0 ? (
                    <Paper sx={{width: "100%", margin: "4px 0"}}>
                        <TableContainer>
                            <Table sx={{maxHeight: 450}} aria-label="simple table">
                                <TableHead>
                                    <TableRow>
                                        <TableCell sx={{width: "40%"}}>
                                            {t("keysGeneration.ceremonyStep.logsHeader.date")}
                                        </TableCell>
                                        <TableCell align="left">
                                            {t("keysGeneration.ceremonyStep.logsHeader.entry")}
                                        </TableCell>
                                    </TableRow>
                                </TableHead>
                                <TableBody>
                                    {logs.map((log, index) => (
                                        <TableRow
                                            key={index}
                                            sx={{
                                                "&:last-child td, &:last-child th": {
                                                    border: 0,
                                                },
                                            }}
                                        >
                                            <TableCell component="th" scope="row">
                                                {log?.created_date &&
                                                    new Date(log.created_date).toLocaleString()}
                                            </TableCell>
                                            <TableCell align="left">{log?.log_text}</TableCell>
                                        </TableRow>
                                    )) ?? null}
                                </TableBody>
                            </Table>
                        </TableContainer>
                    </Paper>
                ) : (
                    <Box>
                        <Typography variant="body2">
                            {t("keysGeneration.ceremonyStep.emptyLogs")}
                        </Typography>
                    </Box>
                )}
            </AccordionDetails>
        </Accordion>
    )
}
