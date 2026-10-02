// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Box, SvgIcon, SvgIconProps} from "@mui/material"

// One outlined set for the messaging channels, drawn on a 24px grid with the
// stroke weight of Material's outlined icons. Material has no Viber or
// Messenger icon, and its WhatsApp and Facebook logos are filled.
export type ChannelIconName = "EMAIL" | "SMS" | "WHATSAPP" | "VIBER" | "MESSENGER"

export const CHANNEL_ICON_NAMES: ChannelIconName[] = [
    "EMAIL",
    "SMS",
    "WHATSAPP",
    "VIBER",
    "MESSENGER",
]

const PATHS: Record<ChannelIconName, string[]> = {
    EMAIL: [
        "M5.5 5h13A2.5 2.5 0 0 1 21 7.5v9a2.5 2.5 0 0 1-2.5 2.5h-13A2.5 2.5 0 0 1 3 16.5v-9A2.5 2.5 0 0 1 5.5 5Z",
        "m3.8 7.2 8.2 6 8.2-6",
    ],
    SMS: [
        "M5.5 3.5h13A2.5 2.5 0 0 1 21 6v9a2.5 2.5 0 0 1-2.5 2.5H11l-4.8 3.4a.4.4 0 0 1-.7-.3v-3.1A2.5 2.5 0 0 1 3 15V6a2.5 2.5 0 0 1 2.5-2.5Z",
        "M8 10.5h.01M12 10.5h.01M16 10.5h.01",
    ],
    WHATSAPP: [
        "M12 3a9 9 0 0 0-7.8 13.5L3 21l4.6-1.2A9 9 0 1 0 12 3Z",
        "M9.1 7.9c.4-.3.9-.2 1.1.2l.8 1.6c.2.4.1.8-.2 1.1l-.5.4a6.4 6.4 0 0 0 2.5 2.5l.4-.5c.3-.3.7-.4 1.1-.2l1.6.8c.4.2.5.7.2 1.1l-.5.6c-.6.6-1.5.8-2.3.4a9.7 9.7 0 0 1-4.7-4.7c-.4-.8-.2-1.7.4-2.3Z",
    ],
    VIBER: [
        "M12 3c-5 0-8.5 1.3-8.5 7.6 0 3.3 1 5.3 3 6.4V21l3.2-2.8c.7.1 1.5.1 2.3.1 5 0 8.5-1.3 8.5-7.7S17 3 12 3Z",
        "M8.5 8.4c.4-.3.9-.2 1.1.2l.7 1.3c.2.4.1.8-.2 1.1l-.4.4a5.4 5.4 0 0 0 2.1 2.1l.4-.4c.3-.3.7-.4 1.1-.2l1.3.7c.4.2.5.7.2 1.1l-.5.5c-.5.5-1.3.7-2 .4a8.2 8.2 0 0 1-4-4c-.3-.7-.1-1.5.4-2Z",
        "M13.2 7.4a2.6 2.6 0 0 1 2.2 2.2",
        "M13.2 5.7a4.3 4.3 0 0 1 3.8 3.8",
    ],
    MESSENGER: [
        "M12 3C7 3 3 6.7 3 11.4c0 2.6 1.2 4.9 3.2 6.4V21l2.9-1.6c.9.3 1.9.4 2.9.4 5 0 9-3.7 9-8.4S17 3 12 3Z",
        "m7.3 13.8 3.3-3.6 2.4 2.2 3.7-3.2-3.3 3.6-2.4-2.2-3.7 3.2Z",
    ],
}

export interface ChannelIconProps extends Omit<SvgIconProps, "children"> {
    channel: ChannelIconName
}

const ChannelIcon: React.FC<ChannelIconProps> = ({channel, sx, ...props}) => (
    <SvgIcon
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth={1.7}
        strokeLinecap="round"
        strokeLinejoin="round"
        data-channel={channel}
        sx={[{fill: "none"}, ...(Array.isArray(sx) ? sx : [sx])]}
        {...props}
    >
        {PATHS[channel].map((d) => (
            <path key={d} d={d} />
        ))}
    </SvgIcon>
)

export interface ChannelLabelProps {
    channel: ChannelIconName
    label: string
}

/** A channel's icon followed by its name, as channel columns show it. */
export const ChannelLabel: React.FC<ChannelLabelProps> = ({channel, label}) => (
    <Box
        component="span"
        sx={{display: "inline-flex", alignItems: "center", gap: 1, whiteSpace: "nowrap"}}
    >
        <ChannelIcon channel={channel} fontSize="small" />
        {label}
    </Box>
)

export default ChannelIcon
