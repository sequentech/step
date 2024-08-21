// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
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
            created: "Nagawa",
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
        },
        homeScreen: {
            title: "Sequent Ballot Verifier",
            description1:
                "Ang ballot verifier ay ginagamit kapag pumipili ang botante na i-audit ang balota sa voting booth. Ang pag-verify ay dapat tumagal ng 1-2 minuto.",
            description2:
                "Ang ballot verifier ay nagpapahintulot sa botante na tiyakin na ang encrypted na balota ay tama na naglalaman ng mga pinili sa voting booth. Ang pagpayag na magsagawa ng tsek na ito ay tinatawag na cast-as-intended verifiability at pumipigil sa mga pagkakamali at mapanlikhang aktibidad sa panahon ng encryption ng balota.",
            descriptionMore: "Matuto pa",
            startButton: "Mag-browse ng file",
            dragDropOption: "O i-drag at i-drop ito dito",
            importErrorDescription:
                "Nagkaroon ng problema sa pag-import ng auditable na balota. Pumili ka ba ng tamang file?",
            importErrorMoreInfo: "Higit pang impormasyon",
            importErrorTitle: "Error",
            useSampleText: "Wala bang auditable na balota?",
            useSampleLink: "Gamitin ang sample na auditable na balota",
        },
        confirmationScreen: {
            title: "Sequent Ballot Verifier",
            topDescription1:
                "Batay sa impormasyon sa na-import na Auditable na Balota, kami ay nagkalkula na:",
            topDescription2: "Kung ito ang Ballot ID na ipinapakita sa Voting Booth:",
            bottomDescription1:
                "Ang iyong balota ay na-encrypt ng tama. Maaari mo na ngayong isara ang window na ito at bumalik sa Voting Booth.",
            bottomDescription2:
                "Kung hindi sila tumutugma, i-click dito upang malaman ang higit pa tungkol sa mga posibleng dahilan at kung ano ang mga hakbang na maaari mong gawin.",
            ballotChoicesDescription: "At ang iyong mga pagpipilian sa balota ay:",
            helpAndFaq: "Tulong at FAQ",
            backButton: "Bumalik",
            markedInvalid: "Ang balota ay tahasang minarkahan na invalid",
        },
        ballotSelectionsScreen: {
            statusModal: {
                title: "Status",
                content:
                    "Ang status panel ay nagbibigay sa iyo ng impormasyon tungkol sa mga verifications na isinagawa.",
                ok: "OK",
            },
        },
        poweredBy: "Pinapatakbo ng",
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
                    "Overvote: Ang bilang ng mga piniling pagpipilian {{numSelected}} ay higit sa maximum {{max}}",
                selectedMin:
                    "Ang bilang ng mga piniling pagpipilian {{numSelected}} ay mas mababa kaysa sa minimum {{min}}",
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
        logout: {
            buttonText: "Mag-logout",
            modal: {
                title: "Sigurado ka bang gusto mong mag-logout?",
                content:
                    "Ikaw ay malapit nang isara ang application na ito. Ang aksyong ito ay hindi maibabalik.",
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
            openElection: "Bukas",
            closedElection: "Nagsara",
            voted: "Naboto",
            notVoted: "Hindi naboto",
            resultsButton: "Mga Resulta ng Balota",
            voteButton: "I-click upang Bumoto",
            openDate: "Bukas: ",
            closeDate: "Nagsara: ",
            ballotLocator: "Hanapin ang iyong balota",
        },
        header: {
            profile: "Profile",
            session: {
                title: "Malapit nang mag-expire ang iyong session.",
                timeLeft: "May natitira kang {{time}} para bumoto.",
                timeLeftMinutesAndSeconds: "{{timeLeftInMinutes}} minuto at {{time}} segundo",
                timeLeftSeconds: "{{timeLeft}} segundo",
            },
        },
    },
}

export default tagalogTranslation
