// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {ApolloLink, Observable} from "@apollo/client"

export const BOARD_AUTHORIZATION_EVENT = "sequent:board-authorization-required"
export interface BoardAuthorizationPending {
    requestId: string
    resume: () => void
    cancel: () => void
}

/** Hasura action errors retain the server's typed signing requirement. */
export function requiredBoardRequest(errors: readonly any[] | undefined): string | undefined {
    for (const error of errors ?? []) {
        let payload: any = error
        const body = error.extensions?.internal?.response?.body
        if (body) {
            try {
                payload = typeof body === "string" ? JSON.parse(body) : body
            } catch {
                continue
            }
        }
        if (payload.extensions?.code === "signing-required") {
            const id = payload.extensions.signing_request?.id
            if (typeof id === "string") return id
        }
    }
}

/** Keep the original mutation callback (including an export's password) alive
 * while the requester collects signatures. Continue reuses the exact variables
 * and obtains a fresh token through the following authentication link. */
export function boardAuthorizationLink(): ApolloLink {
    return new ApolloLink(
        (operation, forward) =>
            new Observable((observer) => {
                let waiting = false
                let subscription: ReturnType<ReturnType<typeof forward>["subscribe"]> | undefined
                const send = () => {
                    waiting = false
                    subscription = forward(operation).subscribe({
                        next: (response) => {
                            const requestId = requiredBoardRequest(response.errors)
                            if (!requestId) {
                                observer.next(response)
                                return
                            }
                            waiting = true
                            window.dispatchEvent(
                                new CustomEvent<BoardAuthorizationPending>(
                                    BOARD_AUTHORIZATION_EVENT,
                                    {
                                        detail: {
                                            requestId,
                                            resume: () => {
                                                if (!observer.closed) send()
                                            },
                                            cancel: () =>
                                                observer.error(
                                                    new Error(
                                                        "Board approval is pending. Reopen the same operation after approval."
                                                    )
                                                ),
                                        },
                                    }
                                )
                            )
                        },
                        error: (error) => observer.error(error),
                        complete: () => {
                            if (!waiting) observer.complete()
                        },
                    })
                }
                send()
                return () => subscription?.unsubscribe()
            })
    )
}
