// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {TemplateLabel} from "../Template"
import {CheckIcon} from "../icons"
import type {Text} from "./text"

export function CheckList({items}: {items: TemplateLabel[]}) {
    return (
        <ul className="auth-check">
            {items.map((item) => (
                <li key={item.text} lang={item.lang}>
                    <CheckIcon />
                    <span>{item.text}</span>
                </li>
            ))}
        </ul>
    )
}

export function AttemptsLeft({text, attemptsLeft}: {text: Text; attemptsLeft: number}) {
    const label =
        attemptsLeft === 1
            ? text("scanovateAttemptsLeftOne")
            : text("scanovateAttemptsLeft", String(attemptsLeft))
    return (
        <p className="auth-choice-note" lang={label.lang}>
            {label.text}
        </p>
    )
}
