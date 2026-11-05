// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The document boundary shared by the admin widgets that save files: a widget
// looks a document up (GetDocument), asks Harvest for its presigned address
// (FetchDocument) and saves it through a temporary link, which stories record
// instead of following.
import type {FetchResult, Operation} from "@apollo/client"
import {spyOn} from "storybook/test"

export interface StoryDocument {
    name: string
    annotations?: Record<string, unknown>
}

/** The presigned address FetchDocument returns for a stored document. */
export const documentUrl = (documentId: string) =>
    `https://s3.admin-story.invalid/documents/${documentId}`

/** GetDocument and FetchDocument answered from the given documents; others are not found. */
export function documentHandlers(
    documents: Record<string, StoryDocument>
): Record<"GetDocument" | "FetchDocument", (operation: Operation) => FetchResult> {
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
            data: {
                fetchDocument: documents[String(variables.documentId)]
                    ? {url: documentUrl(String(variables.documentId))}
                    : null,
            },
        }),
    }
}

export interface RecordedDownload {
    /** The file name the link asks the browser to save. */
    name: string
    href: string
}

/** Records the links a story clicks to save files; call `restore` in the story's cleanup. */
export function recordDownloads() {
    const downloads: RecordedDownload[] = []
    const click = spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(function (
        this: HTMLAnchorElement
    ) {
        downloads.push({name: this.download, href: this.href})
    })
    return {downloads, restore: () => click.mockRestore()}
}
