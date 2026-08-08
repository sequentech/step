// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {TranslationType} from "./en"

const catalanInformalTranslation: TranslationType = {
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
            clearButton: "Netejar seleccions",
            ballotHelpDialog: {
                title: "Sobre aquesta pantalla",
                content:
                    "Aquesta pantalla mostra les preguntes en les quals ets elegible per votar. Pots fer la teva selecció activant la casella a la dreta del Candidat/Resposta. Per restablir les teves seleccions, fes clic al botó “<b>Netejar seleccions</b>”, per passar al següent pas, fes clic al botó “<b>Següent</b>”.",
                ok: "D'acord",
            },
            nonVotedDialog: {
                title: "El teu vot és invàlid o en blanc",
                content:
                    "Algunes de les teves respostes podrien fer que la papereta en una o més preguntes sigui invàlida o en blanc.",
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
            instructionsDescription: "Segueix aquests passos per emetre el teu vot",
            step1Title: "1. Fes les teves seleccions",
            step1Description:
                "Escull els teus candidats preferits i respon cada pregunta de la papereta segons aparegui. Pots canviar les teves seleccions en qualsevol moment abans d'emetre el teu vot",
            step2Title: "2. Revisa les teves seleccions",
            step2Description:
                "Quan estiguis satisfet amb les teves seleccions, xifrem la teva papereta de forma segura i et mostrarem una revisió final. També rebràs un ID de seguiment únic com a referència",
            step3Title: "3. Emet la teva papereta",
            step3Description:
                "Quan estiguis a punt, emet la teva papereta perquè quedi registrada oficialment. O tria auditar primer per confirmar que va ser capturada i xifrada correctament",
        },
        reviewScreen: {
            title: "Revisa el teu vot",
            description:
                "Per fer canvis a les teves seleccions, fes clic al botó “<b>Edita el teu vot</b>”, per confirmar les teves seleccions, fes clic al botó “<b>Envia el vot</b>” a sota, i per auditar la teva papereta fes clic al botó “<b>Auditar papereta</b>” a sota.",
            descriptionNoAudit:
                "Per fer canvis a les teves seleccions, fes clic al botó “<b>Edita el teu vot</b>”, per confirmar les teves seleccions, fes clic al botó “<b>Envia el vot</b>” a sota.",
            backButton: "Edita el teu vot",
            castBallotButton: "Envia el vot",
            auditButton: "Auditar papereta",
            reviewScreenHelpDialog: {
                title: "Sobre la pantalla de revisió",
                content:
                    "Aquesta pantalla et permet revisar les teves seleccions abans d'emetre el vot",
                ok: "D'acord",
            },
            ballotIdHelpDialog: {
                title: "El teu vot no ha estat emès",
                content:
                    "<p>Aquest és el teu Localitzador del Vot, però <b>el teu vot encara no s'ha emès</b>. Si intentes buscar-lo ara, no apareixerà.</p><p>Mostrem el Localitzador del Vot en aquesta etapa perquè puguis auditar la papereta xifrada abans d'emetre-la.</p>",
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
                title: "Estàs segur que vols emetre el teu vot?",
                content: "Un cop confirmis, el teu vot serà emès.",
                ok: "Sí, vull emetre el meu vot",
                cancel: "Cancel·lar",
            },
            error: {
                NETWORK_ERROR:
                    "Hi ha hagut un problema de xarxa. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
                UNABLE_TO_FETCH_DATA:
                    "Hi ha hagut un problema en recuperar les dades. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
                LOAD_ELECTION_EVENT:
                    "No es pot carregar l'esdeveniment electoral. Si us plau, torna-ho a provar més tard.",
                CAST_VOTE:
                    "Hi ha hagut un error desconegut en emetre el vot. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
                CAST_VOTE_CheckStatusFailed:
                    "L'elecció no permet emetre el vot. L'elecció pot estar tancada, arxivada o potser estàs intentant votar fora del període de gràcia.",
                CAST_VOTE_AreaNotFound:
                    "Hi ha hagut un error en emetre el vot: àrea no trobada. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
                CAST_VOTE_InternalServerError:
                    "Hi ha hagut un error intern en emetre el vot. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
                CAST_VOTE_QueueError:
                    "Hi ha hagut un problema en processar el teu vot. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
                CAST_VOTE_Unauthorized:
                    "No estàs autoritzat per emetre un vot. Si us plau, contacta amb el servei d'assistència.",
                CAST_VOTE_ElectionEventNotFound:
                    "No s'ha pogut trobar l'esdeveniment electoral. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
                CAST_VOTE_ElectoralLogNotFound:
                    "No s'ha pogut trobar el teu registre de vot. Si us plau, contacta amb el servei d'assistència.",
                CAST_VOTE_CheckPreviousVotesFailed:
                    "Hi ha hagut un error en comprovar el teu estat de votació. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
                CAST_VOTE_GetClientCredentialsFailed:
                    "No s'han pogut verificar les teves credencials. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
                CAST_VOTE_GetAreaIdFailed:
                    "Hi ha hagut un error en verificar la teva àrea de votació. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
                CAST_VOTE_GetTransactionFailed:
                    "Hi ha hagut un error en processar el teu vot. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
                CAST_VOTE_DeserializeBallotFailed:
                    "Hi ha hagut un error en llegir la teva papereta. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
                CAST_VOTE_DeserializeContestsFailed:
                    "Hi ha hagut un error en llegir les teves seleccions. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
                CAST_VOTE_PokValidationFailed:
                    "No s'ha pogut validar el teu vot. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
                CAST_VOTE_UuidParseFailed:
                    "Hi ha hagut un error en processar la teva sol·licitud. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
                CAST_VOTE_unexpected:
                    "Hi ha hagut un error desconegut en emetre el vot. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
                CAST_VOTE_timeout:
                    "Error de temps d'espera en emetre el vot. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
                CAST_VOTE_InsertFailedExceedsAllowedRevotes:
                    "Has superat el límit de revots. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
                CAST_VOTE_CheckRevotesFailed:
                    "Has superat el nombre permès de revots. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
                CAST_VOTE_CheckVotesInOtherAreasFailed:
                    "Ja has votat en una altra àrea. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
                CAST_VOTE_UnknownError:
                    "Hi ha hagut un error desconegut en emetre el vot. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
                NO_BALLOT_SELECTION:
                    "No es troba l'estat de selecció per a aquesta elecció. Si us plau, assegura't d'haver seleccionat les teves opcions correctament o contacta amb el servei d'assistència.",
                NO_BALLOT_STYLE:
                    "L'estil de la papereta no està disponible. Si us plau, contacta amb el servei d'assistència.",
                NO_AUDITABLE_BALLOT:
                    "No hi ha cap papereta auditable disponible. Si us plau, contacta amb el servei d'assistència.",
                INCONSISTENT_HASH:
                    "Hi ha hagut un error relacionat amb el procés de hashing de la papereta. El BallotId: {{ballotId}} no és consistent amb el Hash de la Papereta Auditable: {{auditableBallotHash}}. Si us plau, informa d'aquest problema al servei d'assistència.",
                ELECTION_EVENT_NOT_OPEN:
                    "L'esdeveniment electoral està tancat. Si us plau, contacta amb el servei d'assistència.",
                PARSE_ERROR:
                    "Hi ha hagut un error en analitzar la papereta. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
                DESERIALIZE_AUDITABLE_ERROR:
                    "Hi ha hagut un error en deserialitzar la papereta auditable. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
                DESERIALIZE_HASHABLE_ERROR:
                    "Hi ha hagut un error en deserialitzar la papereta hashable. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
                CONVERT_ERROR:
                    "Hi ha hagut un error en convertir la papereta. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
                SERIALIZE_ERROR:
                    "Hi ha hagut un error en serialitzar la papereta. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
                UNKNOWN_ERROR:
                    "Hi ha hagut un error. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
                REAUTH_FAILED:
                    "L'autenticació ha fallat. Si us plau, torna-ho a provar o contacta amb el servei d'assistència.",
                SESSION_EXPIRED:
                    "La teva sessió ha caducat. Si us plau, torna a començar des del principi.",
                CAST_VOTE_BallotIdMismatch:
                    "L'identificador de la papereta no coincideix amb el del vot emès.",
                SESSION_STORAGE_ERROR:
                    "L'emmagatzematge de sessió no està disponible. Si us plau, torna-ho a provar o contacta amb el servei d'assistència.",
                PARSE_BALLOT_DATA_ERROR:
                    "S'ha produït un error en analitzar les dades de la papereta. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
                NOT_VALID_BALLOT_DATA_ERROR:
                    "Les dades de la papereta no són vàlides. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
                FETCH_DATA_TIMEOUT_ERROR:
                    "Error de temps d'espera en obtenir les dades. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
                TO_HASHABLE_BALLOT_ERROR:
                    "Error en convertir a papereta hashable. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
                INTERNAL_ERROR:
                    "S'ha produït un error intern en emetre el vot. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
            },
            declineToVote: "Declinar votar",
        },
        confirmationScreen: {
            title: "El teu vot ha estat emès",
            description:
                "La teva papereta va ser emesa correctament. Utilitza el codi a continuació per verificar que va ser comptabilitzada",
            ballotId: "Localitzador del Vot",
            printButton: "Imprimir",
            finishButton: "Finalitzar",
            verifyCastTitle: "Comprova que el teu vot va ser emès",
            verifyCastDescription:
                "Pots verificar en qualsevol moment que la teva papereta va ser emesa correctament usant el codi QR a continuació",
            confirmationHelpDialog: {
                title: "Sobre la pantalla de confirmació",
                content:
                    "Aquesta pantalla confirma que el teu vot va ser emès correctament. La informació aquí et permet verificar que la papereta va ser emmagatzemada a l'urna, tant durant el període de votació com després del seu tancament",
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
                    "El Localitzador del Vot és un codi únic que et permet trobar la teva papereta a l'urna. No conté informació sobre les teves seleccions.",
                ok: "D'acord",
            },
            ballotIdDemoHelpDialog: {
                title: "Sobre el Localitzador del Vot",
                content:
                    "L'identificador de papereta de vot és un codi que et permet trobar la teva papereta a l'urna. Aquest identificador és únic i no conté informació sobre les teves seleccions.",
                ok: "D'acord",
            },
            errorDialogPrintBallotReceipt: {
                title: "Error",
                content: "Hi ha hagut un error, si us plau intenta-ho de nou",
                ok: "Acceptar",
            },
            demoQRText: "El rastrejador de butlletes està deshabilitat en mode de demostració",
        },
        auditScreen: {
            printButton: "Imprimir",
            restartButton: "Iniciar votació",
            title: "Comprova la teva papereta",
            description: "Per comprovar la teva papereta, segueix els passos a continuació:",
            step1Title: "1. Desa les dades següents:",
            step1Description:
                "el teu <b>Localitzador del Vot</b> a la part superior de la pantalla i la teva papereta encriptada a continuació",
            step1HelpDialog: {
                title: "Copiar el codi de la papereta",
                content:
                    "Pots descarregar o copiar el codi de la teva papereta per verificar que reflecteix correctament les teves seleccions.",
                ok: "D'acord",
            },
            downloadButton: "Descarregar",
            step2Title: "2. Comprova la teva papereta",
            step2Description:
                "Fes clic a <VerifierLink>Comprova el codi de la teva papereta</VerifierLink>. S'obrirà en una nova pestanya",
            step2HelpDialog: {
                title: "Com comprovar el codi de la papereta",
                content:
                    "Per comprovar el codi de la teva papereta, segueix els passos de la guia. Inclou la descàrrega d'una aplicació d'escriptori per verificar la teva papereta de forma independent al lloc web.",
                ok: "D'acord",
            },
            bottomWarning:
                "Per motius de seguretat, quan auditis la teva papereta, hauràs d'invalidar-la. Per continuar amb el procés de votació, fes clic a ‘<b>Iniciar votació</b>’.",
        },
        electionSelectionScreen: {
            title: "Llista de Votacions",
            description: "Selecciona la papereta en la qual vols votar",
            chooserHelpDialog: {
                title: "Sobre la Llista de Votacions",
                content:
                    "Aquesta pantalla mostra la llista de paperetes a les quals pots accedir. Poden estar obertes, programades o tancades. Només pots votar en les que estan obertes",
                ok: "D'acord",
            },
            noResults: "No hi ha eleccions per ara.",
            resultsButton: "Veure resultats",
            demoDialog: {
                title: "Cabina de votació de demostració",
                content:
                    "Estàs entrant en una cabina de votació de demostració. <strong>El teu vot no serà comptabilitzat.</strong> Aquesta cabina és només per a fins de demostració.",
                ok: "Entenc que el meu vot no serà comptabilitzat",
            },
            errors: {
                noVotingArea:
                    "No estàs registrat com a votant en aquesta elecció. Si us plau, contacta amb el servei d'assistència.",
                networkError:
                    "Hi ha hagut un problema de xarxa. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
                unableToFetchData:
                    "Hi ha hagut un problema en obtenir les dades. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
                noElectionEvent:
                    "L'esdeveniment electoral no existeix. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
                ballotStylesEmlError:
                    "Hi ha hagut un error amb la publicació de l'estil de la papereta. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
                obtainingElectionFromID:
                    "Hi ha hagut un error en obtenir les eleccions associades amb els següents IDs d'eleccions: {{electionIds}}. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
            },
            alerts: {
                noElections:
                    "No hi ha eleccions en les quals puguis votar. Això podria ser perquè l'àrea no té cap pregunta associada. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
                electionEventNotPublished:
                    "L'esdeveniment electoral encara no ha estat publicat. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
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
                    "No s'ha pogut verificar el teu certificat. Comprova que estàs utilitzant un certificat de votant vàlid i torna-ho a provar.",
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
            title: "Troba la teva papereta",
            titleResult: "Resultats de la cerca de la teva papereta",
            description: "Confirma que la teva papereta va ser emesa correctament",
            locate: "Troba la teva papereta",
            locateAgain: "Troba una altra papereta",
            found: "El teu ID de Papereta {{ballotId}} ha estat trobat",
            notFound: "El teu ID de Papereta {{ballotId}} no ha estat trobat",
            ambiguous:
                "Més d'una de les teves paperetes coincideix amb {{ballotId}}. Utilitza l'ID complet de la papereta.",
            contentDesc: "Aquest és el contingut de la teva papereta: ",
            wrongFormatBallotId: "Format incorrecte per l'ID de la Papereta",
            ballotIdNotFoundAtFilter:
                "No trobat, comprova que l'ID de la Papereta sigui correcte i pertanyi a l'usuari actual.",
            filterByBallotId: "Filtra per ID de la Papereta",
            totalBallots: "Paperetes: {{total}}",
            steps: {
                lookup: "Troba la teva papereta",
                result: "Resultat",
            },
            titleHelpDialog: {
                title: "Sobre el Cercador de Paperetes",
                content:
                    "El Cercador de Paperetes et permet introduir el teu ID de Papereta per localitzar el teu vot i confirmar que va ser registrat correctament.",
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

export default catalanInformalTranslation
