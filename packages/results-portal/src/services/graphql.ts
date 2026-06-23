// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export interface GraphqlResponse<T> {
    data?: T
    errors?: Array<{message: string}>
}

export const graphqlFetch = async <T>(
    hasuraUrl: string,
    query: string,
    variables: Record<string, unknown>,
    token?: string
): Promise<T> => {
    const response = await fetch(hasuraUrl, {
        method: "POST",
        headers: {
            "Content-Type": "application/json",
            ...(token ? {Authorization: `Bearer ${token}`} : {}),
        },
        body: JSON.stringify({query, variables}),
    })

    if (!response.ok) {
        throw new Error(`GraphQL request failed with HTTP ${response.status}`)
    }

    const body = (await response.json()) as GraphqlResponse<T>

    if (body.errors?.length) {
        throw new Error(body.errors.map((error) => error.message).join("; "))
    }

    if (!body.data) {
        throw new Error("GraphQL request returned no data")
    }

    return body.data
}
