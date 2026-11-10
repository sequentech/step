// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useState} from "react"
import {gql, useMutation, useQuery} from "@apollo/client"
import {
    Alert,
    Button,
    CircularProgress,
    Stack,
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableRow,
    Typography,
} from "@mui/material"
import {useNotify, useRecordContext} from "react-admin"
import {useTranslation} from "react-i18next"
import {Dialog} from "@sequentech/ui-essentials"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useAliasRenderer} from "@/hooks/useAliasRenderer"
import {getGraphQLActionErrorReason} from "@/services/graphqlActionError"
import {IPermissions} from "@/types/keycloak"

const AS_ADMINISTRATOR = {headers: {"x-hasura-role": IPermissions.ADMIN_USER}}

// Only columns that the administrator's role selects. `administration_settings`
// carries the election's localized names; its alias is the one the portal's
// settings projection gives it (`administratorSettingsQuery`), so the request
// goes out as written.
export const GET_ELECTION_REVOTING = gql`
    query GetElectionRevoting($electionEventId: uuid!) {
        sequent_backend_election(
            where: {election_event_id: {_eq: $electionEventId}}
            order_by: {id: asc}
        ) {
            id
            presentation: administration_settings
            num_allowed_revotes
        }
    }
`
export const DISABLE_ELECTION_REVOTING = gql`
    mutation DisableElectionRevoting($electionEventId: String!, $electionId: String!) {
        disable_election_revoting(election_event_id: $electionEventId, election_id: $electionId) {
            election_id
            num_allowed_revotes
        }
    }
`

interface ElectionRevoting {
    id: string
    presentation?: unknown
    num_allowed_revotes?: number | null
}

/** Zero is unlimited and a missing value counts as one vote, as the server reads it. */
export const allowsRevoting = (allowed: number | null | undefined): allowed is number =>
    allowed === 0 || (typeof allowed === "number" && allowed > 1)

/** The administrator's control that disables re-voting, one election at a time.
 * The server refuses it outside Preparation, during a lockdown and once voting
 * has started on any channel of the election event. */
export function ElectionRevotingSettings() {
    const {t} = useTranslation()
    const {hasRole} = useContext(AuthContext)
    const record = useRecordContext<{id: string}>()
    const notify = useNotify()
    const aliasRenderer = useAliasRenderer()
    const electionEventId = record?.id
    const allowed = !!electionEventId && hasRole(IPermissions.ADMIN_USER)
    const {data, loading, error, refetch} = useQuery<{
        sequent_backend_election: ElectionRevoting[]
    }>(GET_ELECTION_REVOTING, {
        variables: {electionEventId},
        skip: !allowed,
        fetchPolicy: "network-only",
        context: AS_ADMINISTRATOR,
    })
    const [disable] = useMutation(DISABLE_ELECTION_REVOTING, {context: AS_ADMINISTRATOR})
    const [selected, setSelected] = useState<ElectionRevoting | null>(null)
    const [saving, setSaving] = useState(false)
    if (!allowed) return null

    const nameOf = (election: ElectionRevoting) => {
        const name = aliasRenderer(election.presentation)
        return name && name !== "-" ? name : election.id
    }
    const stateOf = (value: number | null | undefined) =>
        value === 0
            ? t("electionRevoting.unlimited")
            : allowsRevoting(value)
              ? t("electionRevoting.upTo", {votes: value})
              : t("electionRevoting.single")
    const confirm = async (election: ElectionRevoting) => {
        const name = nameOf(election)
        setSaving(true)
        try {
            const response = await disable({
                variables: {electionEventId, electionId: election.id},
            })
            if (response.data?.disable_election_revoting?.election_id !== election.id)
                throw new Error("Unconfirmed change")
            notify(t("electionRevoting.disabled", {name}), {type: "success"})
        } catch (failure) {
            const reason = getGraphQLActionErrorReason(failure)
            notify(
                reason
                    ? t("electionRevoting.failed", {name, reason})
                    : t("electionRevoting.failedNoReason", {name}),
                {type: "error"}
            )
        } finally {
            setSaving(false)
            void refetch()
        }
    }

    const elections = data?.sequent_backend_election
    return (
        <Stack spacing={2} sx={{p: 2, width: "100%"}}>
            <Typography component="h2" variant="h5">
                {t("electionRevoting.title")}
            </Typography>
            <Alert severity="info">{t("electionRevoting.description")}</Alert>
            {loading && !data ? (
                <CircularProgress aria-label={t("electionRevoting.loading")} />
            ) : error || !elections ? (
                <Alert severity="error">{t("electionRevoting.readFailed")}</Alert>
            ) : elections.length === 0 ? (
                <Typography>{t("electionRevoting.empty")}</Typography>
            ) : (
                <Table size="small">
                    <TableHead>
                        <TableRow>
                            <TableCell>{t("electionRevoting.election")}</TableCell>
                            <TableCell>{t("electionRevoting.state")}</TableCell>
                            <TableCell />
                        </TableRow>
                    </TableHead>
                    <TableBody>
                        {elections.map((election) => (
                            <TableRow key={election.id}>
                                <TableCell>{nameOf(election)}</TableCell>
                                <TableCell>{stateOf(election.num_allowed_revotes)}</TableCell>
                                <TableCell align="right">
                                    {allowsRevoting(election.num_allowed_revotes) && (
                                        <Button
                                            variant="outlined"
                                            color="warning"
                                            disabled={saving}
                                            aria-label={t("electionRevoting.disableFor", {
                                                name: nameOf(election),
                                            })}
                                            onClick={() => setSelected(election)}
                                        >
                                            {t("electionRevoting.disable")}
                                        </Button>
                                    )}
                                </TableCell>
                            </TableRow>
                        ))}
                    </TableBody>
                </Table>
            )}
            <Dialog
                variant="warning"
                open={!!selected}
                ok={t("electionRevoting.disable")}
                cancel={String(t("common.label.cancel"))}
                title={t("electionRevoting.confirmTitle")}
                handleClose={(result: boolean) => {
                    const election = selected
                    setSelected(null)
                    if (result && election) void confirm(election)
                }}
            >
                {selected ? t("electionRevoting.confirm", {name: nameOf(selected)}) : null}
            </Dialog>
        </Stack>
    )
}
