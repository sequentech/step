// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {TranslationType} from "./en"

const basqueTranslation: TranslationType = {
    translations: {
        language: "Euskara",
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
            preferential: {
                position: "Posizioa",
                none: "Bat ere ez",
                ordinals: {
                    first: ".",
                    second: ".",
                    third: ".",
                    other: ".",
                },
            },
        },
        homeScreen: {
            title: "Sequent Txartel Egiaztatzailea",
            description1:
                "Txartel egiaztatzailea bozkatzaileak kabinan txartela auditatzea aukeratzen duenean erabiltzen da. Egiaztapenak 1-2 minutu iraun beharko luke.",
            description2:
                "Txartel egiaztatzaileak bozkatzaileari aukera ematen dio enkriptatutako txartelak kabinan egindako hautaketak zuzen jasotzen dituela ziurtatzeko. Egiaztapen hau egiteari nahi bezala emandakoaren egiaztagarritasuna deitzen zaio eta txartelaren enkriptazioan akatsak eta jarduera maltzurrak saihesten ditu.",
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
            topDescription1: "Inportatutako Txartel Auditagarrian oinarrituta, hau kalkulatu dugu:",
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
                content: "Egoera panelak egindako egiaztapenei buruzko informazioa ematen dizu.",
                ok: "Ados",
            },
        },
        footer: {
            poweredBy: "Honek bultzatuta: <sequent />",
        },
        errors: {
            encoding: {
                notEnoughChoices: "Ez dago nahikoa aukera deskodetzeko",
                writeInChoiceOutOfRange: "Idatzitako aukera barrutitik kanpo: {{index}}",
                writeInNotEndInZero: "Idatzitakoa ez da 0n amaitzen",
                writeInCharsExceeded_one: "Laburtu eskuz idatzitakoa {{count}} karakterez.",
                writeInCharsExceeded_many: "Laburtu eskuz idatzitakoa {{count}} karakterez.",
                writeInCharsExceeded_other: "Laburtu eskuz idatzitakoa {{count}} karakterez.",
                bytesToUtf8Conversion:
                    "Errorea idatzitakoa byte-etatik UTF-8 kate-ra bihurtzerakoan: {{errorMessage}}",
                ballotTooLarge: "Bozketa esperotakoa baino handiagoa",
            },
            implicit: {
                selectedMax_one: "Kendu {{count}} hautagai.",
                selectedMax_many: "Kendu {{count}} hautagai.",
                selectedMax_other: "Kendu {{count}} hautagai.",
                selectedMin_one: "Hautatu {{count}} hautagai gehiago.",
                selectedMin_many: "Hautatu {{count}} hautagai gehiago.",
                selectedMin_other: "Hautatu {{count}} hautagai gehiago.",
                maxSelectionsPerType_one: "Kendu {{count}} hautagai {{type}} zerrendatik.",
                maxSelectionsPerType_many: "Kendu {{count}} hautagai {{type}} zerrendatik.",
                maxSelectionsPerType_other: "Kendu {{count}} hautagai {{type}} zerrendatik.",
                underVote_one: "Gehienez {{count}} hautagai gehiago hauta ditzakezu.",
                underVote_many: "Gehienez {{count}} hautagai gehiago hauta ditzakezu.",
                underVote_other: "Gehienez {{count}} hautagai gehiago hauta ditzakezu.",
                overVoteDisabled_one:
                    "Gehienezko {{count}} hautagai hautatu dituzu. Beste bat aukeratzeko, kendu bat.",
                overVoteDisabled_many:
                    "Gehienezko {{count}} hautagai hautatu dituzu. Beste bat aukeratzeko, kendu bat.",
                overVoteDisabled_other:
                    "Gehienezko {{count}} hautagai hautatu dituzu. Beste bat aukeratzeko, kendu bat.",
                blankVote: "Ez duzu hautagairik hautatu.",
                preferenceOrderWithGaps:
                    "Boto baliogabea! Lehentasunaren ordenak hutsune bat edo gehiago ditu.",
                duplicatedPosition:
                    "Boto baliogabea! Posizio bera hautatu da bi kandidatu edo gehiagorentzat.",
            },
            explicit: {
                notAllowed:
                    "Bozketa espresuki baliogabe markatu da baina galderak ez du baimentzen",
                alert: "Markatutako hautaketa baliogabeko bototzat hartuko da.",
            },
            configuration: {
                multipleExplicitInvalidCandidates:
                    "Boto-konfigurazio baliogabea: galderak esplizituki baliogabe diren {{count}} hautagai definitzen ditu, baina bakarra onartzen da.",
                multipleExplicitBlankCandidates:
                    "Boto-konfigurazio baliogabea: galderak esplizituki zuri gisa markatutako {{count}} hautagai definitzen ditu, baina bakarra onartzen da.",
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
                content: "Aplikazio hau ixtear zaude. Ekintza hau ezin da desegin.",
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

export default basqueTranslation
