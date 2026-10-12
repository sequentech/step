// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useCallback, useContext, useEffect, useMemo, useState} from "react"
import Button from "@mui/material/Button"
import {BreadCrumbSteps, BreadCrumbStepsVariant, DropFile} from "@sequentech/ui-essentials"
import ChevronRightIcon from "@mui/icons-material/ChevronRight"
import ArrowBackIosIcon from "@mui/icons-material/ArrowBackIos"
import {useTranslation} from "react-i18next"
import ElectionHeader from "@/components/ElectionHeader"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"
import {styled} from "@mui/material/styles"
import {TallyElectionsList} from "./TallyElectionsList"
import {TallyTrusteesList} from "./TallyTrusteesList"
import {TallyStyles} from "@/components/styles/TallyStyles"
import {useGetList, useGetOne, useRecordContext} from "react-admin"
import {WizardStyles} from "@/components/styles/WizardStyles"
import {RESTORE_PRIVATE_KEY} from "@/queries/RestorePrivateKey"
import {
    KEY_SHARE_SIGNATURE_STATUS,
    type IKeyShareSignatureStatusQuery,
    type IKeyShareSignatureStatusVariables,
} from "@/queries/KeyShareSignatureStatus"
import {useMutation, useQuery} from "@apollo/client"
import {
    ETallyKeyRestoreEligibility,
    ITallyExecutionStatus,
    ITallyTrusteeStatus,
} from "@/types/ceremonies"
import {Alert, Box, Typography} from "@mui/material"
import {
    RestorePrivateKeyMutation,
    RestorePrivateKeyMutationVariables,
    RestorePrivateKeyOutcome,
    Sequent_Backend_Election,
    Sequent_Backend_Election_Event,
    Sequent_Backend_Tally_Session,
    Sequent_Backend_Tally_Session_Execution,
} from "@/gql/graphql"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {
    KeyShareCheck,
    useKeyShareSigning,
    type IKeyShareAnswer,
    type IKeyShareSubmission,
} from "@/components/keys-ceremony/useKeyShareSigning"
import {getTallyTrusteeStatus} from "@/services/tallyCeremonyParticipation"
import {getTallyKeyRestoreEligibility} from "./utils"

const WizardSteps = {
    Start: 0,
    Status: 1,
}

export const TallyCeremonyTrustees: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election_Event>()

    const {t} = useTranslation()
    const {tallyId, setTallyId} = useElectionEventTallyStore()
    const [tenantId] = useTenantStore()
    const authContext = useContext(AuthContext)

    const [hasContinuedToStatus, setHasContinuedToStatus] = useState<boolean>(false)
    const [selectedElections, setSelectedElections] = useState<string[]>([])
    const [selectedTrustees, setSelectedTrustees] = useState<boolean>(false)
    const [verified, setVerified] = useState<boolean>(false)
    const [alreadyRestored, setAlreadyRestored] = useState<boolean>(false)
    const [uploading, setUploading] = useState<boolean>(false)
    const [errors, setErrors] = useState<String | null>(null)
    const {globalSettings} = useContext(SettingsContext)
    const [isTallyCompleted, setIsTallyCompleted] = useState<boolean>(false)

    // The execution status decides whether the key upload step is offered, so
    // the session is polled while the ceremony is open. Otherwise a trustee
    // who is still waiting keeps the upload step after the tally moves past
    // key collection, and the backend rejects the key they upload there.
    const {data: tally, isPending: isTallyPending} = useGetOne<Sequent_Backend_Tally_Session>(
        "sequent_backend_tally_session",
        {
            id: tallyId,
        },
        {
            refetchInterval: isTallyCompleted
                ? undefined
                : globalSettings.QUERY_FAST_POLL_INTERVAL_MS,
            refetchOnWindowFocus: false,
            refetchOnReconnect: false,
            refetchOnMount: false,
        }
    )

    // TODO: fix the "perPage 9999"
    const {data: elections} = useGetList<Sequent_Backend_Election>("sequent_backend_election", {
        pagination: {page: 1, perPage: 9999},
        filter: {
            election_event_id: record?.id,
            tenant_id: tenantId,
            id: tallyId
                ? {
                      format: "hasura-raw-query",
                      value: {
                          _in: tally?.election_ids ?? [],
                      },
                  }
                : undefined,
        },
    })

    const {data: tallySessionExecutions, isPending: areExecutionsPending} =
        useGetList<Sequent_Backend_Tally_Session_Execution>(
            "sequent_backend_tally_session_execution",
            {
                pagination: {page: 1, perPage: 1},
                sort: {field: "created_at", order: "DESC"},
                filter: {
                    tally_session_id: tallyId,
                    tenant_id: tenantId,
                },
            },
            {
                refetchInterval: isTallyCompleted
                    ? undefined
                    : globalSettings.QUERY_FAST_POLL_INTERVAL_MS,
                refetchOnWindowFocus: false,
                refetchOnReconnect: false,
                refetchOnMount: false,
            }
        )

    useEffect(() => {
        if (tally?.is_execution_completed && !isTallyCompleted) {
            setIsTallyCompleted(true)
        }
    }, [tally?.is_execution_completed, isTallyCompleted])

    // The tally session and its execution load separately, so the wizard waits
    // for both before choosing a step. Otherwise the key upload step would show
    // up while the participation of the trustee is still unknown. Without a
    // tally id both queries stay disabled, so there is nothing to wait for.
    const isCeremonyPending = !!tallyId && (isTallyPending || areExecutionsPending)

    const trusteeStatus = useMemo(
        () => getTallyTrusteeStatus(tallySessionExecutions?.[0], authContext?.trustee),
        [authContext?.trustee, tallySessionExecutions]
    )

    // A trustee who restored their key unsigned before the rule made trustees
    // sign contributes it again, signed.
    const restored = trusteeStatus === ITallyTrusteeStatus.KEY_RESTORED
    const {data: signatureStatus, refetch: refetchSignatureStatus} = useQuery<
        IKeyShareSignatureStatusQuery,
        IKeyShareSignatureStatusVariables
    >(KEY_SHARE_SIGNATURE_STATUS, {
        variables: {electionEventId: tally?.election_event_id ?? "", tallySessionId: tally?.id},
        skip:
            !restored ||
            !tally ||
            tally.execution_status === ITallyExecutionStatus.CANCELLED ||
            !!tally.is_execution_completed,
        fetchPolicy: "network-only",
    })
    const redoSigned =
        restored &&
        !!signatureStatus?.key_share_signature_status?.signature_needed &&
        !signatureStatus.key_share_signature_status.signed

    // A trustee contributing their key again, signed, uploads it like one still waiting.
    const keyRestoreEligibility = useMemo(
        () =>
            getTallyKeyRestoreEligibility(
                redoSigned ? ITallyTrusteeStatus.WAITING : trusteeStatus,
                tally?.execution_status
            ),
        [redoSigned, trusteeStatus, tally?.execution_status]
    )

    const page =
        keyRestoreEligibility === ETallyKeyRestoreEligibility.ALLOWED && !hasContinuedToStatus
            ? WizardSteps.Start
            : WizardSteps.Status

    const CancelButton = styled(Button)`
        background-color: ${({theme}) => theme.palette.white};
        color: ${({theme}) => theme.palette.brandColor};
        border-color: ${({theme}) => theme.palette.brandColor};
        padding: 0 4rem;

        &:hover {
            background-color: ${({theme}) => theme.palette.brandColor};
        }
    `

    const NextButton = styled(Button)`
        background-color: ${({theme}) => theme.palette.brandColor};
        color: ${({theme}) => theme.palette.white};
        border-color: ${({theme}) => theme.palette.brandColor};
        padding: 0 4rem;

        &:hover {
            background-color: ${({theme}) => theme.palette.white};
            color: ${({theme}) => theme.palette.brandColor};
        }
    `

    const [restorePrivateKeyMutation] = useMutation<
        RestorePrivateKeyMutation,
        RestorePrivateKeyMutationVariables
    >(RESTORE_PRIVATE_KEY)
    const submit = useCallback(
        async (submission: IKeyShareSubmission): Promise<IKeyShareAnswer> => {
            const {data, errors} = await restorePrivateKeyMutation({
                variables: {
                    electionEventId: tally?.election_event_id ?? "",
                    tallySessionId: tally?.id ?? "",
                    ...submission,
                },
            })
            if (errors) throw new Error(errors.toString())
            setAlreadyRestored(
                data?.restore_private_key?.outcome === RestorePrivateKeyOutcome.AlreadyRestored
            )
            return data?.restore_private_key ?? {is_valid: false}
        },
        [restorePrivateKeyMutation, tally?.election_event_id, tally?.id]
    )
    const keyShareSigning = useKeyShareSigning({
        submit,
        onRecorded: useCallback(() => {
            setVerified(true)
            // A redone contribution is now signed.
            if (redoSigned) refetchSignatureStatus().catch(() => undefined)
        }, [redoSigned, refetchSignatureStatus]),
        onFailed: useCallback(
            (error: string) => setErrors(t("signing.keyShare.failed", {error})),
            [t]
        ),
    })

    const uploadPrivateKey = async (files: FileList | null) => {
        setErrors(null)
        setVerified(false)
        setAlreadyRestored(false)
        setUploading(false)
        if (!files || files.length === 0) {
            setErrors(t("keysGeneration.checkStep.noFileSelected"))
            return
        }
        const firstFile = files[0]
        const readFileContent = (file: File) => {
            return new Promise<string>((resolve, reject) => {
                const fileReader = new FileReader()
                fileReader.onload = () => resolve(fileReader.result as string)
                fileReader.onerror = (error) => reject(error)
                // Read the file as a data URL (base64 encoded string)
                fileReader.readAsText(file)
            })
        }
        try {
            const fileContent = await readFileContent(firstFile)
            if (fileContent == null) {
                setErrors(t("keysGeneration.checkStep.noFileSelected"))
                return
            }
            setUploading(true)
            const checked = await keyShareSigning.check(fileContent)
            setUploading(false)
            if (checked === KeyShareCheck.Invalid) {
                setErrors(t("keysGeneration.checkStep.errorUploading"))
            } else if (checked === KeyShareCheck.Verified) {
                setVerified(true)
            }
        } catch (exception) {
            setUploading(false)
            setErrors(t("keysGeneration.checkStep.errorUploading", {error: String(exception)}))
        }
    }

    return (
        <TallyStyles.WizardContainer>
            <TallyStyles.ContentWrapper>
                <WizardStyles.WizardWrapper>
                    {isCeremonyPending ? (
                        <WizardStyles.StatusBox
                            sx={{display: "flex", justifyContent: "center", padding: "32px"}}
                        >
                            <WizardStyles.DownloadProgress />
                        </WizardStyles.StatusBox>
                    ) : (
                        <>
                            <TallyStyles.StyledHeader>
                                <BreadCrumbSteps
                                    labels={[
                                        "tally.breadcrumbSteps.start",
                                        "tally.breadcrumbSteps.finish",
                                    ]}
                                    selected={page}
                                    variant={BreadCrumbStepsVariant.Circle}
                                    colorPreviousSteps={true}
                                />
                            </TallyStyles.StyledHeader>

                            {page === WizardSteps.Start && (
                                <>
                                    <ElectionHeader
                                        title={"tally.ceremonyTitle"}
                                        subtitle={"tally.ceremonySubTitle"}
                                    />

                                    <TallyElectionsList
                                        elections={elections}
                                        electionEventPresentation={record?.presentation}
                                        electionEventId={record?.id}
                                        disabled={true}
                                        update={(elections) => setSelectedElections(elections)}
                                        keysCeremonyId={tally?.keys_ceremony_id ?? null}
                                    />

                                    <Box>
                                        <ElectionHeader
                                            title={"tally.trusteeTitle"}
                                            subtitle={"tally.trusteeSubTitle"}
                                        />
                                        {redoSigned && !verified ? (
                                            <Typography variant="body1">
                                                {t("signing.keyShare.redo")}
                                            </Typography>
                                        ) : null}

                                        {!uploading && !verified ? (
                                            <DropFile handleFiles={uploadPrivateKey} />
                                        ) : null}

                                        <WizardStyles.StatusBox>
                                            {uploading ? <WizardStyles.DownloadProgress /> : null}
                                            {errors ? (
                                                <WizardStyles.ErrorMessage variant="body2">
                                                    {errors}
                                                </WizardStyles.ErrorMessage>
                                            ) : null}
                                            {keyShareSigning.waiting && !errors ? (
                                                <Typography variant="body1">
                                                    {t("signing.keyShare.signing")}
                                                </Typography>
                                            ) : null}
                                            {verified && !alreadyRestored && (
                                                <WizardStyles.SucessMessage variant="body1">
                                                    {t("keysGeneration.checkStep.verified")}
                                                </WizardStyles.SucessMessage>
                                            )}
                                            {alreadyRestored && (
                                                <Alert severity="info">
                                                    {t("keysGeneration.checkStep.alreadyRestored")}
                                                </Alert>
                                            )}
                                        </WizardStyles.StatusBox>
                                    </Box>
                                </>
                            )}

                            {page === WizardSteps.Status && (
                                <>
                                    <ElectionHeader
                                        title={"tally.ceremonyTitle"}
                                        subtitle={"tally.ceremonySubTitle"}
                                    />

                                    <TallyElectionsList
                                        elections={elections}
                                        electionEventPresentation={record?.presentation}
                                        electionEventId={record?.id}
                                        disabled={true}
                                        update={(elections) => setSelectedElections(elections)}
                                        keysCeremonyId={tally?.keys_ceremony_id ?? null}
                                    />

                                    <TallyTrusteesList
                                        tally={tally}
                                        update={(trustees) => setSelectedTrustees(trustees)}
                                        tallySessionExecutions={tallySessionExecutions}
                                    />
                                </>
                            )}
                        </>
                    )}
                </WizardStyles.WizardWrapper>
            </TallyStyles.ContentWrapper>

            <TallyStyles.FooterContainer>
                <TallyStyles.StyledFooter>
                    <CancelButton className="list-actions" onClick={() => setTallyId(null)}>
                        <ArrowBackIosIcon />
                        {t("tally.common.cancel")}
                    </CancelButton>
                    {page < WizardSteps.Status && (
                        <NextButton
                            color="primary"
                            onClick={() => setHasContinuedToStatus(true)}
                            disabled={!verified}
                        >
                            <>
                                {t("tally.common.next")}
                                <ChevronRightIcon />
                            </>
                        </NextButton>
                    )}
                </TallyStyles.StyledFooter>
            </TallyStyles.FooterContainer>
        </TallyStyles.WizardContainer>
    )
}
