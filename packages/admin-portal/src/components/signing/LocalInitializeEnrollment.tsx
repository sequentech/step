// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect, useMemo, useRef, useState} from "react"
import {useApolloClient} from "@apollo/client"
import {Alert, Button, Checkbox, FormControlLabel, Stack, Typography} from "@mui/material"
import {AuthContext} from "@/providers/AuthContextProvider"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {
    createLocalInitializeOfficerApi, localOfficerSettings, readPublicLeaf,
    type LocalOfficerApi, type LocalOfficerRef,
} from "@/lib/signing/localInitializeOfficerApi"
import {isId, observedPage, signingReturnPath} from "@/lib/signing/operationalGoldReturn"

export function LocalNomineePresence({api, request, digest}: {
    api: LocalOfficerApi
    request: string
    digest: string
}) {
    const [consented, setConsented] = useState(false)
    const [active, setActive] = useState(false)
    const [failed, setFailed] = useState(false)
    useEffect(() => {
        if (!consented) return
        let cancelled = false
        let timer: number | undefined
        const stop = () => {
            if (document.visibilityState !== "visible") {
                cancelled = true
                setConsented(false)
                setActive(false)
                window.clearTimeout(timer)
            }
        }
        const renew = async () => {
            if (cancelled || document.visibilityState !== "visible") return
            try {
                await api.presence(request, digest)
                if (!cancelled) {
                    setActive(true)
                    timer = window.setTimeout(() => void renew(), 10000)
                }
            } catch {
                if (!cancelled) {
                    setActive(false)
                    setFailed(true)
                    setConsented(false)
                }
            }
        }
        document.addEventListener("visibilitychange", stop)
        stop()
        void renew()
        return () => {
            cancelled = true
            window.clearTimeout(timer)
            document.removeEventListener("visibilitychange", stop)
        }
    }, [api, request, digest, consented])
    return (
        <Stack spacing={1}>
            <Typography variant="body2">
                Your Security Officer must review the public request and attach the issued certificate in their own
                session. Stay on this dialog while they do each step.
            </Typography>
            {failed && <Alert severity="warning">Could not confirm your presence. Try again.</Alert>}
            {active && <Alert severity="info">You are available for your Security Officer.</Alert>}
            <Button onClick={() => {
                setFailed(false)
                setActive(false)
                setConsented(!consented)
            }}>
                {consented ? "Stop sharing my presence" : "I’m here for certificate review"}
            </Button>
        </Stack>
    )
}

export function LocalOfficerCertificates({event, onAttached}: {event: string; onAttached: () => void}) {
    const auth = useContext(AuthContext)
    const {globalSettings} = useContext(SettingsContext)
    const apollo = useApolloClient()
    const origin = window.location.origin
    const registration = (globalSettings as {FELIX3_LOCAL_INITIALIZE?: unknown}).FELIX3_LOCAL_INITIALIZE
    const local = localOfficerSettings(origin, auth.tenantId, globalSettings)
    const api = useMemo(() => createLocalInitializeOfficerApi(apollo, origin, registration),
        [apollo, origin, registration])
    if (!auth.isAuthenticated || !auth.hasRole("signing-certificates-register") || !isId(auth.userId) || !isId(event) || !local?.setupClient || !api)
        return null
    return <OfficerForm key={auth.tenantId + "/" + event + "/" + auth.userId} api={api}
        event={event} officer={auth.userId} onAttached={onAttached} />
}

function OfficerForm({api, event, officer, onAttached}: {
    api: LocalOfficerApi; event: string; officer: string; onAttached: () => void
}) {
    const {isGoldUser, reauthWithGold} = useContext(AuthContext)
    const [refs, setRefs] = useState<LocalOfficerRef[]>()
    const [review, setReview] = useState<Awaited<ReturnType<LocalOfficerApi["review"]>>>()
    const [leaf, setLeaf] = useState<Awaited<ReturnType<typeof readPublicLeaf>>>()
    const [receipt, setReceipt] = useState<Awaited<ReturnType<LocalOfficerApi["attach"]>>>()
    const [confirmed, setConfirmed] = useState(false)
    const [busy, setBusy] = useState(false)
    const [error, setError] = useState("")
    const inFlight = useRef(false)
    const alive = useRef(false)
    useEffect(() => {
        alive.current = true
        return () => { alive.current = false }
    }, [])
    const run = async (work: () => Promise<void>, message: string) => {
        if (inFlight.current) return
        inFlight.current = true
        setBusy(true)
        setError("")
        try {
            await work()
        } catch {
            if (alive.current) setError(message)
        } finally {
            inFlight.current = false
            if (alive.current) setBusy(false)
        }
    }
    const requireGold = () => {
        if (!isGoldUser()) throw new Error("Fresh Gold required")
    }
    const stepUp = () => void run(async () => {
        // Fixed same-origin current page path. Do not carry a request, uploaded
        // certificate or pending mutation across authentication or replay it.
        const page = observedPage(window.location.pathname, event, "signatures-requests")
        if (!page) throw new Error("Invalid return")
        const url = new URL(signingReturnPath(event, page), "http://localhost:3002")
        url.searchParams.set("tabId", "local-initialize-officer")
        await reauthWithGold(url.href, true)
    }, "Could not start Gold sign-in.")
    const clearReview = () => {
        setReview(undefined)
        setLeaf(undefined)
        setConfirmed(false)
        setReceipt(undefined)
    }
    const list = () => void run(async () => {
        setRefs(undefined)
        clearReview()
        requireGold()
        const items = await api.pending(event)
        if (alive.current) setRefs(items.filter((item) => item.official_id !== officer))
    }, "Could not load requests. Verify with Gold and make sure the nominee is present.")
    const fetchReview = (selected: LocalOfficerRef) => void run(async () => {
        clearReview()
        requireGold()
        const value = await api.review(selected)
        if (alive.current) setReview(value)
    }, "Could not review the public request. Check that the nominee is present and Gold is fresh.")
    const download = () => {
        if (!review || !isGoldUser()) {
            setError("Verify with Gold before downloading the public request.")
            return
        }
        const href = URL.createObjectURL(new Blob([review.csr_pem], {type: "application/pkcs10"}))
        const link = document.createElement("a")
        link.href = href
        link.download = `initialize-${review.enrollment}.csr.pem`
        link.click()
        window.setTimeout(() => URL.revokeObjectURL(href), 1000)
    }
    const selectFile = (file?: File) => void run(async () => {
        setLeaf(undefined)
        setConfirmed(false)
        if (!file) return
        const parsed = await readPublicLeaf(file)
        if (alive.current) setLeaf(parsed)
    }, "Select exactly one public X.509 certificate in PEM or DER format, not a chain, key or P12.")
    const attach = () => void run(async () => {
        if (!review || !leaf || !confirmed || receipt) return
        requireGold()
        const value = await api.attach(review, leaf)
        if (!alive.current) return
        setReceipt(value)
        setLeaf(undefined)
        setConfirmed(false)
        onAttached()
    }, "Attachment not confirmed. Check nominee presence and Gold before explicitly trying again.")

    return (
        <Stack spacing={1} aria-label="Initialize signing certificate enrollment">
            <Typography variant="h6">Initialize signing certificate enrollment</Typography>
            <Typography variant="body2">Use your own Security Officer sign-in. The trusted staff CA must issue
                the public certificate from the reviewed request.</Typography>
            {error && <Alert severity="error">{error}</Alert>}
            {receipt && <Alert severity="success">Certificate attached: {receipt.certificate}</Alert>}
            <Stack direction="row" spacing={1}>
                <Button onClick={stepUp} disabled={busy}>Verify with Gold</Button>
                <Button onClick={list} disabled={busy}>Check pending requests</Button>
            </Stack>
            {refs?.length === 0 && <Typography>No pending requests from present nominees.</Typography>}
            {refs?.map((item) => (
                <Button key={item.request_id + "/" + item.official_id} onClick={() => fetchReview(item)} disabled={busy}>
                    Review Post {item.election_id}, nominee {item.official_id}
                </Button>
            ))}
            {review && !receipt && (
                <Stack spacing={1}>
                    <Typography variant="body2" sx={{overflowWrap: "anywhere"}}>
                        Post: {review.election_id}<br />
                        Nominee: {review.official_id}<br />
                        Request: {review.request_id}<br />
                        Request payload SHA-256: {review.payload_sha256}<br />
                        CSR SHA-256: {review.csr_sha256}<br />
                        Public key SHA-256: {review.spki_sha256}
                    </Typography>
                    <Button onClick={download} disabled={busy}>Download public certificate request</Button>
                    <Button component="label" disabled={busy}>
                        Select issued public certificate
                        <input hidden type="file" accept=".pem,.crt,.cer,.der" onChange={(e) => {
                            const file = e.currentTarget.files?.[0]
                            e.currentTarget.value = ""
                            selectFile(file)
                        }} />
                    </Button>
                    {leaf && <Typography variant="body2" sx={{overflowWrap: "anywhere"}}>
                        Issued certificate SHA-256: {leaf.leaf_sha256}
                    </Typography>}
                    <FormControlLabel control={<Checkbox checked={confirmed} disabled={!leaf || busy}
                        onChange={(e) => setConfirmed(e.target.checked)} />}
                        label="I reviewed this Post, nominee, request and public certificate." />
                    <Button variant="contained" onClick={attach} disabled={!leaf || !confirmed || busy}>
                        Register and attach certificate
                    </Button>
                </Stack>
            )}
        </Stack>
    )
}
