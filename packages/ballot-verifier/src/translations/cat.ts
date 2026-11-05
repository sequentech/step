// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {TranslationType} from "./en"

const catalanTranslation: TranslationType = {
    translations: {
        "404": {
            title: "Pàgina no trobada",
            subtitle: "La pàgina que busqueu no existeix",
        },
        "homeScreen": {
            step1: "Pas 1: Importeu la vostra papereta electoral.",
            description1:
                "Per continuar, si us plau importeu les dades de les paperetes encriptades proporcionades al Portal de Votació:",
            importBallotHelpDialog: {
                title: "Informació: Importeu la vostra papereta electoral",
                ok: "D'acord",
                content:
                    "Per continuar, si us plau importeu les dades de les paperetes encriptades proporcionades al Portal de Votació.",
            },
            step2: "Pas 2: Introduïu el vostre ID de seguiment.",
            description2:
                "Enganxeu l'ID de seguiment complet del Portal de Votació: el botó de copiar al costat del vostre ID de seguiment el copia.",
            ballotIdHelpDialog: {
                title: "Informació: El vostre ID de seguiment",
                ok: "D'acord",
                content:
                    "Enganxeu l'ID de seguiment complet proporcionat al Portal de Votació. El botó de copiar que hi ha al costat el copia.",
            },
            startButton: "Seleccioneu fitxer",
            dragDropOption: "O arrossegueu el fitxer aquí",
            importErrorDescription:
                "Hi ha hagut un problema en importar el vot auditable. Heu triat el fitxer correcte?",
            importErrorMoreInfo: "Més informació",
            importErrorTitle: "Error",
            ciphertextErrorTitle: "La verificació del vot ha fallat",
            ciphertextErrorDescription:
                "El text xifrat d'aquest vot auditable no és el xifratge del text en clar i l'aleatorietat que conté. No es pot confiar en el vot.",
            verificationErrorTitle: "La verificació del vot ha fallat",
            unpublishedStyleErrorDescription:
                "Aquest vot auditable no correspon a cap papereta publicada per a aquest esdeveniment electoral, per la qual cosa no es pot verificar.",
            unavailableStyleErrorDescription:
                "No s'han pogut carregar les paperetes publicades per a aquest esdeveniment electoral, per la qual cosa aquest vot auditable encara no es pot verificar. Torneu-ho a provar.",
            styleErrorDescription:
                "L'estil de vot d'aquest vot auditable no és el publicat per a l'elecció: la seva clau pública o les seves contendes i candidatures difereixen, o no està publicat. No es pot confiar en el vot.",
            legacyBallotWarning:
                "Aquest vot auditable usa un format anterior l'ID de seguiment del qual no cobreix l'estil de vot. El seu ID de seguiment no pot confirmar que es va xifrar amb la clau publicada de l'elecció.",
            useSampleLink: "Utilitzeu vot d'exemple",
            nextButton: "Continuar",
            fileUploaded: "Carregat",
            receiptTitle: "Rebut de transparència",
            receiptDescription:
                "Aquesta elecció publica un rebut de cada papereta emesa. Comproveu el vostre: que és al registre públic, sense canvis i igual per a tothom.",
            receiptButton: "Comprova un rebut",
        },
        "confirmationScreen": {
            ballotIdTitle: "ID de seguiment",
            ballotIdDescription:
                "A continuació, el sistema mostra l'ID de seguiment de la papereta descodificada i el que heu proporcionat.",
            ballotIdError: "No coincideix amb l'ID de seguiment descodificat.",
            decodedBallotId: "ID de seguiment descodificat",
            decodedBallotIdHelpDialog: {
                title: "Informació: ID de seguiment descodificat",
                ok: "D'acord",
                content:
                    "Aquest és l'ID de seguiment extret del fitxer de la Papereta Auditable descodificada que vas proporcionar.",
            },
            yourBallotId: "L'ID de seguiment que vas proporcionar",
            userBallotIdHelpDialog: {
                title: "Informació: L'ID de seguiment que vas proporcionar",
                ok: "D'acord",
                content:
                    "Aquest és l'ID de seguiment que vas escriure en l'anterior pas i que vas recollir de la Cabina de Votació.",
            },
            backButton: "Enrere",
            printButton: "Imprimir",
            finishButton: "Verificat",
            verifySelectionsTitle: "Verifiqueu les vostres seleccions a la papereta",
            verifySelectionsDescription:
                "Les següents seleccions de la papereta han estat descodificades de la papereta que vau importar. Si us plau, reviseu-les i assegureu-vos que coincideixin amb les seleccions que vau fer al Portal de Votació. Si les vostres seleccions no coincideixen, si us plau, contacteu amb les autoritats electorals...",
            verifySelectionsHelpDialog: {
                title: "Informació: Verifiqueu les vostres seleccions a la papereta",
                ok: "D'acord",
                content:
                    "Les següents seleccions de la papereta han estat descodificades de la papereta que vau importar. Si us plau, reviseu-les i assegureu-vos que coincideixin amb les seleccions que vau fer al Portal de Votació. Si les vostres seleccions no coincideixen, si us plau, contacteu amb les autoritats electorals...",
            },
            markedInvalid: "Vot explícitament marcat invàlid",
            points_one: "({{count}} Punt)",
            points_many: "({{count}} Punts)",
            points_other: "({{count}} Punts)",
            contestNotFound: "Pregunta no trobada: {{contestId}}",
            declineToVote: "Vot no emès",
            blankBallot: "Papereta en blanc",
            acclamationDescription:
                "Aquesta votació s'ha resolt per aclamació. Les seves candidatures resulten elegides sense votació, per la qual cosa no es pot seleccionar cap opció ni es registra cap vot.",
        },
        "errors": {
            encoding: {
                notEnoughChoices: "No hi ha prou opcions per a descodificar",
                writeInChoiceOutOfRange: "Opció de vot escrita fora de rang: {{index}}",
                writeInNotEndInZero: "Opció de vot escrita no finalitza en 0",
                bytesToUtf8Conversion:
                    "Error convertint bytes de l'opció de vot escrita a cadena UTF-8: {{errorMessage}}",
                ballotTooLarge: "Vot més gran de l'esperat",
            },
            explicit: {
                notAllowed: "Vot marcat explícitament com a invàlid però la pregunta no ho permet",
            },
        },
        "receiptScreen": {
            title: "Comprovar un rebut de transparència",
            description:
                "Un rebut de transparència mostra que la vostra papereta és al registre públic de l'elecció, sense canvis i igual per a tothom. Aquesta pàgina el comprova al vostre navegador, amb la política de confiança que va publicar l'autoritat electoral.",
            loading: "S'està carregant el comprovador de rebuts…",
            loadFailed:
                "No s'ha pogut carregar el comprovador de rebuts. Torneu a carregar la pàgina per tornar-ho a provar.",
            receipt: {
                title: "El vostre rebut",
                description:
                    "Obriu el fitxer del rebut que vau desar en votar, o enganxeu el rebut, el seu enllaç o la seva referència (el text del seu codi QR).",
                fileFormat: "Els fitxers de rebut acaben en .vitrine-receipt",
                pasteLabel: "Rebut, enllaç o referència",
                pastePlaceholder: "Enganxeu-lo aquí",
                open: "Obre",
                another: "Comprova un altre rebut",
                problems: {
                    "empty": "Primer enganxeu un rebut, el seu enllaç o la seva referència.",
                    "too-large": "És massa gran per ser un rebut.",
                    "bad-link":
                        "Aquest enllaç no conté cap rebut que es pugui llegir. Obriu el fitxer del rebut.",
                    "bad-reference":
                        "Aquesta referència és incompleta o està malmesa. Copieu-la de nou o obriu el fitxer del rebut.",
                    "bad-chain": "Aquest rebut està malmès: no es pot llegir.",
                    "not-https":
                        "L'enllaç indica que la política és en una adreça que no és segura (https), així que no s'utilitza.",
                    "wrong-box":
                        "Això és una política de confiança, no un rebut. Primer obriu el rebut.",
                    "unrecognised": "Això no és un rebut, ni l'enllaç o la referència d'un rebut.",
                    "unreadable": "No s'ha pogut llegir el fitxer.",
                },
            },
            policy: {
                title: "Política de confiança",
                description:
                    "Els rebuts es comproven amb la política de confiança que va publicar l'autoritat electoral. Indiqueu-la aquí, amb el seu text o la seva adreça https, tal com apareix a l'anunci oficial de l'elecció.",
                fromLink:
                    "L'enllaç del rebut indica on és la seva política de confiança. Abans de fixar-la en aquest dispositiu, compareu els quatre primers grups del seu ID de política amb l'anunci oficial de l'elecció.",
                compareGroups: "Compareu els quatre primers grups, en negreta, amb l'anunci. Les eines comparen els 13.",
                inputLabel: "Política de confiança o la seva adreça",
                inputPlaceholder: "Enganxeu la política o la seva adreça https",
                load: "Carrega la política",
                openFile: "Obre un fitxer de política",
                loading: "S'està carregant la política de confiança…",
                pin: "Coincideixen: fixa aquesta política",
                reject: "No coincideixen",
                rejected:
                    "No utilitzeu aquesta política. Aviseu l'autoritat electoral i indiqueu la política de l'anunci oficial de l'elecció.",
                pinned: "Comprovat amb la política de confiança que vau fixar en aquest dispositiu.",
                unpin: "Oblida aquesta política",
                draft: "Això és un esborrany de la política, no la política segellada, així que no es pot fixar. El seu ID d'esborrany:",
                invalid: "Això no és una política de confiança vàlida.",
                otherPolicy:
                    "Aquesta política de confiança és vàlida, però el rebut és d'una altra.",
                ambiguous:
                    "Més d'una política fixada coincideix amb aquesta referència, així que no s'utilitza. Obriu el fitxer del rebut.",
                problems: {
                    "empty": "Primer enganxeu la política de confiança o la seva adreça.",
                    "too-large": "És massa gran per ser una política de confiança.",
                    "not-https": "Només s'utilitzen adreces que comencen per https.",
                    "wrong-box": "Això és un rebut, no una política de confiança.",
                    "unrecognised": "Això no és una política de confiança ni la seva adreça.",
                    "unreadable": "No s'ha pogut llegir el fitxer.",
                },
                fetchProblems: {
                    "not-https": "Només s'utilitzen adreces que comencen per https.",
                    "unreachable":
                        "No s'ha pogut carregar la política de confiança. Comproveu l'adreça o enganxeu la política.",
                    "status":
                        "El servidor no ha retornat la política de confiança (estat {{status}}).",
                    "too-large": "És massa gran per ser una política de confiança.",
                    "not-text": "L'adreça no ha retornat cap política de confiança.",
                },
            },
            ballot: {
                title: "La vostra papereta",
                description:
                    "El rebut només es mostra com a vostre per a la papereta que vau emetre. Enganxeu l'ID de seguiment complet que us va mostrar el portal de votació.",
                election: "ID de l'elecció",
                electionPlaceholder: "Trieu l'elecció",
                event: "ID de l'esdeveniment electoral",
                check: "Comprova el rebut",
                change: "Canvia l'ID de seguiment",
                lookedFor: "ID de seguiment cercat",
                fullId: "ID de seguiment complet",
                stage: "Pas de la papereta",
                stages: {received: "Rebuda", cast: "Emesa", audited: "Auditada"},
            },
            result: {
                title: "Resultat",
                rounds: {
                    log: "Comprovat amb els registres de l'elecció. Durant la mitja hora següent, aquesta pàgina també consultarà les rèpliques dels registres, els hubs d'ancoratge i els monitors, en un moment aleatori, perquè no puguin saber quan ho vau comprovar.",
                    full: "Comprovat amb els registres de l'elecció, les seves rèpliques, els hubs d'ancoratge i els monitors.",
                    offline:
                        "Els registres d'aquesta elecció són privats, així que el rebut s'ha comprovat amb el que conté.",
                },
                checkEverything: "Comprova-ho tot ara",
                checkAgain: "Torna a comprovar",
                retry: "Torna-ho a provar",
                problems: {
                    "policy-mismatch": "Aquesta referència és d'una altra política de confiança.",
                    "unknown-log": "La política de confiança no té cap registre amb aquest nom.",
                    "quiet-period":
                        "El registre no serveix proves durant el seu període de silenci. Torneu-ho a provar després de {{time}}.",
                    "not-found":
                        "El registre encara no té aquesta entrada. Si heu votat fa poc, torneu-ho a provar més tard.",
                    "entry-mismatch":
                        "L'entrada del registre en aquesta posició no és la de la referència.",
                    "private-reference":
                        "Els registres d'aquesta elecció són privats, així que aquí no es pot comprovar una referència. Obriu el fitxer del rebut.",
                    "failed": "No s'ha pogut comprovar el rebut. Torneu-ho a provar.",
                },
                evidenceSaved:
                    "S'ha desat el fitxer del rebut. Conté els objectes signats que es contradiuen: un auditor el pot comprovar, i crear-ne el fitxer de proves, amb el verificador de Vitrine.",
                advice: {
                    failed: "Una comprovació ha fallat: hi ha dades signades que es contradiuen entre elles o amb la política de confiança. Deseu les proves i lliureu-les, amb el fitxer del rebut, a l'autoritat electoral i als observadors que anomena la política de confiança.",
                    late: "Una cosa que ja hi hauria de ser encara no hi és. Potser només va lenta: torneu-ho a comprovar més tard i, si continua amb retard, aviseu l'autoritat electoral. Deseu el fitxer del rebut.",
                },
            },
            status: {
                checking: {
                    head: "Comprovant…",
                    sub: "El vostre navegador està comprovant que la vostra papereta és al registre públic.",
                },
                recorded: {
                    head: "Al registre públic",
                    sub: "El vostre navegador ha trobat la vostra papereta al registre públic. A continuació la confirmen els testimonis.",
                },
                witnessed: {
                    head: "Confirmat per testimonis",
                    sub: "Testimonis independents han confirmat el registre. A continuació es consulta els hubs públics, durant la mitja hora següent.",
                    subLater:
                        "Testimonis independents han confirmat el registre. Els ancoratges públics arriben en menys de 10 minuts; no cal esperar.",
                },
                verified: {
                    head: "Verificada",
                    sub: "La vostra papereta és al registre públic, confirmada per testimonis independents i ancorada públicament.",
                },
                late: {
                    head: "Triga més del previst",
                    sub: "La vostra papereta és al registre públic i confirmada per testimonis, però els seus ancoratges públics van amb retard.",
                    subOther: "Alguns dels passos de sota triguen més del previst.",
                },
                problem: {
                    head: "El registre no coincideix",
                    sub: "L'entrada núm. {{index}} del registre públic no és la d'aquest rebut.",
                    subOther:
                        "El registre públic, aquest rebut i el vostre ID de seguiment no coincideixen entre ells.",
                },
                damaged: {
                    head: "No s'ha pogut comprovar",
                    sub: "Algunes dades semblen malmeses: el rebut o la resposta del servidor. Això no demostra que ningú hagi fet trampa.",
                },
                evidence:
                    "El vostre navegador ha desat les proves: aquest rebut i l'entrada signada del registre. Envieu-les a l'autoritat electoral o a un observador; qualsevol les pot comprovar.",
                saveEvidence: "Desa les proves",
                howToReport: "Com denunciar-ho",
                captionChecking:
                    "El vostre navegador està comprovant el registre públic, amb codi que qualsevol pot inspeccionar.",
                captionChecked:
                    "Comprovat pel vostre navegador, amb codi que qualsevol pot inspeccionar.",
                captionProblem:
                    "Detectat pel vostre navegador, amb codi que qualsevol pot inspeccionar.",
                rows: {
                    record: "Al registre públic",
                    witnesses: "Confirmat per testimonis",
                    anchors: "Ancorat públicament",
                    recordDone:
                        "Entrada núm. {{index}}. El vostre navegador n'ha comprovat la prova.",
                    recordDoneNoIndex: "El vostre navegador n'ha comprovat la prova.",
                    witnessesDone:
                        "{{signed}} de {{total}} testimonis independents l'han signat; la política en demana {{needed}}.",
                    witnessesDoneAny: "Testimonis independents l'han signat.",
                    hubs_one: "A {{count}} hub públic",
                    hubs_many: "A {{count}} hubs públics",
                    hubs_other: "A {{count}} hubs públics",
                    browser: ", i allà on comproven els navegadors",
                    standIn: ", mitjançant un hub suplent de moment",
                    unused: "No la fa servir la política d'aquesta elecció.",
                    checking: "El vostre navegador està comprovant…",
                    anchorsActive:
                        "Es consulta durant la mitja hora següent, en un moment aleatori.",
                    anchorsPending:
                        "Encara no està ancorada. Torneu-ho a comprovar d'aquí a uns minuts.",
                    waiting: "Esperant el pas anterior.",
                    lateAnchors: "Els hubs l'haurien d'haver ancorada a les {{time}}.",
                    lateOther: "Ja hauria d'estar fet.",
                    recordFailed:
                        "L'entrada núm. {{index}} del registre difereix del vostre rebut.",
                    failed: "No coincideix amb el vostre rebut i el vostre ID de seguiment.",
                    damaged: "No s'ha pogut comprovar: les dades semblen malmeses.",
                    skipped: "No comprovat, perquè el registre no coincideix.",
                    skippedDamaged: "No comprovat, perquè un pas anterior no s'ha pogut comprovar.",
                    statusChecking: "Comprovant…",
                    withinSeconds: "En segons",
                    within10: "En 10 min",
                    within30: "En 30 min",
                    statusLate: "Amb retard",
                    statusMismatch: "No coincideix",
                    statusDamaged: "Malmès",
                },
            },
        },
        "footer": {
            poweredBy: "Funciona amb <1></1>",
        },
        "ballotIdField": {
            label: "ID de seguiment",
            placeholder: "Enganxeu el vostre ID de seguiment complet",
            help: "Els espais són opcionals.",
            shortError:
                "Aquest és l'ID curt. Enganxeu l'ID de seguiment complet: la comprovació el compara sencer.",
            formatError:
                "Reviseu l'ID de seguiment: té 64 caràcters, dígits i les lletres de la a a la f.",
        },
        "auditedScreen": {
            title: "Papereta",
            tag: "Auditada",
            loading: "S'està comprovant la papereta al registre públic…",
            retry: "Torna-ho a provar",
            header: {
                strip: {
                    withOperator:
                        "Un centre de transparència, gestionat per {{operator}}. No és el web de l'elecció i mai us demana votar, iniciar sessió ni pagar.",
                    noOperator:
                        "Un centre de transparència. No és el web de l'elecció i mai us demana votar, iniciar sessió ni pagar.",
                },
                subline: {
                    withOperator: "Gestionat per {{operator}} · un centre Vitrine",
                    noOperator: "Un centre Vitrine",
                },
                unnamed: "Centre de transparència",
            },
            trackingId: {
                full: "ID de seguiment complet",
                copy: "Copia l'ID de seguiment complet",
                copied: "ID de seguiment copiat",
                copyText: "Copia",
                copiedText: "Copiat",
                selected: "Seleccionat. Premeu Ctrl+C, o ⌘C en un Mac, per copiar-lo.",
            },
            checks: {
                title: "Comprovat al vostre navegador",
                record: {
                    title: "Al registre públic",
                    text: "Entrada núm. {{index}}, auditada a les {{time}}. Mai emesa.",
                    waiting: "S'està cercant la seva auditoria al registre de l'elecció.",
                    failed: "La prova del registre per a aquesta auditoria no és vàlida.",
                },
                secret: {
                    title: "El secret correspon a la papereta",
                    text: "En tornar a xifrar amb ell s'obté la mateixa papereta, byte a byte.",
                    waiting: "Es comprova quan s'hagi comprovat el registre.",
                    failed: "En tornar a xifrar amb ell no s'obté aquesta papereta.",
                },
                decoded: {
                    title: "Descodificada",
                    text: "Les opcions de l'esquerra, segons el disseny de papereta publicat de l'elecció.",
                    waiting: "Només quan el secret correspongui.",
                    failed: "No s'ha pogut descodificar.",
                },
            },
            note: {
                named: "Amb codi que tothom pot inspeccionar. {{hub}} només l'emmagatzema i el serveix, i mai pregunta què vau triar. La vostra resposta es queda en aquest navegador.",
                unnamed:
                    "Amb codi que tothom pot inspeccionar. Aquest centre només l'emmagatzema i el serveix, i mai pregunta què vau triar. La vostra resposta es queda en aquest navegador.",
            },
            states: {
                "bad-link": {
                    title: "Aquest enllaç no és complet",
                    text: "Necessita l'ID de seguiment complet de la papereta i l'ID de política de l'elecció. Torneu-lo a obrir des del lloc de votació.",
                },
                "no-policy": {
                    title: "No s'ha trobat la política de l'elecció",
                    text: "Aquesta pàgina només comprova una papereta amb la política que indica l'enllaç, i no l'ha pogut obtenir de l'elecció. Torneu-ho a provar més tard.",
                },
                "other-election": {
                    title: "Aquest enllaç és d'una altra elecció",
                    text: "La política que indica l'enllaç no cobreix aquesta elecció, de manera que la papereta no es pot comprovar amb ella.",
                },
                "not-found": {
                    title: "No hi ha cap papereta amb aquest ID de seguiment",
                    text: "El registre públic de l'elecció no té cap papereta amb aquest ID de seguiment. Comproveu que l'enllaç és complet o torneu-ho a provar d'aquí a un minut.",
                },
                "cast": {
                    title: "Aquesta papereta es va emetre, no es va auditar",
                    text: "Només es pot descodificar una papereta auditada. Una papereta emesa continua xifrada, de manera que ningú no veu com es va votar, ni aquí ni enlloc.",
                },
                "pending": {
                    title: "Aquesta papereta encara no està auditada",
                    text: "El registre públic té aquesta papereta, però encara no la seva auditoria. Torneu-ho a provar d'aquí a un minut.",
                },
                "no-content": {
                    title: "La papereta auditada encara no està publicada",
                    text: "El registre públic té l'auditoria d'aquesta papereta, però no la papereta i el seu secret, de manera que encara no es pot descodificar aquí. Torneu-ho a provar més tard.",
                },
                "unavailable": {
                    title: "No s'ha pogut comprovar la papereta",
                    text: "Aquesta pàgina no ha pogut accedir al registre públic de l'elecció. Torneu-ho a provar més tard.",
                },
                "record-failed": {
                    title: "El registre no prova aquesta auditoria",
                    text: "El registre de l'elecció ha donat una prova de l'auditoria d'aquesta papereta que no és vàlida, de manera que no es descodifica res. Aviseu l'autoritat electoral.",
                },
                "mismatch": {
                    title: "Auditoria fallida",
                    text: "La papereta del registre públic no és la que indica el seu ID de seguiment, o el secret que va publicar el portal no la reprodueix. No voteu al dispositiu des del qual vau auditar i aviseu l'autoritat electoral.",
                },
                "unchecked": {
                    title: "No s'ha pogut comprovar l'auditoria",
                    text: "Aquesta pàgina no ha pogut comprovar aquesta papereta auditada, de manera que no es mostra cap opció. Torneu-ho a provar més tard o en un altre dispositiu.",
                },
            },
            compare: {
                kicker: "Auditoria de la papereta",
                title: "Comproveu que la vostra papereta es va xifrar tal com vau triar",
                note: "Aquesta és la papereta que vau auditar, descodificada pel vostre navegador. Només vosaltres sabeu què vau triar, així que només vosaltres podeu fer aquesta comprovació.",
                steps: {
                    label: "Com funciona la comprovació",
                    encrypted: {
                        title: "Xifrada al vostre dispositiu",
                        text: "El portal va xifrar les vostres opcions i va donar a la papereta el seu ID de seguiment, abans que decidíssiu auditar-la.",
                    },
                    decoded: {
                        title: "Descodificada al vostre navegador",
                        text: "La papereta i el secret que va publicar el portal vénen del registre públic. Aquesta pàgina ha descodificat l'una amb l'altre.",
                    },
                    compare: {
                        title: "Compareu vosaltres",
                        text: "Mireu les opcions de sota. Són les que vau triar?",
                    },
                },
                choices: "Opcions descodificades d'aquesta papereta",
                question: "Coincideixen amb el que vau triar?",
                yes: "Sí, coincideixen",
                no: "No, hi ha alguna cosa diferent",
                change: "Canviar la meva resposta",
                match: {
                    title: "La vostra papereta es va xifrar tal com vau triar",
                    text: "Aquesta papereta auditada ja està gastada: mai es va emetre i mai es comptarà. Torneu enrere i voteu. El vostre vot rebrà un ID de seguiment nou, i si voleu podeu auditar-ne una altra abans.",
                    back: "Tornar al lloc de votació",
                },
                mismatch: {
                    title: "No voteu en aquest dispositiu",
                    text: "El que es va xifrar no és el que vau triar, així que aquest dispositiu o el portal de votació poden haver estat manipulats. El vostre navegador ha desat la prova: la papereta xifrada del registre públic, el secret que va revelar el portal i el que descodifica.",
                    report: "Aviseu l'autoritat electoral",
                    reportNoContact: "Aviseu l'autoritat electoral.",
                    fromPolicy: "De la política publicada de l'elecció, no d'aquesta pàgina.",
                    fromSettings: "De la configuració de l'elecció.",
                    then: "Després voteu des d'un altre dispositiu.",
                    save: "Desar la prova",
                    email: "Escriure a l'autoritat",
                    emailSubject:
                        "La meva papereta auditada no es descodifica en les meves opcions",
                    emailBody:
                        "He auditat una papereta en aquesta elecció i les opcions que ha descodificat el meu navegador no són les que vaig triar.\n\nID de seguiment: {{id}}\n\nHe desat la prova en un fitxer i la puc enviar.",
                },
                how: "Com funciona?",
                faq: {
                    why: {
                        question: "Per què auditar?",
                        answer: "El vostre vot es xifra al vostre dispositiu, així que ningú pot llegir-lo, ni tan sols vosaltres. Auditar us permet comprovar que el portal va xifrar el que vau triar abans de votar de debò.",
                    },
                    catch: {
                        question: "Per què detecta l'engany?",
                        answer: "El portal xifra abans que decidiu auditar, així que no pot saber quines paperetes es comprovaran. Si canviés vots, qualsevol votant que auditi ho podria descobrir.",
                    },
                    spent: {
                        question: "Per què no pot comptar aquesta papereta?",
                        answer: "Amb el secret revelat, la papereta mostra com vau votar. Per mantenir secret el vostre vot, una papereta auditada no s'emet mai.",
                    },
                    differ: {
                        question: "I si no coincideixen?",
                        answer: "No voteu en aquest dispositiu. Deseu la prova i aviseu l'autoritat electoral, el contacte de la qual surt de la política de l'elecció. Després voteu des d'un altre dispositiu.",
                    },
                    tip: "Per a la comprovació més sòlida, obriu aquesta pàgina en un dispositiu diferent d'aquell en què vau votar, per exemple el mòbil. Un dispositiu que canviés la vostra papereta també podria canviar el que us mostra aquí.",
                },
            },
        },
    },
}

export default catalanTranslation
