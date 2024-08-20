// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
const tagalogTranslation = {
    translations: {
        welcome: "Kamusta <br/> <strong>World</strong>",
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
            step2: "Hakbang 2: Ipasok ang iyong ID ng balota",
            description2: "Mangyaring i-type ang ID ng balota na ibinigay sa Voting Portal:",
            ballotIdHelpDialog: {
                title: "Impormasyon: Ang iyong ID ng balota",
                ok: "OK",
                content: "Mangyaring i-type ang ID ng balota na ibinigay sa Voting Portal.",
            },
            startButton: "Mag-browse ng file",
            dragDropOption: "O i-drag at i-drop ito dito",
            importErrorDescription:
                "Nagkaroon ng problema sa pag-import ng nasusuri na balota. Pumili ka ba ng tamang file?",
            importErrorMoreInfo: "Higit pang impormasyon",
            importErrorTitle: "Error",
            useSampleLink: "Gamitin ang sample na balota",
            nextButton: "Susunod",
            ballotIdLabel: "ID ng Balota",
            ballotIdPlaceholder: "I-type ang iyong ID ng Balota",
            fileUploaded: "Na-upload",
        },
        confirmationScreen: {
            ballotIdTitle: "ID ng Balota",
            ballotIdDescription:
                "Sa ibaba, ipinapakita ng sistema ang ID ng balota ng na-decode na balota, at ang isa na nalikha ng verifier",
            ballotIdError: "Hindi tumutugma sa na-decode na ID ng balota",
            decodedBallotId: "Na-decode na ID ng Balota",
            decodedBallotIdHelpDialog: {
                title: "Impormasyon: Na-decode na ID ng Balota",
                ok: "OK",
                content:
                    "Ito ang ID ng balota na nabasa mula sa pag-decode ng nasusuri na file ng balota na iyong ibinigay.",
            },
            yourBallotId: "Ang ID ng Balota na iyong ibinigay",
            userBallotIdHelpDialog: {
                title: "Impormasyon: Ang ID ng Balota na iyong ibinigay",
                ok: "OK",
                content:
                    "Ito ang ID ng balota na iyong tinype sa nakaraang hakbang at na nakolekta mula sa Voting Booth.",
            },
            backButton: "Bumalik",
            printButton: "I-print",
            finishButton: "Na-verify",
            verifySelectionsTitle: "Suriin ang iyong mga pagpili sa balota",
            verifySelectionsDescription:
                "Ang mga sumusunod na pagpili sa balota ay na-decode mula sa balota na iyong in-import. Mangyaring suriin ang mga ito at tiyakin na tumutugma ang mga ito sa mga pagpili na ginawa mo sa Voting Portal. Kung ang iyong mga pagpili ay hindi tumutugma, mangyaring makipag-ugnayan sa mga awtoridad ng halalan...",
            verifySelectionsHelpDialog: {
                title: "Impormasyon: Suriin ang iyong mga pagpili sa balota",
                ok: "OK",
                content:
                    "Ang mga sumusunod na pagpili sa balota ay na-decode mula sa balota na iyong in-import. Mangyaring suriin ang mga ito at tiyakin na tumutugma ang mga ito sa mga pagpili na ginawa mo sa Voting Portal. Kung ang iyong mga pagpili ay hindi tumutugma, mangyaring makipag-ugnayan sa mga awtoridad ng halalan...",
            },
            markedInvalid: "Ang balota ay tahasang minarkahan bilang hindi wasto",
            points: "({{points}} Mga Punto)",
            contestNotFound: "Kumpetisyon hindi natagpuan: {{contestId}}",
        },
        poweredBy: "Pinapagana ng",
        errors: {
            encoding: {
                notEnoughChoices: "Hindi sapat na mga pagpipilian upang ma-decode",
                writeInChoiceOutOfRange: "Pagpipilian ng write-in labas ng saklaw: {{index}}",
                writeInNotEndInZero: "Ang write-in ay hindi nagtatapos sa 0",
                bytesToUtf8Conversion:
                    "Error sa pag-convert ng write-in mula sa bytes papuntang UTF-8 string: {{errorMessage}}",
                ballotTooLarge: "Ang balota ay mas malaki kaysa sa inaasahan",
            },
            implicit: {
                selectedMax:
                    "Bilang ng mga napiling pagpipilian {{numSelected}} ay higit sa maximum {{max}}",
                selectedMin:
                    "Bilang ng mga napiling pagpipilian {{numSelected}} ay mas mababa sa minimum {{min}}",
            },
            explicit: {
                notAllowed:
                    "Ang balota ay minarkahan nang tahasan bilang hindi wasto ngunit hindi ito pinapayagan ng tanong",
            },
        },
    },
}

export type TranslationType = typeof tagalogTranslation

export default tagalogTranslation
