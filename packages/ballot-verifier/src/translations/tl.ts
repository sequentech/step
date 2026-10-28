// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
const tagalogTranslation = {
    translations: {
        404: {
            title: "Hindi natagpuan ang pahina",
            subtitle: "Ang pahinang hinahanap mo ay wala",
        },
        homeScreen: {
            step1: "Hakbang 1: I-import ang iyong balota",
            description1:
                "Upang magpatuloy, mangyaring i-import ang naka-encrypt na data ng balota na ibinigay sa Voting Portal:",
            importBallotHelpDialog: {
                title: "Impormasyon: I-import ang iyong balota",
                ok: "OK",
                content:
                    "Upang magpatuloy, mangyaring i-import ang naka-encrypt na data ng balota na ibinigay sa Voting Portal.",
            },
            step2: "Hakbang 2: I-type ang ID ng iyong balota",
            description2: "Mangyaring i-type ang ID ng balota na ibinigay sa Voting Portal:",
            ballotIdHelpDialog: {
                title: "Impormasyon: Ang ID ng iyong balota",
                ok: "OK",
                content: "Mangyaring i-type ang ID ng balota na ibinigay sa Voting Portal.",
            },
            startButton: "Mag-browse ng file",
            dragDropOption: "O i-drag at i-drop ito dito",
            importErrorDescription:
                "Nagkaroon ng problema sa pag-import ng sinisiyasat na balota. Tama ba ang napili mong file?",
            importErrorMoreInfo: "Karagdagang impormasyon",
            importErrorTitle: "Error",
            ciphertextErrorTitle: "Nabigo ang pagberipika ng balota",
            ciphertextErrorDescription:
                "Ang ciphertext ng sinisiyasat na balotang ito ay hindi ang pag-encrypt ng plaintext at randomness na nilalaman nito. Hindi mapagkakatiwalaan ang balota.",
            useSampleLink: "Gamitin ang sample na balota",
            nextButton: "Susunod",
            ballotIdLabel: "ID ng Balota",
            ballotIdPlaceholder: "I-type ang ID ng iyong Balota",
            fileUploaded: "Na-upload",
            receiptTitle: "Resibo ng transparency",
            receiptDescription:
                "Naglalathala ang halalang ito ng resibo para sa bawat balotang naibigay. Suriin ang sa iyo: na ito ay nasa pampublikong talaan, hindi binago at pareho para sa lahat.",
            receiptButton: "Suriin ang isang resibo",
        },
        confirmationScreen: {
            ballotIdTitle: "ID ng Balota",
            ballotIdDescription:
                "Sa ibaba, ipinapakita ng sistema ang ID ng na-decode na balota, at ang nalikha ng verifier",
            ballotIdError: "Hindi tumutugma sa na-decode na ID ng balota",
            decodedBallotId: "Na-decode na ID ng Balota",
            decodedBallotIdHelpDialog: {
                title: "Impormasyon: Na-decode na ID ng Balota",
                ok: "OK",
                content:
                    "Ito ang ID ng balota na nabasa mula sa pag-decode ng sinisiyasat na file ng balota na iyong ibinigay.",
            },
            yourBallotId: "Ang ID ng Balota na iyong ibinigay",
            userBallotIdHelpDialog: {
                title: "Impormasyon: Ang ID ng Balota na iyong ibinigay",
                ok: "OK",
                content:
                    "Ito ang ID ng balota na iyong tinype sa nakaraang hakbang at nakuha mo mula sa Voting Booth.",
            },
            backButton: "Bumalik",
            printButton: "I-print",
            finishButton: "Na-verify",
            verifySelectionsTitle: "Suriin ang iyong mga napiling balota",
            verifySelectionsDescription:
                "Ang mga susunod na piling balota ay na-decode mula sa balota na iyong in-import. Suriin ang mga ito at tiyakin na tumutugma ang mga ito sa mga napili mo sa Voting Portal. Kung ang iyong mga napili ay hindi tumutugma, mangyaring makipag-ugnayan sa mga awtoridad ng halalan...",
            verifySelectionsHelpDialog: {
                title: "Impormasyon: Suriin ang iyong mga napiling balota",
                ok: "OK",
                content:
                    "Ang mga susunod na piling balota ay na-decode mula sa balota na iyong in-import. Suriin ang mga ito at tiyakin na tumutugma ang mga ito sa mga napili mo sa Voting Portal. Kung ang iyong mga napili ay hindi tumutugma, mangyaring makipag-ugnayan sa mga awtoridad ng halalan...",
            },
            markedInvalid: "Ang balota ay tahasang minarkahan bilang hindi wasto",
            points_one: "({{count}} Mga Punto)",
            points_many: "({{count}} Mga Punto)",
            points_other: "({{count}} Mga Punto)",
            contestNotFound: "Paligsahan hindi natagpuan: {{contestId}}",
            declineToVote: "Hindi bumoto",
            blankBallot: "Blangkong balota",
            acclamationDescription:
                "Ang paligsahang ito ay napagpasyahan sa pamamagitan ng aklamasyon. Ang mga kandidato nito ay nahalal nang walang botohan, kaya walang opsyong mapipili at walang botong naitatala.",
        },
        receiptScreen: {
            title: "Suriin ang isang resibo ng transparency",
            description:
                "Ipinapakita ng resibo ng transparency na ang iyong balota ay nasa pampublikong talaan ng halalan, hindi binago at pareho para sa lahat. Sinusuri ito ng pahinang ito sa iyong browser, gamit ang trust policy na inilathala ng awtoridad ng halalan.",
            loading: "Nilo-load ang pansuri ng resibo…",
            loadFailed:
                "Hindi ma-load ang pansuri ng resibo. I-reload ang pahina upang subukang muli.",
            receipt: {
                title: "Ang iyong resibo",
                description:
                    "Buksan ang file ng resibo na na-save mo pagkatapos bumoto, o i-paste ang resibo, ang link nito o ang reference nito (ang teksto ng QR code nito).",
                fileFormat: "Ang mga file ng resibo ay nagtatapos sa .vitrine-receipt",
                pasteLabel: "Resibo, link o reference",
                pastePlaceholder: "I-paste ito dito",
                open: "Buksan",
                another: "Suriin ang ibang resibo",
                problems: {
                    "empty": "Mag-paste muna ng resibo, ng link nito o ng reference nito.",
                    "too-large": "Masyadong malaki ito para maging resibo.",
                    "bad-link":
                        "Walang mababasang resibo sa link na ito. Buksan na lang ang file ng resibo.",
                    "bad-reference":
                        "Kulang o sira ang reference na ito. Kopyahin itong muli, o buksan ang file ng resibo.",
                    "bad-chain": "Sira ang resibong ito: hindi ito mabasa.",
                    "not-https":
                        "Sinasabi ng link na ang policy ay nasa address na hindi secure (https), kaya hindi ito ginagamit.",
                    "wrong-box": "Isa itong trust policy, hindi resibo. Buksan muna ang resibo.",
                    "unrecognised": "Hindi ito resibo, link ng resibo o reference ng resibo.",
                    "unreadable": "Hindi mabasa ang file.",
                },
            },
            policy: {
                title: "Trust policy",
                description:
                    "Sinusuri ang mga resibo gamit ang trust policy na inilathala ng awtoridad ng halalan. Ibigay ito dito, bilang teksto nito o https address nito, mula sa opisyal na abiso ng halalan.",
                fromLink:
                    "Sinasabi ng link ng resibo kung nasaan ang trust policy nito. Bago mo ito i-pin sa device na ito, ihambing ang mga salita at ID nito sa opisyal na abiso ng halalan.",
                inputLabel: "Trust policy o ang address nito",
                inputPlaceholder: "I-paste ang policy, o ang https address nito",
                load: "I-load ang policy",
                openFile: "Magbukas ng file ng policy",
                loading: "Nilo-load ang trust policy…",
                pin: "Tugma sila: i-pin ang policy na ito",
                reject: "Hindi sila tugma",
                rejected:
                    "Huwag gamitin ang policy na ito. Sabihan ang awtoridad ng halalan, at ibigay ang policy mula sa opisyal na abiso ng halalan.",
                pinned: "Sinuri gamit ang trust policy na in-pin mo sa device na ito.",
                unpin: "Kalimutan ang policy na ito",
                draft: "Isa itong draft ng policy, hindi ang selyadong policy, kaya hindi ito mai-pin. Ang mga draft na salita nito:",
                invalid: "Hindi ito wastong trust policy.",
                otherPolicy: "Wasto ang trust policy na ito, pero para sa ibang policy ang resibo.",
                ambiguous:
                    "Higit sa isang policy na in-pin mo ang tumutugma sa reference na ito, kaya hindi ito ginagamit. Buksan na lang ang file ng resibo.",
                problems: {
                    "empty": "I-paste muna ang trust policy o ang address nito.",
                    "too-large": "Masyadong malaki ito para maging trust policy.",
                    "not-https": "Ang mga address lang na nagsisimula sa https ang ginagamit.",
                    "wrong-box": "Isa itong resibo, hindi trust policy.",
                    "unrecognised": "Hindi ito trust policy o ang address nito.",
                    "unreadable": "Hindi mabasa ang file.",
                },
                fetchProblems: {
                    "not-https": "Ang mga address lang na nagsisimula sa https ang ginagamit.",
                    "unreachable":
                        "Hindi ma-load ang trust policy. Suriin ang address, o i-paste na lang ang policy.",
                    "status": "Hindi ibinalik ng server ang trust policy (status {{status}}).",
                    "too-large": "Masyadong malaki ito para maging trust policy.",
                    "not-text": "Hindi nagbalik ng trust policy ang address.",
                },
            },
            ballot: {
                title: "Ang iyong balota",
                description:
                    "Ipinapakita lang na sa iyo ang resibo para sa balotang iyong ibinoto. I-type ang ID ng balota na ipinakita sa iyo ng voting portal.",
                ballotId: "ID ng balota",
                ballotIdPlaceholder: "I-type ang ID ng iyong balota",
                election: "ID ng halalan",
                electionPlaceholder: "Piliin ang halalan",
                event: "ID ng election event",
                check: "Suriin ang resibo",
                change: "Palitan ang ID ng balota",
                invalid:
                    "Suriin ang ID ng balota: mayroon itong 64 na character, mga digit at ang mga titik a hanggang f.",
                lookedFor: "Hinahanap na ID ng balota:",
            },
            result: {
                title: "Resulta",
                checking: "Sinusuri…",
                rounds: {
                    log: "Sinuri gamit ang mga log ng halalan. Sa loob ng susunod na kalahating oras, tatanungin din ng pahinang ito ang mga mirror ng mga log, ang mga anchor hub at ang mga monitor, sa random na sandali, upang hindi nila malaman kung kailan mo binuksan ang iyong resibo. Panatilihing bukas ang pahina, o suriin na ang lahat ngayon.",
                    full: "Sinuri gamit ang mga log ng halalan, ang mga mirror nito, ang mga anchor hub at ang mga monitor.",
                    offline:
                        "Pribado ang mga log ng halalang ito, kaya sinuri ang resibo gamit ang laman nito.",
                },
                checkEverything: "Suriin ang lahat ngayon",
                checkAgain: "Suriin muli",
                retry: "Subukang muli",
                problems: {
                    "policy-mismatch": "Para sa ibang trust policy ang reference na ito.",
                    "unknown-log": "Walang log na may ganitong pangalan ang trust policy.",
                    "quiet-period":
                        "Hindi nagbibigay ng mga patunay ang log sa panahon ng quiet period nito. Subukang muli pagkatapos ng {{time}}.",
                    "not-found":
                        "Wala pa sa log ang entry na ito. Kung kababoto mo lang, subukang muli mamaya.",
                    "entry-mismatch":
                        "Ang entry ng log sa posisyong ito ay hindi ang nasa reference.",
                    "private-reference":
                        "Pribado ang mga log ng halalang ito, kaya hindi masusuri dito ang isang reference. Buksan na lang ang file ng resibo.",
                    "failed": "Hindi masuri ang resibo. Subukang muli.",
                },
                evidenceSaved:
                    "Na-save ang file ng resibo. Naglalaman ito ng mga pinirmahang object na nagkakasalungatan: masusuri ito ng isang auditor, at makagagawa mula rito ng evidence file, gamit ang Vitrine verifier.",
                advice: {
                    failed: "Bumagsak ang isang pagsusuri: may mga pinirmahang data na nagkakasalungatan, o salungat sa trust policy. I-save ang ebidensya at ibigay ito, kasama ang file ng resibo, sa awtoridad ng halalan at sa mga tagamasid na pinangalanan ng trust policy.",
                    late: "May dapat nang naroon na wala pa. Maaaring mabagal lang ito: suriin muli mamaya, at kung huli pa rin, sabihan ang awtoridad ng halalan. Itago ang file ng resibo.",
                },
            },
        },
        footer: {
            poweredBy: "Pinapagana ng <1></1>",
        },
        errors: {
            encoding: {
                notEnoughChoices: "Hindi sapat na mga pagpipilian upang ma-decode",
                writeInChoiceOutOfRange: "Pagpipilian ng write-in labas ng saklaw: {{index}}",
                writeInNotEndInZero: "Ang write-in ay hindi nagtatapos sa 0",
                bytesToUtf8Conversion:
                    "Error sa pag-convert ng write-in mula sa bytes papuntang UTF-8 string: {{errorMessage}}",
                ballotTooLarge: "Ang balota ay mas malaki kaysa sa inaasahan",
            },
            explicit: {
                notAllowed:
                    "Ang balota ay minarkahan nang tahasan bilang hindi wasto ngunit hindi ito pinapayagan ng paligsahan",
            },
        },
    },
}

export type TranslationType = typeof tagalogTranslation

export default tagalogTranslation
