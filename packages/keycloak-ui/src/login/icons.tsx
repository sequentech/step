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

export function EyeIcon({hidden}: {hidden: boolean}) {
    return (
        <Icon>
            <path d="M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12Z" />
            <circle cx="12" cy="12" r="3" />
            {hidden && <path d="m3 3 18 18" />}
        </Icon>
    )
}

export function FaceScanIcon() {
    return (
        <Icon>
            <path d="M3 8V5.5A2.5 2.5 0 0 1 5.5 3H8M16 3h2.5A2.5 2.5 0 0 1 21 5.5V8M21 16v2.5a2.5 2.5 0 0 1-2.5 2.5H16M8 21H5.5A2.5 2.5 0 0 1 3 18.5V16" />
            <circle cx="12" cy="10" r="3" />
            <path d="M7.5 17c.9-2 2.5-3 4.5-3s3.6 1 4.5 3" />
        </Icon>
    )
}

export function IdCardIcon() {
    return (
        <Icon>
            <rect x="2.5" y="5" width="19" height="14" rx="2.5" />
            <circle cx="8.5" cy="11" r="2.2" />
            <path d="M5.5 16c.6-1.6 1.7-2.4 3-2.4s2.4.8 3 2.4M14 10h4.5M14 13.5h3" />
        </Icon>
    )
}

export function FaceIcon() {
    return (
        <Icon>
            <path d="M3 8V5.5A2.5 2.5 0 0 1 5.5 3H8M16 3h2.5A2.5 2.5 0 0 1 21 5.5V8M21 16v2.5a2.5 2.5 0 0 1-2.5 2.5H16M8 21H5.5A2.5 2.5 0 0 1 3 18.5V16" />
            <path d="M9 10h.01M15 10h.01M9.5 15c1.4 1 3.6 1 5 0" />
        </Icon>
    )
}

export function VideoIcon() {
    return (
        <Icon>
            <rect x="2.5" y="6" width="13" height="12" rx="2.5" />
            <path d="m15.5 10.5 6-3.5v10l-6-3.5" />
        </Icon>
    )
}

export function CheckIcon() {
    return (
        <Icon>
            <path d="m5 12.5 4.5 4.5L19 7.5" />
        </Icon>
    )
}

export function CheckCircleIcon() {
    return (
        <Icon>
            <circle cx="12" cy="12" r="9" />
            <path d="m8 12.5 2.8 2.8L16.5 9.5" />
        </Icon>
    )
}

export function WarningIcon() {
    return (
        <Icon>
            <path d="M12 3.5 2.5 20h19L12 3.5Z" />
            <path d="M12 10v4.5M12 17.5h.01" />
        </Icon>
    )
}

export function LockIcon() {
    return (
        <Icon>
            <rect x="4.5" y="10.5" width="15" height="10" rx="2.5" />
            <path d="M8 10.5V7.5a4 4 0 0 1 8 0v3" />
        </Icon>
    )
}

export function CloseIcon() {
    return (
        <Icon>
            <path d="M6 6l12 12M18 6 6 18" />
        </Icon>
    )
}

export function HelpIcon() {
    return (
        <Icon>
            <circle cx="12" cy="12" r="9" />
            <path d="M9.5 9.5a2.5 2.5 0 1 1 3.5 2.3c-.6.3-1 .9-1 1.6v.6M12 17h.01" />
        </Icon>
    )
}

export function LightIcon() {
    return (
        <Icon>
            <circle cx="12" cy="12" r="4" />
            <path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4" />
        </Icon>
    )
}

export function CopyIcon() {
    return (
        <Icon>
            <rect x="8" y="8" width="12" height="12" rx="2" />
            <path d="M16 8V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h2" />
        </Icon>
    )
}

export function CameraOffIcon() {
    return (
        <Icon>
            <path d="M3 3l18 18" />
            <path d="M9.5 5.5h5l1.5 2h2.5A2.5 2.5 0 0 1 21 10v7M18.5 20h-13A2.5 2.5 0 0 1 3 17.5V10a2.5 2.5 0 0 1 2.5-2.5" />
            <path d="M10 11a3 3 0 0 0 4 4" />
        </Icon>
    )
}
