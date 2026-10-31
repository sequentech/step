// SPDX-FileCopyrightText: 2025 Enric Badia <enric@xtremis.com>
//
// SPDX-License-Identifier: AGPL-3.0-only
import englishTranslation, {TranslationType} from "./en"

const basqueTranslation: TranslationType = {
    translations: {
        ballotPrivacy: {
            ...englishTranslation.translations.ballotPrivacy,
            title: "Boto-paper guztiak",
            subtitle:
                "Hauteskunde honetan bidalitako boto-paper guztiak, bakoitza behin, azken urratsean. Hautesleak ezizen batekin agertzen dira, izenik gabe.",
            own: "Zure boto-papera erregistroan dago",
            copy: "Kopiatu jarraipen IDa",
            ballotIdHelp: "Zure jarraipen IDari buruz",
            stageTime: "{{stage}} {{time}}",
            noMatchHelp:
                "Begiratu idazketa-akatsik ez dagoela. Jarraipen IDak marratxo bat eta gutxienez 8 karaktere ditu, FTBE-MHRX bezala. Ezizenak bi puntu ditu, K7QM:2XWD bezala.",
            copied: "Jarraipen IDa kopiatu da",
            copyFailed: "Ezin izan da jarraipen IDa kopiatu. Hautatu kodea kopiatzeko.",
            search: "Bilatu jarraipen ID bat edo ezizen bat",
            placeholder: "FTBE-MHRX edo K7QM:2XWD",
            matchesBoth: "{{tracking}} bat datoz jarraipen IDagatik · {{pseudonym}} ezizenagatik",
            ballotId: "Jarraipen IDa",
            lastStep: "Azken urratsa",
            time: "Ordua",
            trackingId: "Jarraipen IDa",
            trackingIdHelp: "Zifratutako boto-paperatik eta hautesleen gakotik kalkulatua.",
            noMatch: "Ez dago {{prefix}}(r)ekin hasten den jarraipen ID edo ezizenik",
            noBallots: "Oraindik ez da boto-paperik bidali.",
            invalidPrefix:
                "Idatzi jarraipen ID baten gutxienez 8 karaktere marratxoarekin, FTBE-MHRX bezala, edo ezizen baten gehienez 8 bi punturekin, K7QM:2XWD bezala. Kodeek 0tik 9ra bitarteko digituak eta letra larriak erabiltzen dituzte, I, L, O eta U gabe.",
            stages: {
                received: {
                    label: "Jasota",
                    title: "Boto-papera jasota",
                    help: "Urnak boto-paper honen ziurtagiria sinatu zuen, eta erregistro publikoak gehitzeko agindu zuen. Ez da zenbatzen, hautesleak igortzen ez badu.",
                    castState: "Oraindik igorri gabe",
                },
                cast: {
                    label: "Igorrita",
                    title: "Boto-papera igorrita",
                    help: "Urnak baieztatu zuen hautesleak boto-paper hau igorri zuela.",
                    castState: "Gaindituta",
                },
                audited: {
                    label: "Auditatuta",
                    title: "Boto-papera auditatuta",
                    help: "Bere hautesleak ireki zuen egiaztatzeko. Auditatutako boto-papera ez da inoiz igortzen eta ez da zenbatzen.",
                    castState: "Inoiz igorri gabe",
                },
            },
            checkLabels: {
                ...englishTranslation.translations.ballotPrivacy.checkLabels,
                ballotId: "Jarraipen IDa",
            },
            checkDescriptions: {
                ...englishTranslation.translations.ballotPrivacy.checkDescriptions,
                ballotId:
                    "Jarraipen IDa zifratutako boto-paperatik eta hautesleen gakotik kalkulatzen da.",
            },
        },
        startup: {
            error: "Ezin izan da bozketa-ataria abiarazi. Saiatu berriro.",
            retry: "Saiatu berriro",
        },
        contactChannel: {
            change: "Harremanetarako modua",
            updated: "Zure kontaktua aldatu da. Kodeak eta abisuak hara bidaltzen dira orain.",
            cancelled: "Zure kontaktua ez da aldatu.",
            failed: "Ezin izan dugu aldatu zurekin harremanetan jartzeko modua. Saiatu berriro geroago edo jarri harremanetan laguntza-zerbitzuarekin.",
            close: "Itxi",
        },
        common: {
            goBack: "Itzuli",
            showMore: "Erakutsi Gehiago",
            showLess: "Erakutsi Gutxiago",
        },
        audioInstructions: {
            screens: {
                "election-chooser":
                    "Orri honetan botoa eman dezakezun hauteskundeak agertzen dira. Erabili Tabulazio tekla hauteskunde batetik hurrengora joateko. Hauteskunde bakoitzak bozketa hasteko botoi bat du. Sakatu Sartu tekla botoi horretan bere boto-papera irekitzeko.",
                "start":
                    "Orri honek botoa nola eman azaltzen du. Lehenik aukerak hautatzen dituzu, gero berrikusten dituzu eta azkenik botoa ematen duzu. Erabili Tabulazio tekla bozketa hasten duen botoira iristeko eta sakatu Sartu. Adierazpen bat agertzen bada, markatu lehenik bere laukia zuriune-barrarekin.",
                "ballot":
                    "Hau da zure boto-papera. Galdera bakoitza aukera-talde bat da, eta galderak esaten du zenbat aukera hauta ditzakezun. Erabili Tabulazio tekla aukeren artean mugitzeko eta zuriune-barra aukera bat markatzeko edo kentzeko. Amaitzean, joan orriaren amaierako jarraitzeko botoira eta sakatu Sartu zure aukerak berrikusteko. Aukera guztiak ezabatzeko botoi bat ere badago. Ez da ezer ematen berrikuspen-orrian baieztatu arte.",
                "review":
                    "Orri honek egin dituzun aukerak erakusten ditu. Egiaztatu arretaz. Zerbait aldatzeko, erabili boto-papera editatzeko botoia. Botoa emateko, joan boto-papera emateko botoira eta sakatu Sartu. Zure boto-paperaren identifikatzailea orriaren hasieran dago. Kopiatu dezakezu boto-papera geroago aurkitzeko.",
                "confirmation":
                    "Zure botoa eman da. Orri honek zure boto-paperaren identifikatzailea erakusten du; harekin egiaztatu dezakezu erregistratu dela. Kopiatu dezakezu edo orri hau inprimatu. Amaitzean, erabili orriaren amaierako botoia bukatzeko.",
                "audit":
                    "Orri honek boto-papera eman beharrean ikuskatzeko aukera ematen dizu. Boto-paperen egiaztatzailearekin egiazta dezakezun informazioa erakusten du. Ikuskatutako boto-papera ez da ematen; beraz, itzuli gero zure boto-paperera botoa emateko.",
                "ballot-locator":
                    "Orri honek boto-paper bat erregistratu dela egiaztatzeko aukera ematen dizu. Idatzi edo itsatsi boto-paperaren identifikatzaile bat testu-eremuan eta sakatu Sartu. Emaitza eremuaren azpian agertzen da.",
                "support-materials":
                    "Orri honetan botoa ematen laguntzen dizuten dokumentuak agertzen dira. Erabili Tabulazio tekla haien artean mugitzeko eta sakatu Sartu dokumentu baten botoian hura irekitzeko. Erabili atzera egiteko botoia hauteskundeen zerrendara itzultzeko.",
            },
        },
        a11y: {
            skipToContent: "Joan eduki nagusira",
            helpAbout: "{{topic}} atalari buruzko laguntza",
            previewMaterial: "{{title}} aurrebista",
            ballotsTable: "Botoak",
            ballotLocatorTabs: "Boto-txartelaren jarraipenaren atalak",
            ballotIdLabel: "Jarraipen IDa",
            votingProgress: "Bozketaren aurrerapena",
            stepOf: "{{total}} urratsetik {{current}}. urratsa",
            selectUpTo_one: "Hautatu gehienez aukera {{count}}",
            selectUpTo_many: "Hautatu gehienez {{count}} aukera",
            selectUpTo_other: "Hautatu gehienez {{count}} aukera",
            selectExactly_one: "Hautatu aukera {{count}}",
            selectExactly_many: "Hautatu {{count}} aukera",
            selectExactly_other: "Hautatu {{count}} aukera",
            selectBetween: "Hautatu {{min}} eta {{max}} aukera artean",
        },
        candidatesList: {
            collapseToggle: "{{listTitle}} zerrenda txandakatu",
            showCandidates: "Hautagaiak erakutsi",
            hideCandidates: "Hautagaiak ezkutatu",
            selectedCandidates_one: "{{count}} hautagai hautatuta",
            selectedCandidates_many: "{{count}} hautagai hautatuta",
            selectedCandidates_other: "{{count}} hautagai hautatuta",
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
        contest: {
            acclamation: {
                description:
                    "Galdera hau aklamazioz erabaki da. Bere hautagaiak bozketarik gabe hautatzen dira, beraz, ezin da aukerarik hautatu eta ez da bozkarik erregistratzen.",
            },
        },
        votingScreen: {
            backButton: "Atzera",
            reviewButton: "Hurrengoa",
            clearButton: "Hautaketa garbitu",
            ballotHelpDialog: {
                title: "Bozketa pantailari buruz",
                content:
                    'Pantaila honek bozkatzeko eskubidea duzun galdera erakusten du. Hautaketa egiteko, aktibatu eskuinaldeko Hautagaia/Erantzunaren kontrol-laukia. Berrezartzeko sakatu "<b>Hautaketa garbitu</b>", aurrera joateko sakatu "<b>Hurrengoa</b>".',
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
            instructionsTitle: "Nola bozkatzen da",
            instructionsDescription: "Jarraitu urrats hauek bozkatzeko:",
            step1Title: "1. Hautatu zure aukerak",
            step1Description:
                "Aukeratu zure hautagaiak eta erantzun galdera bakoitza. Editatu bozketa prest egon arte.",
            step2Title: "2. Berrikusi zure bozketa",
            step2Description:
                "Zure bozketa zifratu eta berrikuspena erakutsiko dizugu. Jarraipen ID bakarra jasoko duzu.",
            step3Title: "3. Eman zure bozketa",
            step3Description:
                "Eman zure bozketa erregistratzeko, edo zifraketa zuzen egin dela egiaztatu.",
        },
        reviewScreen: {
            reloaded: {
                title: "Zure boto-papera ez da igorri",
                text: "Orria birkargatzean, igortzen duen aldi bakarreko gakoa ezabatu da. Egiaztatu zure aukerak eta bidali berriro zure boto-papera. Bidali zenuen boto-papera jasota gisa geratzen da erregistroan eta ez da inoiz zenbatzen.",
                back: "Itzuli nire boto-paperera",
            },
            castRecovery: {
                uncertain:
                    "Botoaren emaitza ez da baieztatu. Eskaera bera berriro bidal dezakezu edo jarraipen IDa egiaztatu berriro hasi aurretik.",
                checkBallot: "Egiaztatu boto-papera",
            },
            acclamation: {
                title: "Aklamazioz erabakia",
                helpDialog: {
                    title: "Informazioa: Aklamazioa",
                    content:
                        "Pantaila honek aklamazioz erabaki dena erakusten du. Aukerarik hautatu ezin izan denez, ez da boto-txartelik ematen eta ez dago ondoren egiaztatzeko ezer.",
                    ok: "Ados",
                },
                description:
                    "Berrikusi hauteskunde honetan aklamazioz erabaki dena. Ez da boto-txartelik emango.",
                finishButton: "Amaitu",
            },
            title: "Berrikusi zure bozketa",
            description:
                '"<b>Editatu bozketa</b>" hautaketak aldatzeko, "<b>Eman bozketa</b>" bidaltzeko, edo "<b>Egiaztatu Bozketa</b>" auditatzeko.',
            descriptionNoAudit:
                '"<b>Editatu bozketa</b>" hautaketak aldatzeko, edo "<b>Eman bozketa</b>" bidaltzeko.',
            backButton: "Editatu bozketa",
            castBallotButton: "Eman bozketa",
            auditButton: "Egiaztatu bozketa",
            copyBallotId: "Kopiatu jarraipen IDa",
            ballotIdCopied: "Jarraipen IDa kopiatu da",
            ballotIdCopyError: "Ezin izan da jarraipen IDa kopiatu",
            reviewScreenHelpDialog: {
                title: "Berrikuspena pantailari buruz",
                content:
                    "Pantaila honek zure hautaketak berrikusteko aukera ematen dizu zure bozketa eman aurretik.",
                ok: "Ados",
            },
            ballotIdHelpDialog: {
                title: "Botoa ez da eman",
                content:
                    "<p>Hau da zure jarraipen IDa, baina <b>zure botoa ez da eman oraindik</b>. Bozketa jarraitzen saiatzen bazara, ez duzu aurkituko.</p><p>Jarraipen IDa etapa honetan erakusten dugun arrazoia zifratu bozketaren zuzentasuna auditatu ahal izatea da eman aurretik.</p>",
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
            confirmCastBlankBallotDialog: {
                title: "Ziur zaude boto-txartel zuria aurkeztu nahi duzula?",
                content:
                    "Ez duzu hautagairik hautatu. Berretsi ondoren, zure boto-txartela zuri gisa aurkeztuko da.",
                ok: "Bai, nire boto-txartel zuria aurkeztu nahi dut",
                cancel: "Ezeztatu",
            },
            unfilledContestsDialog: {
                title: "Hautaketa batzuk osatu gabe daude",
                content:
                    "Hautaketak hutsik utz ditzakezu edo baimendutakoak baino aukera gutxiago hauta ditzakezu. Berrikusi honako hau jarraitu aurretik.",
                selected: "{{max}}tik {{selected}} hautatuta",
                nothingSelected: "Hautaketarik ez",
                ok: "Jarraitu hautaketa hauekin",
                cancel: "Berrikusi hautaketak",
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
                    "Errorea izan da bozketa hash prozesuan. Jarraipen IDa: {{ballotId}} ez da koherentea auditatu daitekeen Bozketa Hash-arekin: {{auditableBallotHash}}. Mesedez, eman arazo honen berri laguntzari.",
                NOT_RECEIVED:
                    "Erregistro publikoak ez du oraindik zure boto-papera jaso, beraz ez da bidali. Itzuli zure boto-paperera eta saiatu berriro.",
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
                CAST_VOTE_BallotIdMismatch: "Jarraipen IDa ez dator bat emandako botoarekin.",
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
            acclamation: {
                title: "Aklamazioz erabakia",
                description:
                    "Hauteskunde honetako galdera guztiak aklamazioz erabaki dira, beraz, ez da boto-txartelik eman eta ez dago jarraipen IDrik.",
                helpDialog: {
                    title: "Informazioa: Aklamazioa",
                    content:
                        "Hauteskunde honetako galdera guztiak aklamazioz erabaki dira: haien hautagaiak bozketarik gabe hautatzen dira. Boto-txartelik eman ez denez, ez dago egiaztatzeko jarraipen IDrik, ordainagiririk edo QR koderik.",
                    ok: "Ados",
                },
            },
            title: "Zure botoa eman da",
            description:
                "Beheko berrespen kodeak egiaztatzen du <b>zure bozketa arrakastaz eman dela</b>. Kode hau erabil dezakezu zure bozketa kontatu dela egiaztatzeko.",
            blankBallot: {
                description:
                    "Zure boto-txartela zuri gisa aurkeztu da, aukera baliozko eta nahitakoa da.",
            },
            ballotId: "Jarraipen IDa",
            printButton: "Inprimatu",
            finishButton: "Amaitu",
            remainingElectionsError:
                "Ezin izan dugu egiaztatu botoa emateko beste hauteskunderik duzun. Saiatu berriro.",
            retryButton: "Saiatu berriro",
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
                title: "Jarraipen IDa",
                content: "Ezin da kodea erabili, desgaituta demo moduan.",
                ok: "Ados",
            },
            ballotIdHelpDialog: {
                title: "Jarraipen IDari buruz",
                content:
                    "Jarraipen IDa zure bozketa bozketa kutxan aurkitzeko ahalbidetzen duen kodea da, ID hau bakarra da eta ez du zure hautaketei buruzko informaziorik.",
                ok: "Ados",
            },
            ballotIdDemoHelpDialog: {
                title: "Jarraipen IDari buruz",
                content:
                    "Jarraipen IDa zure bozketa bozketa kutxan aurkitzeko ahalbidetzen duen kodea da, ID hau bakarra da eta ez du zure hautaketei buruzko informaziorik.",
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
            confirmingAudit: "Boto-paper hau bidali ezin dela berresten…",
            auditUnavailable:
                "Ezin izan da boto-papera egiaztatu. Ez da haren edukia erakutsi. Saiatu berriro.",
            printButton: "Inprimatu",
            restartButton: "Hasi Bozketa",
            title: "Egiaztatu zure Bozketa",
            description: "Zure bozketa egiaztatzeko, jarraitu beheko urratsak:",
            step1Title: "1. Deskargatu edo kopiatu hurrengo informazioa",
            step1Description:
                "Pantailaren goialdean agertzen den zure <b>Jarraipen IDa</b> eta beheko zure zifratutako bozketa:",
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
            noResults: "Ez dago bozketarik oraingoz.",
            resultsButton: "Ikusi emaitzak",
            demoDialog: {
                title: "Demo bozketa kabina",
                content:
                    "Demo bozketa kabina batean sartzen ari zara. <strong>Zure botoa ez da emango.</strong> Erakusteko helburuetarako soilik da.",
                ok: "Ulertzen dut nire botoa ez dela emango",
            },
            errors: {
                noVotingArea:
                    "Ez zaude bozkatzaile gisa erregistratuta hauteskunde honetan. Mesedez, jarri harremanetan laguntzarekin.",
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
                    "Ez dago bozkatu dezakezun hauteskunderik. Hau eremua ez duelako galdera asoziaturik ez duelako izan daiteke. Mesedez, saiatu berriro geroago edo jarri harremanetan laguntzarekin.",
                electionEventNotPublished:
                    "Hauteskunde gertaera ez da argitaratu oraindik. Mesedez, saiatu berriro geroago edo jarri harremanetan laguntzarekin.",
            },
            materialsGate: {
                instructions:
                    "<MaterialsLink>{{materialsTitle}}</MaterialsLink> irakurri behar duzu bozkatu ahal izateko.",
            },
            votingClosedAt: "Bozketa itxi zen: {{close}} ({{localClose}}).",
            votingClosedOn: "Bozketa itxi zen: {{close}}.",
        },
        errors: {
            encoding: {
                notEnoughChoices: "Ez dago nahikoa aukera deskodetzeko",
                writeInChoiceOutOfRange: "Eskuz idatzitako aukera barrutitik kanpo dago: {{index}}",
                writeInNotEndInZero: "Eskuz idatzitakoa ez da 0-rekin amaitzen",
                bytesToUtf8Conversion:
                    "Errorea eskuz idatzitakoa byte-etatik UTF-8 kate bihurtzerakoan: {{errorMessage}}",
                ballotTooLarge: "Bozketa espero baino handiagoa da",
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
                invalidLoginHintParametersTitle: "Bozkatzeko esteka baliogabea",
                invalidLoginHintParametersMessage:
                    "Bozkatzeko esteka honek saioa hasteko informazio baliogabea dauka. Eskatu beste esteka bat eta saiatu berriro.",
                certAuthFailedTitle: "Ziurtagiriaren Autentifikazio Errorea",
                certAuthFailedMessage:
                    "Ezin izan da zure ziurtagiria egiaztatu. Mesedez, egiaztatu bozkatzaile ziurtagiri baliogarri bat erabiltzen ari zarela eta saiatu berriro.",
            },
        },
        materials: {
            common: {
                label: "Laguntza Materialak",
                back: "Itzuli bozketa zerrendara",
                close: "Itxi",
                preview: "Aurrebista",
                download: "Deskargatu",
            },
            mandatory: {
                checkboxLabel: "Laguntza Materialak irakurri ditut",
                continueButton: "Jarraitu",
                error: "Arazoa izan da zure berrespena erregistratzean. Mesedez, saiatu berriro.",
            },
        },
        slates: {
            title: "Hautagai-zerrendak",
            description:
                "Hautagai-zerrenda elkarrekin aurkezten diren hautagaien taldea da. Hautagai bakoitzak zein zerrendatakoa den erakusten du.",
            independent: "Independentea",
            contestMembers: "{{slate}} zerrendako hautagaiak: {{contest}}",
            noCandidate: "Hautagairik ez",
            coverage: {
                full: "Hautagaitza osoa",
                singleContest: "{{contest}} bakarrik",
                partial: "Hautagaitza partziala",
                candidates_one: "Hautagai {{count}}",
                candidates_other: "{{count}} hautagai",
                offices_one: "Kargu {{count}}",
                offices_other: "{{count}} kargu",
            },
            candidateList: {
                show: "Erakutsi hautagaiak",
                hide: "Ezkutatu hautagaiak",
            },
            tabs: {
                label: "Boto-papera betetzeko moduak",
                slates: "Aukeratu zerrenda bat",
                candidates: "Hautagaiak banaka",
            },
            review: {
                title: "Zure hautaketak",
                total: "Hautatutako hautagaiak: {{seats}}tik {{selected}}",
                slate: "{{slate}}: {{status}}",
                independent: "Hautatutako hautagai independenteak: {{count}}",
                note: "Zure botoa hautatutako hautagai bakoitzarentzat erregistratzen da. Zerrenda bat ez da berez boto bat.",
                contestCount: "{{max}}tik {{selected}} hautatuta",
                edit: "Editatu",
                editLabel: "Editatu {{contest}}",
            },
            selection: {
                all: "{{total}}ak hautatuta",
                mixed: "Mistoa · {{total}}tik {{selected}} hautatuta",
                partly: "Zati bat hautatuta · {{total}}tik {{selected}}",
                selected: "Hautatuta",
                edit: "Editatu hautaketa",
            },
            apply: {
                button: "Aukeratu hautagai-zerrenda hau",
                buttonLabel: "Aukeratu {{slate}} hautagai-zerrenda",
                chosen: "{{slate}} aukeratuta. Hautatutako hautagaiak: {{candidates}}. Lehiak: {{contests}}.",
                overMaximum:
                    "{{slate}} ezin da aukeratu: {{candidates}} hautagai ditu {{contest}} lehiarako, eta {{max}} onartzen dira. Hautagaiak banaka aukeratzen jarrai dezakezu.",
                unavailable:
                    "{{slate}} ezin da aukeratu boto-paper honetan. Hautagaiak banaka aukeratzen jarrai dezakezu.",
                replaceDialog: {
                    title: "Zure uneko aukerak ordeztu nahi dituzu?",
                    content:
                        "{{slate}} aukeratzeak beheko lehietako zure aukerak ordezten ditu. Gainerako aukerak ez dira aldatzen.",
                    removed: "Kenduko da:",
                    added: "Horren ordez hautatuko da:",
                    ok: "Ordeztu aukerak",
                    cancel: "Mantendu nire aukerak",
                },
            },
        },
        ballotLocator: {
            title: "Boto-txartelaren jarraipena",
            titleResult: "Boto-txartelaren jarraipena",
            description: "Egiaztatu zure Bozketa zuzen bidali dela",
            descriptionAudited:
                "Boto-txartel hau auditatu zen, beraz, ez da inoiz eman. Egiaztatu behean dekodetutako hautaketak zuk egindakoak direla.",
            descriptionAuditedBrief: "Boto-txartel hau auditatu zen, beraz, ez da inoiz eman.",
            locate: "Bilatu zure Bozketa",
            locateAgain: "Bilatu beste Bozketa bat",
            found: "Zure jarraipen IDa {{ballotId}} aurkitu da",
            notFound: "Zure jarraipen IDa {{ballotId}} ez da aurkitu",
            contentDesc: "Hau da zure Bozketa edukia: ",
            wrongFormatBallotId: "Jarraipen IDaren formatu okerra",
            ambiguous: "Zure boto-paper batzuk {{ballotId}}(r)ekin hasten dira. Aukeratu bat:",
            ambiguousMany:
                "Zure boto-paper batzuk {{ballotId}}(r)ekin hasten dira. Idatzi jarraipen IDaren zati gehiago.",
            ballotIdNotFoundAtFilter:
                "Ez da aurkitu. Egiaztatu zure jarraipen IDa zuzena dela eta erabiltzaile honi dagokiola.",
            filterByBallotId: "Filtratu jarraipen IDaren arabera",
            totalBallots: "Bozketa kopurua: {{total}}",
            idHelp: "Jarraipen ID laburra, FTBE-MHRX bezalakoa, edo osoa",
            foundShort:
                "ID laburraren bidez aurkitu da. Zurea dela ziur egoteko, alderatu jarraipen ID osoa zure hartu-agiriarekin.",
            search: {
                title: "Egiaztatu zure boto-txartela erregistro publikoan",
                intro: "Sartu zure berrespen pantailako edo hartu-agiriko jarraipen IDa. Zure nabigatzaileak bilatzen du, eta frogak berak egiaztatzen ditu.",
                find: "Bilatu",
                invalid: "Jarraipen IDak digituak eta a-tik f-ra bitarteko letrak baino ez ditu.",
                chips: {
                    short: "<b>Laburra</b> FTBE-MHRX",
                    full: "<b>Osoa</b> lauko 16 talde",
                    spaces: "Zuriuneak aukerakoak",
                },
                features: {
                    cast: {
                        title: "Emandako boto-txartelak",
                        text: "Ikusi erregistro publikoan dagoela, lekuko independenteek baieztatua eta publikoki ainguratua.",
                    },
                    audited: {
                        title: "Auditatutako boto-txartelak",
                        text: "Ikusi dituen hautaketak, zure nabigatzailean deszifratuta, eta egiaztatu zuk egindakoak direla.",
                    },
                    secret: {
                        title: "Zure botoa sekretu geratzen da",
                        text: "Emandako boto-txartela zifratuta geratzen da. Txartelen jarraitzaileak zenbatu dela erakusten du, inoiz ez nola bozkatu duzun.",
                    },
                },
            },
            matches: {
                title: "{{total}} boto-txartelek ID labur bera dute: <b>{{shortId}}</b>. Aukeratu zurea, edo sartu jarraipen ID osoa.",
                cast: "Emana: {{time}}",
                audited: "Auditatua: {{time}}",
            },
            audit: {
                title: "Zure hautaketak, dekodetuta",
                note: "Zure nabigatzaileak auditatutako boto-txartel hau deszifratu du atariak agerian utzi duen gakoarekin. Hauek ez badira zuk egindako hautaketak, jarri harremanetan hauteskunde-agintaritzarekin.",
                failed: {
                    title: "Auditoriak huts egin du",
                    text: "Erregistro publikoko boto-txartela ez da deszifratzen atariak agerian utzi duen gakoarekin. Ez eman botorik gailu honetatik; jakinarazi hauteskunde-agintaritzari.",
                },
                unchecked: {
                    title: "Ezin izan da auditoria egiaztatu",
                    text: "Gailu honek ezin izan du auditatutako boto-txartel hau egiaztatu, eta, beraz, ez da hautaketarik erakusten. Saiatu berriro geroago, edo egiaztatu boto-txartelaren edukia boto-txartelen egiaztatzailean.",
                },
            },
            result: {
                receipt: "Boto-txartelaren hartu-agiria",
                idComment: "Erabili ID laburra edo osoa boto-txartel hau berriro aurkitzeko.",
                fullId: "Jarraipen ID osoa",
                copy: "Kopiatu jarraipen ID osoa",
                copied: "Jarraipen IDa kopiatu da",
                tag: {
                    found: "Aurkitua",
                    auditFailed: "Auditoriak huts egin du",
                },
                record: {
                    title: "Zure boto-txartelaren edukia",
                    subtitle: "Zifratuta, erregistro publikoan bezala",
                },
                recordEntry: "Erregistroko sarrera",
                save: "Gorde hartu-agiria",
                sealLabel: "Boto-txartelaren egoera: {{status}}",
                check: "Egiaztatu modu independentean",
                caption: "Zure nabigatzaileak egiaztatu du, edonork ikuska dezakeen kodearekin.",
                captionVerified:
                    "Zure nabigatzaileak baieztatu du, edonork ikuska dezakeen kodearekin.",
                captionProblem:
                    "Zure nabigatzaileak aurkitu du, edonork ikuska dezakeen kodearekin.",
                late: "Normalean gune bat motel edo lineaz kanpo dagoela esan nahi du; ez da manipulazioaren frogarik. Egiaztatu berriro geroago, hemen edo beste edozein gailutan.",
                problem:
                    "Atariak boto-txartel hau emandakotzat jotzen du, baina hauteskundeen erregistro publikoak ez du sarrerarik berarentzat. Zure nabigatzaileak froga gorde du: atariaren erantzuna eta erregistroaren egiaztapen-puntu sinatua. Bidali hauteskunde-agintaritzari edo behatzaile bati.",
                problemOther:
                    "Boto-txartel honen froga sinatuek elkarren aurka egiten dute. Akats bat edo eraso bat izan daiteke. Zure nabigatzaileak froga gorde du. Bidali hauteskunde-agintaritzari edo behatzaile bati.",
                saveEvidence: "Gorde froga",
                howToReport: "Nola salatu",
                howToReportText:
                    "Gorde frogaren fitxategia eta bidali hauteskunde-agintaritzari edo behatzaile independente bati. Beren ordenagailuetan egiaztatu dezakete. Ez du ezer jasotzen nola bozkatu duzun.",
                noReceipt:
                    "Gailu honek ez du boto-txartel honen hartu-agiririk. Egiaztatu bozkatzeko erabili zenuen gailuan, edo egiaztatu hartu-agiriaren fitxategia boto-txartelen egiaztatzailean.",
                receiptPending:
                    "Gailu honek boto-txartel honen hartu-agiria gordetzen du, baina ez dago osorik edo ezin izan da orain egiaztatu. Saiatu berriro geroago, edo egiaztatu hartu-agiriaren fitxategia boto-txartelen egiaztatzailean.",
                status: {
                    checking: {
                        head: "Egiaztatzen…",
                        sub: "Zure nabigatzailea boto-txartela erregistro publikoan bilatzen ari da.",
                    },
                    recorded: {
                        head: "Erregistro publikoan",
                        sub: "Zure nabigatzaileak boto-txartela aurkitu du. Lekukoak egiaztatzen…",
                    },
                    witnessed: {
                        head: "Lekukoek baieztatua",
                        sub: "Ainguraketa publikoak egiaztatzen…",
                        subAnchored_one:
                            "Gune publikoetan ainguratuta. Nabigatzaileek egiaztatzen duten tokira {{count}} minutuan helduko da.",
                        subAnchored_other:
                            "Gune publikoetan ainguratuta. Nabigatzaileek egiaztatzen duten tokira {{count}} minututan helduko da.",
                    },
                    verified: {
                        head: "Egiaztatuta",
                        sub: "Erregistro publikoan, lekuko independenteek baieztatua eta publikoki ainguratua.",
                    },
                    late: {
                        head: "Espero baino gehiago irauten du",
                        sub: "Erregistro publikoan dago eta lekukoek baieztatu dute, baina ainguraketa publikoak berandu dabiltza.",
                    },
                    problem: {
                        head: "Ez dago erregistro publikoan",
                        sub: "Zure nabigatzaileak ezin izan du boto-txartel hau hauteskundeen erregistro publikoan aurkitu.",
                    },
                    otherProblem: {
                        head: "Egiaztapen batek huts egin du",
                        sub: "Boto-txartel honen froga sinatuek elkarren aurka egiten dute. Ikusi huts egin duen urratsa.",
                    },
                },
                steps: {
                    record: {
                        title: "Erregistro publikoan gehitu da",
                        done: "{{index}}. sarrera hauteskundeen boto-txartelen erregistroan. Zure nabigatzaileak bere froga egiaztatu du.",
                        failed: "Hauteskundeen boto-txartelen erregistroko sarrera batek ere ez du jarraipen ID hau.",
                        missing: "Falta da",
                    },
                    witnesses: {
                        title: "Lekukoek baieztatua",
                        done: "Lekuko independenteek hura barne hartzen duen erregistro bat sinatu dute.",
                        count: "{{signed}} / {{total}} · {{needed}} behar dira",
                        countOf: "{{signed}} / {{total}}",
                    },
                    hubs: {
                        title_one: "Gune publiko batean ainguratuta",
                        title_other: "{{count}} gune publikotan ainguratuta",
                        titleAny: "Gune publikoetan ainguratuta",
                        done: "Partekatutako erregistroek ere badute erregistro hau: {{hubs}}.",
                    },
                    browser: {
                        title: "Nabigatzaileek egiaztatzen duten tokian ainguratuta",
                        done: "Gune ordezko batek gordeta dauka, nabigatzaileek erregistroak berek egiaztatu arte.",
                    },
                    checking: "Zure nabigatzailea egiaztatzen ari da…",
                    scheduled_one: "{{count}} minutuan. Ez da itxaron beharrik.",
                    scheduled_other: "{{count}} minututan. Ez da itxaron beharrik.",
                    by: "{{time}} baino lehen",
                    late: "Berandu",
                    lateHubs: "Guneek {{time}} baino lehen ainguratu beharko zuten.",
                    lateOther: "Hau {{time}} baino lehen egin beharko zen.",
                    waiting: "Aurreko urratsaren zain.",
                    skipped: "Ez da egiaztatu.",
                    failedStep: "Urrats honen froga sinatuek elkarren aurka egiten dute.",
                    failedTime: "Arazoa",
                },
            },
            checksAvailableUntil: "Zure boto-papera {{date}} arte egiazta dezakezu.",
            checksEnded: "Egiaztapenak {{date}} amaitu ziren.",
            castAt: "Noiz eman zen: {{date}}",
            steps: {
                lookup: "Bilatu zure Bozketa",
                result: "Emaitza",
            },
            titleHelpDialog: {
                title: "Boto-txartelaren jarraipenari buruz",
                content:
                    "Pantaila honek jarraipen IDa sartzeko aukera ematen dizu zure botoa aurkitu eta zuzen erregistratu dela egiaztatzeko.",
                ok: "Ados",
            },
            tabs: {
                logs: "Logs",
                ballotLocator: "Boto-txartelaren jarraipena",
            },
            column: {
                statement_kind: "Adierazpen mota",
                statement_timestamp: "Adierazpen denbora-marka",
                ballot_id: "Jarraipen IDa",
            },
        },
        vitrine: {
            ballotId: {
                label: "Jarraipen IDa",
                full: "Jarraipen ID osoa",
                yours: "Zure jarraipen IDa:",
                copy: "Kopiatu jarraipen ID osoa",
                copied: "Jarraipen IDa kopiatu da",
                showFull: "Erakutsi jarraipen ID osoa",
                selected: "Hautatuta. Sakatu Ctrl+C, edo ⌘C Mac-en, kopiatzeko.",
                copyByHand:
                    "Zure nabigatzaileak ez du kopiatu. Ireki jarraipen ID osoa hautatzeko.",
            },
            elsewhere: {
                title: "Bilatu beste nonbait",
                count_one: "Esploratzaile {{count}}, {{independent}} independente",
                count_other: "{{count}} esploratzaile, {{independent}} independente",
                introBallot:
                    "Besteek kudeatutako esploratzaileek berek irakurtzen dute erregistro publikoa. Bakoitzak zure boto-papera aurkitu beharko luke, egiaztapen berberekin, webgune hau gabe.",
                introEntry:
                    "Besteek kudeatutako esploratzaileek berek irakurtzen dute erregistro publikoa. Bakoitzak sarrera hau hemen dagoen bezala erakutsi beharko luke.",
                foot: "Zerrenda hauteskundearen politikatik dator. Vitrine-ren edozein esploratzailek ireki dezake erregistro bera: eman jarraipen ID osoa.",
                name: "{{name}} esploratzailea",
                operatorName: "Hauteskundearen esploratzailea",
                independent: "Independentea",
                operator: "Operadorea",
                newTab: "Fitxa berri batean irekitzen da",
                who: {
                    monitor: "Hauteskundearen politikan aipatutako monitore bat",
                    hub: "Hauteskundearen politikan aipatutako aingura-hub bat",
                    operator: "Hauteskunde-agintaritzak kudeatua",
                },
            },
            status: {
                title: "Bozketaren egoera",
                audit: "Auditatu boto-papera",
                auditNote:
                    "Ez zaude ziur atariak ondo zifratu dituela zure aukerak? Auditatu boto-paper hau: atariak nola zifratu zuen agertzen du, zuk egiazta dezazun. Auditatutako boto-papera ezin da inoiz bidali, beraz gero berriro bozkatzen duzu.",
                cast: "{{when}}(e)an eman da",
                audited: "{{when}}(e)an auditatu da",
                ballotCast: "Boto emana",
                aria: "Bozketaren egoera: {{state}}. Erakutsi xehetasunak",
                ariaCast: "Boto emana. Bozketaren egoera: {{state}}. Erakutsi xehetasunak",
                tooltip: "{{state}} · Erakutsi xehetasunak",
                tracker: "Ireki txartelen jarraitzailean",
                save: "Gorde ordainagiria",
                ok: "Ados",
                tag: {
                    cast: "Emana",
                    notCast: "Eman gabe",
                    auditable: "Auditagarria",
                    audited: "Auditatua",
                },
                state: {
                    notcast: "Oraindik ez da eman",
                    checking: "Egiaztatzen…",
                    recorded: "Erregistro publikoan",
                    witnessed: "Lekukoek berretsia",
                    verified: "Egiaztatua",
                    late: "Berandu",
                    problem: "Arazoa",
                    auditable: "Auditagarria",
                    auditfail: "Auditoriak huts egin du",
                },
                head: {
                    late: "Espero baino gehiago tardatzen ari da",
                    mismatch: "Erregistroa ez dator bat",
                    damaged: "Ordainagiria hondatuta dago",
                    noReceipt: "Zure bozketak ez du ordainagirik",
                    unchecked: "Ezin izan da zure bozketa egiaztatu",
                },
                sub: {
                    notcast:
                        "Bozketa hau gailu honetan dago zifratuta eta ez da ezer bidali. Eman ondoren, zure nabigatzaileak erregistro publikora iristen dela egiaztatuko du.",
                    checking:
                        "Zure nabigatzailea zure bozketa erregistro publikoan dagoela egiaztatzen ari da.",
                    recorded:
                        "Zure nabigatzaileak zure bozketa erregistro publikoan aurkitu du. Orain lekukoek berretsiko dute.",
                    witnessed:
                        "Lekuko independenteek erregistroa berretsi dute. Aingura publikoak {{minutes}} minuturen buruan iristen dira; ez dago itxaron beharrik.",
                    verified:
                        "Zure bozketa erregistro publikoan dago, lekuko independenteek berretsia eta publikoki ainguratua.",
                    late: "Zure bozketa erregistro publikoan dago eta lekukoek berretsi dute, baina aingura publikoak berandu doaz.",
                    lateStep:
                        "Urrats bat espero baino gehiago tardatzen ari da. Ez da manipulazioaren froga, eta zure nabigatzaileak egiaztatzen jarraitzen du.",
                    mismatch:
                        "Erregistro publikoko #{{index}} sarrera ez da ordainagiri honetakoa.",
                    mismatchNoEntry: "Erregistro publikoko sarrera ez da ordainagiri honetakoa.",
                    damaged:
                        "Ordainagiriaren edo erregistroaren zati bat ezin izan da irakurri edo egiaztatu. Datu-akatsa da, ez manipulazioaren froga. Zure botoa eman da.",
                    noReceipt:
                        "Webgune honek ez du zure bozketaren ordainagirik jaso. Zure botoa eman da, eta hauteskundeen behatzaileei jakinarazi zaie.",
                    unchecked:
                        "Webgune honek ezin izan du ordainagiria egiaztatu. Zure botoa eman da.",
                    auditable:
                        "Bozketa hau auditatu duzu, beraz, ez da inoiz eman. Erregistro publikoan dago, eta txartelen jarraitzaileak deszifratu egiten du, dituen hautaketak erakusteko.",
                    auditfail:
                        "Erregistro publikoko bozketa ez da deszifratzen atariak agerian utzi duen gakoarekin. Ez eman botorik gailu honetatik; jakinarazi hauteskunde-agintaritzari.",
                },
                step: {
                    record: {
                        title: "Erregistro publikoan",
                        upcoming:
                            "Zure nabigatzaileak bozketa hauteskundeen erregistro publikoan aurkitzen du.",
                    },
                    witnesses: {
                        title: "Lekukoek berretsia",
                        upcoming:
                            "Lekuko independenteek bozketa barne hartzen duen erregistro bat sinatzen dute.",
                    },
                    anchors: {
                        title: "Publikoki ainguratua",
                        upcoming:
                            "Gune publikoek eta nabigatzaileek egiaztatzen dituzten erregistroek erregistro hori gordetzen dute.",
                    },
                },
                row: {
                    waiting: "Aurreko urratsaren zain.",
                    checking: "Zure nabigatzailea egiaztatzen ari da…",
                    anchoring:
                        "Gune publikoak eta, ondoren, nabigatzaileek egiaztatzen duten tokia: {{minutes}} minuturen buruan.",
                    recorded: "Zure nabigatzaileak bere froga egiaztatu du.",
                    recordedEntry:
                        "#{{index}} sarrera. Zure nabigatzaileak bere froga egiaztatu du.",
                    witnessed:
                        "Politikako lekuko independente nahikok ({{count}}) sinatu dute bozketa barne hartzen duen erregistro bat.",
                    anchoredWaict:
                        "Gune publikoetan ({{count}}) eta nabigatzaileek egiaztatzen duten tokian.",
                    anchoredBootstrap:
                        "Gune publikoetan ({{count}}) eta nabigatzaileek egiaztatzen duten tokian, oraingoz ordezko gune baten bidez.",
                    anchoredHubs: "Gune publikoetan ({{count}}).",
                    lateNow: "Dagoeneko eginda egon beharko luke.",
                    lateStep: "{{time}} baino lehen eginda egon beharko luke.",
                    lateAnchors: "Guneek {{time}} baino lehen ainguratu beharko zuten.",
                    mismatch:
                        "Erregistroko #{{index}} sarrera ez da zure ordainagirikoa bezalakoa.",
                    mismatchNoEntry:
                        "Erregistroko sarrera bat ez da zure ordainagirikoa bezalakoa.",
                    contradiction: "Bere froga sinatuek elkarren aurka egiten dute.",
                    damaged: "Bere froga hondatuta dago eta ezin izan da egiaztatu.",
                    noReceipt: "Webgune honek ez du ordainagirik jaso.",
                    unchecked: "Webgune honek ezin izan du ordainagiria egiaztatu.",
                    skipped: "Ez da egiaztatu, aurreko urratsa ezin izan delako osatu.",
                    skippedMismatch: "Ez da egiaztatu, erregistroa ez datorrelako bat.",
                },
                word: {
                    withinSeconds: "Segundo gutxiren buruan",
                    withinMinutes: "{{minutes}} min barru",
                    checking: "Egiaztatzen…",
                    late: "Berandu",
                    mismatch: "Ez dator bat",
                    damaged: "Hondatua",
                    noReceipt: "Ordainagirik gabe",
                    unchecked: "Egiaztatu gabe",
                    skipped: "—",
                },
                callout: {
                    late: "Normalean gune bat motela edo lineaz kanpo dagoela esan nahi du; ez da manipulazioaren froga.",
                    lateKept:
                        "Gailu honek berriro egiaztatzen du itzultzen zaren bakoitzean, eta txartelen jarraitzaileak edozein gailutatik egiazta dezake.",
                    lateTracker: "Txartelen jarraitzaileak edozein gailutatik egiazta dezake.",
                    evidence:
                        "Zure nabigatzaileak frogak gorde ditu: ordainagiri hau eta erregistroaren sarrera sinatua. Bidali hauteskunde-agintaritzari edo behatzaile bati; edonork egiazta dezake.",
                    saveEvidence: "Gorde frogak",
                },
                caption: {
                    notcast:
                        "Ez da ezer egiaztatzen eman arte. Gero, ikur honek zure bozketa zenbateraino egiaztatu den erakusten du.",
                    checking:
                        "Zure nabigatzailea erregistro publikoa egiaztatzen ari da, edonork ikuska dezakeen kodearekin.",
                    problem: "Zure nabigatzaileak aurkitua, edonork ikuska dezakeen kodearekin.",
                    auditable:
                        "Zure nabigatzaileak deszifratua, edonork ikuska dezakeen kodearekin.",
                    verified: "Zure nabigatzaileak egiaztatua, edonork ikuska dezakeen kodearekin.",
                    checked: "Zure nabigatzaileak egiaztatua, edonork ikuska dezakeen kodearekin.",
                    kept: "Gailu honek ordainagiria gordetzen du eta bisita bakoitzean berriro egiaztatzen du.",
                },
            },
            booth: {
                title: "Zure boto-txartela ezin da oraindik prestatu",
                explanation:
                    "Zure boto-txartela zifratu aurretik, webgune honek egiaztatzen du hauteskundearen konfigurazioa hauteskundearen erregistro publikoan argitaratutakoa dela. Zure boto-txartela ez da zifratu ez bidali, eta zure aukerak hemen daude oraindik.",
                pending:
                    "Argitaratutako konfigurazioa ez dago oraindik eskuragarri. Saiatu berriro une batean.",
                invalid:
                    "Ezin izan da argitaratutako konfigurazioa irakurri. Saiatu berriro une batean.",
                failed: "Konfigurazioa ez dator bat argitaratutakoarekin. Hauteskundearen behatzaileei jakinarazi zaie. Jarri harremanetan laguntza-zerbitzuarekin.",
                retry: "Saiatu berriro",
                close: "Itxi",
            },
            record: {
                json: "JSON",
                text: "Testua",
                fields_one: "{{count}} eremu",
                fields_other: "{{count}} eremu",
                items_one: "{{count}} elementu",
                items_other: "{{count}} elementu",
                size: {
                    bytes: "{{value}} B",
                    kilobytes: "{{value}} KB",
                },
                copy: "Kopiatu {{file}}",
                copied: "Kopiatuta",
                download: "Deskargatu {{file}}",
            },
        },
    },
}

export default basqueTranslation
