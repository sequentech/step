// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {act, renderHook, waitFor} from "@testing-library/react"
import {ApolloClient, ApolloLink, InMemoryCache, Observable} from "@apollo/client"
import {ApolloProvider} from "@apollo/client/react"
import {useReceiptDocument} from "./useReceiptDocument"

it("polls only the owned receipt action and never caches its private download URL", async () => {
    let requests = 0
    const cache = new InMemoryCache()
    const client = new ApolloClient({
        cache,
        link: new ApolloLink((operation) => {
            expect(operation.operationName).toBe("ReceiptDocument")
            expect(operation.variables).toEqual({electionEventId: "event", documentId: "receipt"})
            expect(operation.getContext().headers).toEqual({"x-hasura-role": "cast-vote"})
            requests++
            return new Observable((observer) => {
                observer.next({
                    data: {
                        receipt_document: {
                            payload: {
                                document:
                                    requests === 1
                                        ? null
                                        : {
                                              id: "receipt",
                                              name: "receipt.pdf",
                                              url: "https://documents.invalid/private-receipt",
                                          },
                            },
                        },
                    },
                })
                observer.complete()
            })
        }),
    })
    const wrapper = ({children}: React.PropsWithChildren) => (
        <ApolloProvider client={client}>{children}</ApolloProvider>
    )
    const {result} = renderHook(() => useReceiptDocument("event", "receipt"), {wrapper})
    await waitFor(() => expect(result.current.data?.receipt_document.payload.document).toBeNull())
    await act(async () => {
        await result.current.refetch()
    })
    expect(result.current.data?.receipt_document.payload.document?.url).toBe(
        "https://documents.invalid/private-receipt"
    )
    expect(JSON.stringify(cache.extract())).not.toContain("private-receipt")
    expect(requests).toBe(2)
})
