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
            step2: "Pas 2: Introduïu el vostre ID de papereta.",
            description2:
                "Si us plau, introduïu l'ID de la papereta proporcionat al Portal de Votació:",
            ballotIdHelpDialog: {
                title: "Informació: El vostre ID de papereta",
                ok: "D'acord",
                content:
                    "Si us plau, introduïu l'ID de la papereta proporcionat al Portal de Votació.",
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
            useSampleLink: "Utilitzeu vot d'exemple",
            nextButton: "Continuar",
            ballotIdLabel: "ID de papereta",
            ballotIdPlaceholder: "Escriviu aquí el vostre ID de papereta",
            fileUploaded: "Carregat",
            receiptTitle: "Rebut de transparència",
            receiptDescription:
                "Aquesta elecció publica un rebut de cada papereta emesa. Comproveu el vostre: que és al registre públic, sense canvis i igual per a tothom.",
            receiptButton: "Comprova un rebut",
        },
        "confirmationScreen": {
            ballotIdTitle: "ID de papereta",
            ballotIdDescription:
                "A continuació, el sistema mostra l'ID de la papereta descodificada i el generat pel verificador.",
            ballotIdError: "No coincideix amb l'ID de papereta descodificat.",
            decodedBallotId: "ID de papereta descodificat",
            decodedBallotIdHelpDialog: {
                title: "Informació: ID de papereta descodificat",
                ok: "D'acord",
                content:
                    "Aquest és l'ID de la papereta extret del fitxer de la Papereta Auditable descodificada que vas proporcionar.",
            },
            yourBallotId: "L'ID de papereta que vas proporcionar",
            userBallotIdHelpDialog: {
                title: "Informació: L'ID de papereta que vas proporcionar",
                ok: "D'acord",
                content:
                    "Aquesta és l'ID de papereta que vas escriure en l'anterior pas i que vas recollir de la Cabina de Votació.",
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
                    "L'enllaç del rebut indica on és la seva política de confiança. Abans de fixar-la en aquest dispositiu, compareu-ne les paraules i l'ID amb l'anunci oficial de l'elecció.",
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
                draft: "Això és un esborrany de la política, no la política segellada, així que no es pot fixar. Les seves paraules d'esborrany:",
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
                    "El rebut només es mostra com a vostre per a la papereta que vau emetre. Escriviu l'ID de papereta que us va mostrar el portal de votació.",
                ballotId: "ID de papereta",
                ballotIdPlaceholder: "Escriviu el vostre ID de papereta",
                election: "ID de l'elecció",
                electionPlaceholder: "Trieu l'elecció",
                event: "ID de l'esdeveniment electoral",
                check: "Comprova el rebut",
                change: "Canvia l'ID de papereta",
                invalid:
                    "Reviseu l'ID de papereta: té 64 caràcters, dígits i lletres de la a a la f.",
                lookedFor: "ID de papereta cercat:",
            },
            result: {
                title: "Resultat",
                checking: "S'està comprovant…",
                rounds: {
                    log: "Comprovat amb els registres de l'elecció. Durant la mitja hora següent, aquesta pàgina també consultarà les rèpliques dels registres, els hubs d'ancoratge i els monitors, en un moment aleatori, perquè no puguin saber quan vau obrir el rebut. Manteniu la pàgina oberta o comproveu-ho tot ara.",
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
        },
        "footer": {
            poweredBy: "Funciona amb <1></1>",
        },
    },
}

export default catalanTranslation
