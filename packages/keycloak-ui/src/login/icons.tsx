// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {ReactNode} from "react"

function Icon({children}: {children: ReactNode}) {
    return (
        <svg
            width="24"
            height="24"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.7"
            strokeLinecap="round"
            strokeLinejoin="round"
            aria-hidden="true"
            focusable="false"
        >
            {children}
        </svg>
    )
}

export function ShieldIcon() {
    return (
        <Icon>
            <path d="m12 3 8 3v5c0 5-4.5 8.5-8 10-3.5-1.5-8-5-8-10V6l8-3Z" />
            <path d="m8.5 11.5 2.5 2.5 4.5-5" />
        </Icon>
    )
}

export function MessageIcon() {
    return (
        <Icon>
            <rect x="3" y="5" width="18" height="14" rx="3" />
            <path d="m4 7 8 6 8-6" />
        </Icon>
    )
}

export function ArrowIcon() {
    return (
        <Icon>
            <path d="M5 12h14m-6-6 6 6-6 6" />
        </Icon>
    )
}

export function GlobeIcon() {
    return (
        <Icon>
            <circle cx="12" cy="12" r="9" />
            <ellipse cx="12" cy="12" rx="4" ry="9" />
            <path d="M3 12h18" />
        </Icon>
    )
}

export function AccessibilityIcon() {
    return (
        <Icon>
            <circle cx="12" cy="4.5" r="1.5" />
            <path d="M4 8.5c5 1.3 11 1.3 16 0M12 9.5v5m0 0-3.5 6m3.5-6 3.5 6" />
        </Icon>
    )
}

export function EyeIcon({hidden}: {hidden: boolean}) {
    return (
        <Icon>
            <path d="M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12Z" />
            <circle cx="12" cy="12" r="3" />
            {hidden && <path d="m3 3 18 18" />}
        </Icon>
    )
}
