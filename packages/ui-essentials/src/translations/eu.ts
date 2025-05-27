// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

const basqueTranslation = {
    translations: {
        language: "Euskara",
        welcome: "Kaixo <br/> <strong>Mundua</strong>",
        breadcrumbSteps: {
            select: "Hautatu egiaztatzaile bat",
            import: "Inportatu datuak",
            verify: "Egiaztatu",
            finish: "Amaitu",
        },
        electionEventBreadcrumbSteps: {
            created: "Sortua",
            keys: "Gakoak",
            publish: "Argitaratu",
            started: "Hasita",
            ended: "Amaituta",
            results: "Emaitzak",
        },
        candidate: {
            moreInformationLink: "Informazio gehiago",
            writeInsPlaceholder: "Idatzi hautagaia hemen",
            blankVote: "Boto zuria",
        },
        homeScreen: {
            title: "Sequent Txartel Egiaztatzailea",
            description1:
                "Txartel egiaztatzailea hautesleak kabinan txartela auditatzea aukeratzen duenean erabiltzen da. Egiaztapenak 1-2 minutu iraun beharko luke.",
            description2:
                "Txartel egiaztatzaileak hautesleari aukera ematen dio enkriptatutako txartelak kabinan egindako hautaketak zuzen jasotzen dituela ziurtatzeko. Egiaztapen hau egiteari nahi bezala emandakoaren egiaztagarritasuna deitzen zaio eta txartelaren enkriptazioan akatsak eta jarduera maltzurrak saihesten ditu.",
            descriptionMore: "Gehiago ikasi",
            startButton: "Arakatu fitxategia",
            dragDropOption: "Edo arrastatu eta jaregin hemen",
            importErrorDescription:
                "Arazo bat egon da txartel auditagarria inportatzean. Fitxategi zuzena aukeratu duzu?",
            importErrorMoreInfo: "Informazio gehiago",
            importErrorTitle: "Errorea",
            useSampleText: "Ez duzu txartel auditagarririk?",
            useSampleLink: "Erabili lagin-txartel auditagarri bat",
        },
        confirmationScreen: {
            title: "Sequent Txartel Egiaztatzailea",
            topDescription1:
                "Inportatutako Txartel Auditagarrian oinarrituta, hau kalkulatu dugu:",
            topDescription2: "Hau Hauteskunde Kabinan erakutsitako Txartelaren IDa bada:",
            bottomDescription1:
                "Zure txartela zuzen enkriptatu da. Orain leiho hau itxi eta Hauteskunde Kabinara itzul zaitezke.",
            bottomDescription2:
                "Bat ez badatoz, egin klik hemen arrazoi posibleei eta har ditzakezun neurriei buruz gehiago jakiteko.",
            ballotChoicesDescription: "Eta zure txartelaren aukerak hauek dira:",
            helpAndFaq: "Laguntza eta Galdera Ohikoenak",
            backButton: "Atzera",
            markedInvalid: "Txartela espresuki baliogabetzat markatuta",
        },
        ballotSelectionsScreen: {
            statusModal: {
                title: "Egoera",
                content:
                    "Egoera panelak egindako egiaztapenei buruzko informazioa ematen dizu.",
                ok: "Ados",
            },
        },
        poweredBy: "Honek bultzatuta:",
        errors: {
            encoding: {
                notEnoughChoices: "Ez dago nahikoa aukera deskodetzeko",
                writeInChoiceOutOfRange: "Idatzitako aukera barrutitik kanpo: {{index}}",
                writeInNotEndInZero: "Idatzitakoa ez da 0-z bukatzen",
                bytesToUtf8Conversion:
                    "Errorea idatzitakoa byteetatik UTF-8 katera bihurtzean: {{errorMessage}}",
                ballotTooLarge: "Txartela espero baino handiagoa da",
            },
            implicit: {
                selectedMax:
                    "Hautatutako aukera kopurua {{numSelected}} maximoa {{max}} baino handiagoa da",
                selectedMin:
                    "Hautatutako aukera kopurua {{numSelected}} minimoa {{min}} baino txikiagoa da",
            },
            explicit: {
                notAllowed: "Txartela espresuki baliogabetzat markatuta baina galderak ez du onartzen",
            },
        },
        ballotHash: "Zure Txartelaren IDa: {{ballotId}}",
        version: {
            header: "Bertsioa:",
        },
        hash: {
            header: "Hasha:",
        },
        logout: {
            buttonText: "Saioa itxi",
            modal: {
                title: "Ziur zaude saioa itxi nahi duzula?",
                content: "Aplikazio hau ixtear zaude. Ekintza hau ezin da desegin. ",
                ok: "Ados",
                close: "Itxi",
            },
        },
        stories: {
            openDialog: "Ireki Elkarrizketa-koadroa",
        },
        dragNDrop: {
            firstLine: "Arrastatu eta jaregin fitxategiak edo",
            browse: "Arakatu",
            format: "Onartutako formatua: txt",
        },
        selectElection: {
            electionWebsite: "Txartelaren Webgunea",
            countdown:
                "Hauteskundeak {{years}} urte, {{months}} hilabete, {{weeks}} aste, {{days}} egun, {{hours}} ordu, {{minutes}} minutu, {{seconds}} segundu barru hasiko dira",
            openElection: "Ireki",
            closedElection: "Itxita",
            voted: "Bozkatua",
            notVoted: "Bozkatu gabe",
            resultsButton: "Txartelaren Emaitzak",
            voteButton: "Egin klik Bozkatzeko",
            openDate: "Irekitze-data: ",
            closeDate: "Ixte-data: ",
            ballotLocator: "Aurkitu zure txartela",
        },
        header: {
            profile: "Profila",
            welcome: "Ongi etorri,<br><span>{{name}}</span>",
            session: {
                title: "Zure saioa iraungitzear dago.",
                timeLeft: "{{time}} geratzen zaizu botoa emateko.",
                timeLeftMinutesAndSeconds: "{{timeLeftInMinutes}} minutu eta {{time}} segundu",
                timeLeftSeconds: "{{timeLeft}} segundu",
            },
        },
    },
}

export type TranslationType = typeof basqueTranslation

export default basqueTranslation