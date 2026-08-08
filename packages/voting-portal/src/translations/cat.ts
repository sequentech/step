// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {TranslationType} from "./en"

const catalanTranslation: TranslationType = {
    translations: {
        common: {
            goBack: "Tornar",
            showMore: "Mostra'n més",
            showLess: "Mostra'n menys",
        },
        candidatesList: {
            collapseToggle: "Alternar llista {{listTitle}}",
            showCandidates: "Mostra els candidats",
            hideCandidates: "Amaga els candidats",
            selectedCandidates_one: "{{count}} candidat seleccionat",
            selectedCandidates_other: "{{count}} candidats seleccionats",
            expandAll: "Expandir tot",
            collapseAll: "Reduir tot",
        },
        breadcrumbSteps: {
            electionList: "Votacions",
            ballot: "Papereta",
            review: "Revisió",
            confirmation: "Confirmar",
            audit: "Auditar",
        },
        footer: {
            poweredBy: "Funciona amb <1></1>",
        },
        votingScreen: {
            backButton: "Enrere",
            reviewButton: "Següent",
            clearButton: "Netejar opcions",
            ballotHelpDialog: {
                title: "Sobre aquesta pantalla",
                content:
                    "Aquesta pantalla mostra la votació en la qual vostè és elegible per votar. Pot seleccionar la seva secció activant la casella de la dreta Candidat/Resposta. Per restablir les seves seleccions, faci clic al botó “<b>Netejar selecció</b>”, per passar al següent pas, faci clic al botó “<b>Següent</b>”.",
                ok: "D'acord",
            },
            nonVotedDialog: {
                title: "El teu vot és invàlid o en blanc",
                content:
                    "Algunes de les seves respostes podrien fer que la papereta en una o més preguntes sigui invàlida o en blanc.",
                ok: "Revisar selecció",
                continue: "Continuar",
                cancel: "Cancel·lar",
            },
            warningDialog: {
                title: "Revisa la teva papereta",
                content:
                    "La teva papereta conté seleccions que poden necessitar la teva atenció (com ara seleccionar menys opcions de les permeses). La teva papereta és vàlida i es comptarà tal com s'ha enviat.",
                ok: "Torna i revisa",
                continue: "Continua",
                cancel: "Cancel·la",
            },
        },
        startScreen: {
            startButton: "Començar a votar",
            declineToVoteButton: "Declinar votar",
            declineToVoteDialog: {
                title: "Confirma que vols declinar votar",
                content:
                    "Segur que vols declinar votar?<br />Aniràs directament a la revisió i el teu estat de participació es desarà com a <b>Ha declinat votar</b>.",
                continue: "Declinar votar",
                cancel: "Cancel·lar",
            },
            instructionsTitle: "Com votar",
            instructionsDescription: "Seguiu aquests passos per emetre el vostre vot",
            step1Title: "1. Escolliu les vostres opcions",
            step1Description:
                "Escolliu els vostres candidats preferits i responeu cada pregunta de la papereta segons aparegui. Podeu canviar les vostres opcions en qualsevol moment abans d'emetre el vostre vot",
            step2Title: "2. Reviseu les vostres eleccions",
            step2Description:
                "Quan estigueu satisfets amb les vostres seleccions, xifrem la vostra papereta de forma segura i us mostrarem una revisió final. També rebreu un ID de seguiment únic com a referència",
            step3Title: "3. Envieu el vostre vot",
            step3Description:
                "Quan estigueu a punt, emeteu la vostra papereta perquè quedi registrada oficialment. O trieu auditar primer per confirmar que va ser capturada i xifrada correctament",
        },
        reviewScreen: {
            title: "Revisa el teu vot",
            description:
                "Per fer canvis a les seves seleccions, faci clic al botó “<b>Edita el teu vot</b>”, per confirmar les seves seleccions, faci clic al botó “<b>Envia el teu vot</b>” a sota, i per auditar la seva papereta faci clic al botó “<b>Auditar papereta</b>” a sota.",
            descriptionNoAudit:
                "Per fer canvis a les seves seleccions, faci clic al botó “<b>Edita el teu vot</b>”, per confirmar les seves seleccions, faci clic al botó “<b>Envia el teu vot</b>” a sota.",
            backButton: "Edita el teu vot",
            castBallotButton: "Envia el vot",
            auditButton: "Auditar papereta",
            reviewScreenHelpDialog: {
                title: "Sobre la pantalla de revisió",
                content:
                    "Aquesta pantalla us permet revisar les vostres seleccions abans d'emetre el vot",
                ok: "D'acord",
            },
            ballotIdHelpDialog: {
                title: "El teu vot no ha estat emès",
                content:
                    "<p>Aquest és el vostre Localitzador del Vot, però <b>el vostre vot encara no s'ha emès</b>. Si intenteu buscar-lo ara, no apareixerà.</p><p>Mostrem el Localitzador del Vot en aquesta etapa perquè pugueu auditar la papereta xifrada abans d'emetre-la.</p>",
                ok: "Entenc que el meu vot no ha estat emès",
                cancel: "Cancel·lar",
            },
            auditBallotHelpDialog: {
                title: "Vols auditar la teva papereta?",
                content:
                    "<p>Auditar la teva papereta l'invalidarà i hauràs de reiniciar el procés de votació. Continua només si et sents còmode amb els passos tècnics avançats. En cas contrari, fes clic a <u>Cancel·la</u> per tornar.</p>",
                ok: "Sí, descartar la meva papereta per auditar-la",
                cancel: "Cancel·lar",
            },
            confirmCastVoteDialog: {
                title: "Esteu segur que voleu emetre el vostre vot?",
                content: "Un cop confirmeu, el vostre vot serà emès.",
                ok: "Sí, vull emetre el meu vot",
                cancel: "Cancel·lar",
            },
            error: {
                NETWORK_ERROR:
                    "Hi ha hagut un problema de xarxa. Si us plau, torna-ho a intentar més tard o contacta amb el servei d'assistència.",
                UNABLE_TO_FETCH_DATA:
                    "Hi ha hagut un problema en recuperar les dades. Si us plau, torna-ho a intentar més tard o contacta amb el servei d'assistència.",
                LOAD_ELECTION_EVENT:
                    "No es pot carregar l'esdeveniment electoral. Si us plau, torna-ho a intentar més tard.",
                CAST_VOTE:
                    "Hi ha hagut un error desconegut en emetre el vot. Si us plau, torni-ho a provar més tard o contacti amb el suport per obtenir ajuda.",
                CAST_VOTE_CheckStatusFailed:
                    "L'elecció no permet emetre el vot. L'elecció pot estar tancada, arxivada o potser estàs intentant votar fora del període de gràcia.",
                CAST_VOTE_AreaNotFound:
                    "Hi ha hagut un error en emetre el vot: Àrea no trobada. Si us plau, torni-ho a provar més tard o contacti amb el suport per obtenir ajuda.",
                CAST_VOTE_InternalServerError:
                    "Hi ha hagut un error intern en emetre el vot. Si us plau, torni-ho a provar més tard o contacti amb el suport per obtenir ajuda.",
                CAST_VOTE_QueueError:
                    "Hi ha hagut un problema en processar el seu vot. Si us plau, torni-ho a provar més tard o contacti amb el suport per obtenir ajuda.",
                CAST_VOTE_Unauthorized:
                    "No està autoritzat per emetre un vot. Si us plau, contacti amb el suport per obtenir ajuda.",
                CAST_VOTE_ElectionEventNotFound:
                    "No s'ha pogut trobar l'esdeveniment electoral. Si us plau, torni-ho a provar més tard o contacti amb el suport per obtenir ajuda.",
                CAST_VOTE_ElectoralLogNotFound:
                    "No s'ha pogut trobar el seu registre de vot. Si us plau, contacti amb el suport per obtenir ajuda.",
                CAST_VOTE_CheckPreviousVotesFailed:
                    "Hi ha hagut un error en comprovar el seu estat de votació. Si us plau, torni-ho a provar més tard o contacti amb el suport per obtenir ajuda.",
                CAST_VOTE_GetClientCredentialsFailed:
                    "No s'han pogut verificar les seves credencials. Si us plau, torni-ho a provar més tard o contacti amb el suport per obtenir ajuda.",
                CAST_VOTE_GetAreaIdFailed:
                    "Hi ha hagut un error en verificar la seva àrea de votació. Si us plau, torni-ho a provar més tard o contacti amb el suport per obtenir ajuda.",
                CAST_VOTE_GetTransactionFailed:
                    "Hi ha hagut un error en processar el seu vot. Si us plau, torni-ho a provar més tard o contacti amb el suport per obtenir ajuda.",
                CAST_VOTE_DeserializeBallotFailed:
                    "Hi ha hagut un error en llegir la seva papereta. Si us plau, torni-ho a provar més tard o contacti amb el suport per obtenir ajuda.",
                CAST_VOTE_DeserializeContestsFailed:
                    "Hi ha hagut un error en llegir les seves seleccions. Si us plau, torni-ho a provar més tard o contacti amb el suport per obtenir ajuda.",
                CAST_VOTE_PokValidationFailed:
                    "No s'ha pogut validar el seu vot. Si us plau, torni-ho a provar més tard o contacti amb el suport per obtenir ajuda.",
                CAST_VOTE_UuidParseFailed:
                    "Hi ha hagut un error en processar la seva sol·licitud. Si us plau, torni-ho a provar més tard o contacti amb el suport per obtenir ajuda.",
                CAST_VOTE_unexpected:
                    "Hi ha hagut un error desconegut en emetre el vot. Si us plau, torni-ho a provar més tard o contacti amb el suport per obtenir ajuda.",
                CAST_VOTE_timeout:
                    "Error de temps d'espera per emetre el vot. Si us plau, torneu-ho a provar més tard o contacteu amb l'assistència per obtenir ajuda.",
                CAST_VOTE_InsertFailedExceedsAllowedRevotes:
                    "Heu superat el límit de revots. Si us plau, torneu-ho a provar més tard o contacteu amb l'assistència per obtenir ajuda.",
                CAST_VOTE_CheckRevotesFailed:
                    "Heu superat el nombre permès de revots. Si us plau, torneu-ho a provar més tard o contacteu amb l'assistència per obtenir ajuda.",
                CAST_VOTE_CheckVotesInOtherAreasFailed:
                    "Ja heu votat en una altra àrea. Si us plau, torneu-ho a provar més tard o contacteu amb l'assistència per obtenir ajuda.",
                CAST_VOTE_UnknownError:
                    "Hi ha hagut un error desconegut en emetre el vot. Si us plau, torni-ho a provar més tard o contacti amb el suport per obtenir ajuda.",
                NO_BALLOT_SELECTION:
                    "No es troba l'estat de selecció per aquesta elecció. Si us plau, assegura't d'haver seleccionat les teves opcions correctament o contacta amb el servei d'assistència.",
                NO_BALLOT_STYLE:
                    "L'estil de la papereta no està disponible. Si us plau, contacta amb el servei d'assistència.",
                NO_AUDITABLE_BALLOT:
                    "No hi ha cap papereta auditable disponible. Si us plau, contacta amb el servei d'assistència.",
                INCONSISTENT_HASH:
                    "Hi ha hagut un error relacionat amb el procés de hashing de la papereta. El BallotId: {{ballotId}} no és consistent amb el Hash de la Papereta Auditable: {{auditableBallotHash}}. Si us plau, informa d'aquest problema al servei d'assistència.",
                ELECTION_EVENT_NOT_OPEN:
                    "L'esdeveniment electoral està tancat. Si us plau, contacta amb el servei d'assistència.",
                PARSE_ERROR:
                    "Hi ha hagut un error en analitzar la papereta. Si us plau, torna-ho a intentar més tard o contacta amb el servei d'assistència.",
                DESERIALIZE_AUDITABLE_ERROR:
                    "Hi ha hagut un error en deserialitzar la papereta auditable. Si us plau, torna-ho a intentar més tard o contacta amb el servei d'assistència.",
                DESERIALIZE_HASHABLE_ERROR:
                    "Hi ha hagut un error en deserialitzar la papereta hashable. Si us plau, torna-ho a intentar més tard o contacta amb el servei d'assistència.",
                CONVERT_ERROR:
                    "Hi ha hagut un error en convertir la papereta. Si us plau, torna-ho a intentar més tard o contacta amb el servei d'assistència.",
                SERIALIZE_ERROR:
                    "Hi ha hagut un error en serialitzar la papereta. Si us plau, torna-ho a intentar més tard o contacta amb el servei d'assistència.",
                UNKNOWN_ERROR:
                    "Hi ha hagut un error. Si us plau, torna-ho a intentar més tard o contacta amb el servei d'assistència.",
                REAUTH_FAILED:
                    "L'autenticació ha fallat. Si us plau, torna-ho a intentar o contacta amb el servei d'assistència.",
                SESSION_EXPIRED:
                    "La teva sessió ha caducat. Si us plau, torna a començar des del principi.",
                CAST_VOTE_BallotIdMismatch:
                    "L'identificador de la papereta no coincideix amb el del vot emès.",
                SESSION_STORAGE_ERROR:
                    "L'emmagatzematge de sessió no està disponible. Si us plau, torneu-ho a provar o contacteu amb el suport.",
                PARSE_BALLOT_DATA_ERROR:
                    "S'ha produït un error en analitzar les dades de la papereta. Si us plau, torneu-ho a provar més tard o contacteu amb el suport per rebre assistència.",
                NOT_VALID_BALLOT_DATA_ERROR:
                    "Les dades de la papereta no són vàlides. Si us plau, torneu-ho a provar més tard o contacteu amb el suport per rebre assistència.",
                FETCH_DATA_TIMEOUT_ERROR:
                    "Error de temps d'espera en obtenir les dades. Si us plau, torneu-ho a provar més tard o contacteu amb el suport per rebre assistència.",
                TO_HASHABLE_BALLOT_ERROR:
                    "Error en convertir a papereta hashable. Si us plau, torneu-ho a provar més tard o contacteu amb el suport per rebre assistència.",
                INTERNAL_ERROR:
                    "S'ha produït un error intern en emetre el vot. Si us plau, torneu-ho a provar més tard o contacteu amb el suport per rebre assistència.",
            },
            declineToVote: "Declinar votar",
        },
        confirmationScreen: {
            title: "El seu vot ha estat emès",
            description:
                "La seva papereta va ser emesa correctament. Usi el codi a continuació per verificar que va ser comptabilitzada",
            ballotId: "Localitzador del Vot",
            printButton: "Imprimir",
            finishButton: "Finalitzar",
            verifyCastTitle: "Comproveu que el vostre vot va ser emès",
            verifyCastDescription:
                "Podeu verificar en qualsevol moment que la vostra papereta va ser emesa correctament usant el codi QR a continuació",
            confirmationHelpDialog: {
                title: "Sobre la pantalla de confirmació",
                content:
                    "Aquesta pantalla confirma que el vostre vot va ser emès correctament. La informació aquí us permet verificar que la papereta va ser emmagatzemada a l'urna, tant durant el període de votació com després del seu tancament",
                ok: "D'acord",
            },
            demoPrintDialog: {
                title: "Impressió de la papereta de vot",
                content: "Impressió desactivada en mode de demostració",
                ok: "D'acord",
            },
            demoBallotUrlDialog: {
                title: "Seguiment de la Butlleta",
                content: "No es pot utilitzar el codi, desactivat en mode de demostració.",
                ok: "D'acord",
            },
            ballotIdHelpDialog: {
                title: "Sobre el Localitzador del Vot",
                content:
                    "El Localitzador del Vot és un codi únic que us permet trobar la vostra papereta a l'urna. No conté informació sobre les vostres seleccions.",
                ok: "D'acord",
            },
            ballotIdDemoHelpDialog: {
                title: "Sobre el Localitzador del Vot",
                content:
                    "L'identificador de papereta de vot és un codi que us permet trobar la vostra papereta a l'urna. Aquest identificador és únic i no conté informació sobre les vostres seleccions.",
                ok: "D'acord",
            },
            errorDialogPrintBallotReceipt: {
                title: "Error",
                content: "Ha ocorregut un error, si us plau intenti de nou",
                ok: "Acceptar",
            },
            demoQRText: "El rastrejador de butlletes està deshabilitat en mode de demostració",
        },
        auditScreen: {
            printButton: "Imprimir",
            restartButton: "Iniciar votació",
            title: "Comproveu la vostra papereta",
            description: "Per comprovar la vostra papereta, seguiu els passos a continuació:",
            step1Title: "1. Deseu les dades següents:",
            step1Description:
                "el vostre <b>Localitzador del Vot</b> a la part superior de la pantalla i la vostra papereta encriptada a continuació",
            step1HelpDialog: {
                title: "Copiar el codi de la papereta",
                content:
                    "Podeu descarregar o copiar el codi de la vostra papereta per verificar que reflecteix correctament les vostres seleccions.",
                ok: "D'acord",
            },
            downloadButton: "Descarregar",
            step2Title: "2. Comproveu la vostra papereta",
            step2Description:
                "Feu clic a <VerifierLink>Comprova el codi de la teva papereta</VerifierLink>. S'obrirà en una nova pestanya",
            step2HelpDialog: {
                title: "Com comprovar el codi de la papereta",
                content:
                    "Per comprovar el codi de la vostra papereta, seguiu els passos de la guia. Inclou la descàrrega d'una aplicació d'escriptori per verificar la vostra papereta de forma independent al lloc web.",
                ok: "D'acord",
            },
            bottomWarning:
                "Per motius de seguretat, quan auditeu la vostra papereta, haurà d'invalidar-la. Per continuar amb el procés de votació, faci clic a ‘<b>Iniciar votació</b>’.",
        },
        electionSelectionScreen: {
            title: "Llista de Votacions",
            description: "Seleccioneu la papereta en la qual voleu votar",
            chooserHelpDialog: {
                title: "Sobre la Llista de Votacions",
                content:
                    "Aquesta pantalla mostra la llista de paperetes a les quals podeu accedir. Poden estar obertes, programades o tancades. Només podeu votar en les que estan obertes",
                ok: "D'acord",
            },
            noResults: "No hi ha eleccions per ara.",
            resultsButton: "Veure resultats",
            demoDialog: {
                title: "Cabina de votació de demostració",
                content:
                    "Esteu entrant en una cabina de votació de demostració. <strong>El vostre vot no serà comptabilitzat.</strong> Aquesta cabina és només per a fins de demostració.",
                ok: "Entenc que el meu vot no serà comptabilitzat",
            },
            errors: {
                noVotingArea:
                    "No esteu registrat com a votant en aquesta elecció. Si us plau, contacteu amb el suport.",
                networkError:
                    "Hi ha hagut un problema de xarxa. Si us plau, torneu-ho a intentar més tard o contacteu amb suport per obtenir ajuda.",
                unableToFetchData:
                    "Hi ha hagut un problema a l'obtenció de les dades. Si us plau, torneu-ho a intentar més tard o contacteu amb suport per obtenir ajuda.",
                noElectionEvent:
                    "L'esdeveniment electoral no existeix. Si us plau, torneu-ho a intentar més tard o contacteu amb suport per obtenir ajuda.",
                ballotStylesEmlError:
                    "Hi ha hagut un error amb la publicació de l'estil de la papereta. Si us plau, torneu-ho a intentar més tard o contacteu amb suport per obtenir ajuda.",
                obtainingElectionFromID:
                    "Hi ha hagut un error a l'obtenció de les eleccions associades amb les següents IDs d'eleccions: {{electionIds}}. Si us plau, torneu-ho a intentar més tard o contacteu amb suport per obtenir ajuda.",
            },
            alerts: {
                noElections:
                    "No hi ha eleccions en les quals pugueu votar. Això podria ser perquè l'àrea no té cap pregunta associada. Si us plau, torneu-ho a intentar més tard o contacteu amb suport per obtenir ajuda.",
                electionEventNotPublished:
                    "L'esdeveniment electoral encara no ha estat publicat. Si us plau, torneu-ho a intentar més tard o contacteu amb suport per obtenir ajuda.",
            },
        },
        errors: {
            encoding: {
                notEnoughChoices: "No hi ha prou opcions per descodificar",
                writeInChoiceOutOfRange: "Opció d'escriptura lliure fora de rang: {{index}}",
                writeInNotEndInZero: "L'escriptura lliure no acaba en 0",
                bytesToUtf8Conversion:
                    "Error en convertir l'escriptura lliure de bytes a cadena UTF-8: {{errorMessage}}",
                ballotTooLarge: "La papereta és més gran de l'esperada",
            },
            explicit: {
                notAllowed:
                    "La papereta està marcada com explícitament invàlida, però la pregunta no ho permet",
                alert: "Aquesta selecció es comptarà com un vot invàlid",
            },
            page: {
                oopsWithStatus: "Vaja! {{status}}",
                oopsWithoutStatus: "Vaja! Error Inesperat",
                somethingWrong: "Alguna cosa ha anat malament.",
                certAuthFailedTitle: "Error d'Autenticació amb Certificat",
                certAuthFailedMessage:
                    "No s'ha pogut verificar el vostre certificat. Comproveu que esteu utilitzant un certificat de votant vàlid i torneu-ho a intentar.",
            },
        },
        materials: {
            common: {
                label: "Materials de Suport",
                back: "Tornar a la llista de votacions",
                close: "Tancar",
                preview: "Vista prèvia",
            },
        },
        ballotLocator: {
            title: "Troba la teva Papereta",
            titleResult: "Resultats de la cerca de la teva Papereta",
            description: "Confirma que la teva papereta va ser emesa correctament",
            locate: "Troba la teva Papereta",
            locateAgain: "Troba una altra Papereta",
            found: "El teu ID de Papereta {{ballotId}} ha estat trobat",
            notFound: "El teu ID de Papereta {{ballotId}} no ha estat trobat",
            ambiguous:
                "Més d'una de les teves paperetes coincideix amb {{ballotId}}. Utilitza l'ID complet de la papereta.",
            contentDesc: "Aquest és el contingut de la teva Papereta: ",
            wrongFormatBallotId: "Format incorrecte per l'ID de la Papereta",
            ballotIdNotFoundAtFilter:
                "No trobat, comprova que l'ID de la Papereta estigui correcte i pertanyi a l'usuari actual.",
            filterByBallotId: "Filtra per ID de la Papereta",
            totalBallots: "Paperetes: {{total}}",
            steps: {
                lookup: "Troba la teva Papereta",
                result: "Resultat",
            },
            titleHelpDialog: {
                title: "Sobre el Cercador de Paperetes",
                content:
                    "El Cercador de Paperetes us permet introduir el vostre ID de Papereta per localitzar el vostre vot i confirmar que va ser registrat correctament.",
                ok: "D'acord",
            },
            tabs: {
                logs: "Logs",
                ballotLocator: "Localitzador de la Papereta",
            },
            column: {
                statement_kind: "Tipus",
                statement_timestamp: "Marca de temps",
                username: "Usuari",
                ballot_id: "ID de la Papereta",
                message: "Missatge",
            },
        },
    },
}

export default catalanTranslation
