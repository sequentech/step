// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {ApolloLink, Observable, execute, gql} from "@apollo/client"
import {
    boardAuthorizationLink,
    requiredBoardRequest,
    type BoardAuthorizationPending,
} from "./boardAuthorizationLink"

describe("board approval error contract", () => {
    const pending = {extensions: {code: "signing-required", signing_request: {id: "request-123"}}}
    it("opens the exact typed request returned by an action", () => {
        expect(requiredBoardRequest([pending])).toBe("request-123")
    })
    it("also reads a structured Hasura action response body", () => {
        expect(
            requiredBoardRequest([
                {extensions: {internal: {response: {body: JSON.stringify(pending)}}}},
            ])
        ).toBe("request-123")
    })
    it("never treats a generic denial or an untyped id as authorisation", () => {
        expect(
            requiredBoardRequest([
                {message: "forbidden", extensions: {signing_request: {id: "request-123"}}},
            ])
        ).toBeUndefined()
        expect(
            requiredBoardRequest([{extensions: {internal: {response: {body: "not JSON"}}}}])
        ).toBeUndefined()
    })
})

describe("pending operation lifecycle", () => {
    const originalWindow = (globalThis as any).window
    const originalCustomEvent = (globalThis as any).CustomEvent
    let pending: BoardAuthorizationPending | undefined
    beforeEach(() => {
        pending = undefined
        ;(globalThis as any).CustomEvent = class {
            detail: unknown
            constructor(_type: string, options: any) {
                this.detail = options.detail
            }
        }
        ;(globalThis as any).window = {
            dispatchEvent(event: any) {
                pending = event.detail
                return true
            },
        }
    })
    afterEach(() => {
        ;(globalThis as any).window = originalWindow
        ;(globalThis as any).CustomEvent = originalCustomEvent
    })
    it("keeps the original export callback pending and resumes exactly the approved parameters", async () => {
        const variables = {event: "event-a", document: "exact-upload"}
        const seen: unknown[] = []
        const transport = new ApolloLink(
            (operation) =>
                new Observable((observer) => {
                    seen.push(operation.variables)
                    observer.next(
                        seen.length === 1
                            ? {
                                  errors: [
                                      {
                                          message: "Pending",
                                          extensions: {
                                              code: "signing-required",
                                              signing_request: {id: "request-a"},
                                          },
                                      },
                                  ] as any,
                              }
                            : {
                                  data: {
                                      export: {
                                          document: "result-a",
                                          password: "one-use-export-password",
                                      },
                                  },
                              }
                    )
                    observer.complete()
                })
        )
        const delivered: unknown[] = []
        const completion = new Promise<void>((resolve, reject) => {
            execute(ApolloLink.from([boardAuthorizationLink(), transport]), {
                query: gql`
                    mutation Export($event: String!, $document: String!) {
                        export(event: $event, document: $document) {
                            document
                            password
                        }
                    }
                `,
                variables,
            }).subscribe({
                next: (result) => delivered.push(result),
                error: reject,
                complete: resolve,
            })
        })
        await new Promise((resolve) => setImmediate(resolve))
        expect(delivered).toEqual([])
        expect(pending?.requestId).toBe("request-a")
        pending!.resume()
        await completion
        expect(seen).toEqual([variables, variables])
        expect(delivered).toEqual([
            {data: {export: {document: "result-a", password: "one-use-export-password"}}},
        ])
    })
    it("closing an unfinished approval cancels the waiting caller without dispatching work", async () => {
        let attempts = 0
        const transport = new ApolloLink(
            () =>
                new Observable((observer) => {
                    attempts++
                    observer.next({
                        errors: [
                            {
                                message: "Pending",
                                extensions: {
                                    code: "signing-required",
                                    signing_request: {id: "request-a"},
                                },
                            },
                        ] as any,
                    })
                    observer.complete()
                })
        )
        const failure = new Promise<Error>((resolve, reject) => {
            execute(ApolloLink.from([boardAuthorizationLink(), transport]), {
                query: gql`
                    mutation {
                        protectedOperation
                    }
                `,
            }).subscribe({
                next: () => reject(new Error("Pending operation released a result")),
                error: resolve,
                complete: () => reject(new Error("Pending operation completed")),
            })
        })
        await new Promise((resolve) => setImmediate(resolve))
        pending!.cancel()
        expect((await failure).message).toContain("Board approval is pending")
        expect(attempts).toBe(1)
    })
})
