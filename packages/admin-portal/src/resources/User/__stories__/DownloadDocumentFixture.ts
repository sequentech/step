// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The document boundary of DownloadDocument: the document row, its presigned
// URL and the anchor click that saves it. Widgets that export a file through
// DownloadDocument reuse these handlers and assert the recorded download.
import type {FetchResult, Operation} from "@apollo/client"
import {spyOn} from "storybook/test"
import {STORY_SETTINGS} from "@/__stories__/AdminStoryProvider"

export interface StoryDocument {
    name: string
    annotations?: Record<string, unknown>
}

/** The presigned address a stored document is downloaded from. */
export const documentUrl = (documentId: string) =>
    `${STORY_SETTINGS.PUBLIC_BUCKET_URL}documents/${documentId}`

/** GetDocument and FetchDocument answered from the given documents. */
export function documentHandlers(
    documents: Record<string, StoryDocument>
): Record<string, (operation: Operation) => FetchResult> {
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
    name: string
    href: string
}

/** Records the files a story saves instead of letting the browser follow the link. */
export function recordDownloads() {
    const downloads: RecordedDownload[] = []
    const click = spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(function (
        this: HTMLAnchorElement
    ) {
        downloads.push({name: this.download, href: this.href})
    })
    return {downloads, restore: () => click.mockRestore()}
}
