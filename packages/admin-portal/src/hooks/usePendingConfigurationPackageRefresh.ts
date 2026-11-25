// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {useContext, useLayoutEffect} from "react"
import {useApolloClient} from "@apollo/client"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {GET_PACKAGE_TASK} from "@/queries/ApplyConfigurationPackage"
import {
    isFinished,
    mayApplyPackages,
    readApplication,
} from "@/resources/Settings/configurationPackage"
import {notifyConfigurationPackageElectionImported} from "./useConfigurationPackageElectionRefresh"

type ApplyScope = {tenantId: string; actor: string}
type Entry = ApplyScope & {taskId: string; addedAt: number; done: boolean}

const storageKey = "configurationPackage.pendingElectionRefresh"
const queuedEvent = "sequent:configuration-package-apply-queued"
const interval = 3000
const maxAge = 24 * 60 * 60 * 1000
const maxEntries = 12
let fallback: Entry[] = []
let writableStorage = true

function entries(): Entry[] {
    let raw: unknown
    try {
        raw = writableStorage ? JSON.parse(sessionStorage.getItem(storageKey) ?? "[]") : fallback
    } catch {
        raw = fallback
    }
    return Array.isArray(raw)
        ? raw
              .filter(
                  (e): e is Entry =>
                      !!e &&
                      typeof e.tenantId === "string" &&
                      typeof e.actor === "string" &&
                      typeof e.taskId === "string" &&
                      typeof e.done === "boolean" &&
                      Number.isFinite(e.addedAt) &&
                      e.addedAt <= Date.now() &&
                      Date.now() - e.addedAt < maxAge
              )
              .slice(-maxEntries)
        : []
}

function save(next: Entry[]) {
    fallback = next.slice(-maxEntries)
    try {
        sessionStorage.setItem(storageKey, JSON.stringify(fallback))
    } catch {
        // The current page still works when storage is unavailable.
    }
}

function matches(a: ApplyScope, b: ApplyScope) {
    return a.actor === b.actor && a.tenantId === b.tenantId
}

export function rememberApplyForElectionRefresh(scope: ApplyScope, taskId: string) {
    const pending = entries()
    if (pending.some((e) => matches(e, scope) && e.taskId === taskId)) return
    save([...pending, {...scope, taskId, addedAt: Date.now(), done: false}])
    window.dispatchEvent(new Event(queuedEvent))
}

function settle(scope: ApplyScope, taskId: string, imported: boolean) {
    const pending = entries()
    const previous = pending.find((e) => matches(e, scope) && e.taskId === taskId)
    if (previous?.done) return
    save([
        ...pending.filter((e) => !matches(e, scope) || e.taskId !== taskId),
        {...scope, taskId, addedAt: Date.now(), done: true},
    ])
    if (imported) notifyConfigurationPackageElectionImported(scope)
}

type Task = {id?: string; execution_status?: string; annotations?: unknown}

/** The Settings status display and persistent menu use the same single claim. */
export function publishCompletedPackageApply(scope: ApplyScope, id: string, task?: Task) {
    if (task?.id !== id || task.execution_status !== "SUCCESS") return false
    const view = readApplication(task.annotations)
    if (
        view?.mode !== "apply" ||
        view.outcome !== "applied" ||
        !view.sections.some(
            (section) => section.section === "election-event" && section.state === "applied"
        )
    ) {
        return false
    }
    settle(scope, id, true)
    return true
}

/** Lives in the navigation menu, not the Settings route. */
export function usePendingConfigurationPackageRefresh() {
    const [tenantId] = useTenantStore()
    const {userId, isAuthorized} = useContext(AuthContext)
    const client = useApolloClient()
    const allowed = !!userId && mayApplyPackages(tenantId ?? null, isAuthorized)
    useLayoutEffect(() => {
        if (!tenantId || !userId || !allowed) return
        const scope = {tenantId, actor: userId}
        let active = true
        let running = false
        let later: number | undefined
        const current = () => entries().filter((e) => !e.done && matches(e, scope))
        const tick = async () => {
            if (running || !active) return
            if (later !== undefined) window.clearTimeout(later)
            later = undefined
            const pending = current()
            if (!pending.length) return
            running = true
            await Promise.all(
                pending.map(async ({taskId}) => {
                    try {
                        const {data} = await client.query({
                            query: GET_PACKAGE_TASK,
                            variables: {tenantId, taskId},
                            fetchPolicy: "network-only",
                        })
                        if (!active) return
                        const task = data?.sequent_backend_tasks_execution?.[0] as Task | undefined
                        if (task?.id !== taskId || !isFinished(task.execution_status)) return
                        if (!publishCompletedPackageApply(scope, taskId, task)) {
                            settle(scope, taskId, false)
                        }
                    } catch {
                        // Retry transient errors only while this scope and entry are live.
                    }
                })
            )
            running = false
            if (active && current().length) later = window.setTimeout(tick, interval)
        }
        const wake = () => void tick()
        window.addEventListener(queuedEvent, wake)
        wake()
        return () => {
            active = false
            if (later !== undefined) window.clearTimeout(later)
            window.removeEventListener(queuedEvent, wake)
        }
    }, [client, tenantId, userId, allowed])
}
