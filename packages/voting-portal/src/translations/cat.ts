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
        a11y: {
            skipToContent: "Vés al contingut principal",
            helpAbout: "Ajuda sobre {{topic}}",
            copyToClipboard: "Copia {{label}} al porta-retalls",
            previewMaterial: "Vista prèvia de {{title}}",
            ballotsTable: "Paperetes",
            ballotLocatorTabs: "Seccions del localitzador de paperetes",
            ballotIdLabel: "ID de vot",
            votingProgress: "Progrés de la votació",
            stepOf: "Pas {{current}} de {{total}}",
            selectUpTo_one: "Seleccioneu fins a {{count}} opció",
            selectUpTo_other: "Seleccioneu fins a {{count}} opcions",
            selectExactly_one: "Seleccioneu {{count}} opció",
            selectExactly_other: "Seleccioneu {{count}} opcions",
            selectBetween: "Seleccioneu entre {{min}} i {{max}} opcions",
        },
        candidatesList: {
            collapseToggle: "Alternar llista {{listTitle}}",
            showCandidates: "Mostra els candidats",
            hideCandidates: "Amaga els candidats",
            selectedCandidate: "{{count}} candidat seleccionat",
            selectedCandidates: "{{count}} candidats seleccionats",
            expandAll: "Expandir tot",
            collapseAll: "Reduir tot",
        },
        breadcrumbSteps: {
            electionList: "Llista de Votacions",
            ballot: "Papereta",
            review: "Revisió",
            confirmation: "Confirmació",
            audit: "Auditar",
        },
        footer: {
            poweredBy: "Funciona amb <1></1>",
        },
        votingScreen: {
            backButton: "Enrere",
            reviewButton: "Següent",
            clearButton: "Netejar selecció",
            ballotHelpDialog: {
                title: "Informació: Pantalla de votació",
                content:
                    "Aquesta pantalla mostra la votació en la qual vostè és elegible per votar. Pot seleccionar la seva secció activant la casella de la dreta Candidat/Resposta. Per restablir les seves seleccions, faci clic al botó “<b>Netejar selecció</b>”, per passar al següent pas, faci clic al botó “<b>Següent</b>”.",
                ok: "D'acord",
            },
            nonVotedDialog: {
                title: "Vot invàlid o en blanc",
                content:
                    "Algunes de les seves respostes podrien fer que la papereta en una o més preguntes sigui invàlida o en blanc.",
                ok: "Tornar i revisar",
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
            blankBallotDialog: {
                title: "No heu seleccionat cap candidat",
                content:
                    "No heu fet cap selecció. La vostra papereta s'emetrà com a papereta en blanc, que és una elecció vàlida i deliberada i es comptabilitzarà com a tal.",
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
            instructionsTitle: "Instruccions",
            instructionsDescription: "Si us plau, segueixi aquests passos per emetre el seu vot:",
            step1Title: "1. Seleccioneu la seva opció de vot",
            step1Description:
                "Seleccioneu els seus candidats preferits i respongueu les preguntes de l'elecció una per una a mesura que apareixin. Pot editar la seva papereta fins que estigui llest per continuar.",
            step2Title: "2. Reviseu la seva papereta",
            step2Description:
                "Una vegada estigui satisfet amb les seves seleccions, encriptarem la seva papereta i li mostrarem una revisió final de les seves eleccions. També rebrà un ID de seguiment únic per la seva papereta.",
            step3Title: "3. Envieu el vostre vot",
            step3Description:
                "Envia la teva papereta: Finalment, pots enviar la teva papereta perquè es registri correctament. Alternativament, pots optar per auditar i confirmar que la teva papereta va ser capturada i xifrada correctament.",
        },
        reviewScreen: {
            title: "Revisa el teu vot",
            description:
                "Per fer canvis a les seves seleccions, faci clic al botó “<b>Edita el teu vot</b>”, per confirmar les seves seleccions, faci clic al botó “<b>Envia el teu vot</b>” a sota, i per auditar la seva papereta faci clic al botó “<b>Auditar papereta</b>” a sota.",
            descriptionNoAudit:
                "Per fer canvis a les seves seleccions, faci clic al botó “<b>Edita el teu vot</b>”, per confirmar les seves seleccions, faci clic al botó “<b>Envia el teu vot</b>” a sota.",
            backButton: "Edita el teu vot",
            castBallotButton: "Envia el teu vot",
            auditButton: "Auditar papereta",
            copyBallotId: "Copia l'ID de la papereta",
            ballotIdCopied: "ID de la papereta copiat",
            ballotIdCopyError: "No s'ha pogut copiar l'ID de la papereta",
            reviewScreenHelpDialog: {
                title: "Informació: Pantalla de revisió",
                content:
                    "Aquesta pantalla li permet revisar les seves seleccions abans d'emetre el seu vot.",
                ok: "D'acord",
            },
            ballotIdHelpDialog: {
                title: "Vot no emès",
                content:
                    "<p>Està a punt de copiar el Localitzador del Vot, però <b>el seu vot encara no s'ha emès</b>. Si intenta buscar el Localitzador del Vot, no el trobarà.</p><p>La raó per la qual mostrem el Localitzador del Vot en aquest moment és perquè pugui auditar la correcció del vot xifrat abans d'emetre'l. Si aquesta és la raó per la qual desitja copiar el Localitzador del Vot, procedeixi a copiar-lo i després auditi el seu vot.</p>",
                ok: "Accepto que el meu vot NO ha estat emès",
                cancel: "Cancel·lar",
            },
            auditBallotHelpDialog: {
                title: "Realment vols Auditar la teva papereta?",
                content:
                    "<p>L'auditoria de la papereta l'invalidarà i hauràs de iniciar el procés de votació de nou si desitges emetre el teu vot. El procés d'auditoria de la papereta permet verificar que està codificada correctament. Fer aquest procés requereix que uns coneixements tècnics importants, per això no es recomana si no saps el que estàs fent.</p><p><b>Si el que desitja és emetre el seu vot, en <u>Cancel·lar</u> per tornar a la pantalla de revisió de votació.</b></p>",
                ok: "Sí, vull INVALIDAR la meva papereta per AUDITAR-LA",
                cancel: "Cancel·lar",
            },
            confirmCastVoteDialog: {
                title: "Esteu segur que voleu emetre el vostre vot?",
                content: "El vostre vot ja no es podrà editar un cop confirmat.",
                ok: "Sí, vull EMETRE el meu vot",
                cancel: "Cancel·lar",
            },
            confirmCastBlankBallotDialog: {
                title: "Esteu segur que voleu emetre una papereta en blanc?",
                content:
                    "No heu seleccionat cap candidat. Un cop confirmeu, la vostra papereta s'emetrà en blanc.",
                ok: "Sí, vull emetre la meva papereta en blanc",
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
            blankBallot: "Papereta en blanc",
        },
        confirmationScreen: {
            title: "El seu vot ha estat emès",
            description:
                "El codi de confirmació que apareix a continuació verifica que <b>el seu vot s'ha emès correctament</b>. Pot utilitzar aquest codi per verificar que el seu vot ha estat comptabilitzat.",
            blankBallot: {
                description:
                    "La vostra papereta s'ha emès en blanc, que és una elecció vàlida i deliberada.",
            },
            ballotId: "Localitzador del Vot",
            printButton: "Imprimir",
            finishButton: "Finalitzar",
            verifyCastTitle: "Comproveu que el seu vot ha estat emès",
            verifyCastDescription:
                "Pot comprovar en tot moment que la seva papereta s'ha emès correctament utilitzant el següent codi QR:",
            confirmationHelpDialog: {
                title: "Informació: Pantalla de confirmació",
                content:
                    "Aquesta pantalla mostra que el seu vot s'ha emès correctament. La informació proporcionada en aquesta pàgina li permet verificar que la papereta ha estat emmagatzemada en l'urna, aquest procés pot ser executat en qualsevol moment durant el període de votació i després que l'elecció hagi estat tancada.",
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
                title: "Informació: Localitzador del Vot",
                content:
                    "El Localitzador del Vot de papereta és un codi que li permet trobar la seva papereta en l'urna, aquest Localitzador és únic i no conté informació sobre les seves seleccions.",
                ok: "D'acord",
            },
            ballotIdDemoHelpDialog: {
                title: "Informació: Identificador de papereta de vot",
                content:
                    "<p>L'identificador de papereta de vot és un codi que us permet trobar la vostra papereta a l'urna. Aquest identificador és únic i no conté informació sobre les vostres seleccions.</p><p><b>Avis:</b> Aquesta cabina de votació és només per a fins de demostració. El vostre vot NO ha estat emès.</p>",
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
            title: "Auditeu la seva Papereta",
            description: "Per verificar la seva papereta haurà de seguir els següents passos:",
            step1Title: "1. Descarregueu o copieu la següent informació",
            step1Description:
                "El teu <b>Localitzador del Vot</b> que apareix a la part superior de la pantalla i la teva papereta encriptada a continuació:",
            step1HelpDialog: {
                title: "Copiar el Vot Xifrat",
                content:
                    "Pot descarregar o copiar el seu Vot Xifrat per auditar-lo i verificar que el contingut encriptat conté les seves seleccions.",
                ok: "D'acord",
            },
            downloadButton: "Descarregar",
            step2Title: "2. Verifica la teva papereta",
            step2Description:
                "<VerifierLink>Accedeix al verificador del vot</VerifierLink>, que s'obrirà una nova pestanya al teu navegador.",
            step2HelpDialog: {
                title: "Tutorial sobre l'Auditoria del Vot",
                content:
                    "Per auditar el seu vot haurà de seguir els passos indicats al tutorial, que inclouen la descàrrega d'una aplicació d'escriptori utilitzada per verificar el vot xifrat independentment del lloc web.",
                ok: "D'acord",
            },
            bottomWarning:
                "Per motius de seguretat, quan auditeu la vostra papereta, haurà d'invalidar-la. Per continuar amb el procés de votació, faci clic a ‘<b>Iniciar votació</b>’.",
        },
        electionSelectionScreen: {
            title: "Llista de Votacions",
            description: "Seleccioneu la votació que desitgeu votar",
            chooserHelpDialog: {
                title: "Informació: Llista de Votacions",
                content:
                    "Benvingut a la cabina de votació, aquesta pantalla mostra la llista d'eleccions en les quals pot emetre el seu vot. Les eleccions que apareixen en aquesta llista poden estar obertes a votació, programades o tancades. Només podrà accedir a la votació si el període de votació està obert.",
                ok: "D'acord",
            },
            noResults: "No hi ha eleccions per ara.",
            resultsButton: "Veure resultats",
            demoDialog: {
                title: "Cabina de votació de demostració",
                content:
                    "Està entrant en una cabina de votació de demostració. <strong>El seu vot NO serà comptabilitzat.</strong> Aquesta cabina de votació és només per a finalitats de demostració.",
                ok: "Accepto que el meu vot NO serà comptabilitzat",
            },
            errors: {
                noVotingArea:
                    "Àrea de votació no assignada al votant. Si us plau, torneu-ho a intentar més tard o contacteu amb suport per obtenir ajuda.",
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
                    "No hi ha eleccions en les quals pugueu votar. Això podria ser perquè l'àrea no té cap concurs associat. Si us plau, torneu-ho a intentar més tard o contacteu amb suport per obtenir ajuda.",
                electionEventNotPublished:
                    "L'esdeveniment electoral encara no ha estat publicat. Si us plau, torneu-ho a intentar més tard o contacteu amb suport per obtenir ajuda.",
            },
        },
        errors: {
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
                back: "Tornar a la Llista de Votacions",
                close: "Tancar",
                preview: "Vista prèvia",
                download: "Descarregar",
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
                lookup: "Localitza la teva Papereta",
                result: "Resultat",
            },
            titleHelpDialog: {
                title: "Informació: pantalla de Localització de la teva Papereta",
                content:
                    "Aquesta pantalla permet al votant trobar la seva Papereta utilitzant l'ID de la Papereta per recuperar-la. Aquest procediment permet comprovar que el seu vot va ser emès correctament i que el vot registrat coincideix amb el vot xifrat que va emetre.",
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
