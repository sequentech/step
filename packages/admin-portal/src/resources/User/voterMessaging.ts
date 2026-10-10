// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {ATTR_RESET_VALUE} from "@/types/keycloak"
import {
    EMessageChannel,
    MESSAGE_CHANNELS,
    VOTER_ATTR_MESSAGE_CHANNEL,
    VOTER_ATTR_MESSENGER_ID,
    VOTER_ATTR_MESSENGER_PAGE,
    VOTER_ATTR_VERIFIED_CHANNELS,
    VOTER_ATTR_VIBER_NUMBER,
    VOTER_ATTR_WHATSAPP_NUMBER,
} from "@/types/messaging"

export interface IVoterMessaging {
    preferredChannel?: EMessageChannel
    whatsappNumber?: string
    viberNumber?: string
    messengerConnected: boolean
    messengerPage?: string
    verifiedChannels: EMessageChannel[]
}

type Attributes = Record<string, unknown> | null | undefined

const MESSAGING_ATTRIBUTES = [
    VOTER_ATTR_MESSAGE_CHANNEL,
    VOTER_ATTR_WHATSAPP_NUMBER,
    VOTER_ATTR_VIBER_NUMBER,
    VOTER_ATTR_MESSENGER_ID,
    VOTER_ATTR_MESSENGER_PAGE,
    VOTER_ATTR_VERIFIED_CHANNELS,
]

export const isVoterMessagingAttribute = (name: string): boolean =>
    MESSAGING_ATTRIBUTES.includes(name)

const values = (attributes: Attributes, name: string): string[] => {
    const value = attributes?.[name]
    const list = Array.isArray(value) ? value : value === undefined || value === null ? [] : [value]
    return list
        .filter((item): item is string => typeof item === "string")
        .filter((item) => item !== "" && item !== ATTR_RESET_VALUE)
}

const first = (attributes: Attributes, name: string) => values(attributes, name)[0]

const asChannel = (value: string | undefined): EMessageChannel | undefined =>
    MESSAGE_CHANNELS.find((channel) => channel === value)

/** The messaging attributes Keycloak keeps on a voter. */
export const voterMessaging = (attributes: Attributes): IVoterMessaging => ({
    preferredChannel: asChannel(first(attributes, VOTER_ATTR_MESSAGE_CHANNEL)),
    whatsappNumber: first(attributes, VOTER_ATTR_WHATSAPP_NUMBER),
    viberNumber: first(attributes, VOTER_ATTR_VIBER_NUMBER),
    messengerConnected: !!first(attributes, VOTER_ATTR_MESSENGER_ID),
    messengerPage: first(attributes, VOTER_ATTR_MESSENGER_PAGE),
    verifiedChannels: values(attributes, VOTER_ATTR_VERIFIED_CHANNELS)
        .map(asChannel)
        .filter((channel): channel is EMessageChannel => channel !== undefined),
})
