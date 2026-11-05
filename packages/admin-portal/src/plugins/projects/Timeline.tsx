// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useLayoutEffect, useRef, useState} from "react"
import {Box, Chip, Paper, Popper, Stack, Typography} from "@mui/material"
import moment from "moment-timezone"
import {useTranslation} from "react-i18next"
import {DAY, Saved, Task, TaskKind, TaskStatus, duration, formatDate, formatOffset} from "./model"

interface Props {
    tasks: Saved<Task>[]
    timezone: string
    selected: string | null
    editable: boolean
    /** Tasks with a write in progress; they cannot move again until it returns. */
    pending: string[]
    onSelect: (task: Saved<Task>) => void
    onMove: (task: Saved<Task>, days: number, resize: boolean) => void
}

const ROW_HEIGHT = 44
const BAR_HEIGHT = 22
const MIN_DAY_WIDTH = 2
const LABEL_CHAR_WIDTH = 7
const LABEL_PADDING = 16
const LABEL_GAP = 6
/** A pointer must travel this far before a press becomes a drag. */
const DRAG_THRESHOLD = 6
export const STATUS_COLORS: Record<TaskStatus, string> = {
    [TaskStatus.NotStarted]: "#64748B",
    [TaskStatus.InProgress]: "#292F99",
    [TaskStatus.Done]: "#0EB048",
}
export const MILESTONE_COLOR = "#837032"

const labelWidth = (text: string) => text.length * LABEL_CHAR_WIDTH + LABEL_PADDING

/** Puts an outside label on the side with room, shortening it if neither fits. */
const sideLabel = (text: string, from: number, to: number, total: number) => {
    const after = total - to - LABEL_GAP
    const before = from - LABEL_GAP
    const right = labelWidth(text) <= after || (labelWidth(text) > before && after >= before)
    return right
        ? {left: to + LABEL_GAP, maxWidth: Math.max(0, after - LABEL_GAP)}
        : {right: total - from + LABEL_GAP, maxWidth: Math.max(0, before - LABEL_GAP)}
}

const focusVisible = (element: Element): boolean => {
    try {
        return element.matches(":focus-visible")
    } catch {
        return true
    }
}

const useWidth = () => {
    const ref = useRef<HTMLDivElement>(null)
    const [width, setWidth] = useState(0)
    useLayoutEffect(() => {
        const element = ref.current
        if (!element) return
        setWidth(element.clientWidth)
        if (typeof ResizeObserver === "undefined") return
        const observer = new ResizeObserver(() => setWidth(element.clientWidth))
        observer.observe(element)
        return () => observer.disconnect()
    }, [])
    return [ref, width] as const
}

export const Timeline: React.FC<Props> = ({
    tasks,
    timezone,
    selected,
    editable,
    pending,
    onSelect,
    onMove,
}) => {
    const {t} = useTranslation()
    const [container, width] = useWidth()
    const [preview, setPreview] = useState<{id: string; anchor: HTMLElement} | null>(null)
    const [drag, setDrag] = useState<{id: string; dx: number} | null>(null)
    const pointer = useRef<{id: string; x: number; moved: boolean} | null>(null)
    const suppressClick = useRef(false)
    const buttons = useRef<Map<string, HTMLButtonElement>>(new Map())
    const touch = typeof window !== "undefined" && window.matchMedia?.("(pointer: coarse)").matches

    useEffect(() => {
        if (preview && !tasks.some((task) => task.id === preview.id)) setPreview(null)
    }, [preview, tasks])

    const instants = tasks.flatMap((task) => [
        Date.parse(task.data.start),
        Date.parse(task.data.end ?? task.data.start),
    ])
    const first = instants.length ? Math.min(...instants) : Date.now()
    const last = instants.length ? Math.max(...instants) : first
    const rangeStart = moment.tz(first, timezone).startOf("month")
    const rangeEnd = moment.tz(last, timezone).endOf("month")
    const days = Math.max(1, (rangeEnd.valueOf() - rangeStart.valueOf()) / DAY)
    const dayWidth = Math.max(MIN_DAY_WIDTH, (width || 1) / days)
    const total = days * dayWidth
    const x = (date: string) => ((Date.parse(date) - rangeStart.valueOf()) / DAY) * dayWidth

    const months: Array<{label: string; left: number; width: number}> = []
    for (const month = rangeStart.clone(); month.isBefore(rangeEnd); month.add(1, "month")) {
        const left = ((month.valueOf() - rangeStart.valueOf()) / DAY) * dayWidth
        const next = month.clone().add(1, "month")
        months.push({
            label: month.format("MMM YYYY"),
            left,
            width: ((next.valueOf() - month.valueOf()) / DAY) * dayWidth,
        })
    }
    const weeks: Array<{label: string; left: number}> = []
    const firstMonday = rangeStart.clone().add((8 - rangeStart.isoWeekday()) % 7, "day")
    for (const week = firstMonday; week.isBefore(rangeEnd); week.add(1, "week")) {
        weeks.push({
            label: week.format("D"),
            left: ((week.valueOf() - rangeStart.valueOf()) / DAY) * dayWidth,
        })
    }
    const showWeekNumbers = dayWidth * 7 >= 22

    const taskLabel = (task: Saved<Task>) =>
        task.data.type === TaskKind.Milestone
            ? t("projects.timeline.milestoneLabel", {
                  title: task.data.title,
                  at: formatDate(task.data.start, timezone),
                  status: t(`projects.status.${task.data.status}`),
              })
            : t("projects.timeline.taskLabel", {
                  title: task.data.title,
                  start: formatDate(task.data.start, timezone),
                  end: formatDate(task.data.end ?? task.data.start, timezone),
                  status: t(`projects.status.${task.data.status}`),
              })

    const focusSibling = (index: number) => {
        const target = tasks[Math.max(0, Math.min(tasks.length - 1, index))]
        if (target) buttons.current.get(target.id)?.focus()
    }

    const previewTask = preview ? tasks.find((task) => task.id === preview.id) : undefined
    const previewDuration = previewTask ? duration(previewTask.data) : null

    return (
        <Box component="section" aria-label={t("projects.timeline.title")}>
            <Stack
                direction={{xs: "column", md: "row"}}
                spacing={{xs: 1, md: 2}}
                alignItems={{md: "center"}}
                sx={{mb: 1.5}}
            >
                <Stack direction="row" spacing={2} flexWrap="wrap" useFlexGap sx={{fontSize: 12}}>
                    {Object.values(TaskStatus).map((status) => (
                        <Stack key={status} direction="row" spacing={0.75} alignItems="center">
                            <Box
                                sx={{
                                    width: 14,
                                    height: 10,
                                    borderRadius: 0.5,
                                    bgcolor: STATUS_COLORS[status],
                                }}
                            />
                            <Typography variant="caption">
                                {t(`projects.status.${status}`)}
                            </Typography>
                        </Stack>
                    ))}
                    <Stack direction="row" spacing={0.75} alignItems="center">
                        <Box
                            sx={{
                                width: 9,
                                height: 9,
                                bgcolor: MILESTONE_COLOR,
                                transform: "rotate(45deg)",
                            }}
                        />
                        <Typography variant="caption">{t("projects.type.milestone")}</Typography>
                    </Stack>
                </Stack>
                <Typography
                    variant="caption"
                    color="text.secondary"
                    sx={{ml: {md: "auto"}}}
                    id="projects-timeline-help"
                >
                    {t(
                        touch
                            ? editable
                                ? "projects.timeline.touchEditHelp"
                                : "projects.timeline.touchViewHelp"
                            : editable
                              ? "projects.timeline.editHelp"
                              : "projects.timeline.viewHelp"
                    )}
                </Typography>
            </Stack>
            <Box
                ref={container}
                sx={{
                    border: 1,
                    borderColor: "divider",
                    borderRadius: 1,
                    bgcolor: "background.paper",
                    overflowX: "auto",
                    overflowY: "hidden",
                    // Scroll inside; never widen the page to the timeline's length.
                    width: "100%",
                    contain: "inline-size",
                }}
            >
                <Box sx={{position: "relative", width: total, minWidth: "100%"}}>
                    <Box
                        aria-hidden
                        sx={{
                            height: 56,
                            borderBottom: 1,
                            borderColor: "divider",
                            position: "relative",
                        }}
                    >
                        {months.map((month) => (
                            <Typography
                                key={month.label}
                                variant="caption"
                                sx={{
                                    position: "absolute",
                                    left: month.left,
                                    width: month.width,
                                    height: 28,
                                    px: 1,
                                    lineHeight: "28px",
                                    fontWeight: 500,
                                    borderRight: 1,
                                    borderBottom: 1,
                                    borderColor: "divider",
                                    boxSizing: "border-box",
                                    overflow: "hidden",
                                    whiteSpace: "nowrap",
                                }}
                            >
                                {month.label}
                            </Typography>
                        ))}
                        {showWeekNumbers &&
                            weeks.map((week) => (
                                <Typography
                                    key={week.left}
                                    variant="caption"
                                    color="text.secondary"
                                    sx={{
                                        position: "absolute",
                                        top: 28,
                                        left: week.left + 4,
                                        lineHeight: "28px",
                                        fontSize: 11,
                                    }}
                                >
                                    {week.label}
                                </Typography>
                            ))}
                    </Box>
                    <Box
                        aria-hidden
                        sx={{
                            position: "absolute",
                            top: 56,
                            bottom: 0,
                            left: 0,
                            right: 0,
                            pointerEvents: "none",
                        }}
                    >
                        {weeks.map((week) => (
                            <Box
                                key={week.left}
                                sx={{
                                    position: "absolute",
                                    top: 0,
                                    bottom: 0,
                                    left: week.left,
                                    borderLeft: "1px solid",
                                    borderColor: "grey.200",
                                }}
                            />
                        ))}
                    </Box>
                    <Box
                        role="list"
                        aria-label={t("projects.timeline.tasks", {count: tasks.length})}
                    >
                        {tasks.map((task, index) => {
                            const left = x(task.data.start)
                            const milestone = task.data.type === TaskKind.Milestone
                            const barWidth = milestone
                                ? 0
                                : Math.max(8, x(task.data.end ?? task.data.start) - left)
                            const inside = !milestone && barWidth >= labelWidth(task.data.title)
                            const outsideText = milestone
                                ? `${task.data.title} · ${moment(task.data.start).tz(timezone).format("HH:mm")}`
                                : task.data.title
                            const outside = inside
                                ? null
                                : sideLabel(
                                      outsideText,
                                      milestone ? left - 9 : left,
                                      milestone ? left + 9 : left + barWidth,
                                      total
                                  )
                            const isSelected = selected === task.id
                            const offset = drag?.id === task.id ? drag.dx : 0
                            return (
                                <Box
                                    key={task.id}
                                    role="listitem"
                                    sx={{
                                        position: "relative",
                                        height: ROW_HEIGHT,
                                        borderBottom: 1,
                                        borderColor: "divider",
                                        bgcolor: isSelected
                                            ? "rgba(67, 227, 161, 0.14)"
                                            : "transparent",
                                    }}
                                >
                                    <Box
                                        component="button"
                                        type="button"
                                        ref={(element: HTMLButtonElement | null) => {
                                            if (element) buttons.current.set(task.id, element)
                                            else buttons.current.delete(task.id)
                                        }}
                                        data-task={task.id}
                                        aria-label={taskLabel(task)}
                                        aria-describedby="projects-timeline-help"
                                        aria-current={isSelected ? "true" : undefined}
                                        aria-busy={pending.includes(task.id) || undefined}
                                        aria-keyshortcuts={
                                            editable
                                                ? "ArrowLeft ArrowRight Shift+ArrowLeft Shift+ArrowRight"
                                                : undefined
                                        }
                                        onPointerEnter={(
                                            event: React.PointerEvent<HTMLButtonElement>
                                        ) => {
                                            if (event.pointerType !== "touch")
                                                setPreview({
                                                    id: task.id,
                                                    anchor: event.currentTarget,
                                                })
                                        }}
                                        onPointerLeave={() => setPreview(null)}
                                        onFocus={(event: React.FocusEvent<HTMLButtonElement>) => {
                                            if (focusVisible(event.currentTarget))
                                                setPreview({
                                                    id: task.id,
                                                    anchor: event.currentTarget,
                                                })
                                        }}
                                        onBlur={() => setPreview(null)}
                                        onClick={() => {
                                            if (suppressClick.current) {
                                                suppressClick.current = false
                                                return
                                            }
                                            setPreview(null)
                                            onSelect(task)
                                        }}
                                        onKeyDown={(
                                            event: React.KeyboardEvent<HTMLButtonElement>
                                        ) => {
                                            if (event.key === "Escape" && preview) {
                                                event.stopPropagation()
                                                setPreview(null)
                                            } else if (
                                                event.key === "ArrowDown" ||
                                                event.key === "ArrowUp"
                                            ) {
                                                event.preventDefault()
                                                focusSibling(
                                                    index + (event.key === "ArrowDown" ? 1 : -1)
                                                )
                                            } else if (
                                                editable &&
                                                (event.key === "ArrowLeft" ||
                                                    event.key === "ArrowRight")
                                            ) {
                                                event.preventDefault()
                                                if (pending.includes(task.id)) return
                                                onMove(
                                                    task,
                                                    event.key === "ArrowLeft" ? -1 : 1,
                                                    event.shiftKey
                                                )
                                            }
                                        }}
                                        onPointerDown={(
                                            event: React.PointerEvent<HTMLButtonElement>
                                        ) => {
                                            if (
                                                !editable ||
                                                event.button !== 0 ||
                                                pending.includes(task.id)
                                            )
                                                return
                                            pointer.current = {
                                                id: task.id,
                                                x: event.clientX,
                                                moved: false,
                                            }
                                            event.currentTarget.setPointerCapture?.(event.pointerId)
                                        }}
                                        onPointerMove={(
                                            event: React.PointerEvent<HTMLButtonElement>
                                        ) => {
                                            const origin = pointer.current
                                            if (!origin || origin.id !== task.id) return
                                            const dx = event.clientX - origin.x
                                            if (!origin.moved && Math.abs(dx) < DRAG_THRESHOLD)
                                                return
                                            origin.moved = true
                                            setPreview(null)
                                            setDrag({
                                                id: task.id,
                                                dx: Math.round(dx / dayWidth) * dayWidth,
                                            })
                                        }}
                                        onPointerCancel={() => {
                                            pointer.current = null
                                            setDrag(null)
                                        }}
                                        onPointerUp={(
                                            event: React.PointerEvent<HTMLButtonElement>
                                        ) => {
                                            const origin = pointer.current
                                            pointer.current = null
                                            setDrag(null)
                                            if (!origin?.moved) return
                                            suppressClick.current = true
                                            const moved = Math.round(
                                                (event.clientX - origin.x) / dayWidth
                                            )
                                            if (moved) onMove(task, moved, event.shiftKey)
                                        }}
                                        sx={{
                                            "position": "absolute",
                                            "top": milestone ? 13 : (ROW_HEIGHT - BAR_HEIGHT) / 2,
                                            "left": milestone ? left - 9 : left,
                                            "width": milestone ? 18 : barWidth,
                                            "height": milestone ? 18 : BAR_HEIGHT,
                                            "transform": `translateX(${offset}px)${milestone ? " rotate(45deg)" : ""}`,
                                            "p": 0,
                                            "px": inside ? 1 : 0,
                                            "border": 0,
                                            "borderRadius": milestone ? "2px" : 1,
                                            "bgcolor": milestone
                                                ? MILESTONE_COLOR
                                                : STATUS_COLORS[task.data.status],
                                            "color": "common.white",
                                            "font": "inherit",
                                            "fontSize": 12,
                                            "lineHeight": `${BAR_HEIGHT}px`,
                                            "textAlign": "left",
                                            "whiteSpace": "nowrap",
                                            "overflow": "hidden",
                                            "textOverflow": "ellipsis",
                                            "cursor": editable ? "grab" : "pointer",
                                            "touchAction": editable ? "none" : "auto",
                                            "boxShadow": isSelected
                                                ? "0 0 0 2px #fff, 0 0 0 4px #0F054C"
                                                : "none",
                                            "opacity": drag?.id === task.id ? 0.85 : 1,
                                            "&:focus-visible": {
                                                outline: "3px solid #43E3A1",
                                                outlineOffset: 2,
                                            },
                                        }}
                                    >
                                        {inside ? task.data.title : null}
                                    </Box>
                                    {outside && (
                                        <Typography
                                            aria-hidden
                                            variant="caption"
                                            sx={{
                                                position: "absolute",
                                                top: 12,
                                                ...outside,
                                                color: "text.primary",
                                                whiteSpace: "nowrap",
                                                overflow: "hidden",
                                                textOverflow: "ellipsis",
                                                pointerEvents: "none",
                                                transform: `translateX(${offset}px)`,
                                            }}
                                        >
                                            {outsideText}
                                        </Typography>
                                    )}
                                </Box>
                            )
                        })}
                    </Box>
                    {!tasks.length && (
                        <Typography sx={{p: 3}} color="text.secondary">
                            {t("projects.timeline.empty")}
                        </Typography>
                    )}
                </Box>
            </Box>
            <Popper
                open={Boolean(previewTask && preview)}
                anchorEl={preview?.anchor}
                placement="bottom-start"
                sx={{zIndex: 1300, pointerEvents: "none"}}
                aria-hidden
            >
                {previewTask && (
                    <Paper
                        elevation={6}
                        sx={{p: 1.5, mt: 1, maxWidth: 280}}
                        data-testid="projects-task-preview"
                    >
                        <Typography variant="subtitle2" color="primary">
                            {previewTask.data.title}
                        </Typography>
                        <Stack direction="row" spacing={1} alignItems="center" sx={{my: 0.75}}>
                            <Chip
                                size="small"
                                label={t(`projects.status.${previewTask.data.status}`)}
                            />
                            <Typography variant="caption" color="text.secondary">
                                {previewDuration
                                    ? (["days", "hours", "minutes"] as const)
                                          .filter((unit) => previewDuration[unit] > 0)
                                          .map((unit) =>
                                              t(`projects.units.${unit}`, {
                                                  count: previewDuration[unit],
                                              })
                                          )
                                          .join(" ") || t("projects.units.minutes", {count: 0})
                                    : t("projects.type.milestone")}
                            </Typography>
                        </Stack>
                        <Typography variant="body2" sx={{fontVariantNumeric: "tabular-nums"}}>
                            {formatDate(previewTask.data.start, timezone)}
                            {previewTask.data.end && (
                                <>
                                    {" "}
                                    →<br />
                                    {formatDate(previewTask.data.end, timezone)}
                                </>
                            )}{" "}
                            <Typography component="span" variant="caption" color="text.secondary">
                                (UTC{formatOffset(previewTask.data.start, timezone)})
                            </Typography>
                        </Typography>
                        <Typography
                            variant="caption"
                            color="text.secondary"
                            sx={{display: "block", mt: 0.75}}
                        >
                            {t(
                                editable
                                    ? "projects.timeline.previewEdit"
                                    : "projects.timeline.previewView"
                            )}
                        </Typography>
                    </Paper>
                )}
            </Popper>
        </Box>
    )
}
