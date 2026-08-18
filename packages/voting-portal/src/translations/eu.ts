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
            electionList: "Bozketa Zerrenda",
            ballot: "Bozketa",
            review: "Berrikusi",
            confirmation: "Berrespena",
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
                title: "Informazioa: Bozketa pantaila",
                content:
                    'Pantaila honek zuk bozkatzeko eskubidea duzun lehiaketa erakusten du. Zure hautaketa egin dezakezu eskuinaldeko Hautagaia/Erantzunaren kontrol-laukia aktibatuz. Zure hautaketak berrezartzeko, sakatu "<b>Hautaketa garbitu</b>" botoia, hurrengo urratsera joateko, sakatu beheko "<b>Hurrengoa</b>" botoia.',
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
            blankBallotDialog: {
                title: "Ez duzu hautagairik hautatu",
                content:
                    "Ez duzu inolako hautaketarik egin. Zure boto-txartela zuri gisa aurkeztuko da, aukera baliozko eta nahitakoa da eta horrela zenbatuko da.",
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
            instructionsTitle: "Jarraibideak",
            instructionsDescription: "Mesedez, jarraitu urrats hauek zure bozketa emateko:",
            step1Title: "1. Hautatu zure aukerak",
            step1Description:
                "Aukeratu zure gogoko hautagaiak eta erantzun Bozketaren galderei bat-batean agertzen diren heinean. Zure bozketa editatu dezakezu prest egon arte.",
            step2Title: "2. Berrikusi zure bozketa",
            step2Description:
                "Zure hautaketekin pozik zaudenean, zure bozketa zifratu eta zure aukeren azken berrikuspena erakutsiko dizugu. Zure bozketarako ID jarraitzaile bakarra ere jasoko duzu.",
            step3Title: "3. Eman zure bozketa",
            step3Description:
                "Eman zure bozketa: Azkenik, zure bozketa eman dezakezu behar bezala erregistratu izan dadin. Bestela, zure bozketa zuzen hartu eta zifratu izan dela egiaztatu eta berresteko aukera duzu.",
        },
        reviewScreen: {
            title: "Berrikusi zure bozketa",
            description:
                'Zure hautaketetan aldaketak egiteko, sakatu "<b>Editatu bozketa</b>" botoia, zure hautaketak berresteko, sakatu beheko "<b>Eman zure bozketa</b>" botoia, eta zure bozketa auditatzeko sakatu beheko "<b>Auditatu Bozketa</b>" botoia.',
            descriptionNoAudit:
                'Zure hautaketetan aldaketak egiteko, sakatu "<b>Editatu bozketa</b>" botoia, zure hautaketak berresteko, sakatu beheko "<b>Eman zure bozketa</b>" botoia.',
            backButton: "Editatu bozketa",
            castBallotButton: "Eman zure bozketa",
            auditButton: "Auditatu bozketa",
            copyBallotId: "Kopiatu bozketa IDa",
            ballotIdCopied: "Bozketa IDa kopiatu da",
            ballotIdCopyError: "Ezin izan da bozketa IDa kopiatu",
            reviewScreenHelpDialog: {
                title: "Informazioa: Berrikuspena Pantaila",
                content:
                    "Pantaila honek zure hautaketak berrikusteko aukera ematen dizu zure bozketa eman aurretik.",
                ok: "Ados",
            },
            ballotIdHelpDialog: {
                title: "Botoa ez da eman",
                content:
                    "<p>Hau da zure Bozketa Jarraitzaile IDa, baina <b>zure botoa ez da eman oraindik</b>. Bozketa jarraitzen saiatzen bazara, ez duzu aurkituko.</p><p>Bozketa Jarraitzaile IDa etapa honetan erakusten dugun arrazoia zifratu bozketaren zuzentasuna auditatu ahal izatea da eman aurretik.</p>",
                ok: "Onartzen dut nire botoa EZ dela eman",
                cancel: "Ezeztatu",
            },
            auditBallotHelpDialog: {
                title: "Bozketa auditatu nahi duzu?",
                content:
                    "<p>Kontuan izan zure bozketa auditatzeak baliogabetu egingo duela, bozketa prozesua berriz hasi beharko duzularik. Auditoria prozesuak zure bozketa zuzen kodetu dela egiaztatzeko aukera ematen dizu, baina urrats tekniko aurreratuak dakartza. Zure trebetasun teknikoetan konfiantza baduzu soilik jarraitzea gomendatzen dugu. Zure bozketa eman besterik ez baduzu nahi, sakatu <u>Ezeztatu</u> bozketa berrikuspena pantailara itzultzeko.</p>",
                ok: "Bai, nire bozketa BAZTERTU nahi dut auditatzeko",
                cancel: "Ezeztatu",
            },
            confirmCastVoteDialog: {
                title: "Ziur zaude zure botoa eman nahi duzula?",
                content: "Zure botoa ez da editagarria izango behin berrestuta.",
                ok: "Bai, nire botoa EMAN nahi dut",
                cancel: "Ezeztatu",
            },
            confirmCastBlankBallotDialog: {
                title: "Ziur zaude boto-txartel zuria aurkeztu nahi duzula?",
                content:
                    "Ez duzu hautagairik hautatu. Berretsi ondoren, zure boto-txartela zuri gisa aurkeztuko da.",
                ok: "Bai, nire boto-txartel zuria aurkeztu nahi dut",
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
            blankBallot: "Boto-txartel zuria",
        },
        confirmationScreen: {
            title: "Zure botoa eman da",
            description:
                "Beheko berrespen kodeak egiaztatzen du <b>zure bozketa arrakastaz eman dela</b>. Kode hau erabil dezakezu zure bozketa kontatu dela egiaztatzeko.",
            blankBallot: {
                description:
                    "Zure boto-txartela zuri gisa aurkeztu da, aukera baliozko eta nahitakoa da.",
            },
            ballotId: "Bozketa IDa",
            printButton: "Inprimatu",
            finishButton: "Amaitu",
            verifyCastTitle: "Egiaztatu zure bozketa eman dela",
            verifyCastDescription:
                "Zure bozketa zuzen eman dela egiaztatu dezakezu edozein unetan hurrengo QR kodea erabiliz:",
            confirmationHelpDialog: {
                title: "Informazioa: Berrespen Pantaila",
                content:
                    "Pantaila honek zure botoa arrakastaz eman dela erakusten du. Orrialde honetan emandako informazioak bozketa kutxan gorde dela egiaztatzeko aukera ematen dizu, prozesu hau bozketa aldian zehar edozein unetan eta Bozketa itxi ondoren exekutatu daiteke.",
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
                title: "Informazioa: Bozketa IDa",
                content:
                    "Bozketa IDa zure bozketa bozketa kutxan aurkitzeko ahalbidetzen duen kodea da, ID hau bakarra da eta ez du zure hautaketei buruzko informaziorik.",
                ok: "Ados",
            },
            ballotIdDemoHelpDialog: {
                title: "Informazioa: Bozketa IDa",
                content:
                    "<p>Bozketa IDa zure bozketa bozketa kutxan aurkitzeko ahalbidetzen duen kodea da, ID hau bakarra da eta ez du zure hautaketei buruzko informaziorik.</p><p><b>Oharra:</b> Bozketa kabina hau erakusteko helburuetarako soilik da. Zure botoa EZ da eman.</p>",
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
            title: "Auditatu zure Bozketa",
            description: "Zure bozketa egiaztatzeko, mesedez jarraitu beheko urratsak:",
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
            title: "Bozketa zerrenda",
            description: "Hautatu bozkatu nahi duzun Bozketa",
            chooserHelpDialog: {
                title: "Informazioa: Bozketa Zerrenda",
                content:
                    "Ongi etorri Bozketa Kabinara, pantaila honek bozketa eman dezakezun Bozketen zerrenda erakusten du. Zerrenda honetan erakutsitako Bozketak bozketarako irekita, programatuta edo itxita egon daitezke. Bozketara sartzeko aukera izango duzu bozketa aldia irekita badago soilik.",
                ok: "Ados",
            },
            noResults: "Ez dago bozketarik oraingoz.",
            resultsButton: "Ikusi emaitzak",
            demoDialog: {
                title: "Demo bozketa kabina",
                content:
                    "Demo bozketa kabina batean sartzen ari zara. <strong>Zure botoa EZ da emango.</strong> Bozketa kabina hau erakusteko helburuetarako soilik da.",
                ok: "Onartzen dut nire botoa Ez dela emango",
            },
            errors: {
                noVotingArea:
                    "Hauteskunde eremua ez da bozkatzaileari esleitu. Mesedez, saiatu berriro geroago edo jarri harremanetan laguntzarekin.",
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
                back: "Itzuli Bozketa Zerrendara",
                close: "Itxi",
                preview: "Aurrebista",
            },
        },
        ballotLocator: {
            title: "Lokalizatu zure Bozketa",
            titleResult: "Zure Bozketa Bilaketak Emaitza",
            description: "Egiaztatu zure Bozketa zuzen bidali dela",
            locate: "Bilatu zure Bozketa",
            locateAgain: "Bilatu beste Bozketa bat",
            found: "Zure bozketa IDa {{ballotId}} aurkitu da",
            notFound: "Zure bozketa IDa {{ballotId}} ez da aurkitu",
            ambiguous:
                "Zure boto bat baino gehiago dator bat {{ballotId}} identifikatzailearekin. Erabili boto-identifikatzaile osoa.",
            contentDesc: "Hau da zure Bozketa edukia: ",
            wrongFormatBallotId: "Bozketa IDaren formatu okerra",
            ballotIdNotFoundAtFilter: "Zure bozketa IDa ez da {{ballotId}} bozketa zerrendan",
            filterByBallotId: "Filtratu Bozketa IDa",
            totalBallots: "Bozketa kopurua: {{total}}",
            steps: {
                lookup: "Lokalizatu zure Bozketa",
                result: "Emaitza",
            },
            titleHelpDialog: {
                title: "Informazioa: Bozketa Lokalizatzaile pantaila",
                content:
                    "Pantaila honek bozkatzaileari bere botoa aurkitzeko aukera ematen dio Bozketa IDa erabiliz berreskuratzeko. Prozedura honek beren bozketa zuzen eman dela eta erregistratutako bozketa bidali zuten zifratutako bozketarekin bat datorrela egiaztatzeko aukera ematen du.",
                ok: "Ados",
            },
            tabs: {
                logs: "Logs",
                ballotLocator: "Bozketa Lokalizatzaile",
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
