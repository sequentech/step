// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The receipt screen (step-integration.md, section 2.7), beside the Benaloh
// audit: it checks a Vitrine receipt chain, from its file, from the link the
// voting portal's "Check independently" opens, or from its reference, against
// a trust policy the user pinned on this device after comparing it with the
// official election notice. It needs no login. Everything it is given is
// data: it is shown as text, and checked with @vitrine/verify; the receipt,
// the proof steps and their wording are the UI kit's.

import React, {useEffect, useMemo, useRef, useState} from "react"
import {useLocation, useNavigate} from "react-router-dom"
import Alert from "@mui/material/Alert"
import Box from "@mui/material/Box"
import Button from "@mui/material/Button"
import Typography from "@mui/material/Typography"
import {styled} from "@mui/material/styles"
import {useTranslation} from "react-i18next"
import {ActionsContainer, PageLimit, StyledButton, theme} from "@sequentech/ui-essentials"
import type {Expect, PolicyHistory, Reason} from "@vitrine/verify"
import {BallotForm} from "../components/Vitrine/BallotForm"
import {PolicyFacts} from "../components/Vitrine/PolicyFacts"
import {PolicyInput} from "../components/Vitrine/PolicyInput"
import {BallotLookedFor, ReceiptFile} from "../components/Vitrine/ReceiptCards"
import {ReceiptInput} from "../components/Vitrine/ReceiptInput"
import {ReceiptStatus} from "../components/Vitrine/ReceiptStatus"
import {useVitrineUi} from "../components/Vitrine/VitrineElements"
import {statusSteps} from "../services/vitrine/ballotStatus"
import {ECheckRound, expectedBallotId} from "../services/vitrine/check"
import {EPolicyFetchProblem, fetchPolicyText} from "../services/vitrine/http"
import {
    CHAIN_MEDIA_TYPE,
    EInputProblem,
    EReceiptInput,
    hasReceiptFragment,
    parseReceiptInput,
    TPolicySource,
    TReceiptInput,
} from "../services/vitrine/input"
import {vitrineLanguage} from "../services/vitrine/language"
import {
    EPinMatch,
    EPolicyRefusal,
    examinePolicy,
    findPinned,
    IPolicyView,
    pinnedPolicies,
    pinPolicy,
    policyFitsReceipt,
    TPinMatch,
    unpinPolicy,
} from "../services/vitrine/policies"
import {IVitrineSdk, loadVitrineSdk} from "../services/vitrine/sdk"
import {
    ICheckProblem,
    IReceiptCheckDeps,
    IReceiptCheckInput,
    useReceiptCheck,
} from "./hooks/useReceiptCheck"
import {useStepTimes} from "./hooks/useStepTimes"

const Section = styled(Box)`
    display: flex;
    flex-direction: column;
    gap: 10px;
`

const SectionTitle = styled(Typography)`
    margin: 28px 0 0;
    font-size: 18px;
    line-height: 1.5;
    font-weight: 700;
`

const Rounds = styled(Typography)`
    margin-top: 16px;
    color: #475569;
`

const Muted = styled(Typography)`
    color: ${theme.palette.customGrey.main};
`

export interface IReceiptScreenDeps extends IReceiptCheckDeps {
    loadSdk: () => Promise<IVitrineSdk>
}

export interface IReceiptScreenProps {
    /** What the screen uses from the outside, for tests. */
    deps?: Partial<IReceiptScreenDeps>
}

enum ECandidate {
    LOADING = "loading",
    /** Valid, for this receipt, and waiting for the user's comparison. */
    OFFERED = "offered",
    /** The user said its words don't match the notice. */
    REJECTED = "rejected",
    DRAFT = "draft",
    INVALID = "invalid",
    /** Valid, but the receipt is for another policy. */
    OTHER_POLICY = "other-policy",
    FETCH_FAILED = "fetch-failed",
}

type TCandidate =
    | {state: ECandidate.LOADING}
    | {state: ECandidate.OFFERED; view: IPolicyView}
    | {state: ECandidate.REJECTED}
    | {state: ECandidate.DRAFT; draftWords: string[]}
    | {state: ECandidate.INVALID; reason: Reason}
    | {state: ECandidate.OTHER_POLICY}
    | {state: ECandidate.FETCH_FAILED; problem: EPolicyFetchProblem; status?: number}

interface ISdkState {
    sdk?: IVitrineSdk
    failed: boolean
}

/** Saves the receipt chain as a file. False when the browser can't. */
const saveChain = (sdk: IVitrineSdk, chain: string): boolean => {
    if (typeof URL.createObjectURL !== "function") return false
    let name = "receipt"
    try {
        name = sdk.verify.electionOf(sdk.pure.parseChain(chain)) ?? name
    } catch {
        // Keep the generic name.
    }
    const url = URL.createObjectURL(new Blob([chain], {type: CHAIN_MEDIA_TYPE}))
    const link = document.createElement("a")
    link.href = url
    link.download = sdk.verify.chainFileName(name)
    document.body.appendChild(link)
    link.click()
    link.remove()
    setTimeout(() => URL.revokeObjectURL(url), 1000)
    return true
}

export const ReceiptScreen: React.FC<IReceiptScreenProps> = ({deps = {}}) => {
    const {t, i18n} = useTranslation()
    const lang = vitrineLanguage(i18n.language)
    const location = useLocation()
    const navigate = useNavigate()
    const {ui, failed: uiFailed} = useVitrineUi()
    const depsRef = useRef(deps)
    depsRef.current = deps

    const [{sdk, failed: sdkFailed}, setSdkState] = useState<ISdkState>({failed: false})
    const [pinned, setPinned] = useState<IPolicyView[]>([])
    const [receipt, setReceipt] = useState<TReceiptInput | undefined>(undefined)
    const [receiptName, setReceiptName] = useState<string | undefined>(undefined)
    const [receiptProblem, setReceiptProblem] = useState<EInputProblem | undefined>(undefined)
    const [candidate, setCandidate] = useState<TCandidate | undefined>(undefined)
    const [formExpect, setFormExpect] = useState<Expect | undefined>(undefined)
    const [evidenceSaved, setEvidenceSaved] = useState(false)
    const candidateRun = useRef(0)
    const histories = useRef(new Map<string, PolicyHistory>())

    useEffect(() => {
        let active = true
        const load = depsRef.current.loadSdk ?? loadVitrineSdk
        load().then(
            (loaded) => {
                if (!active) return
                setSdkState({sdk: loaded, failed: false})
                setPinned(pinnedPolicies(loaded))
            },
            () => {
                if (active) setSdkState({failed: true})
            }
        )
        return () => {
            active = false
        }
    }, [])

    const openReceipt = (raw: string, fileName?: string) => {
        if (!sdk) return
        const parsed = parseReceiptInput(sdk, raw, window.location.href)
        if (!parsed.ok) {
            setReceiptProblem(parsed.problem)
            return
        }
        candidateRun.current++
        setReceipt(parsed.value)
        setReceiptName(fileName)
        setReceiptProblem(undefined)
        setCandidate(undefined)
        setFormExpect(undefined)
        setEvidenceSaved(false)
    }

    // The link the voting portal opens, or a reference's URL.
    useEffect(() => {
        if (sdk && hasReceiptFragment(location.hash)) openReceipt(location.hash)
    }, [sdk, location.hash])

    const closeReceipt = () => {
        candidateRun.current++
        setReceipt(undefined)
        setReceiptName(undefined)
        setReceiptProblem(undefined)
        setCandidate(undefined)
        setFormExpect(undefined)
        setEvidenceSaved(false)
        if (location.hash) {
            navigate({pathname: location.pathname, search: location.search}, {replace: true})
        }
    }

    const match: TPinMatch | undefined = useMemo(
        () => (sdk && receipt ? findPinned(sdk, receipt, pinned) : undefined),
        [sdk, receipt, pinned]
    )
    const view = match?.match === EPinMatch.FOUND ? match.view : undefined

    const loadCandidate = async (source: TPolicySource) => {
        if (!sdk || !receipt) return
        const run = ++candidateRun.current
        setCandidate({state: ECandidate.LOADING})
        let text: string
        if ("url" in source) {
            const fetched = await fetchPolicyText(source.url, {fetch: depsRef.current.fetch})
            if (run !== candidateRun.current) return
            if (!fetched.ok) {
                setCandidate({
                    state: ECandidate.FETCH_FAILED,
                    problem: fetched.problem,
                    ...(fetched.status !== undefined ? {status: fetched.status} : {}),
                })
                return
            }
            text = fetched.text
        } else {
            text = source.text
        }
        const examined = examinePolicy(sdk, text)
        if (!examined.ok) {
            setCandidate(
                examined.refusal === EPolicyRefusal.DRAFT
                    ? {state: ECandidate.DRAFT, draftWords: examined.draftWords}
                    : {state: ECandidate.INVALID, reason: examined.reason}
            )
        } else if (!policyFitsReceipt(sdk, receipt, examined.view)) {
            setCandidate({state: ECandidate.OTHER_POLICY})
        } else {
            setCandidate({state: ECandidate.OFFERED, view: examined.view})
        }
    }

    // Where the portal's link says the policy is: offered, never pinned,
    // until the user has compared it with the notice.
    const linkPolicy =
        receipt?.kind === EReceiptInput.CHAIN && match?.match === EPinMatch.NONE
            ? receipt.policySource
            : undefined
    useEffect(() => {
        if (linkPolicy && !candidate) void loadCandidate(linkPolicy)
    }, [linkPolicy, candidate])

    const pin = (offered: IPolicyView) => {
        pinPolicy(offered)
        setPinned((list) =>
            list.some((v) => v.policy.id === offered.policy.id) ? list : [...list, offered]
        )
        setCandidate(undefined)
    }

    const unpin = (pinnedView: IPolicyView) => {
        if (sdk) unpinPolicy(sdk, pinnedView.policy.id)
        histories.current.delete(pinnedView.policy.id)
        setPinned((list) => list.filter((v) => v.policy.id !== pinnedView.policy.id))
        setCandidate(undefined)
        setFormExpect(undefined)
    }

    const linkExpect: Expect | undefined = useMemo(() => {
        if (!receipt) return undefined
        if (receipt.kind === EReceiptInput.REFERENCE) return {entry: receipt.reference.entry}
        return receipt.entry ? {entry: receipt.entry} : undefined
    }, [receipt])
    const expect = linkExpect ?? formExpect

    const history = useMemo(() => {
        if (!sdk || !view) return undefined
        const id = view.policy.id
        const known = histories.current.get(id)
        if (known) return known
        const created = new sdk.verify.PolicyHistory(view.policy)
        histories.current.set(id, created)
        return created
    }, [sdk, view])

    const checkInput: IReceiptCheckInput | undefined = useMemo(
        () =>
            receipt && view && expect && history
                ? {
                      source:
                          receipt.kind === EReceiptInput.CHAIN
                              ? {chain: receipt.chain}
                              : {reference: receipt.reference},
                      view,
                      expect,
                      history,
                  }
                : undefined,
        [receipt, view, expect, history]
    )
    const check = useReceiptCheck(sdk, checkInput, {
        fetch: deps.fetch,
        now: deps.now,
        random: deps.random,
        schedule: deps.schedule,
    })

    const stepTimes = useStepTimes(
        statusSteps(check.outcome?.result),
        depsRef.current.now ?? (() => Math.floor(Date.now() / 1000)),
        checkInput
    )

    const onEvidence = () => {
        if (sdk && check.outcome) setEvidenceSaved(saveChain(sdk, check.outcome.chain))
    }

    const checkProblemText = (p: ICheckProblem): string => {
        let time = ""
        if (p.quietUntil !== undefined) {
            try {
                time = new Date(p.quietUntil * 1000).toLocaleString(lang)
            } catch {
                time = new Date(p.quietUntil * 1000).toISOString()
            }
        }
        return t(`receiptScreen.result.problems.${p.problem}`, {time})
    }

    const lookedFor = sdk && expect ? expectedBallotId(sdk, expect) : undefined

    const candidateBlock = (): React.ReactNode => {
        switch (candidate?.state) {
            case ECandidate.LOADING:
                return <Typography role="status">{t("receiptScreen.policy.loading")}</Typography>
            case ECandidate.OFFERED:
                return ui ? (
                    <Box data-testid="policy-offer">
                        <PolicyFacts
                            ui={ui}
                            lang={lang}
                            summary={candidate.view.summary}
                            compare
                            witnesses
                        />
                        <Box display="flex" gap="8px" flexWrap="wrap">
                            <Button onClick={() => pin(candidate.view)} data-testid="policy-pin">
                                {t("receiptScreen.policy.pin")}
                            </Button>
                            <Button
                                variant="secondary"
                                onClick={() => setCandidate({state: ECandidate.REJECTED})}
                                data-testid="policy-reject"
                            >
                                {t("receiptScreen.policy.reject")}
                            </Button>
                        </Box>
                    </Box>
                ) : null
            case ECandidate.REJECTED:
                return (
                    <Alert severity="info" data-testid="policy-rejected">
                        {t("receiptScreen.policy.rejected")}
                    </Alert>
                )
            case ECandidate.DRAFT:
                return (
                    <Alert severity="warning" data-testid="policy-draft">
                        {t("receiptScreen.policy.draft")}{" "}
                        <strong data-testid="policy-draft-words">
                            {candidate.draftWords.join(" ")}
                        </strong>
                    </Alert>
                )
            case ECandidate.INVALID:
                return (
                    <Alert severity="warning" data-testid="policy-invalid">
                        {t("receiptScreen.policy.invalid")}{" "}
                        {ui ? ui.wordsFor(lang).reason(candidate.reason) : null}
                    </Alert>
                )
            case ECandidate.OTHER_POLICY:
                return (
                    <Alert severity="warning" data-testid="policy-other">
                        {t("receiptScreen.policy.otherPolicy")}
                    </Alert>
                )
            case ECandidate.FETCH_FAILED:
                return (
                    <Alert severity="warning" data-testid="policy-fetch-failed">
                        {t(`receiptScreen.policy.fetchProblems.${candidate.problem}`, {
                            status: candidate.status ?? "",
                        })}
                    </Alert>
                )
            default:
                return null
        }
    }

    const policySection = (): React.ReactNode => {
        if (!receipt || !match) return null
        if (match.match === EPinMatch.AMBIGUOUS) {
            return (
                <Alert severity="warning" data-testid="policy-ambiguous">
                    {t("receiptScreen.policy.ambiguous")}
                </Alert>
            )
        }
        if (view) {
            return (
                <Box data-testid="policy-pinned">
                    <Muted variant="body2">{t("receiptScreen.policy.pinned")}</Muted>
                    {ui ? <PolicyFacts ui={ui} lang={lang} summary={view.summary} /> : null}
                    <Button
                        variant="secondary"
                        onClick={() => unpin(view)}
                        data-testid="policy-unpin"
                    >
                        {t("receiptScreen.policy.unpin")}
                    </Button>
                </Box>
            )
        }
        const waiting =
            candidate?.state === ECandidate.LOADING || candidate?.state === ECandidate.OFFERED
        const fromLink = linkPolicy && (!candidate || waiting)
        return (
            <>
                <Muted variant="body2">
                    {fromLink
                        ? t("receiptScreen.policy.fromLink")
                        : t("receiptScreen.policy.description")}
                </Muted>
                {candidateBlock()}
                {waiting ? null : <PolicyInput onSource={(source) => void loadCandidate(source)} />}
            </>
        )
    }

    const ballotSection = (): React.ReactNode => {
        if (!receipt || !view || !sdk) return null
        return (
            <Section data-testid="ballot-section">
                <SectionTitle variant="h3">{t("receiptScreen.ballot.title")}</SectionTitle>
                {lookedFor ? (
                    <BallotLookedFor
                        ballotId={lookedFor}
                        onChange={linkExpect ? undefined : () => setFormExpect(undefined)}
                    />
                ) : null}
                {expect ? null : (
                    <>
                        <Muted variant="body2">{t("receiptScreen.ballot.description")}</Muted>
                        <BallotForm sdk={sdk} policy={view.policy} onExpect={setFormExpect} />
                    </>
                )}
            </Section>
        )
    }

    const resultSection = (): React.ReactNode => {
        if (!checkInput || !view || !expect) return null
        const {outcome, problem, fullDue} = check
        return (
            <Section data-testid="receipt-result">
                <SectionTitle variant="h3">{t("receiptScreen.result.title")}</SectionTitle>
                {problem ? (
                    <Alert
                        severity="warning"
                        data-testid="check-problem"
                        action={
                            <Button variant="secondary" onClick={check.retry}>
                                {t("receiptScreen.result.retry")}
                            </Button>
                        }
                    >
                        {checkProblemText(problem)}
                    </Alert>
                ) : null}
                {outcome || !problem ? (
                    <ReceiptStatus
                        result={outcome?.result}
                        policy={view.policy}
                        fullDue={fullDue}
                        times={stepTimes}
                        lang={lang}
                        onEvidence={onEvidence}
                    />
                ) : null}
                {evidenceSaved ? (
                    <Alert severity="info" data-testid="evidence-saved">
                        {t("receiptScreen.result.evidenceSaved")}
                    </Alert>
                ) : null}
                {outcome ? (
                    <Rounds variant="body2" data-testid={`round-${outcome.round}`}>
                        {t(`receiptScreen.result.rounds.${outcome.round}`)}
                    </Rounds>
                ) : null}
            </Section>
        )
    }

    const actions = (): React.ReactNode => {
        if (!receipt) return null
        const {outcome, running, fullDue} = check
        return (
            <ActionsContainer sx={{margin: "10px 0 20px", paddingTop: "24px"}}>
                <StyledButton
                    variant="secondary"
                    onClick={closeReceipt}
                    data-testid="receipt-another"
                    sx={{margin: "auto 0", width: {xs: "100%", sm: "200px"}}}
                >
                    <span>{t("receiptScreen.receipt.another")}</span>
                </StyledButton>
                {outcome && outcome.round !== ECheckRound.OFFLINE ? (
                    <StyledButton
                        onClick={check.checkEverything}
                        disabled={!!running}
                        data-testid="check-everything"
                        sx={{margin: "auto 0", width: {xs: "100%", sm: "200px"}}}
                    >
                        <span>
                            {fullDue
                                ? t("receiptScreen.result.checkEverything")
                                : t("receiptScreen.result.checkAgain")}
                        </span>
                    </StyledButton>
                ) : null}
            </ActionsContainer>
        )
    }

    return (
        <PageLimit maxWidth="md" className="receipt-screen screen">
            <Typography variant="h5" component="h1" sx={{padding: "40px 0 8px"}}>
                {t("receiptScreen.title")}
            </Typography>
            <Muted variant="body2">{t("receiptScreen.description")}</Muted>
            {sdkFailed ? (
                <Alert severity="error" data-testid="receipt-load-failed">
                    {t("receiptScreen.loadFailed")}
                </Alert>
            ) : !sdk ? (
                <Typography role="status">{t("receiptScreen.loading")}</Typography>
            ) : (
                <>
                    <Section data-testid="receipt-section">
                        <SectionTitle variant="h3">{t("receiptScreen.receipt.title")}</SectionTitle>
                        {receiptProblem ? (
                            <Alert severity="warning" data-testid="receipt-problem">
                                {t(`receiptScreen.receipt.problems.${receiptProblem}`)}
                            </Alert>
                        ) : null}
                        {receipt ? (
                            <ReceiptFile
                                name={receiptName ?? t("receiptScreen.receipt.pasteLabel")}
                                detail={view?.summary.event}
                            />
                        ) : (
                            <>
                                <Muted variant="body2">
                                    {t("receiptScreen.receipt.description")}
                                </Muted>
                                <ReceiptInput onText={openReceipt} onProblem={setReceiptProblem} />
                            </>
                        )}
                    </Section>
                    {receipt ? (
                        <Section data-testid="policy-section">
                            <SectionTitle variant="h3">
                                {t("receiptScreen.policy.title")}
                            </SectionTitle>
                            {uiFailed ? (
                                <Alert severity="error">{t("receiptScreen.loadFailed")}</Alert>
                            ) : null}
                            {policySection()}
                        </Section>
                    ) : null}
                    {ballotSection()}
                    {resultSection()}
                    {actions()}
                </>
            )}
        </PageLimit>
    )
}
