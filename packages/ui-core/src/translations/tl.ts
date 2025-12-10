// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {TranslationType} from "./en"

const tagalogTranslation: TranslationType = {
    translations: {
        language: "Tagalog",
        welcome: "Simulan natin: I-import ang auditable na balota..",
        breadcrumbSteps: {
            select: "Pumili ng Tagasuri",
            import: "I-import ang Data",
            verify: "I-verify",
            finish: "Tapos na",
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
            writeInsPlaceholder: "I-type ang write-in candidate dito",
            blankVote: "Blangkong Boto",
            preferential: {
                position: "Posisyon",
                none: "Wala",
                ordinals: {
                    first: "st",
                    second: "nd",
                    third: "rd",
                    other: "th",
                },
            },
        },
        homeScreen: {
            title: "Sequent Ballot Verifier",
            description1:
                "Ang ballot verifier ay ginagamit kapag pinili ng botante na i-audit ang balota sa voting booth. Ang pag-verify ay tatagal ng 1-2 minuto.",
            description2:
                "Ang ballot verifier ay nagbibigay-daan sa botante na tiyakin na ang encrypted na balota ay wastong nakuha ang mga pinili sa voting booth. Ang pagpayag na magsagawa ng pagsusuri na ito ay tinatawag na cast-as-intended verifiability at pinipigilan ang mga pagkakamali at malisyosong aktibidad habang ine-encrypt ang balota.",
            descriptionMore: "Alamin pa",
            startButton: "Mag-browse ng file",
            dragDropOption: "O i-drag at i-drop ito dito",
            importErrorDescription:
                "Nagkaroon ng problema sa pag-import ng sisiyasating balota. Tama ba ang napili mong file?",
            importErrorMoreInfo: "Karagdagang impormasyon",
            importErrorTitle: "Error",
            useSampleText: "Walang sisiyasatin na balota?",
            useSampleLink: "Gamitin ang sample na sisiyasating balota",
        },
        confirmationScreen: {
            title: "Sequent Ballot Verifier",
            topDescription1:
                "Batay sa impormasyon sa na-import na Sisiyasating Balota, nakalkula namin na:",
            topDescription2: "Kung ito ang Ballot ID na ipinakita sa Voting Booth:",
            bottomDescription1:
                "Ang iyong balota ay na-encrypt ng tama. Maaari mo nang isara ang window na ito at bumalik sa Voting Booth.",
            bottomDescription2:
                "Kung hindi sila nagtutugma, i-click dito upang malaman ang mga posibleng dahilan at kung ano ang mga hakbang na maaari mong gawin.",
            ballotChoicesDescription: "At ang iyong mga napili sa balota ay:",
            helpAndFaq: "Help at FAQ",
            backButton: "Bumalik",
            markedInvalid: "Ang balota ay tahasang minarkahan na invalid",
        },
        ballotSelectionsScreen: {
            statusModal: {
                title: "Status",
                content:
                    "Ang status panel ay nagbibigay sa iyo ng impormasyon tungkol sa mga beripikasyon na isinagawa.",
                ok: "OK",
            },
        },
        footer: {
            poweredBy: "Pinapatakbo ng <sequent />",
        },
        errors: {
            encoding: {
                notEnoughChoices: "Hindi sapat ang mga pagpipilian upang i-decode",
                writeInChoiceOutOfRange: "Ang write-in choice ay nasa labas ng saklaw: {{index}}",
                writeInNotEndInZero: "Ang write-in ay hindi nagtatapos sa 0",
                bytesToUtf8Conversion:
                    "Error sa pag-convert ng write-in mula sa bytes patungo sa UTF-8 string: {{errorMessage}}",
                ballotTooLarge: "Ang balota ay mas malaki kaysa sa inaasahan",
            },
            implicit: {
                selectedMax:
                    "Overvote: Ang bilang ng mga pinili {{numSelected}} ay higit sa maximum {{max}}",
                selectedMin:
                    "Ang bilang ng mga pinili {{numSelected}} ay mas mababa kaysa sa minimum {{min}}",
            },
            explicit: {
                notAllowed:
                    "Ang balota ay tahasang minarkahan na invalid ngunit hindi ito pinapayagan ng tanong",
            },
        },
        ballotHash: "Ang Iyong Ballot ID: {{ballotId}}",
        version: {
            header: "Bersyon:",
        },
        hash: {
            header: "Hash:",
        },
        logout: {
            buttonText: "Mag-logout",
            modal: {
                title: "Sigurado ka bang gusto mong mag-logout?",
                content:
                    "Malapit mo nang isara ang application na ito. Ang aksyong ito ay hindi maibabalik.",
                ok: "OK",
                close: "Isara",
            },
        },
        stories: {
            openDialog: "Buksan ang Dialog",
        },
        dragNDrop: {
            firstLine: "I-drag at i-drop ang mga file o",
            browse: "Mag-browse",
            format: "Suportadong format: txt",
        },
        selectElection: {
            electionWebsite: "Website ng Balota",
            countdown:
                "Magsisimula ang halalan sa loob ng {{years}} taon, {{months}} buwan, {{weeks}} linggo, {{days}} araw, {{hours}} oras, {{minutes}} minuto, {{seconds}} segundo",
            openElection: "Bukas na",
            closedElection: "Sarado",
            voted: "Nakaboto",
            notVoted: "Hindi pa nakaboto",
            resultsButton: "Mga Resulta ng Balota",
            voteButton: "I-click upang Bumoto",
            openDate: "Simula: ",
            closeDate: "Pagtapos: ",
            ballotLocator: "Hanapin ang iyong balota",
        },
        header: {
            profile: "Profile",
            welcome: "Sumalubong,<br><span>{{name}}</span>",
            session: {
                title: "Malapit nang mag-expire ang iyong session.",
                timeLeft: "May natitira ka pang {{time}} para bumoto.",
                timeLeftMinutesAndSeconds: "{{timeLeftInMinutes}} minuto at {{time}} segundo",
                timeLeftSeconds: "{{timeLeft}} segundo",
            },
        },
    },
}

export default tagalogTranslation
