// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useCallback, useEffect, useRef, useState} from "react"
import {
    Alert,
    Box,
    Button,
    CircularProgress,
    List,
    ListItem,
    ListItemText,
    Typography,
} from "@mui/material"
import {useTranslation} from "react-i18next"
import {PluginHost, errorMessage} from "../contracts"
import {HistoryEvent, Operation, formatDate} from "./model"

const PAGE_SIZE = 20
const KNOWN_ACTIONS = ["create", "update", "delete", "import", "export"]

interface Props {
    host: PluginHost
    projectId: string
    /** A task of the project, possibly deleted; without it, the whole project. */
    taskId?: string
    timezone: string
    /** Changes whenever the history may have grown. */
    revision: number
}

/** Append-only change history, newest first. Deleted tasks keep their events. */
export const History: React.FC<Props> = ({host, projectId, taskId, timezone, revision}) => {
    const {t} = useTranslation()
    const [events, setEvents] = useState<HistoryEvent[]>([])
    const [more, setMore] = useState(false)
    const [loading, setLoading] = useState(true)
    const [error, setError] = useState("")
    /** Only the latest request may update the list. */
    const generation = useRef(0)

    const load = useCallback(
        async (before?: number) => {
            const request = ++generation.current
            setLoading(true)
            setError("")
            if (!before) setEvents([])
            try {
                const page = await host.execute<{events: HistoryEvent[]}>(Operation.History, {
                    project_id: projectId,
                    ...(taskId ? {task_id: taskId} : {}),
                    ...(before ? {before_id: before} : {}),
                    limit: PAGE_SIZE,
                })
                if (request !== generation.current) return
                setEvents((previous) => (before ? [...previous, ...page.events] : page.events))
                setMore(page.events.length === PAGE_SIZE)
            } catch (cause) {
                if (request === generation.current) setError(errorMessage(cause))
            } finally {
                if (request === generation.current) setLoading(false)
            }
        },
        [host, projectId, taskId]
    )

    useEffect(() => {
        void load()
        return () => {
            generation.current += 1
        }
    }, [load, revision])

    const describe = (event: HistoryEvent) => {
        const action = KNOWN_ACTIONS.includes(event.action) ? event.action : "other"
        const title = event.after?.data?.title ?? event.before?.data?.title
        const subject = !taskId && event.record_id !== projectId && title ? title : ""
        return t(`projects.history.actions.${action}`, {
            subject,
            count: event.after?.task_count ?? 0,
            action: event.action,
        })
    }

    return (
        <Box>
            <Typography variant="overline" color="text.secondary" component={taskId ? "h3" : "h2"}>
                {t(taskId ? "projects.history.task" : "projects.history.project")}
            </Typography>
            {error && (
                <Alert
                    severity="error"
                    action={<Button onClick={() => void load()}>{t("projects.retry")}</Button>}
                >
                    {error}
                </Alert>
            )}
            {!error && !loading && events.length === 0 && (
                <Typography variant="body2" color="text.secondary">
                    {t("projects.history.empty")}
                </Typography>
            )}
            <List dense disablePadding>
                {events.map((event) => (
                    <ListItem key={event.id} disableGutters>
                        <ListItemText
                            primary={describe(event)}
                            secondary={t("projects.history.by", {
                                actor: event.actor,
                                at: formatDate(event.at, timezone),
                            })}
                        />
                    </ListItem>
                ))}
            </List>
            {loading && <CircularProgress size={20} aria-label={t("projects.loading")} />}
            {more && !loading && (
                <Button size="small" onClick={() => void load(events[events.length - 1]?.id)}>
                    {t("projects.history.more")}
                </Button>
            )}
        </Box>
    )
}
