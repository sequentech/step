// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Document downloads and background tasks of the admin widgets. DownloadDocument
// looks a document up (GetDocument), asks Harvest for its address
// (FetchDocument) and saves it through a temporary link; stories answer both
// queries and record the link instead of following it.
import type {FetchResult, Operation} from "@apollo/client"
import {expect, spyOn, waitFor, within} from "storybook/test"
import {EVENT_ID, TENANT_ID} from "@/__stories__/AdminStoryProvider"
import {FIXED_TIME} from "@/__stories__/fixtures"

/** The presigned address a story's FetchDocument returns for a document. */
export const documentUrl = (documentId: string) =>
    `https://s3.admin-story.invalid/documents/${documentId}`

type Handler = (operation: Operation) => FetchResult

/** GetDocument and FetchDocument answers for the named documents; others are not found. */
export function documentHandlers(
    documents: Record<string, {name: string; annotations?: Record<string, unknown>}>
): Record<"GetDocument" | "FetchDocument", Handler> {
    return {
        GetDocument: ({variables}) => {
            const document = documents[String(variables.id)]
            return {
                data: {
                    sequent_backend_document: document
                        ? [{name: document.name, annotations: document.annotations ?? {}}]
                        : [],
                },
            }
        },
        FetchDocument: ({variables}) => ({
            data: {fetchDocument: {url: documentUrl(String(variables.documentId))}},
        }),
    }
}

export interface RecordedDownload {
    /** The file name the link asks the browser to save. */
    name: string
    href: string
}

/** Records the links the story clicks to save files; call `restore` in the cleanup. */
export function recordDownloads() {
    const downloads: RecordedDownload[] = []
    const spy = spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(function (
        this: HTMLAnchorElement
    ) {
        downloads.push({name: this.download, href: this.href})
    })
    return {downloads, restore: () => spy.mockRestore()}
}

/** GetTaskById answers of the task widget: every started task reports this status. */
export function taskHandler(
    status: "IN_PROGRESS" | "SUCCESS" | "FAILED",
    type: string,
    annotations: Record<string, unknown> = {}
): Record<"GetTaskById", Handler> {
    return {
        GetTaskById: ({variables}) => ({
            data: {
                sequent_backend_tasks_execution: [
                    {
                        id: variables.task_id,
                        tenant_id: TENANT_ID,
                        election_event_id: EVENT_ID,
                        execution_status: status,
                        type,
                        start_at: FIXED_TIME,
                        end_at: status === "IN_PROGRESS" ? null : FIXED_TIME,
                        logs: [{created_date: FIXED_TIME, log_text: `${type} ${status}`}],
                        annotations,
                        executed_by_user: "admin",
                    },
                ],
            },
        }),
    }
}

/** The `task_execution` a mutation returns when it starts a background task. */
export const startedTask = (id: string, type: string) => ({
    id,
    name: type,
    execution_status: "IN_PROGRESS",
    created_at: FIXED_TIME,
    start_at: FIXED_TIME,
    end_at: null,
    logs: [],
    annotations: {},
    labels: {},
    executed_by_user: "admin",
    tenant_id: TENANT_ID,
    election_event_id: EVENT_ID,
    type,
})

/**
 * Axe defects of the task widget that a started background task opens; the
 * status chip of a successful task also has white text below 4.5 contrast.
 */
export const taskWidgetDefects = (status: "SUCCESS" | "FAILED") => ({
    expectedFailure: {
        reason:
            "The task widget's icon buttons have no accessible name and sit inside its accordion summary button" +
            (status === "SUCCESS" ? "; its success chip has white text below 4.5 contrast." : "."),
        a11y: [
            "button-name",
            ...(status === "SUCCESS" ? ["color-contrast"] : []),
            "nested-interactive",
        ],
    },
})

/**
 * The export menus stay mounted when closed and hide only when their exit
 * transition ends; until then axe finds their focusable first item.
 */
export const exportMenuHidden = (id = "menu-export-election") =>
    waitFor(() => {
        const root = document.getElementById(id)
        if (!root) throw new Error(`Missing menu ${id}`)
        const menu = within(root).getByRole("menu", {hidden: true})
        expect(getComputedStyle(menu.parentElement ?? menu).visibility).toBe("hidden")
    })
