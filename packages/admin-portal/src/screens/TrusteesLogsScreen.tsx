// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useMemo, useState, useCallback, useRef} from "react"
import {useQuery} from "@apollo/client"
import {useGetOne} from "react-admin"
import {
    Box,
    Button,
    CircularProgress,
    MenuItem,
    Select,
    TextField,
    Typography,
} from "@mui/material"
import {styled} from "@mui/material/styles"
import {useTranslation} from "react-i18next"

import {LIST_KEYS_CEREMONY} from "@/queries/ListKeysCeremonies"
import {GET_TRUSTEE_MESSAGES} from "@/queries/GetTrusteeMessages"
import {POST_TRUSTEE_MESSAGES} from "@/queries/PostTrusteeMessages"
import {ElectionEventContext} from "@/providers/ElectionEventContextProvider"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"
import Logs from "@/components/Logs"
import {IExecutionStatus} from "@/services/KeyCeremony"
import {ApolloContext} from "@/providers/ApolloContextProvider"
import {TrusteeWasmService, IBraidWasmModule} from "@/services/TrusteeWasmService"
import * as BraidWasm from "braid-wasm"

const Container = styled(Box)`
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 16px;
`

const Header = styled(Box)`
    display: flex;
    flex-direction: row;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
`

const Title = styled(Typography)`
    font-size: 1.25rem;
    font-weight: 600;
`

const SelectWrapper = styled(Box)`
    min-width: 260px;
`

const RunnerContainer = styled(Box)`
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin-top: 8px;
`

const RunnerRow = styled(Box)`
    display: flex;
    flex-direction: row;
    gap: 12px;
    align-items: center;
    flex-wrap: wrap;
`

const RunnerSectionTitle = styled(Typography)`
    font-weight: 500;
`

const KEY_COMMITMENT_ITERATIONS = 600000

const BOARD_NAME_DEFAULT = "dkg"

export const TrusteesLogsScreen: React.FC = () => {
    const {t} = useTranslation()
    const {ElectionEventId} = useContext(ElectionEventContext)
    const authContext = useContext(AuthContext)
    const {apolloClient} = useContext(ApolloContext)

    const [selectedCeremonyId, setSelectedCeremonyId] = useState<string | "">("")

    // Local state for protocol runner
    const [boardName, setBoardName] = useState<string>(BOARD_NAME_DEFAULT)
    const [passphrase, setPassphrase] = useState<string>("")
    const [isRunning, setIsRunning] = useState(false)
    const [runError, setRunError] = useState<string | null>(null)
    const [runStatus, setRunStatus] = useState<string | null>(null)
    const fileInputRef = useRef<HTMLInputElement | null>(null)

    const wasmService = useMemo(
        () => new TrusteeWasmService(BraidWasm as unknown as IBraidWasmModule, apolloClient),
        [apolloClient],
    )

    const {data, loading: listLoading, error: listError} = useQuery(LIST_KEYS_CEREMONY, {
        skip: !ElectionEventId,
        variables: {electionEventId: ElectionEventId},
        context: {
            headers: {
                "x-hasura-role": authContext.hasRole(IPermissions.TRUSTEE_CEREMONY)
                    ? IPermissions.TRUSTEE_CEREMONY
                    : IPermissions.ADMIN_CEREMONY,
            },
        },
    })

    const ceremonies = useMemo(() => {
        return (
            data?.list_keys_ceremony?.items?.filter((k: any) => !!k) ?? []
        ) as Array<any>
    }, [data])

    const effectiveCeremonyId = useMemo(() => {
        if (selectedCeremonyId) return selectedCeremonyId
        if (ceremonies.length === 0) return ""
        // Default to the first ceremony in the list
        return ceremonies[0].id as string
    }, [selectedCeremonyId, ceremonies])

    const {
        data: ceremonyRecord,
        isLoading: ceremonyLoading,
        error: ceremonyError,
    } = useGetOne<any>(
        "sequent_backend_keys_ceremony",
        {
            id: effectiveCeremonyId,
        },
        {
            enabled: !!effectiveCeremonyId,
        },
    )

    const status: IExecutionStatus | undefined = ceremonyRecord?.status

    const runProtocol = useCallback(async () => {
        if (!ElectionEventId) {
            setRunError(t("electionEventScreen.keys.selectElectionEventFirst"))
            return
        }
        if (!authContext.trustee) {
            setRunError(t("keysGeneration.ceremonyStep.missingTrusteeIdentifier"))
            return
        }
        if (!effectiveCeremonyId) {
            setRunError(t("electionEventScreen.keys.noCeremoniesForElectionEvent"))
            return
        }
        if (!boardName) {
            setRunError(t("keysGeneration.ceremonyStep.missingBoardName"))
            return
        }

        const file = fileInputRef.current?.files?.[0]
        if (!file) {
            setRunError(t("keysGeneration.checkStep.missingKeyFile"))
            return
        }
        if (!passphrase) {
            setRunError(t("keysGeneration.checkStep.missingPassphrase"))
            return
        }

        setIsRunning(true)
        setRunError(null)
        setRunStatus(t("keysGeneration.ceremonyStep.startingProtocol"))

        const readFileAsText = (f: File): Promise<string> =>
            new Promise((resolve, reject) => {
                const reader = new FileReader()
                reader.onload = () => resolve(reader.result as string)
                reader.onerror = () => reject(reader.error)
                reader.readAsText(f)
            })

        try {
            const text = await readFileAsText(file)
            let parsed: any
            try {
                parsed = JSON.parse(text)
            } catch (e) {
                setRunError(t("keysGeneration.checkStep.invalidJsonKeyFile"))
                return
            }

            const imported = await wasmService.importKeyFile(
                parsed,
                passphrase,
                ElectionEventId,
                authContext.trustee,
                KEY_COMMITMENT_ITERATIONS,
            )

            if (!imported.isValid) {
                setRunError(t("keysGeneration.checkStep.commitmentMismatch"))
                return
            }

            const signingKeyId = imported.key_id
            const trusteeId = wasmService.initTrustee(
                boardName,
                authContext.trustee,
                signingKeyId,
            )

            let lastId = -1

            // Basic pull-process-push loop; relies on backend implementing
            // GET_TRUSTEE_MESSAGES and POST_TRUSTEE_MESSAGES.
            // We stop when no new messages arrive and no outgoing messages are produced.
            // NOTE: This is a synchronous loop; consider adding limits if needed.
            // eslint-disable-next-line no-constant-condition
            while (true) {
                const {data: msgData} = await apolloClient.query({
                    query: GET_TRUSTEE_MESSAGES,
                    variables: {
                        electionEventId: ElectionEventId,
                        boardName,
                        sinceId: lastId,
                    },
                    fetchPolicy: "network-only",
                })

                const payload = msgData?.get_trustee_messages
                const messagesB64: string | undefined = payload?.messages_b64
                const nextLastId: number | undefined = payload?.last_id

                if (!messagesB64 || messagesB64.length === 0) {
                    break
                }

                const incoming = Uint8Array.from(Buffer.from(messagesB64, "base64"))
                if (incoming.byteLength === 0) {
                    break
                }

                const stepResult = wasmService.runTrusteeStep(trusteeId, incoming)

                if (stepResult.outgoing_messages_b64.length > 0) {
                    await apolloClient.mutate({
                        mutation: POST_TRUSTEE_MESSAGES,
                        variables: {
                            electionEventId: ElectionEventId,
                            boardName,
                            messagesB64: stepResult.outgoing_messages_b64,
                        },
                    })
                }

                setRunStatus(
                    t("keysGeneration.ceremonyStep.protocolProgress", {
                        added: stepResult.added_messages,
                        outgoing: stepResult.outgoing_messages_b64.length,
                    }),
                )

                if (
                    stepResult.added_messages === 0 &&
                    stepResult.outgoing_messages_b64.length === 0
                ) {
                    break
                }

                if (typeof nextLastId === "number") {
                    lastId = nextLastId
                } else {
                    break
                }
            }

            setRunStatus(t("keysGeneration.ceremonyStep.protocolCompleted"))
        } catch (e: any) {
            setRunError(e?.message ?? String(e))
        } finally {
            setIsRunning(false)
        }
    }, [
        ElectionEventId,
        authContext.trustee,
        effectiveCeremonyId,
        boardName,
        passphrase,
        apolloClient,
        t,
        wasmService,
    ])

    if (!ElectionEventId) {
        return (
            <Container>
                <Title>{t("keysGeneration.ceremonyStep.logsHeader.logs")}</Title>
                <Typography>{t("electionEventScreen.keys.selectElectionEventFirst")}</Typography>
            </Container>
        )
    }

    if (listLoading) {
        return (
            <Container>
                <CircularProgress />
            </Container>
        )
    }

    if (listError) {
        return (
            <Container>
                <Typography color="error">
                    {t("keysGeneration.ceremonyStep.errorLoadingLogs")}: {listError.message}
                </Typography>
            </Container>
        )
    }

    if (ceremonies.length === 0) {
        return (
            <Container>
                <Title>{t("keysGeneration.ceremonyStep.logsHeader.logs")}</Title>
                <Typography>
                    {t("electionEventScreen.keys.noCeremoniesForElectionEvent")}
                </Typography>
            </Container>
        )
    }

    return (
        <Container>
            <Header>
                <Title>{t("keysGeneration.ceremonyStep.logsHeader.logs")}</Title>
                <SelectWrapper>
                    <Select
                        fullWidth
                        size="small"
                        value={effectiveCeremonyId}
                        onChange={(e) => setSelectedCeremonyId(e.target.value as string)}
                    >
                        {ceremonies.map((c) => (
                            <MenuItem key={c.id} value={c.id}>
                                {c.name || c.id}
                            </MenuItem>
                        ))}
                    </Select>
                </SelectWrapper>
            </Header>

            {ceremonyLoading && (
                <Box mt={2}>
                    <CircularProgress />
                </Box>
            )}

            {ceremonyError && (
                <Typography mt={2} color="error">
                    {t("keysGeneration.ceremonyStep.errorLoadingLogs")}: {ceremonyError.message}
                </Typography>
            )}

            {status && <Logs logs={status.logs} />}

            {/* Protocol runner UI */}
            <RunnerContainer>
                <RunnerSectionTitle>
                    {t("keysGeneration.ceremonyStep.runTrusteeProtocolTitle")}
                </RunnerSectionTitle>
                <RunnerRow>
                    <TextField
                        type="password"
                        size="small"
                        label={t("keysGeneration.checkStep.passphraseLabel")}
                        value={passphrase}
                        onChange={(e) => setPassphrase(e.target.value)}
                    />
                    <TextField
                        size="small"
                        label={t("keysGeneration.ceremonyStep.boardNameLabel")}
                        value={boardName}
                        onChange={(e) => setBoardName(e.target.value)}
                    />
                    <Button
                        variant="contained"
                        component="label"
                        disabled={isRunning}
                    >
                        {t("keysGeneration.checkStep.selectKeyFile")}
                        <input
                            ref={fileInputRef}
                            type="file"
                            accept="application/json"
                            hidden
                        />
                    </Button>
                    <Button
                        variant="contained"
                        color="primary"
                        disabled={isRunning}
                        onClick={runProtocol}
                    >
                        {isRunning
                            ? t("keysGeneration.ceremonyStep.runningProtocol")
                            : t("keysGeneration.ceremonyStep.runProtocolButton")}
                    </Button>
                </RunnerRow>

                {runStatus && (
                    <Typography variant="body2" color="textSecondary">
                        {runStatus}
                    </Typography>
                )}

                {runError && (
                    <Typography variant="body2" color="error">
                        {runError}
                    </Typography>
                )}
            </RunnerContainer>
        </Container>
    )
}

export default TrusteesLogsScreen
