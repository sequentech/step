// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useCallback, useEffect, useRef, useState} from "react"
import {
    Alert,
    Box,
    Button,
    CircularProgress,
    Dialog,
    DialogActions,
    DialogContent,
    DialogContentText,
    DialogTitle,
    IconButton,
    Link,
    Paper,
    Stack,
    Typography,
} from "@mui/material"
import ArrowBackIcon from "@mui/icons-material/ArrowBack"
import CloseIcon from "@mui/icons-material/Close"
import DownloadIcon from "@mui/icons-material/Download"
import AddIcon from "@mui/icons-material/Add"
import {useTranslation} from "react-i18next"
import {PluginErrorCode, PluginHost, PluginRequestError, errorMessage} from "../contracts"
import {
    Operation,
    Permission,
    ProjectDetail,
    ProjectState,
    Saved,
    Task,
    byOrder,
    formatDate,
    isDirty,
    moveTask,
    newTask,
} from "./model"
import {Timeline} from "./Timeline"
import {TaskEditor} from "./TaskEditor"
import {History} from "./History"
import {NavigationGuard} from "../NavigationGuard"

/** What the details panel holds: nothing, a saved task, or a new task. */
type Selection = {kind: "none"} | {kind: "task"; id: string} | {kind: "new"}

interface Draft {
    task: Task
    /** The saved task or blank new task that the draft started from. */
    baseline: Task
    /** The version the draft would replace; `null` for a new task. */
    version: number | null
}

interface Guard {
    action: string
    proceed: () => void
    /** Cancels a held navigation when the user keeps editing. */
    reset?: () => void
}

const UTF8_BOM = "﻿"

const download = (filename: string, text: string) => {
    const url = URL.createObjectURL(new Blob([UTF8_BOM, text], {type: "text/csv;charset=utf-8"}))
    const link = document.createElement("a")
    link.href = url
    link.download = filename
    link.click()
    URL.revokeObjectURL(url)
}

interface Props {
    host: PluginHost
    projectId: string
    onBack: () => void
}

export const ProjectView: React.FC<Props> = ({host, projectId, onBack}) => {
    const {t} = useTranslation()
    const canWrite = host.can(Permission.Write)
    const canManage = host.can(Permission.Manage)
    const [detail, setDetail] = useState<ProjectDetail | null>(null)
    const [loadError, setLoadError] = useState("")
    const [selection, setSelection] = useState<Selection>({kind: "none"})
    const [draft, setDraft] = useState<Draft | null>(null)
    const [pendingDates, setPendingDates] = useState(false)
    const [busy, setBusy] = useState(false)
    const [panelError, setPanelError] = useState("")
    const [conflict, setConflict] = useState<Saved<Task> | null>(null)
    const [guard, setGuard] = useState<Guard | null>(null)
    const [confirmDelete, setConfirmDelete] = useState(false)
    const [notice, setNotice] = useState<{severity: "success" | "error"; text: string} | null>(null)
    const [revision, setRevision] = useState(0)
    const [moving, setMoving] = useState<string[]>([])
    const movingNow = useRef(new Set<string>())
    /** Identifies the edit in the panel; a late response for another edit leaves it alone. */
    const edit = useRef(0)
    const heading = useRef<HTMLHeadingElement>(null)
    const keepEditing = useRef<HTMLButtonElement>(null)

    const load = useCallback(async () => {
        setLoadError("")
        try {
            const result = await host.execute<ProjectDetail>(Operation.GetProject, {id: projectId})
            setDetail({...result, tasks: [...result.tasks].sort(byOrder)})
        } catch (cause) {
            setLoadError(errorMessage(cause))
        }
    }, [host, projectId])

    useEffect(() => {
        void load()
    }, [load])

    const project = detail?.project
    const timezone = project?.data.timezone ?? "UTC"
    const archived = project?.data.state === ProjectState.Archived
    const editable = canWrite && !archived
    const selectedId = selection.kind === "task" ? selection.id : null
    const savedTask = selectedId ? detail?.tasks.find((task) => task.id === selectedId) : undefined
    const dirty = Boolean(
        draft && editable && (isDirty(draft.task, draft.baseline) || pendingDates)
    )

    useEffect(() => {
        if (guard) keepEditing.current?.focus()
    }, [guard])

    /** After closing the details, focus returns to the task's bar once it renders. */
    const [returnFocus, setReturnFocus] = useState<{id: string} | null>(null)
    useEffect(() => {
        if (returnFocus)
            document.querySelector<HTMLElement>(`[data-task="${returnFocus.id}"]`)?.focus()
    }, [returnFocus])
    /** Moves focus to the details once they render, unless the user is already in them. */
    const [focusRequest, setFocusRequest] = useState(0)
    const focusPanel = () => setFocusRequest((request) => request + 1)
    useEffect(() => {
        if (!focusRequest) return
        const panel = document.getElementById("project-task-details")
        if (panel?.contains(document.activeElement)) return
        heading.current?.focus()
        heading.current?.scrollIntoView?.({block: "nearest"})
    }, [focusRequest])

    /**
     * Runs `proceed` now, or after the user discards unsaved changes. Nothing
     * changes the panel while its save or delete is pending.
     */
    const guarded = (action: string, proceed: () => void) => {
        if (busy) return
        if (dirty) {
            setGuard((previous) => {
                previous?.reset?.()
                return {action, proceed}
            })
        } else proceed()
    }

    const clearPanel = () => {
        edit.current += 1
        setPanelError("")
        setConflict(null)
        setPendingDates(false)
        setGuard(null)
    }

    const select = (task: Saved<Task>) => {
        if (selectedId === task.id) return focusPanel()
        guarded(t("projects.unsaved.actions.open", {title: task.data.title}), () => {
            clearPanel()
            setSelection({kind: "task", id: task.id})
            setDraft({task: task.data, baseline: task.data, version: task.version})
            focusPanel()
        })
    }

    const addTask = () =>
        guarded(t("projects.unsaved.actions.add"), () => {
            clearPanel()
            const blank = newTask(timezone, detail?.tasks.length ?? 0)
            setSelection({kind: "new"})
            setDraft({task: blank, baseline: blank, version: null})
            focusPanel()
        })

    const close = () => {
        const id = selectedId
        clearPanel()
        setSelection({kind: "none"})
        setDraft(null)
        if (id) setReturnFocus({id})
    }

    const replaceTask = (saved: Saved<Task>) =>
        setDetail(
            (previous) =>
                previous && {
                    ...previous,
                    tasks: [...previous.tasks.filter((task) => task.id !== saved.id), saved].sort(
                        byOrder
                    ),
                }
        )

    const failed = (cause: unknown) => {
        if (
            cause instanceof PluginRequestError &&
            cause.code === PluginErrorCode.Conflict &&
            cause.current
        ) {
            setConflict(cause.current as Saved<Task>)
        } else {
            setPanelError(errorMessage(cause))
        }
    }

    const save = async () => {
        if (!draft || !project || busy) return
        const token = edit.current
        setBusy(true)
        setPanelError("")
        try {
            const saved = await host.execute<Saved<Task>>(Operation.SaveTask, {
                project_id: project.id,
                task: draft.task,
                ...(draft.version === null
                    ? {}
                    : {id: selectedId, expected_version: draft.version}),
            })
            replaceTask(saved)
            if (token === edit.current) {
                setSelection({kind: "task", id: saved.id})
                setDraft({task: saved.data, baseline: saved.data, version: saved.version})
                setConflict(null)
            }
            setNotice({
                severity: "success",
                text: t("projects.notices.saved", {title: saved.data.title}),
            })
            setRevision((value) => value + 1)
        } catch (cause) {
            if (token === edit.current) failed(cause)
        } finally {
            setBusy(false)
        }
    }

    const loadTheirs = () => {
        if (!conflict) return
        replaceTask(conflict)
        setDraft({task: conflict.data, baseline: conflict.data, version: conflict.version})
        setConflict(null)
        setPendingDates(false)
        setRevision((value) => value + 1)
    }

    const remove = async () => {
        if (!draft || !project || !selectedId || draft.version === null || busy) return
        const token = edit.current
        setConfirmDelete(false)
        setBusy(true)
        try {
            await host.execute(Operation.DeleteTask, {
                project_id: project.id,
                id: selectedId,
                expected_version: draft.version,
            })
            setDetail(
                (previous) =>
                    previous && {
                        ...previous,
                        tasks: previous.tasks.filter((task) => task.id !== selectedId),
                    }
            )
            setNotice({
                severity: "success",
                text: t("projects.notices.deleted", {title: draft.baseline.title}),
            })
            if (token === edit.current) {
                clearPanel()
                setSelection({kind: "none"})
                setDraft(null)
            }
            setRevision((value) => value + 1)
        } catch (cause) {
            if (token === edit.current) failed(cause)
        } finally {
            setBusy(false)
        }
    }

    /**
     * The open task's move changes only its draft; any other task is saved at
     * once. A task's next move waits for its previous versioned write.
     */
    const move = async (task: Saved<Task>, days: number, resize: boolean) => {
        if (!project || movingNow.current.has(task.id)) return
        if (task.id === selectedId && draft) {
            if (!busy) {
                setDraft(
                    (previous) =>
                        previous && {
                            ...previous,
                            task: moveTask(previous.task, days, resize, timezone),
                        }
                )
            }
            return
        }
        const moved = moveTask(task.data, days, resize, timezone)
        if (moved === task.data) return
        movingNow.current.add(task.id)
        setMoving((ids) => [...ids, task.id])
        replaceTask({...task, data: moved})
        try {
            const saved = await host.execute<Saved<Task>>(Operation.SaveTask, {
                project_id: project.id,
                id: task.id,
                expected_version: task.version,
                task: moved,
            })
            replaceTask(saved)
            setNotice({
                severity: "success",
                text: t("projects.notices.moved", {
                    title: saved.data.title,
                    start: formatDate(saved.data.start, timezone),
                    end: formatDate(saved.data.end ?? saved.data.start, timezone),
                }),
            })
            setRevision((value) => value + 1)
        } catch (cause) {
            replaceTask(task)
            setNotice({severity: "error", text: errorMessage(cause)})
            if (cause instanceof PluginRequestError && cause.code === PluginErrorCode.Conflict)
                void load()
        } finally {
            movingNow.current.delete(task.id)
            setMoving((ids) => ids.filter((id) => id !== task.id))
        }
    }

    const setState = async (state: ProjectState) => {
        if (!project) return
        setBusy(true)
        try {
            const saved = await host.execute<ProjectDetail["project"]>(Operation.SetProjectState, {
                id: project.id,
                expected_version: project.version,
                state,
            })
            setDetail((previous) => previous && {...previous, project: saved})
            setNotice({
                severity: "success",
                text: t(
                    state === ProjectState.Archived
                        ? "projects.notices.archived"
                        : "projects.notices.restored"
                ),
            })
            setRevision((value) => value + 1)
        } catch (cause) {
            setNotice({severity: "error", text: errorMessage(cause)})
            if (cause instanceof PluginRequestError && cause.code === PluginErrorCode.Conflict)
                void load()
        } finally {
            setBusy(false)
        }
    }

    const exportCsv = async () => {
        if (!project) return
        try {
            const result = await host.execute<{csv: string; filename: string}>(Operation.Export, {
                id: project.id,
            })
            download(result.filename, result.csv)
            setRevision((value) => value + 1)
        } catch (cause) {
            setNotice({severity: "error", text: errorMessage(cause)})
        }
    }

    if (loadError) {
        return (
            <Alert
                severity="error"
                action={<Button onClick={() => void load()}>{t("projects.retry")}</Button>}
            >
                {loadError}
            </Alert>
        )
    }
    if (!detail || !project) return <CircularProgress aria-label={t("projects.loading")} />

    const panelTitle =
        selection.kind === "new"
            ? t("projects.panel.add")
            : editable
              ? t("projects.panel.edit")
              : t("projects.panel.details")

    return (
        <Box sx={{minWidth: 0}}>
            <NavigationGuard
                active={dirty}
                onBlocked={(proceed, reset) =>
                    setGuard((previous) => {
                        previous?.reset?.()
                        return {action: t("projects.unsaved.actions.leave"), proceed, reset}
                    })
                }
            />
            <Stack
                direction={{xs: "column", md: "row"}}
                spacing={2}
                justifyContent="space-between"
                sx={{mb: 2}}
            >
                <Box sx={{minWidth: 0}}>
                    <Link
                        component="button"
                        type="button"
                        underline="hover"
                        disabled={busy}
                        onClick={() => guarded(t("projects.unsaved.actions.leave"), onBack)}
                        sx={{display: "inline-flex", alignItems: "center", gap: 0.5}}
                    >
                        <ArrowBackIcon fontSize="small" />
                        {t("projects.title")}
                    </Link>
                    <Typography variant="h5" component="h1" sx={{mt: 1, overflowWrap: "anywhere"}}>
                        {project.data.name}
                    </Typography>
                    <Typography variant="body2" color="text.secondary">
                        {t("projects.detail.timezone", {timezone})}
                    </Typography>
                </Box>
                <Stack
                    direction="row"
                    spacing={1}
                    flexWrap="wrap"
                    useFlexGap
                    alignItems="flex-start"
                >
                    <Button
                        variant="outlined"
                        startIcon={<DownloadIcon />}
                        onClick={() => void exportCsv()}
                    >
                        {t("projects.detail.export")}
                    </Button>
                    {canManage && (
                        <Button
                            variant="outlined"
                            disabled={busy}
                            onClick={() =>
                                guarded(
                                    t("projects.unsaved.actions.state"),
                                    () =>
                                        void setState(
                                            archived ? ProjectState.Active : ProjectState.Archived
                                        )
                                )
                            }
                        >
                            {t(archived ? "projects.detail.restore" : "projects.detail.archive")}
                        </Button>
                    )}
                    {editable && (
                        <Button
                            variant="contained"
                            startIcon={<AddIcon />}
                            disabled={busy}
                            onClick={addTask}
                        >
                            {t("projects.detail.addTask")}
                        </Button>
                    )}
                </Stack>
            </Stack>
            <Stack spacing={1.5} sx={{mb: 2}}>
                {!canWrite && <Alert severity="info">{t("projects.detail.viewOnly")}</Alert>}
                {archived && <Alert severity="info">{t("projects.detail.archivedNotice")}</Alert>}
                {notice && (
                    <Alert severity={notice.severity} role="status" onClose={() => setNotice(null)}>
                        {notice.text}
                    </Alert>
                )}
            </Stack>
            <Timeline
                tasks={detail.tasks.map((task) =>
                    task.id === selectedId && draft
                        ? {
                              ...task,
                              data: {...task.data, start: draft.task.start, end: draft.task.end},
                          }
                        : task
                )}
                timezone={timezone}
                selected={selectedId}
                editable={editable}
                pending={busy && selectedId ? [...moving, selectedId] : moving}
                onSelect={select}
                onMove={(task, days, resize) => void move(task, days, resize)}
            />
            <Paper
                variant="outlined"
                component="section"
                aria-labelledby="projects-panel-title"
                id="project-task-details"
                sx={{mt: 2, ...(draft ? {} : {borderStyle: "dashed"})}}
            >
                {draft ? (
                    <>
                        <Stack
                            direction="row"
                            alignItems="center"
                            spacing={1.5}
                            sx={{
                                px: {xs: 2, md: 3},
                                py: 1.25,
                                borderBottom: 1,
                                borderColor: "divider",
                            }}
                        >
                            <Typography
                                id="projects-panel-title"
                                ref={heading}
                                tabIndex={-1}
                                variant="h6"
                                component="h2"
                                color="primary"
                                sx={{outline: "none", minWidth: 0}}
                            >
                                {panelTitle}
                            </Typography>
                            {savedTask && (
                                <Typography
                                    variant="body2"
                                    color="text.secondary"
                                    noWrap
                                    sx={{flex: 1, minWidth: 0}}
                                >
                                    {savedTask.data.title}
                                </Typography>
                            )}
                            <IconButton
                                aria-label={t("projects.panel.close")}
                                disabled={busy}
                                onClick={() => guarded(t("projects.unsaved.actions.close"), close)}
                                sx={{ml: "auto"}}
                            >
                                <CloseIcon />
                            </IconButton>
                        </Stack>
                        <Box
                            sx={{p: {xs: 2, md: 3}}}
                            onKeyDown={(event) => {
                                if (event.key === "Escape" && !guard && !confirmDelete) {
                                    event.stopPropagation()
                                    guarded(t("projects.unsaved.actions.close"), close)
                                }
                            }}
                        >
                            <Stack spacing={2} sx={{mb: guard || conflict || panelError ? 2 : 0}}>
                                {guard && (
                                    <Alert severity="warning" role="alert">
                                        {t("projects.unsaved.body", {action: guard.action})}
                                        <Stack
                                            direction="row"
                                            spacing={1}
                                            flexWrap="wrap"
                                            useFlexGap
                                            sx={{mt: 1}}
                                        >
                                            <Button
                                                ref={keepEditing}
                                                color="inherit"
                                                variant="outlined"
                                                size="small"
                                                onClick={() => {
                                                    guard.reset?.()
                                                    setGuard(null)
                                                }}
                                            >
                                                {t("projects.unsaved.keep")}
                                            </Button>
                                            <Button
                                                color="error"
                                                size="small"
                                                onClick={() => {
                                                    const {proceed} = guard
                                                    setGuard(null)
                                                    setDraft(null)
                                                    setPendingDates(false)
                                                    proceed()
                                                }}
                                            >
                                                {t("projects.unsaved.discard")}
                                            </Button>
                                        </Stack>
                                    </Alert>
                                )}
                                {conflict && (
                                    <Alert severity="warning" role="alert">
                                        {t("projects.conflict.body", {
                                            actor: conflict.updated_by,
                                            at: formatDate(conflict.updated_at, timezone),
                                        })}
                                        <Box sx={{mt: 1}}>
                                            <Button
                                                color="inherit"
                                                variant="outlined"
                                                size="small"
                                                onClick={loadTheirs}
                                            >
                                                {t("projects.conflict.load")}
                                            </Button>
                                        </Box>
                                    </Alert>
                                )}
                                {panelError && (
                                    <Alert severity="error" role="alert">
                                        {panelError}
                                    </Alert>
                                )}
                            </Stack>
                            <TaskEditor
                                key={`${selection.kind === "task" ? selection.id : "new"}:${draft.version ?? 0}`}
                                draft={draft.task}
                                setDraft={(task) =>
                                    setDraft((previous) => previous && {...previous, task})
                                }
                                timezone={timezone}
                                editable={editable}
                                busy={busy}
                                existing={selection.kind === "task"}
                                onSave={() => void save()}
                                onDelete={() => setConfirmDelete(true)}
                                onCancel={() => guarded(t("projects.unsaved.actions.close"), close)}
                                onPendingChange={setPendingDates}
                            />
                            {selectedId && (
                                <Box sx={{mt: 3}}>
                                    <History
                                        host={host}
                                        projectId={project.id}
                                        taskId={selectedId}
                                        timezone={timezone}
                                        revision={revision}
                                    />
                                </Box>
                            )}
                        </Box>
                    </>
                ) : (
                    <Box sx={{px: {xs: 2, md: 3}, py: 2}}>
                        <Typography
                            id="projects-panel-title"
                            variant="body2"
                            color="text.secondary"
                        >
                            {detail.tasks.length
                                ? t("projects.panel.hint")
                                : t("projects.panel.noTasks")}
                        </Typography>
                        <Box sx={{mt: 2}}>
                            <History
                                host={host}
                                projectId={project.id}
                                timezone={timezone}
                                revision={revision}
                            />
                        </Box>
                    </Box>
                )}
            </Paper>
            <Dialog open={confirmDelete} onClose={() => setConfirmDelete(false)}>
                <DialogTitle>{t("projects.deleteDialog.title")}</DialogTitle>
                <DialogContent>
                    <DialogContentText>
                        {t("projects.deleteDialog.body", {title: draft?.baseline.title ?? ""})}
                    </DialogContentText>
                </DialogContent>
                <DialogActions>
                    <Button onClick={() => setConfirmDelete(false)}>{t("projects.cancel")}</Button>
                    <Button color="error" variant="contained" onClick={() => void remove()}>
                        {t("projects.deleteDialog.confirm")}
                    </Button>
                </DialogActions>
            </Dialog>
        </Box>
    )
}
