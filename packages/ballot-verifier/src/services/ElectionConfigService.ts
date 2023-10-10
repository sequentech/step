// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {IAnswer, IQuestion} from "sequent-core"

export const findUrlByTitle = (answer: IAnswer, urlTitle: string): string | undefined =>
    answer.urls.find((url) => urlTitle === url.title)?.url

export const checkIsWriteIn = (answer: IAnswer): boolean =>
    "true" === findUrlByTitle(answer, "isWriteIn")