// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import type {ErrorLike} from "@apollo/client"
import {ServerError, ServerParseError} from "@apollo/client/errors"

/** Apollo 4 returns transport errors directly, without an ApolloError wrapper. */
export function isApolloTransportError(error?: ErrorLike): boolean {
    return (
        !!error &&
        (ServerError.is(error) ||
            ServerParseError.is(error) ||
            error.name === "TypeError" ||
            error.name === "NetworkError")
    )
}
