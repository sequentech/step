// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {EChannelSelection, EMessageChannel, IInstantMessageConfig} from "@/types/messaging"
import {
    IEmail,
    IMethods,
    ISmsConfig,
    ITemplateMethod,
    MESSAGE_TEMPLATE_METHODS,
} from "@/types/templates"
import {templateContentKey, templateOffersMethod} from "@/services/templateMethods"

export interface ISendContent {
    email?: IEmail
    sms?: ISmsConfig
    whatsapp?: IInstantMessageConfig
    viber?: IInstantMessageConfig
    messenger?: IInstantMessageConfig
}

export interface ISendTemplateState<Audience extends string, VoterId> {
    audienceSelection: Audience
    voterIds?: VoterId[]
    channelSelection: EChannelSelection
    communicationMethod: ITemplateMethod
    scheduleNow: boolean
    scheduleDate?: Date
    content: ISendContent
    secretAttributeNames: string[]
}

export interface ISendTemplatePayload<Audience extends string, VoterId> extends ISendContent {
    audience_selection: Audience
    audience_voter_ids?: VoterId[]
    channel_selection: EChannelSelection
    communication_method?: ITemplateMethod
    schedule_now: boolean
    schedule_date?: Date
    secret_attribute_names: string[]
}

const contentFor = (content: ISendContent, method: ITemplateMethod): ISendContent => {
    const key = templateContentKey(method)
    switch (key) {
        case "email":
        case "sms":
        case "whatsapp":
        case "viber":
        case "messenger":
            return {[key]: content[key]}
        case "document":
            return {}
    }
}

export const buildSendTemplatePayload = <Audience extends string, VoterId>(
    state: ISendTemplateState<Audience, VoterId>
): ISendTemplatePayload<Audience, VoterId> => {
    const single = state.channelSelection === EChannelSelection.SINGLE_CHANNEL
    return {
        audience_selection: state.audienceSelection,
        audience_voter_ids: state.voterIds,
        channel_selection: state.channelSelection,
        communication_method: single ? state.communicationMethod : undefined,
        schedule_now: state.scheduleNow,
        schedule_date: state.scheduleDate,
        ...(single ? contentFor(state.content, state.communicationMethod) : state.content),
        secret_attribute_names: state.secretAttributeNames,
    }
}

interface ITemplateRow {
    communication_method?: string | null
    template: {alias?: string; selected_methods?: IMethods | null}
}

/** Templates that can be sent with the chosen channel selection. */
export const templatesForChannelSelection = <Row extends ITemplateRow>(
    rows: Row[],
    selection: EChannelSelection,
    method: ITemplateMethod
): Row[] =>
    rows.filter((row) =>
        selection === EChannelSelection.SINGLE_CHANNEL
            ? templateOffersMethod(row, method)
            : MESSAGE_TEMPLATE_METHODS.some((candidate) => templateOffersMethod(row, candidate))
    )

const hasContent = (content: ISendContent, channel: EMessageChannel): boolean => {
    switch (channel) {
        case EMessageChannel.EMAIL:
            return !!content.email?.subject?.trim() || !!content.email?.plaintext_body?.trim()
        case EMessageChannel.SMS:
            return !!content.sms?.message?.trim()
        case EMessageChannel.WHATSAPP:
            return !!content.whatsapp?.message?.trim()
        case EMessageChannel.VIBER:
            return !!content.viber?.message?.trim()
        case EMessageChannel.MESSENGER:
            return !!content.messenger?.message?.trim()
    }
}

/** Channels among `channels` the content has nothing to send on. */
export const missingChannelContent = (
    content: ISendContent,
    channels: EMessageChannel[]
): EMessageChannel[] => channels.filter((channel) => !hasContent(content, channel))
