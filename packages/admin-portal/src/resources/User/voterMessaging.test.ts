// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {EMessageChannel} from "@/types/messaging"
import {isVoterMessagingAttribute, voterMessaging} from "./voterMessaging"

describe("voterMessaging", () => {
    it("reads the preferred channel, numbers and Messenger link", () => {
        expect(
            voterMessaging({
                "sequent.read-only.message-channel": ["WHATSAPP"],
                "sequent.read-only.whatsapp-number": ["+966501234567"],
                "sequent.read-only.viber-number": ["+971552345678"],
                "sequent.read-only.messenger-id": ["7304419925518842"],
                "sequent.read-only.messenger-page": ["118204557331906"],
                "sequent.read-only.verified-channels": ["WHATSAPP", "EMAIL", "BOGUS"],
            })
        ).toEqual({
            preferredChannel: EMessageChannel.WHATSAPP,
            whatsappNumber: "+966501234567",
            viberNumber: "+971552345678",
            messengerConnected: true,
            messengerPage: "118204557331906",
            verifiedChannels: [EMessageChannel.WHATSAPP, EMessageChannel.EMAIL],
        })
    })

    it("reports nothing set for a voter without messaging attributes", () => {
        expect(voterMessaging(undefined)).toEqual({
            preferredChannel: undefined,
            whatsappNumber: undefined,
            viberNumber: undefined,
            messengerConnected: false,
            messengerPage: undefined,
            verifiedChannels: [],
        })
    })

    it("ignores an unknown or reset preferred channel", () => {
        expect(
            voterMessaging({"sequent.read-only.message-channel": ["NONE"]}).preferredChannel
        ).toBeUndefined()
        expect(
            voterMessaging({"sequent.read-only.messenger-id": ["NONE"]}).messengerConnected
        ).toBe(false)
    })

    it("recognises the messaging attributes", () => {
        expect(isVoterMessagingAttribute("sequent.read-only.viber-number")).toBe(true)
        expect(isVoterMessagingAttribute("sequent.read-only.mobile-number")).toBe(false)
    })
})
