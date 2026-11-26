// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect, useId, useRef, useState} from "react"
import {gql, useApolloClient, useMutation, useQuery} from "@apollo/client"
import {Alert, Box, Button, Drawer, TextField, Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import {CustomDropFile} from "@sequentech/ui-essentials"
import forge from "node-forge/lib/forge"
import "node-forge/lib/sha256"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"

type RosterMode = "not_enabled" | "site_pinned" | "portal_managed" | "unavailable"

export type RosterStatus = {
    mode: RosterMode
    generation: string
    active_sha256: string | null
    required_approvals: number
    can_manage: boolean
}

type RosterReview = {
    generation: string
    candidate_sha256: string
    approver_count: number
    final_signer_count: number
    rp_id: string
    origins: string[]
}

type Candidate = {
    body: string
    expectedSha: string
    matchedEnteredFingerprint: boolean
    review: RosterReview
}

export const ARCHITECT_SIGNER_ROSTER_STATUS = gql`
    query ArchitectSignerRosterStatus($tenantId: uuid!) {
        get_architect_signer_roster_status(tenant_id: $tenantId) {
            mode
            generation
            active_sha256
            required_approvals
            can_manage
        }
    }
`

const PREVIEW = gql`
    query PreviewArchitectSignerRoster($tenantId: uuid!, $roster: String!, $sha: String!) {
        preview_architect_signer_roster(
            tenant_id: $tenantId
            roster_json: $roster
            expected_sha256: $sha
        ) {
            generation
            candidate_sha256
            approver_count
            final_signer_count
            rp_id
            origins
        }
    }
`

const ACTIVATE = gql`
    mutation ActivateArchitectSignerRoster(
        $tenantId: uuid!
        $roster: String!
        $sha: String!
        $generation: String!
    ) {
        activate_architect_signer_roster(
            tenant_id: $tenantId
            roster_json: $roster
            expected_sha256: $sha
            generation: $generation
        ) {
            mode
            generation
            active_sha256
            required_approvals
            can_manage
        }
    }
`

const role = {headers: {"x-hasura-role": IPermissions.TENANT_WRITE}}
const fingerprint = /^[0-9a-f]{64}$/
const generation = /^(?:0|[1-9][0-9]*)$/
const MAX_ROSTER_BYTES = 262144

type Failure =
    | "file"
    | "computeHash"
    | "freshAuth"
    | "forbidden"
    | "conflict"
    | "preview"
    | "activate"
    | "login"

const hasError = (error: unknown, code: string): boolean => {
    if (!error || typeof error !== "object") return false
    const {graphQLErrors} = error as {graphQLErrors?: Array<{extensions?: {code?: unknown}}>}
    return graphQLErrors?.some((item) => item.extensions?.code === code) ?? false
}

const readRoster = async (file: File): Promise<{body: string; bytes: ArrayBuffer}> => {
    if (!file.size || file.size > MAX_ROSTER_BYTES) throw new Error("roster size")
    const bytes = await file.arrayBuffer()
    const data = new Uint8Array(bytes)
    if (data[0] === 0xef && data[1] === 0xbb && data[2] === 0xbf) throw new Error("roster BOM")
    // A signature pin names the actual source bytes. Never normalize quotes,
    // whitespace or line endings, or repair invalid UTF-8.
    const body = new TextDecoder("utf-8", {fatal: true, ignoreBOM: true}).decode(data)
    return {body, bytes}
}

export const readExactRoster = async (file: File): Promise<string> => (await readRoster(file)).body

export const computeRosterFingerprint = async (bytes: ArrayBuffer): Promise<string> => {
    const subtle = globalThis.crypto?.subtle
    if (subtle) {
        try {
            const digest = await subtle.digest("SHA-256", bytes)
            return Array.from(new Uint8Array(digest), (byte) =>
                byte.toString(16).padStart(2, "0")
            ).join("")
        } catch {
            // The existing offline LAN portal can be served without Web Crypto.
        }
    }
    const digest = forge.md.sha256.create()
    const raw = new Uint8Array(bytes)
    // Forge's default is a raw byte string. Do not decode/re-encode it as UTF-8.
    // Bounded chunks avoid spreading the full 256 KiB upload into one call.
    for (let offset = 0; offset < raw.length; offset += 0x4000) {
        digest.update(String.fromCharCode(...Array.from(raw.subarray(offset, offset + 0x4000))))
    }
    return digest.digest().toHex()
}

export const rosterStateAllowsImport = (status: RosterStatus | null): boolean =>
    status?.mode === "not_enabled" ||
    ((status?.mode === "site_pinned" || status?.mode === "portal_managed") &&
        typeof status.active_sha256 === "string" &&
        fingerprint.test(status.active_sha256) &&
        Number.isSafeInteger(status.required_approvals) &&
        status.required_approvals > 0 &&
        (status.mode !== "portal_managed" || status.generation !== "0"))

type RosterData = {get_architect_signer_roster_status: RosterStatus | null}

const parseStatus = (status: RosterStatus | null | undefined): RosterStatus | null => {
    if (
        !status ||
        !["not_enabled", "site_pinned", "portal_managed", "unavailable"].includes(status.mode) ||
        typeof status.generation !== "string" ||
        !generation.test(status.generation) ||
        (status.active_sha256 !== null &&
            (typeof status.active_sha256 !== "string" ||
                !fingerprint.test(status.active_sha256))) ||
        !Number.isSafeInteger(status.required_approvals) ||
        status.required_approvals < 0 ||
        typeof status.can_manage !== "boolean"
    )
        return null
    switch (status.mode) {
        case "not_enabled":
            return status.generation === "0" && status.active_sha256 === null && !status.can_manage
                ? status
                : null
        case "unavailable":
            return status.active_sha256 === null && !status.can_manage ? status : null
        case "site_pinned":
            return rosterStateAllowsImport(status) && !status.can_manage ? status : null
        case "portal_managed":
            return status.required_approvals > 0 &&
                ((status.generation === "0" && status.active_sha256 === null) ||
                    rosterStateAllowsImport(status))
                ? status
                : null
    }
}

export const useArchitectSignerRoster = (
    tenantId: string | null,
    actor: string,
    allowed: boolean
) => {
    const {data, error, loading, refetch} = useQuery<RosterData>(ARCHITECT_SIGNER_ROSTER_STATUS, {
        variables: {tenantId},
        // The variables are the same on an actor switch. Do not attach a
        // fresh observer to an old actor's in-flight response via dedup.
        context: {...role, queryDeduplication: false},
        skip: !tenantId || !actor || !allowed,
        fetchPolicy: "no-cache",
        pollInterval: 5000,
    })
    const read = async (): Promise<RosterStatus | null> => {
        if (!tenantId || !actor || !allowed) return null
        try {
            const response = await refetch({tenantId})
            return response.errors?.length
                ? null
                : parseStatus(response.data?.get_architect_signer_roster_status)
        } catch {
            return null
        }
    }
    return {
        status:
            error || !tenantId || !actor || !allowed
                ? null
                : parseStatus(data?.get_architect_signer_roster_status),
        loading,
        read,
    }
}

const RosterSetup: React.FC<{
    tenantId: string
    status: RosterStatus
    refresh: () => Promise<RosterStatus | null>
}> = ({tenantId, status, refresh}) => {
    const {t} = useTranslation()
    const {reauthenticate} = useContext(AuthContext)
    const client = useApolloClient()
    const dialogId = useId()
    const comparisonId = useId()
    const [open, setOpen] = useState(false)
    const [file, setFile] = useState<File | null>(null)
    const [sha, setSha] = useState("")
    const enteredSha = sha.trim().toLowerCase()
    const malformedSha = enteredSha !== "" && !fingerprint.test(enteredSha)
    const [candidate, setCandidate] = useState<Candidate | null>(null)
    const [failure, setFailure] = useState<Failure | null>(null)
    const [busy, setBusy] = useState(false)
    const [confirmedSha, setConfirmedSha] = useState<string | null>(null)
    const inFlight = useRef(false)
    const operation = useRef<"review" | "activate" | "login" | null>(null)
    const sequence = useRef(0)
    const fileInput = useRef<HTMLInputElement>(null)
    const reviewedHeading = useRef<HTMLHeadingElement>(null)
    const mounted = useRef(true)
    const current = useRef(status)
    current.current = status
    useEffect(() => {
        mounted.current = true
        return () => {
            mounted.current = false
        }
    }, [])
    const discardFiles = () => {
        if (fileInput.current) fileInput.current.value = ""
        setFile(null)
        setSha("")
        setCandidate(null)
    }
    const [activate] = useMutation<{activate_architect_signer_roster: RosterStatus | null}>(
        ACTIVATE,
        {
            context: role,
            fetchPolicy: "no-cache",
        }
    )
    const reset = () => {
        discardFiles()
        setFailure(null)
    }
    useEffect(() => {
        // Rotate an in-flight read-only review, without invalidating an activation that
        // legitimately causes the authority to advance.
        if (operation.current === "review") {
            sequence.current++
            operation.current = null
            inFlight.current = false
            setBusy(false)
        }
        discardFiles()
    }, [status.generation, status.active_sha256, status.required_approvals])
    useEffect(() => {
        if (candidate) reviewedHeading.current?.focus()
    }, [candidate])
    const selectFiles = (files: FileList) => {
        if (inFlight.current) return
        const next = files.length === 1 ? files[0] : null
        setCandidate(null)
        const valid =
            next && /\.json$/i.test(next.name) && next.size > 0 && next.size <= MAX_ROSTER_BYTES
        setFile(valid ? next : null)
        setFailure(valid ? null : "file")
    }
    const close = () => {
        if (inFlight.current && operation.current !== "review") return
        // Read-only verification can be abandoned; its result must never refill a reopened drawer.
        sequence.current++
        release()
        reset()
        setOpen(false)
    }
    const hold = (kind: NonNullable<typeof operation.current>) => {
        operation.current = kind
        inFlight.current = true
        setBusy(true)
        setFailure(null)
        return ++sequence.current
    }
    const release = () => {
        operation.current = null
        inFlight.current = false
        setBusy(false)
    }
    const review = async () => {
        if (!file || malformedSha || inFlight.current) return
        const request = hold("review")
        const isCurrent = () =>
            mounted.current &&
            sequence.current === request &&
            current.current.generation === status.generation &&
            current.current.active_sha256 === status.active_sha256 &&
            current.current.required_approvals === status.required_approvals
        setCandidate(null)
        try {
            let roster: Awaited<ReturnType<typeof readRoster>>
            try {
                roster = await readRoster(file)
            } catch {
                if (isCurrent()) setFailure("file")
                return
            }
            if (!isCurrent()) return
            const matchedEnteredFingerprint = enteredSha !== ""
            let reviewedSha = enteredSha
            if (!matchedEnteredFingerprint) {
                try {
                    reviewedSha = await computeRosterFingerprint(roster.bytes)
                    if (!fingerprint.test(reviewedSha)) throw new Error("invalid computed SHA-256")
                } catch {
                    if (isCurrent()) setFailure("computeHash")
                    return
                }
                if (!isCurrent()) return
            }
            const {data} = await client.query<{
                preview_architect_signer_roster: RosterReview | null
            }>({
                query: PREVIEW,
                variables: {tenantId, roster: roster.body, sha: reviewedSha},
                context: {
                    ...role,
                    fetchOptions: {method: "POST"},
                    queryDeduplication: false,
                },
                fetchPolicy: "no-cache",
            })
            if (!isCurrent()) return
            const result = data.preview_architect_signer_roster
            if (
                !result ||
                result.candidate_sha256 !== reviewedSha ||
                typeof result.generation !== "string" ||
                !generation.test(result.generation) ||
                !Number.isSafeInteger(result.approver_count) ||
                result.approver_count < current.current.required_approvals ||
                !Number.isSafeInteger(result.final_signer_count) ||
                result.final_signer_count < 1 ||
                typeof result.rp_id !== "string" ||
                !Array.isArray(result.origins) ||
                !result.origins.every((origin) => typeof origin === "string")
            ) {
                throw new Error("unconfirmed roster review")
            }
            if (result.generation !== current.current.generation) {
                discardFiles()
                setFailure("conflict")
                void refresh().catch(() => undefined)
                return
            }
            setCandidate({
                body: roster.body,
                expectedSha: reviewedSha,
                matchedEnteredFingerprint,
                review: result,
            })
        } catch (error) {
            if (isCurrent())
                setFailure(
                    hasError(error, "architect-roster-management-forbidden")
                        ? "forbidden"
                        : "preview"
                )
        } finally {
            if (isCurrent()) release()
        }
    }
    const alreadyActive = candidate !== null && candidate.expectedSha === status.active_sha256
    const confirm = async () => {
        if (
            !candidate ||
            inFlight.current ||
            candidate.review.generation !== current.current.generation ||
            candidate.review.approver_count < current.current.required_approvals ||
            candidate.expectedSha === current.current.active_sha256
        )
            return
        hold("activate")
        try {
            const {data} = await activate({
                variables: {
                    tenantId,
                    roster: candidate.body,
                    sha: candidate.expectedSha,
                    generation: candidate.review.generation,
                },
            })
            if (!mounted.current) return
            if (!parseStatus(data?.activate_architect_signer_roster))
                throw new Error("activation unconfirmed")
            const updated = await refresh()
            if (!mounted.current) return
            if (
                updated?.mode !== "portal_managed" ||
                !rosterStateAllowsImport(updated) ||
                updated.active_sha256 !== candidate.expectedSha ||
                updated.generation === candidate.review.generation
            ) {
                throw new Error("active roster not observed")
            }
            reset()
            setOpen(false)
            setConfirmedSha(candidate.expectedSha)
        } catch (error) {
            if (!mounted.current) return
            discardFiles()
            if (hasError(error, "architect-roster-fresh-auth-required")) {
                setFailure("freshAuth")
            } else if (hasError(error, "architect-roster-management-forbidden")) {
                setFailure("forbidden")
            } else if (
                hasError(error, "architect-roster-stale-generation") ||
                hasError(error, "architect-roster-package-busy")
            ) {
                setFailure("conflict")
                void refresh().catch(() => undefined)
            } else {
                setFailure("activate")
            }
        } finally {
            release()
        }
    }
    const signInAgain = async () => {
        if (!reauthenticate || inFlight.current) return
        // Leave the roster out of navigation, storage, and any replay after login.
        reset()
        hold("login")
        try {
            await reauthenticate()
        } catch {
            setFailure("login")
        } finally {
            release()
        }
    }

    return (
        <>
            <Button
                variant="outlined"
                onClick={() => {
                    setConfirmedSha(null)
                    setOpen(true)
                }}
            >
                {t("configurationPackage.roster.manage")}
            </Button>
            {confirmedSha !== null && confirmedSha === status.active_sha256 && (
                <Alert severity="success" sx={{mt: 1}}>
                    {t("configurationPackage.roster.activeConfirmed")}
                </Alert>
            )}
            <Drawer
                anchor="right"
                open={open}
                onClose={close}
                PaperProps={{"role": "dialog", "aria-modal": true, "aria-labelledby": dialogId}}
                sx={{"& .MuiDrawer-paper": {width: {xs: "100%", sm: "min(640px, 100%)"}}}}
            >
                <Box sx={{p: 2}} data-testid="signer-roster-drawer">
                    <Typography component="h2" variant="h5" id={dialogId} sx={{mb: 2}}>
                        {t("configurationPackage.roster.manage")}
                    </Typography>
                    {/* Choose file, Verify, Activate: where this drawer is, never a gate. */}
                    <Box
                        component="ol"
                        aria-label={String(t("configurationPackage.roster.steps.label"))}
                        data-testid="signer-roster-steps"
                        sx={{
                            display: "flex",
                            flexWrap: "wrap",
                            gap: 2,
                            listStyle: "none",
                            m: 0,
                            mb: 2,
                            p: 0,
                        }}
                    >
                        {(["choose", "verify", "activate"] as const).map((step, index) => {
                            const current = candidate ? 1 : 0
                            return (
                                <Box
                                    component="li"
                                    key={step}
                                    aria-current={index === current ? "step" : undefined}
                                    sx={{
                                        typography: "body2",
                                        fontWeight: index === current ? 500 : 400,
                                        color:
                                            index === current ? "text.primary" : "text.secondary",
                                    }}
                                >
                                    {index < current ? "✓ " : `${index + 1}. `}
                                    {t(`configurationPackage.roster.steps.${step}`)}
                                </Box>
                            )
                        })}
                    </Box>
                    <Typography variant="body2" paragraph>
                        {t("configurationPackage.roster.independent")}
                    </Typography>
                    <CustomDropFile
                        accept=".json,application/json"
                        disabled={busy}
                        handleFiles={selectFiles}
                        ref={fileInput}
                        inputLabel={t("configurationPackage.roster.file")}
                        browseLabel={t("configurationPackage.roster.browse")}
                        showFileName={false}
                        errorMessage={t("configurationPackage.roster.error.file")}
                    >
                        <Box sx={{p: 2}}>
                            <Typography sx={{fontWeight: "bold"}}>
                                {t("configurationPackage.roster.drop")}
                            </Typography>
                            <Typography component="span" sx={{textDecoration: "underline"}}>
                                {t("configurationPackage.roster.browse")}
                            </Typography>
                            <Typography variant="body2">
                                {t("configurationPackage.roster.file")}
                            </Typography>
                        </Box>
                    </CustomDropFile>
                    <Typography variant="body2" aria-live="polite" sx={{overflowWrap: "anywhere"}}>
                        {file?.name}
                    </Typography>
                    <TextField
                        label={t("configurationPackage.roster.expected")}
                        value={sha}
                        inputProps={{autoCapitalize: "none", spellCheck: false}}
                        helperText={t("configurationPackage.roster.hashHelp")}
                        error={malformedSha}
                        fullWidth
                        disabled={busy}
                        onChange={(event) => {
                            setSha(event.target.value)
                            setCandidate(null)
                            setFailure(null)
                        }}
                        sx={{mt: 2}}
                    />
                    {candidate && (
                        <Box sx={{mt: 2, overflowWrap: "anywhere"}}>
                            <Typography
                                component="h3"
                                variant="subtitle1"
                                tabIndex={-1}
                                ref={reviewedHeading}
                                aria-describedby={comparisonId}
                            >
                                {t("configurationPackage.roster.candidate", {
                                    hash: candidate.review.candidate_sha256,
                                })}
                            </Typography>
                            <Alert
                                id={comparisonId}
                                severity={candidate.matchedEnteredFingerprint ? "info" : "warning"}
                            >
                                {t(
                                    candidate.matchedEnteredFingerprint
                                        ? "configurationPackage.roster.fingerprintMatched"
                                        : "configurationPackage.roster.fingerprintComputed"
                                )}
                            </Alert>
                            <Typography variant="body2">
                                {t("configurationPackage.roster.signers", {
                                    approvers: candidate.review.approver_count,
                                    signers: candidate.review.final_signer_count,
                                })}
                            </Typography>
                            <Typography variant="body2">
                                {t("configurationPackage.roster.origin", {
                                    rp: candidate.review.rp_id,
                                    origins: candidate.review.origins.join(", "),
                                })}
                            </Typography>
                            {alreadyActive ? (
                                <Alert severity="info" sx={{mt: 1}}>
                                    {t("configurationPackage.roster.alreadyActive")}
                                </Alert>
                            ) : (
                                <Typography variant="body2" paragraph sx={{mt: 1}}>
                                    {t("configurationPackage.roster.confirmEffect")}
                                </Typography>
                            )}
                        </Box>
                    )}
                    {failure && (
                        <Alert severity="error" sx={{mt: 2}}>
                            {t(`configurationPackage.roster.error.${failure}`)}
                        </Alert>
                    )}
                </Box>
                <Box
                    sx={{
                        p: 2,
                        pt: 0,
                        display: "flex",
                        justifyContent: "flex-end",
                        gap: 1,
                        flexWrap: "wrap",
                    }}
                >
                    <Button onClick={close} disabled={busy && operation.current !== "review"}>
                        {t("configurationPackage.roster.cancel")}
                    </Button>
                    {failure === "freshAuth" && reauthenticate && (
                        <Button onClick={() => void signInAgain()} disabled={busy}>
                            {t("configurationPackage.roster.signIn")}
                        </Button>
                    )}
                    {candidate ? (
                        <Button
                            variant="contained"
                            color="warning"
                            disabled={
                                busy ||
                                alreadyActive ||
                                candidate.review.approver_count < status.required_approvals ||
                                candidate.review.generation !== status.generation
                            }
                            onClick={() => void confirm()}
                        >
                            {t("configurationPackage.roster.confirm")}
                        </Button>
                    ) : (
                        <Button
                            variant="contained"
                            disabled={busy || !file || malformedSha}
                            onClick={() => void review()}
                        >
                            {t("configurationPackage.roster.review")}
                        </Button>
                    )}
                </Box>
            </Drawer>
        </>
    )
}

export const ArchitectSignerRoster: React.FC<{
    tenantId: string
    status: RosterStatus | null
    loading: boolean
    statusTextId?: string
    refresh: () => Promise<RosterStatus | null>
}> = ({tenantId, status, loading, statusTextId, refresh}) => {
    const {t} = useTranslation()
    const {userId} = useContext(AuthContext)
    if (status?.mode === "not_enabled") return null
    const managing = !!userId && status?.mode === "portal_managed" && status.can_manage
    const ready = status !== null && rosterStateAllowsImport(status)

    return (
        <Box
            sx={{mt: 2}}
            aria-label={String(t("configurationPackage.roster.title"))}
            data-testid="signer-roster"
        >
            <Typography component="h3" variant="subtitle1">
                {t("configurationPackage.roster.title")}
            </Typography>
            <Typography id={statusTextId} variant="body2" aria-live="polite">
                {ready
                    ? t("configurationPackage.roster.activeSummary", {
                          approvals: status.required_approvals,
                      })
                    : t(
                          loading && status === null
                              ? "configurationPackage.roster.loading"
                              : status?.mode === "portal_managed"
                                ? "configurationPackage.roster.noRoster"
                                : "configurationPackage.roster.unavailable"
                      )}
            </Typography>
            {ready && (
                <Box component="details" sx={{mt: 0.5}} data-testid="signer-roster-fingerprint">
                    <Typography
                        component="summary"
                        variant="body2"
                        color="text.secondary"
                        sx={{cursor: "pointer"}}
                    >
                        {t("configurationPackage.roster.fingerprint")}
                    </Typography>
                    <Typography
                        component="code"
                        variant="caption"
                        sx={{
                            display: "block",
                            mt: 0.5,
                            fontFamily: "monospace",
                            overflowWrap: "anywhere",
                            userSelect: "all",
                        }}
                    >
                        {status.active_sha256}
                    </Typography>
                </Box>
            )}
            {!loading && (!status || status.mode === "unavailable") && (
                <Button variant="text" onClick={() => void refresh()}>
                    {t("configurationPackage.roster.retry")}
                </Button>
            )}
            {status?.mode === "site_pinned" && (
                <Typography variant="body2">
                    {t("configurationPackage.roster.sitePinned")}
                </Typography>
            )}
            {status?.mode === "portal_managed" && !status.can_manage && (
                <Typography variant="body2">{t("configurationPackage.roster.locked")}</Typography>
            )}
            {managing && (
                <RosterSetup
                    key={JSON.stringify([tenantId, userId])}
                    tenantId={tenantId}
                    status={status}
                    refresh={refresh}
                />
            )}
        </Box>
    )
}
