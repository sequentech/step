// SPDX-FileCopyrightText: 2025 Enric Badia <enric@xtremis.com>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {TranslationType} from "./en"

const basqueTranslation: TranslationType = {
    translations: {
        common: {
            goBack: "Itzuli",
            showMore: "Erakutsi Gehiago",
            showLess: "Erakutsi Gutxiago",
        },
        candidatesList: {
            collapseToggle: "{{listTitle}} zerrenda txandakatu",
            showCandidates: "Hautagaiak erakutsi",
            hideCandidates: "Hautagaiak ezkutatu",
            selectedCandidate: "{{count}} hautagai hautatuta",
            selectedCandidates: "{{count}} hautagai hautatuta",
            expandAll: "Dena zabaldu",
            collapseAll: "Dena tolestu",
        },
        breadcrumbSteps: {
            electionList: "Bozketak",
            ballot: "Bozketa",
            review: "Berrikusi",
            confirmation: "Berretsi",
            audit: "Auditoria",
        },
        footer: {
            poweredBy: "Honek bultzatuta: <1></1>",
        },
        votingScreen: {
            backButton: "Atzera",
            reviewButton: "Hurrengoa",
            clearButton: "Hautaketa garbitu",
            ballotHelpDialog: {
                title: "Bozketa pantailari buruz",
                content:
                    'Pantaila honek bozkatzeko eskubidea duzun lehiaketa erakusten du. Hautaketa egiteko, aktibatu eskuinaldeko Hautagaia/Erantzunaren kontrol-laukia. Berrezartzeko sakatu "<b>Hautaketa garbitu</b>", aurrera joateko sakatu "<b>Hurrengoa</b>".',
                ok: "Ados",
            },
            nonVotedDialog: {
                title: "Baliogabeko edo boto zuria",
                content:
                    "Zure erantzun batzuek bozketa galdera batean edo gehiagotan baliogabea edo zuria bihurtuko dute.",
                ok: "Itzuli eta berrikusi",
                continue: "Jarraitu",
                cancel: "Ezeztatu",
            },
            warningDialog: {
                title: "Berrikusi zure boto-txartela",
                content:
                    "Zure boto-txartelak arreta behar dezaketen hautaketak ditu (adibidez, baimendutakoak baino aukera gutxiago hautatzea). Zure boto-txartela baliozkoa da eta bidali bezala zenbatuko da.",
                ok: "Atzera eta berrikusi",
                continue: "Jarraitu",
                cancel: "Utzi",
            },
        },
        startScreen: {
            startButton: "Hasi Bozketa",
            declineToVoteButton: "Bozkatzeari uko egin",
            declineToVoteDialog: {
                title: "Berretsi bozkatzeari uko egitea",
                content:
                    "Ziur zaude bozkatzeari uko egin nahi diozula?<br />Zuzenean berrikuspen pantailara joango zara, eta zure parte-hartze egoera <b>Bozkatzeari uko egina</b> bezala gordeko da.",
                continue: "Bozkatzeari uko egin",
                cancel: "Utzi",
            },
            instructionsTitle: "Nola bozkatzen da",
            instructionsDescription: "Jarraitu urrats hauek bozkatzeko:",
            step1Title: "1. Hautatu zure aukerak",
            step1Description:
                "Aukeratu zure hautagaiak eta erantzun galderak. Editatu bozketa prest egon arte.",
            step2Title: "2. Berrikusi zure bozketa",
            step2Description:
                "Zure bozketa zifratu eta berrikuspena erakutsiko dizugu. Bozketa ID jarraitzaile bakarra jasoko duzu.",
            step3Title: "3. Eman zure bozketa",
            step3Description:
                "Eman zure bozketa erregistratzeko, edo zifraketa zuzen egin dela egiaztatu.",
        },
        reviewScreen: {
            title: "Berrikusi zure bozketa",
            description:
                '"<b>Editatu bozketa</b>" hautaketak aldatzeko, "<b>Eman bozketa</b>" bidaltzeko, edo "<b>Egiaztatu Bozketa</b>" auditatzeko.',
            descriptionNoAudit:
                '"<b>Editatu bozketa</b>" hautaketak aldatzeko, edo "<b>Eman bozketa</b>" bidaltzeko.',
            backButton: "Editatu bozketa",
            castBallotButton: "Eman bozketa",
            auditButton: "Egiaztatu bozketa",
            reviewScreenHelpDialog: {
                title: "Berrikuspena pantailari buruz",
                content:
                    "Pantaila honek zure hautaketak berrikusteko aukera ematen dizu zure bozketa eman aurretik.",
                ok: "Ados",
            },
            ballotIdHelpDialog: {
                title: "Botoa ez da eman",
                content:
                    "<p>Hau da zure Bozketa Jarraitzaile IDa, baina <b>zure botoa ez da eman oraindik</b>. Bozketa jarraitzen saiatzen bazara, ez duzu aurkituko.</p><p>Bozketa Jarraitzaile IDa etapa honetan erakusten dugun arrazoia zifratu bozketaren zuzentasuna auditatu ahal izatea da eman aurretik.</p>",
                ok: "Ulertzen dut nire botoa ez dela eman",
                cancel: "Ezeztatu",
            },
            auditBallotHelpDialog: {
                title: "Bozketa auditatu nahi duzu?",
                content:
                    "<p>Kontuan izan zure bozketa auditatzeak baliogabetu egingo duela, bozketa prozesua berriz hasi beharko duzularik. Auditoria prozesuak zure bozketa zuzen kodetu dela egiaztatzeko aukera ematen dizu, baina urrats tekniko aurreratuak dakartza. Zure trebetasun teknikoetan konfiantza baduzu soilik jarraitzea gomendatzen dugu. Zure bozketa eman besterik ez baduzu nahi, sakatu <u>Ezeztatu</u> bozketa berrikuspena pantailara itzultzeko.</p>",
                ok: "Bai, nire bozketa baztertu nahi dut auditatzeko",
                cancel: "Ezeztatu",
            },
            confirmCastVoteDialog: {
                title: "Ziur zaude zure botoa eman nahi duzula?",
                content: "Berretsi ondoren, zure botoa emango da.",
                ok: "Bai, nire botoa eman nahi dut",
                cancel: "Ezeztatu",
            },
            error: {
                NETWORK_ERROR:
                    "Sare arazoa izan da. Mesedez, saiatu berriro geroago edo jarri harremanetan laguntzarekin.",
                UNABLE_TO_FETCH_DATA:
                    "Arazoa izan da datuak eskuratzean. Mesedez, saiatu berriro geroago edo jarri harremanetan laguntzarekin.",
                LOAD_ELECTION_EVENT:
                    "Ezin da hauteskunde gertaera kargatu. Mesedez, saiatu berriro geroago.",
                CAST_VOTE:
                    "Errore ezezaguna izan da botoa ematean. Mesedez, saiatu berriro geroago edo jarri harremanetan laguntzarekin.",
                CAST_VOTE_AreaNotFound:
                    "Errorea izan da botoa ematean: Eremua ez da aurkitu. Mesedez, saiatu berriro geroago edo jarri harremanetan laguntzarekin.",
                CAST_VOTE_CheckStatusFailed:
                    "Hauteskundeak ez du botoa emateko baimenik ematen. Hauteskunde itxita, artxibatuta egon daiteke edo garapen aldian bozkatu saiatzen ari zara.",
                CAST_VOTE_InternalServerError:
                    "Barne errorea izan da botoa ematean. Mesedez, saiatu berriro geroago edo jarri harremanetan laguntzarekin.",
                CAST_VOTE_QueueError:
                    "Arazoa izan da zure botoa prozesatzean. Mesedez, saiatu berriro geroago edo jarri harremanetan laguntzarekin.",
                CAST_VOTE_Unauthorized:
                    "Ez duzu botorik emateko baimenik. Mesedez, jarri harremanetan laguntzarekin.",
                CAST_VOTE_ElectionEventNotFound:
                    "Hauteskunde gertaera ez da aurkitu. Mesedez, saiatu berriro geroago edo jarri harremanetan laguntzarekin.",
                CAST_VOTE_ElectoralLogNotFound:
                    "Zure bozketa erregistroa ez da aurkitu. Mesedez, jarri harremanetan laguntzarekin.",
                CAST_VOTE_CheckPreviousVotesFailed:
                    "Errorea izan da zure bozketa egoera egiaztatzerakoan. Mesedez, saiatu berriro geroago edo jarri harremanetan laguntzarekin.",
                CAST_VOTE_GetClientCredentialsFailed:
                    "Huts egin du zure kredentzialak egiaztatzean. Mesedez, saiatu berriro geroago edo jarri harremanetan laguntzarekin.",
                CAST_VOTE_GetAreaIdFailed:
                    "Errorea izan da zure bozketa eremua egiaztatzerakoan. Mesedez, saiatu berriro geroago edo jarri harremanetan laguntzarekin.",
                CAST_VOTE_GetTransactionFailed:
                    "Errorea izan da zure botoa prozesatzerakoan. Mesedez, saiatu berriro geroago edo jarri harremanetan laguntzarekin.",
                CAST_VOTE_DeserializeBallotFailed:
                    "Errorea izan da zure bozketa irakurtzerakoan. Mesedez, saiatu berriro geroago edo jarri harremanetan laguntzarekin.",
                CAST_VOTE_DeserializeContestsFailed:
                    "Errorea izan da zure hautaketak irakurtzerakoan. Mesedez, saiatu berriro geroago edo jarri harremanetan laguntzarekin.",
                CAST_VOTE_PokValidationFailed:
                    "Huts egin du zure botoa balioztatzean. Mesedez, saiatu berriro geroago edo jarri harremanetan laguntzarekin.",
                CAST_VOTE_UuidParseFailed:
                    "Errorea izan da zure eskaria prozesatzerakoan. Mesedez, saiatu berriro geroago edo jarri harremanetan laguntzarekin.",
                CAST_VOTE_unexpected:
                    "Errore ezezaguna izan da botoa ematean. Mesedez, saiatu berriro geroago edo jarri harremanetan laguntzarekin.",
                CAST_VOTE_timeout:
                    "Botoa emateko denbora-muga errorea. Saiatu berriro geroago edo jarri harremanetan laguntza-zerbitzuarekin laguntza jasotzeko.",
                CAST_VOTE_InsertFailedExceedsAllowedRevotes:
                    "Berriro botoen muga gainditu duzu. Saiatu berriro geroago edo jarri harremanetan laguntza-zerbitzuarekin laguntza jasotzeko.",
                CAST_VOTE_CheckRevotesFailed:
                    "Baimendutako berriro boto kopurua gainditu duzu. Saiatu berriro geroago edo jarri harremanetan laguntza-zerbitzuarekin laguntza jasotzeko.",
                CAST_VOTE_CheckVotesInOtherAreasFailed:
                    "Beste eremu batean bozkatu duzu dagoeneko. Saiatu berriro geroago edo jarri harremanetan laguntza-zerbitzuarekin laguntza jasotzeko.",
                CAST_VOTE_UnknownError:
                    "Errore ezezaguna izan da botoa ematean. Mesedez, saiatu berriro geroago edo jarri harremanetan laguntzarekin.",
                NO_BALLOT_SELECTION:
                    "Hauteskunde honetarako hautaketa egoera ez dago presente. Mesedez, ziurtatu zure aukerak zuzen hautatu dituzula edo jarri harremanetan laguntzarekin.",
                NO_BALLOT_STYLE:
                    "Bozketa estiloa ez dago eskuragarri. Mesedez, jarri harremanetan laguntzarekin.",
                NO_AUDITABLE_BALLOT:
                    "Ez dago auditatu daitekeen bozketarik eskuragarri. Mesedez, jarri harremanetan laguntzarekin.",
                INCONSISTENT_HASH:
                    "Errorea izan da bozketa hash prozesuan. BallotId: {{ballotId}} ez da koherentea auditatu daitekeen Bozketa Hash-arekin: {{auditableBallotHash}}. Mesedez, eman arazo honen berri laguntzari.",
                ELECTION_EVENT_NOT_OPEN:
                    "Hauteskunde gertaera itxita dago. Mesedez, jarri harremanetan laguntzarekin.",
                PARSE_ERROR:
                    "Errorea izan da bozketa aztertzerakoan. Mesedez, saiatu berriro geroago edo jarri harremanetan laguntzarekin.",
                DESERIALIZE_AUDITABLE_ERROR:
                    "Errorea izan da auditatu daitekeen bozketa deserializatzerakoan. Mesedez, saiatu berriro geroago edo jarri harremanetan laguntzarekin.",
                DESERIALIZE_HASHABLE_ERROR:
                    "Errorea izan da hash egin daitekeen bozketa deserializatzerakoan. Mesedez, saiatu berriro geroago edo jarri harremanetan laguntzarekin.",
                CONVERT_ERROR:
                    "Errorea izan da bozketa bihurtzerakoan. Mesedez, saiatu berriro geroago edo jarri harremanetan laguntzarekin.",
                SERIALIZE_ERROR:
                    "Errorea izan da bozketa serializatzerakoan. Mesedez, saiatu berriro geroago edo jarri harremanetan laguntzarekin.",
                UNKNOWN_ERROR:
                    "Errorea izan da. Mesedez, saiatu berriro geroago edo jarri harremanetan laguntzarekin.",
                REAUTH_FAILED:
                    "Autentifikazioak huts egin du. Saiatu berriro edo jarri harremanetan laguntza-zerbitzuarekin laguntza jasotzeko.",
                SESSION_EXPIRED: "Zure saioa iraungi da. Saiatu berriro hasieratik.",
                CAST_VOTE_BallotIdMismatch: "Boto-paperaren IDa ez dator bat emandako botoarekin.",
                SESSION_STORAGE_ERROR:
                    "Saio-biltegia ez dago erabilgarri. Mesedez, saiatu berriro edo jarri harremanetan laguntza-zerbitzuarekin.",
                PARSE_BALLOT_DATA_ERROR:
                    "Errore bat gertatu da boto-datuen analisian. Mesedez, saiatu berriro geroago edo jarri harremanetan laguntza-zerbitzuarekin.",
                NOT_VALID_BALLOT_DATA_ERROR:
                    "Boto-datuak ez dira baliozkoak. Mesedez, saiatu berriro geroago edo jarri harremanetan laguntza-zerbitzuarekin.",
                FETCH_DATA_TIMEOUT_ERROR:
                    "Denbora-muga gainditu da datuak eskuratzean. Mesedez, saiatu berriro geroago edo jarri harremanetan laguntza-zerbitzuarekin.",
                TO_HASHABLE_BALLOT_ERROR:
                    "Errorea gertatu da hash bihurgarri bihurtzean. Mesedez, saiatu berriro geroago edo jarri harremanetan laguntza-zerbitzuarekin.",
                INTERNAL_ERROR:
                    "Barne-errore bat gertatu da botoa ematean. Mesedez, saiatu berriro geroago edo jarri harremanetan laguntza-zerbitzuarekin.",
            },
            declineToVote: "Bozkatzeari uko egin",
        },
        confirmationScreen: {
            title: "Zure botoa eman da",
            description:
                "Beheko berrespen kodeak egiaztatzen du <b>zure bozketa arrakastaz eman dela</b>. Kode hau erabil dezakezu zure bozketa kontatu dela egiaztatzeko.",
            ballotId: "Bozketa IDa",
            printButton: "Inprimatu",
            finishButton: "Amaitu",
            verifyCastTitle: "Egiaztatu zure bozketa eman dela",
            verifyCastDescription:
                "Zure bozketa zuzen eman dela egiaztatu dezakezu edozein unetan hurrengo QR kodea erabiliz:",
            confirmationHelpDialog: {
                title: "Berrespen pantailari buruz",
                content:
                    "Pantaila honek zure botoa arrakastaz eman dela erakusten du. Bozketa kutxan gorde dela egiaztatzeko aukera ematen dizu.",
                ok: "Ados",
            },
            demoPrintDialog: {
                title: "Bozketa inprimatzen",
                content: "Inprimatzea desgaituta demo moduan",
                ok: "Ados",
            },
            demoBallotUrlDialog: {
                title: "Bozketa IDa",
                content: "Ezin da kodea erabili, desgaituta demo moduan.",
                ok: "Ados",
            },
            ballotIdHelpDialog: {
                title: "Bozketa IDari buruz",
                content:
                    "Bozketa IDa zure bozketa bozketa kutxan aurkitzeko ahalbidetzen duen kodea da, ID hau bakarra da eta ez du zure hautaketei buruzko informaziorik.",
                ok: "Ados",
            },
            ballotIdDemoHelpDialog: {
                title: "Bozketa IDari buruz",
                content:
                    "Bozketa IDa zure bozketa bozketa kutxan aurkitzeko ahalbidetzen duen kodea da, ID hau bakarra da eta ez du zure hautaketei buruzko informaziorik.",
                ok: "Ados",
            },
            errorDialogPrintBallotReceipt: {
                title: "Errorea",
                content: "Errorea gertatu da, mesedez saiatu berriro",
                ok: "Ados",
            },
            demoQRText: "Bozketa jarraitzailea desgaituta dago demo moduan",
        },
        auditScreen: {
            printButton: "Inprimatu",
            restartButton: "Hasi Bozketa",
            title: "Egiaztatu zure Bozketa",
            description: "Zure bozketa egiaztatzeko, jarraitu beheko urratsak:",
            step1Title: "1. Deskargatu edo kopiatu hurrengo informazioa",
            step1Description:
                "Pantailaren goialdean agertzen den zure <b>Bozketa IDa</b> eta beheko zure zifratutako bozketa:",
            step1HelpDialog: {
                title: "Kopiatu Zifratutako Bozketa",
                content:
                    "Zure zifratutako bozketa deskargatu edo kopiatu dezakezu bozketa auditatzeko eta zifratutako edukiak zure hautaketak dituela egiaztatzeko.",
                ok: "Ados",
            },
            downloadButton: "Deskargatu",
            step2Title: "2. Egiaztatu zure bozketa",
            step2Description:
                "<VerifierLink>Sartu bozketa egiaztatzailera</VerifierLink>, fitxa berri bat irekiko da zure nabigatzailean.",
            step2HelpDialog: {
                title: "Bozketa auditoria tutoriala",
                content:
                    "Zure bozketa auditatzeko tutorialean erakutsitako urratsak jarraitu beharko dituzu, honek zifratutako bozketa webgunetik independenteki egiaztatzeko erabiltzen den mahaigaineko aplikazio bat deskargatzea barne hartzen du.",
                ok: "Ados",
            },
            bottomWarning:
                "Segurtasun arrazoiengatik, zure bozketa audtatzen duzunean, hondatu egin behar da. Bozketa prozesuarekin jarraitzeko, beheko '<b>Hasi Bozketa</b>' sakatu behar duzu.",
        },
        electionSelectionScreen: {
            title: "Bozketak",
            description: "Hautatu bozkatu nahi duzun Bozketa",
            chooserHelpDialog: {
                title: "Bozketa zerrendari buruz",
                content:
                    "Pantaila honek bozkatu dezakezun Bozketen zerrenda erakusten du. Bozketara sar zaitezke bozketa aldia irekita dagoenean.",
                ok: "Ados",
            },
            noResults: "Ez dago bozketarik eskuragarri oraingoz.",
            demoDialog: {
                title: "Demo bozketa kabina",
                content:
                    "Demo bozketa kabina batean sartzen ari zara. <strong>Zure botoa ez da emango.</strong> Erakusteko helburuetarako soilik da.",
                ok: "Ulertzen dut nire botoa ez dela emango",
            },
            errors: {
                noVotingArea: "Hauteskunde eremua ez da esleitu. Saiatu berriro geroago.",
                networkError:
                    "Sare arazoa izan da. Mesedez, saiatu berriro geroago edo jarri harremanetan laguntzarekin.",
                unableToFetchData:
                    "Arazoa izan da datuak eskuratzean. Mesedez, saiatu berriro geroago edo jarri harremanetan laguntzarekin.",
                noElectionEvent:
                    "Hauteskunde gertaera ez da existitzen. Mesedez, saiatu berriro geroago edo jarri harremanetan laguntzarekin.",
                ballotStylesEmlError:
                    "Errorea izan da bozketa estilo argitalpenarekin. Mesedez, saiatu berriro geroago edo jarri harremanetan laguntzarekin.",
                obtainingElectionFromID:
                    "Errorea izan da hurrengo hauteskunde IDekin lotutako hauteskundeak lortzerakoan: {{electionIds}}. Mesedez, saiatu berriro geroago edo jarri harremanetan laguntzarekin.",
            },
            alerts: {
                noElections:
                    "Ez dago bozkatu dezakezun hauteskunderik. Hau eremua ez duelako lehiaketa asoziaturik ez duelako izan daiteke. Mesedez, saiatu berriro geroago edo jarri harremanetan laguntzarekin.",
                electionEventNotPublished:
                    "Hauteskunde gertaera ez da argitaratu oraindik. Mesedez, saiatu berriro geroago edo jarri harremanetan laguntzarekin.",
            },
        },
        errors: {
            encoding: {
                notEnoughChoices: "Ez dago nahikoa aukera deskodetzeko",
                writeInChoiceOutOfRange: "Eskuz idatzitako aukera barrutitik kanpo dago: {{index}}",
                writeInNotEndInZero: "Eskuz idatzitakoa ez da 0-rekin amaitzen",
                writeInCharsExceeded:
                    "Eskuz idatzitakoak gehieneko luzera gainditzen du {{numCharsExceeded}} karakteretan. Mesedez, laburtu ezazu.",
                bytesToUtf8Conversion:
                    "Errorea eskuz idatzitakoa byte-etatik UTF-8 kate bihurtzerakoan: {{errorMessage}}",
                ballotTooLarge: "Bozketa espero baino handiagoa da",
            },
            implicit: {
                selectedMax:
                    "Gehiegizko botoa: hautatutako aukeren kopurua {{numSelected}} gehieneko {{max}} baino handiagoa da",
                selectedMin:
                    "Hautatutako aukeren kopurua {{numSelected}} gutxieneko {{min}} baino txikiagoa da",
                maxSelectionsPerType:
                    "Hautatutako aukeren kopurua {{numSelected}} {{type}} zerrendarako gehieneko {{max}} baino handiagoa da",
                underVote:
                    "Boto gutxiegiko: hautatutako aukeren kopurua {{numSelected}} gehieneko {{max}} baino txikiagoa da",
                overVoteDisabled:
                    "Gehienekoa lortu da: {{numSelected}} aukera hautatu dituzu, gehieneko kopurua. Hautaketa aldatzeko, lehenik beste aukera bat kendu ezazu.",
                blankVote: "Boto zuria: 0 aukera hautatuta",
            },
            explicit: {
                notAllowed:
                    "Bozketa berariaz baliogabetzat markatu da, baina galderak ez du hori onartzen",
                alert: "Hautaketa hau boto baliogabe gisa zenbatuko da",
            },
            page: {
                oopsWithStatus: "Hara! {{status}}",
                oopsWithoutStatus: "Hara! Ustekabeko Errorea",
                somethingWrong: "Zerbait oker joan da.",
                certAuthFailedTitle: "Ziurtagiriaren Autentifikazio Errorea",
                certAuthFailedMessage:
                    "Ezin izan da zure ziurtagiria egiaztatu. Mesedez, egiaztatu boto-emaile ziurtagiri baliogarri bat erabiltzen ari zarela eta saiatu berriro.",
            },
        },
        materials: {
            common: {
                label: "Laguntza Materialak",
                back: "Itzuli bozketa zerrendara",
                close: "Itxi",
                preview: "Aurrebista",
            },
        },
        ballotLocator: {
            title: "Bilatu zure Bozketa",
            titleResult: "Zure Bozketa Bilaketak Emaitza",
            description: "Egiaztatu zure Bozketa zuzen bidali dela",
            locate: "Bilatu zure Bozketa",
            locateAgain: "Bilatu beste Bozketa bat",
            found: "Zure bozketa IDa {{ballotId}} aurkitu da",
            notFound: "Zure bozketa IDa {{ballotId}} ez da aurkitu",
            contentDesc: "Hau da zure Bozketa edukia: ",
            wrongFormatBallotId: "Bozketa IDaren formatu okerra",
            ballotIdNotFoundAtFilter: "Zure bozketa IDa ez da {{ballotId}} bozketa zerrendan",
            filterByBallotId: "Filtratu Bozketa IDa",
            totalBallots: "Bozketa kopurua: {{total}}",
            steps: {
                lookup: "Bilatu zure Bozketa",
                result: "Emaitza",
            },
            titleHelpDialog: {
                title: "Bozketa Bilatzaileari buruz",
                content:
                    "Bozketa Bilatzaileak Bozketa IDa sartzeko aukera ematen dizu zure botoa aurkitu eta zuzen erregistratu dela egiaztatzeko.",
                ok: "Ados",
            },
            tabs: {
                logs: "Logs",
                ballotLocator: "Bozketa Bilatzaile",
            },
            column: {
                statement_kind: "Adierazpen mota",
                statement_timestamp: "Adierazpen denbora-marka",
                username: "Erabiltzaile izena",
                ballot_id: "Bozketa IDa",
                message: "Mezua",
            },
        },
    },
}

export default basqueTranslation
