// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {TranslationType} from "./en"

const tagalogTranslation: TranslationType = {
    translations: {
        language: "Tagalog",
        welcome: "Kamusta <br/> <strong>Mundo</strong>",
        breadcrumbSteps: {
            select: "Pumili ng Verifier",
            import: "I-import ang Data",
            verify: "I-verify",
            finish: "Tapusin",
        },
        electionEventBreadcrumbSteps: {
            created: "Nalikha",
            keys: "Mga Susi",
            publish: "I-publish",
            started: "Nagsimula",
            ended: "Natapos",
            results: "Mga Resulta",
        },
        candidate: {
            moreInformationLink: "Karagdagang impormasyon",
            writeInsPlaceholder: "I-type ang pangalan ng write-in na kandidato dito",
            blankVote: "Blangkong Boto",
        },
        homeScreen: {
            title: "Sequent Ballot Verifier",
            description1:
                "Ginagamit ang ballot verifier kapag pinili ng botante na i-audit ang balota sa voting booth. Ang pag-verify ay dapat tumagal ng 1-2 minuto.",
            description2:
                "Ang ballot verifier ay nagbibigay-daan sa botante na tiyakin na ang naka-encrypt na balota ay tama ang pagkakasalaysay ng mga pagpipilian sa voting booth. Ang pagsasagawa ng pag-suri na ito ay tinatawag na cast-as-intended verifiability at pumipigil sa mga error at masamang aktibidad sa panahon ng pag-encrypt ng balota.",
            descriptionMore: "Matuto pa",
            startButton: "Mag-browse ng file",
            dragDropOption: "O i-drag at i-drop ito dito",
            importErrorDescription:
                "Nagkaroon ng problema sa pag-import ng auditable ballot. Sigurado ka bang tamang file ang napili?",
            importErrorMoreInfo: "Karagdagang impormasyon",
            importErrorTitle: "Error",
            useSampleText: "Wala kang auditable ballot?",
            useSampleLink: "Gumamit ng sample na auditable ballot",
        },
        confirmationScreen: {
            title: "Sequent Ballot Verifier",
            topDescription1:
                "Batay sa impormasyong nasa imported na Auditable Ballot, napag-alaman namin na:",
            topDescription2: "Kung ito ang Ballot ID na ipinakita sa Voting Booth:",
            bottomDescription1:
                "Ang iyong balota ay na-encrypt nang tama. Maaari mo nang isara ang window na ito at bumalik sa Voting Booth.",
            bottomDescription2:
                "Kung hindi ito tumutugma, mag-click dito upang matuto pa tungkol sa mga posibleng dahilan at kung ano ang maaari mong gawin.",
            ballotChoicesDescription: "At ang iyong mga pagpili sa balota ay:",
            helpAndFaq: "Tulong at FAQ",
            backButton: "Bumalik",
            markedInvalid: "Balota na tahasang minarkahang hindi wasto",
        },
        ballotSelectionsScreen: {
            statusModal: {
                title: "Katayuan",
                content:
                    "Ang panel ng katayuan ay nagbibigay ng impormasyon tungkol sa mga verification na isinagawa.",
                ok: "OK",
            },
        },
        footer: {
            poweredBy: "Pinapagana ng <1></1>",
        },
        errors: {
            encoding: {
                notEnoughChoices: "Hindi sapat ang mga pagpili upang ma-decode",
                writeInChoiceOutOfRange: "Ang write-in na pagpili ay wala sa saklaw: {{index}}",
                writeInNotEndInZero: "Ang write-in ay hindi nagtatapos sa 0",
                bytesToUtf8Conversion:
                    "Error sa pag-convert ng write-in mula bytes patungong UTF-8 string: {{errorMessage}}",
                ballotTooLarge: "Ang balota ay mas malaki kaysa sa inaasahan",
            },
            implicit: {
                selectedMax:
                    "Ang bilang ng napiling mga pagpipilian na {{numSelected}} ay higit sa maximum na {{max}}",
                selectedMin:
                    "Ang bilang ng napiling mga pagpipilian na {{numSelected}} ay mas mababa kaysa sa minimum na {{min}}",
            },
            explicit: {
                notAllowed:
                    "Ang balota ay tahasang minarkahang hindi wasto ngunit hindi pinapayagan ng tanong",
            },
        },
        ballotHash: "Ang iyong Ballot ID: {{ballotId}}",
        version: {
            header: "Bersyon:",
        },
        hash: {
            header: "Hash:",
        },
        logout: {
            buttonText: "Logout",
            modal: {
                title: "Sigurado ka bang nais mong mag-logout?",
                content:
                    "Malapit mo nang isara ang application na ito. Hindi na mababawi ang aksyon na ito.",
                ok: "OK",
                close: "Isara",
            },
        },
        stories: {
            openDialog: "Buksan ang Dialog",
        },
        dragNDrop: {
            firstLine: "I-drag & i-drop ang mga file o",
            browse: "Mag-browse",
            format: "Suportadong format: txt",
        },
        selectElection: {
            electionWebsite: "Website ng Balota",
            countdown:
                "Magsisimula ang halalan sa loob ng {{years}} taon, {{months}} buwan, {{weeks}} linggo, {{days}} araw, {{hours}} oras, {{minutes}} minuto, {{seconds}} segundo",
            openElection: "Bukas",
            closedElection: "Sarado",
            voted: "Nakaboto",
            notVoted: "Hindi Nakaboto",
            resultsButton: "Mga Resulta ng Balota",
            voteButton: "I-click para Bumoto",
            openDate: "Bukas: ",
            closeDate: "Sarado: ",
            ballotLocator: "Hanapin ang iyong balota",
        },
        header: {
            profile: "Profile",
            welcome: "Sumalubong,<br><span>{{name}}</span>",
            session: {
                title: "Malapit nang mag-expire ang iyong session.",
                timeLeft: "May natitirang {{time}} ka upang iboto ang iyong balota.",
                timeLeftMinutesAndSeconds: "{{timeLeftInMinutes}} minuto at {{time}} segundo",
                timeLeftSeconds: "{{timeLeft}} segundo",
            },
        },
    },
}

export default tagalogTranslation
