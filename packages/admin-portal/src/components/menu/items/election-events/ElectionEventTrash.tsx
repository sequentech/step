// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useCallback, useContext, useEffect, useRef, useState} from "react"
import {useApolloClient, useMutation} from "@apollo/client"
import {useNotify} from "react-admin"
import {useTranslation} from "react-i18next"
import {
    Alert,
    Box,
    Button,
    Chip,
    CircularProgress,
    IconButton,
    List,
    ListItem,
    ListItemText,
    Tooltip,
    Typography,
} from "@mui/material"
import RefreshIcon from "@mui/icons-material/Refresh"
import RestoreFromTrashIcon from "@mui/icons-material/RestoreFromTrash"
import {Dialog} from "@sequentech/ui-essentials"
import {LIST_TRASHED_ELECTION_EVENTS} from "@/queries/ListTrashedElectionEvents"
import {RESTORE_ELECTION_EVENT} from "@/queries/RestoreElectionEvent"
import {
    ListTrashedElectionEventsQuery,
    ListTrashedElectionEventsQueryVariables,
    RestoreElectionEventMutation,
    RestoreElectionEventMutationVariables,
} from "@/gql/graphql"
import {IPermissions} from "@/types/keycloak"
import {useAliasRenderer} from "@/hooks/useAliasRenderer"
import {SettingsContext} from "@/providers/SettingsContextProvider"

export const TRASH_PAGE_SIZE = 50

type TrashedElectionEvent =
    ListTrashedElectionEventsQuery["list_trashed_election_events"]["items"][number]

const context = {headers: {"x-hasura-role": IPermissions.ELECTION_EVENT_DELETE}}

export const formatTrashDate = (value: string, language: string): string =>
    new Intl.DateTimeFormat(language, {dateStyle: "medium", timeStyle: "short"}).format(
        new Date(value)
    )

interface TrashState {
    items: TrashedElectionEvent[]
    total: number
    retentionDays: number | null
}

/**
 * The first `pages` pages of the Trash, fetched again every `pollInterval`
 * milliseconds so that events claimed for permanent deletion, restored or
 * moved to Trash elsewhere show up without leaving the tab.
 */
export function useElectionEventTrash(pollInterval: number) {
    const client = useApolloClient()
    const [pages, setPages] = useState(1)
    const [trash, setTrash] = useState<TrashState | null>(null)
    const [error, setError] = useState(false)
    const [loading, setLoading] = useState(true)
    const latest = useRef(0)

    const load = useCallback(
        async (pageCount: number, quiet = false) => {
            const request = ++latest.current
            if (!quiet) {
                setLoading(true)
            }
            try {
                const results = await Promise.all(
                    Array.from({length: pageCount}, (_, page) =>
                        client.query<
                            ListTrashedElectionEventsQuery,
                            ListTrashedElectionEventsQueryVariables
                        >({
                            query: LIST_TRASHED_ELECTION_EVENTS,
                            variables: {limit: TRASH_PAGE_SIZE, offset: page * TRASH_PAGE_SIZE},
                            fetchPolicy: "network-only",
                            context,
                        })
                    )
                )
                if (request !== latest.current) {
                    return
                }
                const seen = new Set<string>()
                const items = results
                    .flatMap(({data}) => data.list_trashed_election_events.items)
                    .filter((item) => !seen.has(item.id) && !!seen.add(item.id))
                const first = results[0].data.list_trashed_election_events
                setTrash({items, total: first.total, retentionDays: first.retention_days ?? null})
                setError(false)
                // Fewer events than the pages loaded: stop asking for empty pages.
                const needed = Math.max(1, Math.ceil(first.total / TRASH_PAGE_SIZE))
                if (needed < pageCount) {
                    setPages(needed)
                }
            } catch (loadError) {
                if (request === latest.current) {
                    setError(true)
                }
            } finally {
                if (request === latest.current) {
                    setLoading(false)
                }
            }
        },
        [client]
    )

    useEffect(() => {
        load(pages)
        const timer = setInterval(() => load(pages, true), pollInterval)
        return () => clearInterval(timer)
    }, [load, pages, pollInterval])

    return {
        trash,
        error,
        loading,
        refresh: () => load(pages),
        loadMore: () => setPages((count) => count + 1),
    }
}

interface ElectionEventTrashProps {
    onRestored: () => void
}

export const ElectionEventTrash: React.FC<ElectionEventTrashProps> = ({onRestored}) => {
    const {t, i18n} = useTranslation()
    const notify = useNotify()
    const aliasRenderer = useAliasRenderer()
    const {globalSettings} = useContext(SettingsContext)
    const [selected, setSelected] = useState<TrashedElectionEvent | null>(null)
    const {trash, error, loading, refresh, loadMore} = useElectionEventTrash(
        globalSettings.QUERY_POLL_INTERVAL_MS
    )
    const [restore, {loading: restoring}] = useMutation<
        RestoreElectionEventMutation,
        RestoreElectionEventMutationVariables
    >(RESTORE_ELECTION_EVENT, {context})

    const items = trash?.items ?? []
    const name = (item: TrashedElectionEvent) => aliasRenderer(item)
    const date = (value: string) => formatTrashDate(value, i18n.language)

    const confirmRestore = async (confirmed: boolean) => {
        const item = selected
        setSelected(null)
        if (!confirmed || !item) {
            return
        }
        try {
            await restore({variables: {electionEventId: item.id}})
        } catch (restoreError) {
            notify(
                t("sideMenu.trash.restoreError", {
                    message: restoreError instanceof Error ? restoreError.message : "",
                }),
                {type: "error"}
            )
            return
        }
        notify(t("sideMenu.trash.restored", {name: name(item)}), {type: "success"})
        onRestored()
        refresh()
    }

    if (!trash && loading) {
        return (
            <Box display="flex" justifyContent="center" p={2}>
                <CircularProgress aria-label={String(t("sideMenu.trash.loading"))} />
            </Box>
        )
    }

    const errorAlert = error ? (
        <Alert
            severity={trash ? "warning" : "error"}
            sx={{m: 1}}
            action={
                <Button color="inherit" size="small" onClick={refresh}>
                    {t("sideMenu.trash.retry")}
                </Button>
            }
        >
            {trash ? t("sideMenu.trash.refreshError") : t("sideMenu.trash.error")}
        </Alert>
    ) : null

    if (!trash) {
        return errorAlert
    }

    const retentionDays = trash.retentionDays

    if (items.length === 0) {
        return (
            <Box p={2} textAlign="center" className="election-event-trash-empty">
                {errorAlert}
                <Typography variant="body1" fontWeight="bold">
                    {t("sideMenu.trash.empty")}
                </Typography>
                <Typography variant="body2" color="text.secondary">
                    {retentionDays
                        ? t("sideMenu.trash.emptyHint", {count: retentionDays})
                        : t("sideMenu.trash.emptyHintNoDays")}
                </Typography>
            </Box>
        )
    }

    return (
        <>
            {errorAlert}
            <Box display="flex" alignItems="center" justifyContent="space-between" px={2}>
                <Typography variant="caption" color="text.secondary">
                    {t("sideMenu.trash.showing", {count: items.length, total: trash.total})}
                </Typography>
                <Tooltip title={t("sideMenu.trash.refresh")}>
                    <span>
                        <IconButton
                            size="small"
                            aria-label={String(t("sideMenu.trash.refresh"))}
                            onClick={refresh}
                            disabled={loading}
                        >
                            <RefreshIcon fontSize="small" />
                        </IconButton>
                    </span>
                </Tooltip>
            </Box>
            <List dense aria-label={String(t("sideMenu.trash.tab"))} sx={{py: 0}}>
                {items.map((item) => {
                    const itemName = name(item)
                    const purging = !!item.purge_started_at
                    return (
                        <ListItem
                            key={item.id}
                            className="election-event-trash-item"
                            divider
                            sx={{pr: 7}}
                            secondaryAction={
                                <Tooltip
                                    title={
                                        purging
                                            ? t("sideMenu.trash.purging")
                                            : t("sideMenu.trash.restoreNamed", {name: itemName})
                                    }
                                >
                                    <span>
                                        <IconButton
                                            edge="end"
                                            aria-label={String(
                                                t("sideMenu.trash.restoreNamed", {name: itemName})
                                            )}
                                            disabled={purging || restoring}
                                            onClick={() => setSelected(item)}
                                        >
                                            <RestoreFromTrashIcon />
                                        </IconButton>
                                    </span>
                                </Tooltip>
                            }
                        >
                            <ListItemText
                                primary={
                                    <Box display="flex" alignItems="center" gap={1} minWidth={0}>
                                        <Typography variant="body2" noWrap title={itemName}>
                                            {itemName}
                                        </Typography>
                                        {item.archived_before_trash ? (
                                            <Chip
                                                size="small"
                                                variant="outlined"
                                                label={t("sideMenu.trash.wasArchived")}
                                            />
                                        ) : null}
                                    </Box>
                                }
                                secondary={
                                    <>
                                        <Typography
                                            component="span"
                                            variant="caption"
                                            display="block"
                                        >
                                            {t("sideMenu.trash.deletedOn", {
                                                date: date(item.trashed_at),
                                            })}
                                        </Typography>
                                        <Typography
                                            component="span"
                                            variant="caption"
                                            display="block"
                                            color={purging ? "error" : "text.secondary"}
                                        >
                                            {purging
                                                ? t("sideMenu.trash.purging")
                                                : t("sideMenu.trash.purgeOn", {
                                                      date: date(item.purge_after),
                                                  })}
                                        </Typography>
                                    </>
                                }
                            />
                        </ListItem>
                    )
                })}
            </List>
            {items.length < trash.total ? (
                <Box display="flex" justifyContent="center" p={1}>
                    <Button size="small" onClick={loadMore} disabled={loading}>
                        {t("sideMenu.trash.loadMore")}
                    </Button>
                </Box>
            ) : null}
            <Dialog
                variant="info"
                open={!!selected}
                ok={String(t("sideMenu.trash.restore"))}
                cancel={String(t("common.label.cancel"))}
                title={String(t("sideMenu.trash.restoreTitle"))}
                handleClose={confirmRestore}
            >
                {selected ? (
                    <>
                        <Typography paragraph>
                            {t("sideMenu.trash.restoreConfirm", {name: name(selected)})}
                        </Typography>
                        <Typography paragraph>
                            {selected.archived_before_trash
                                ? t("sideMenu.trash.restoreToArchived")
                                : t("sideMenu.trash.restoreToActive")}{" "}
                            {t("sideMenu.trash.restoreVoting")}
                        </Typography>
                    </>
                ) : null}
            </Dialog>
        </>
    )
}

export default ElectionEventTrash
