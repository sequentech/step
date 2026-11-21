// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useRef, useState} from "react"
import {gql, useMutation, useQuery} from "@apollo/client"
import {Alert, Box, Button, TextField, Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import {IPermissions} from "@/types/keycloak"

interface Policy {
    required_approvals: number | null
    generation: string
}

const READ = gql`
    query ArchitectApprovalPolicy($tenantId: String!) {
        get_architect_approval_policy(tenant_id: $tenantId) {
            required_approvals
            generation
        }
    }
`

const WRITE = gql`
    mutation SetArchitectApprovalPolicy($tenantId: String!, $count: Int, $generation: String!) {
        set_architect_approval_policy(
            tenant_id: $tenantId
            required_approvals: $count
            generation: $generation
        ) {
            required_approvals
            generation
        }
    }
`

const policyRole = {headers: {"x-hasura-role": IPermissions.TENANT_WRITE}}

const isCount = (value: string) => /^(?:[1-9]|1[0-6])$/.test(value)

const PolicyForm: React.FC<{tenantId: string; initial: Policy}> = ({tenantId, initial}) => {
    const {t} = useTranslation()
    const [draft, setDraft] = useState(String(initial.required_approvals ?? ""))
    const [saved, setSaved] = useState(initial)
    const [notice, setNotice] = useState<"saved" | "failed" | null>(null)
    const [write, result] = useMutation<{set_architect_approval_policy: Policy | null}>(WRITE)
    const loading = result?.loading ?? false
    const saving = useRef(false)
    const invalid = draft !== "" && !isCount(draft)
    const unchanged = draft === String(saved.required_approvals ?? "")
    const save = async () => {
        if (invalid || unchanged || saving.current) return
        saving.current = true
        setNotice(null)
        try {
            const {data} = await write({
                context: policyRole,
                variables: {
                    tenantId,
                    count: draft === "" ? null : Number(draft),
                    generation: saved.generation,
                },
            })
            const policy = data?.set_architect_approval_policy
            if (!policy) throw new Error("policy was not saved")
            setSaved(policy)
            setDraft(String(policy.required_approvals ?? ""))
            setNotice("saved")
        } catch {
            setNotice("failed")
        } finally {
            saving.current = false
        }
    }

    return (
        <Box sx={{my: 2}}>
            <Typography variant="h6" component="h3">
                {t("architectApprovalPolicy.title")}
            </Typography>
            <TextField
                label={t("architectApprovalPolicy.field")}
                value={draft}
                placeholder="2"
                inputProps={{inputMode: "numeric"}}
                error={invalid}
                disabled={loading}
                helperText={t(
                    invalid ? "architectApprovalPolicy.invalid" : "architectApprovalPolicy.help"
                )}
                onChange={(event) => {
                    setDraft(event.target.value)
                    setNotice(null)
                }}
                sx={{my: 1}}
                fullWidth
            />
            <Typography variant="body2" paragraph>
                {t("architectApprovalPolicy.receiver")}
            </Typography>
            <Button disabled={invalid || unchanged || loading} onClick={save} variant="contained">
                {t("architectApprovalPolicy.save")}
            </Button>
            {notice && (
                <Alert sx={{mt: 1}} severity={notice === "saved" ? "success" : "error"}>
                    {t(`architectApprovalPolicy.${notice}`)}
                </Alert>
            )}
        </Box>
    )
}

export const ArchitectApprovalPolicy: React.FC<{tenantId: string}> = ({tenantId}) => {
    const {t} = useTranslation()
    const {data, error, loading} = useQuery<{get_architect_approval_policy: Policy | null}>(READ, {
        variables: {tenantId},
        context: policyRole,
        fetchPolicy: "network-only",
    })
    const initial = data?.get_architect_approval_policy
    if (loading) return <Typography>{t("architectApprovalPolicy.loading")}</Typography>
    if (error || !initial)
        return <Alert severity="error">{t("architectApprovalPolicy.unavailable")}</Alert>
    return <PolicyForm key={tenantId} tenantId={tenantId} initial={initial} />
}
