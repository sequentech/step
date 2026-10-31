// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import englishTranslation, {TranslationType} from "./en"

const catalanTranslation: TranslationType = {
    translations: {
        ballotPrivacy: englishTranslation.translations.ballotPrivacy,
        startup: {
            error: "No s’ha pogut iniciar el portal de votació. Torneu-ho a provar.",
            retry: "Torneu-ho a provar",
        },
        contactChannel: {
            change: "Mètode de contacte",
            updated: "S’ha canviat el vostre contacte. Els codis i els avisos ara s’hi envien.",
            cancelled: "No s’ha canviat el vostre contacte.",
            failed: "No hem pogut canviar com us contactem. Torneu-ho a provar més tard o contacteu amb el suport.",
            close: "Tanca",
        },
        common: {
            goBack: "Tornar",
            showMore: "Mostra'n més",
            showLess: "Mostra'n menys",
        },
        audioInstructions: {
            screens: {
                "election-chooser":
                    "Aquesta pàgina mostra les eleccions en què podeu votar. Feu servir la tecla Tabulador per passar d'una elecció a la següent. Cada elecció té un botó per començar a votar. Premeu Retorn en aquest botó per obrir-ne la papereta.",
                "start":
                    "Aquesta pàgina explica com votar. Primer trieu les opcions, després les reviseu i finalment emeteu el vot. Feu servir la tecla Tabulador per arribar al botó que inicia la votació i premeu Retorn. Si es mostra una declaració, marqueu-ne abans la casella amb la barra espaiadora.",
                "ballot":
                    "Aquesta és la vostra papereta. Cada pregunta és un grup d'opcions, i la pregunta indica quantes en podeu triar. Feu servir la tecla Tabulador per moure-us entre les opcions i la barra espaiadora per marcar-ne o desmarcar-ne una. Quan acabeu, aneu al botó per continuar, al final de la pàgina, i premeu Retorn per revisar les opcions. També hi ha un botó per esborrar totes les opcions. No s'emet res fins que ho confirmeu a la pàgina de revisió.",
                "review":
                    "Aquesta pàgina mostra les opcions que heu triat. Comproveu-les amb atenció. Per canviar alguna cosa, feu servir el botó per editar la papereta. Per emetre el vot, aneu al botó per emetre la papereta i premeu Retorn. L'identificador de la papereta és al principi de la pàgina. El podeu copiar per localitzar la papereta més endavant.",
                "confirmation":
                    "El vostre vot s'ha emès. Aquesta pàgina mostra l'identificador de la papereta, amb què podeu comprovar que s'ha registrat. El podeu copiar o podeu imprimir aquesta pàgina. Quan acabeu, feu servir el botó del final de la pàgina per finalitzar.",
                "audit":
                    "Aquesta pàgina us permet auditar la papereta en lloc d'emetre-la. Mostra la informació que podeu comprovar amb el verificador de paperetes. Una papereta auditada no s'emet, de manera que després heu de tornar a la papereta per votar.",
                "ballot-locator":
                    "Aquesta pàgina us permet comprovar que una papereta s'ha registrat. Escriviu o enganxeu un identificador de papereta al camp de text i premeu Retorn. El resultat apareix sota el camp.",
                "support-materials":
                    "Aquesta pàgina mostra documents que us ajuden a votar. Feu servir la tecla Tabulador per moure-us entre ells i premeu Retorn al botó d'un document per obrir-lo. Feu servir el botó per tornar a la llista d'eleccions.",
            },
        },
        a11y: {
            skipToContent: "Vés al contingut principal",
            helpAbout: "Ajuda sobre {{topic}}",
            previewMaterial: "Vista prèvia de {{title}}",
            ballotsTable: "Paperetes",
            ballotLocatorTabs: "Seccions del seguiment de la papereta",
            ballotIdLabel: "ID de seguiment",
            votingProgress: "Progrés de la votació",
            stepOf: "Pas {{current}} de {{total}}",
            selectUpTo_one: "Seleccioneu fins a {{count}} opció",
            selectUpTo_many: "Seleccioneu fins a {{count}} opcions",
            selectUpTo_other: "Seleccioneu fins a {{count}} opcions",
            selectExactly_one: "Seleccioneu {{count}} opció",
            selectExactly_many: "Seleccioneu {{count}} opcions",
            selectExactly_other: "Seleccioneu {{count}} opcions",
            selectBetween: "Seleccioneu entre {{min}} i {{max}} opcions",
        },
        candidatesList: {
            collapseToggle: "Alternar llista {{listTitle}}",
            showCandidates: "Mostra els candidats",
            hideCandidates: "Amaga els candidats",
            selectedCandidates_one: "{{count}} candidat seleccionat",
            selectedCandidates_many: "{{count}} candidats seleccionats",
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
        contest: {
            acclamation: {
                description:
                    "Aquesta votació s'ha resolt per aclamació. Les seves candidatures resulten elegides sense votació, per la qual cosa no es pot seleccionar cap opció ni es registra cap vot.",
            },
        },
        votingScreen: {
            backButton: "Enrere",
            reviewButton: "Següent",
            clearButton: "Netejar seleccions",
            ballotHelpDialog: {
                title: "Sobre aquesta pantalla",
                content:
                    "Aquesta pantalla mostra les preguntes en les quals sou elegible per votar. Podeu fer la vostra selecció activant la casella a la dreta del Candidat/Resposta. Per restablir les vostres seleccions, feu clic al botó “<b>Netejar seleccions</b>”, per passar al següent pas, feu clic al botó “<b>Següent</b>”.",
                ok: "D'acord",
            },
            nonVotedDialog: {
                title: "El vostre vot és invàlid o en blanc",
                content:
                    "Algunes de les vostres respostes podrien fer que la papereta en una o més preguntes sigui invàlida o en blanc.",
                ok: "Revisar selecció",
                continue: "Continuar",
                cancel: "Cancel·lar",
            },
            warningDialog: {
                title: "Reviseu la vostra papereta",
                content:
                    "La vostra papereta conté seleccions que poden necessitar la vostra atenció (com ara seleccionar menys opcions de les permeses). La vostra papereta és vàlida i es comptarà tal com s'ha enviat.",
                ok: "Torneu i reviseu",
                continue: "Continueu",
                cancel: "Cancel·lar",
            },
            blankBallotDialog: {
                title: "No heu seleccionat cap candidat",
                content:
                    "No heu fet cap selecció. La vostra papereta s'emetrà com a papereta en blanc, que és una elecció vàlida i deliberada i es comptabilitzarà com a tal.",
                continue: "Continuar",
                cancel: "Cancel·lar",
            },
        },
        startScreen: {
            startButton: "Començar a votar",
            declineToVoteButton: "Declinar votar",
            declineToVoteDialog: {
                title: "Confirmeu que voleu declinar votar",
                content:
                    "Segur que voleu declinar votar?<br />Anireu directament a la revisió i el vostre estat de participació es desarà com a <b>Ha declinat votar</b>.",
                continue: "Declinar votar",
                cancel: "Cancel·lar",
            },
            instructionsTitle: "Com votar",
            instructionsDescription: "Seguiu aquests passos per emetre el vostre vot",
            step1Title: "1. Feu les vostres seleccions",
            step1Description:
                "Escolliu els vostres candidats preferits i responeu cada pregunta de la papereta segons aparegui. Podeu canviar les vostres seleccions en qualsevol moment abans d'emetre el vostre vot",
            step2Title: "2. Reviseu les vostres seleccions",
            step2Description:
                "Quan estigueu satisfets amb les vostres seleccions, xifrem la vostra papereta de forma segura i us mostrarem una revisió final. També rebreu un ID de seguiment únic com a referència",
            step3Title: "3. Emeteu la vostra papereta",
            step3Description:
                "Quan estigueu a punt, emeteu la vostra papereta perquè quedi registrada oficialment. O trieu auditar primer per confirmar que va ser capturada i xifrada correctament",
        },
        reviewScreen: {
            castRecovery: {
                uncertain:
                    "El resultat de l’emissió no s’ha confirmat. Podeu reintentar la mateixa sol·licitud o comprovar l’identificador de la papereta abans de tornar a començar.",
                checkBallot: "Comprova la papereta",
            },
            acclamation: {
                title: "Resolt per aclamació",
                helpDialog: {
                    title: "Informació: Aclamació",
                    content:
                        "Aquesta pantalla mostra el que s'ha resolt per aclamació. Com que no s'ha pogut seleccionar cap opció, no s'emet cap papereta i no hi ha res a verificar després.",
                    ok: "D'acord",
                },
                description:
                    "Reviseu el que s'ha resolt per aclamació en aquesta elecció. No s'emetrà cap papereta.",
                finishButton: "Finalitzar",
            },
            title: "Reviseu el vostre vot",
            description:
                "Per fer canvis a les vostres seleccions, feu clic al botó “<b>Editeu el vostre vot</b>”, per confirmar les vostres seleccions, feu clic al botó “<b>Envieu el vot</b>” a sota, i per auditar la vostra papereta feu clic al botó “<b>Auditar papereta</b>” a sota.",
            descriptionNoAudit:
                "Per fer canvis a les vostres seleccions, feu clic al botó “<b>Editeu el vostre vot</b>”, per confirmar les vostres seleccions, feu clic al botó “<b>Envieu el vot</b>” a sota.",
            backButton: "Editeu el vostre vot",
            castBallotButton: "Envieu el vot",
            auditButton: "Auditar papereta",
            copyBallotId: "Copiar l'ID de seguiment",
            ballotIdCopied: "ID de seguiment copiat",
            ballotIdCopyError: "No s'ha pogut copiar l'ID de seguiment",
            reviewScreenHelpDialog: {
                title: "Sobre la pantalla de revisió",
                content:
                    "Aquesta pantalla us permet revisar les vostres seleccions abans d'emetre el vot",
                ok: "D'acord",
            },
            ballotIdHelpDialog: {
                title: "El vostre vot no ha estat emès",
                content:
                    "<p>Aquest és el vostre ID de seguiment, però <b>el vostre vot encara no s'ha emès</b>. Si intenteu buscar-lo ara, no apareixerà.</p><p>Mostrem l'ID de seguiment en aquesta etapa perquè pugueu auditar la papereta xifrada abans d'emetre-la.</p>",
                ok: "Entenc que el meu vot no ha estat emès",
                cancel: "Cancel·lar",
            },
            auditBallotHelpDialog: {
                title: "Voleu auditar la vostra papereta?",
                content:
                    "<p>Auditar la vostra papereta l'invalidarà i haureu de reiniciar el procés de votació. Continueu només si us sentiu còmode amb els passos tècnics avançats. En cas contrari, feu clic a <u>Cancel·lar</u> per tornar.</p>",
                ok: "Sí, descartar la meva papereta per auditar-la",
                cancel: "Cancel·lar",
            },
            confirmCastVoteDialog: {
                title: "Esteu segur que voleu emetre el vostre vot?",
                content: "Un cop confirmeu, el vostre vot serà emès.",
                ok: "Sí, vull emetre el meu vot",
                cancel: "Cancel·lar",
            },
            confirmCastBlankBallotDialog: {
                title: "Esteu segur que voleu emetre una papereta en blanc?",
                content:
                    "No heu seleccionat cap candidat. Un cop confirmeu, la vostra papereta s'emetrà en blanc.",
                ok: "Sí, vull emetre la meva papereta en blanc",
                cancel: "Cancel·lar",
            },
            unfilledContestsDialog: {
                title: "Hi ha seleccions sense completar",
                content:
                    "Podeu deixar seleccions en blanc o triar menys opcions de les permeses. Reviseu el següent abans de continuar.",
                selected: "{{selected}} de {{max}} seleccionades",
                nothingSelected: "Sense selecció",
                ok: "Continuar amb aquestes seleccions",
                cancel: "Revisar seleccions",
            },
            error: {
                NETWORK_ERROR:
                    "Hi ha hagut un problema de xarxa. Si us plau, torneu-ho a provar més tard o contacteu amb el servei d'assistència.",
                UNABLE_TO_FETCH_DATA:
                    "Hi ha hagut un problema en recuperar les dades. Si us plau, torneu-ho a provar més tard o contacteu amb el servei d'assistència.",
                LOAD_ELECTION_EVENT:
                    "No es pot carregar l'esdeveniment electoral. Si us plau, torneu-ho a provar més tard.",
                CAST_VOTE:
                    "Hi ha hagut un error desconegut en emetre el vot. Si us plau, torneu-ho a provar més tard o contacteu amb el servei d'assistència.",
                CAST_VOTE_CheckStatusFailed:
                    "L'elecció no permet emetre el vot. L'elecció pot estar tancada, arxivada o potser esteu intentant votar fora del període de gràcia.",
                CAST_VOTE_AreaNotFound:
                    "Hi ha hagut un error en emetre el vot: àrea no trobada. Si us plau, torneu-ho a provar més tard o contacteu amb el servei d'assistència.",
                CAST_VOTE_InternalServerError:
                    "Hi ha hagut un error intern en emetre el vot. Si us plau, torneu-ho a provar més tard o contacteu amb el servei d'assistència.",
                CAST_VOTE_QueueError:
                    "Hi ha hagut un problema en processar el vostre vot. Si us plau, torneu-ho a provar més tard o contacteu amb el servei d'assistència.",
                CAST_VOTE_Unauthorized:
                    "No esteu autoritzat per emetre un vot. Si us plau, contacteu amb el servei d'assistència.",
                CAST_VOTE_ElectionEventNotFound:
                    "No s'ha pogut trobar l'esdeveniment electoral. Si us plau, torneu-ho a provar més tard o contacteu amb el servei d'assistència.",
                CAST_VOTE_ElectoralLogNotFound:
                    "No s'ha pogut trobar el vostre registre de vot. Si us plau, contacteu amb el servei d'assistència.",
                CAST_VOTE_CheckPreviousVotesFailed:
                    "Hi ha hagut un error en comprovar el vostre estat de votació. Si us plau, torneu-ho a provar més tard o contacteu amb el servei d'assistència.",
                CAST_VOTE_GetClientCredentialsFailed:
                    "No s'han pogut verificar les vostres credencials. Si us plau, torneu-ho a provar més tard o contacteu amb el servei d'assistència.",
                CAST_VOTE_GetAreaIdFailed:
                    "Hi ha hagut un error en verificar la vostra àrea de votació. Si us plau, torneu-ho a provar més tard o contacteu amb el servei d'assistència.",
                CAST_VOTE_GetTransactionFailed:
                    "Hi ha hagut un error en processar el vostre vot. Si us plau, torneu-ho a provar més tard o contacteu amb el servei d'assistència.",
                CAST_VOTE_DeserializeBallotFailed:
                    "Hi ha hagut un error en llegir la vostra papereta. Si us plau, torneu-ho a provar més tard o contacteu amb el servei d'assistència.",
                CAST_VOTE_DeserializeContestsFailed:
                    "Hi ha hagut un error en llegir les vostres seleccions. Si us plau, torneu-ho a provar més tard o contacteu amb el servei d'assistència.",
                CAST_VOTE_PokValidationFailed:
                    "No s'ha pogut validar el vostre vot. Si us plau, torneu-ho a provar més tard o contacteu amb el servei d'assistència.",
                CAST_VOTE_UuidParseFailed:
                    "Hi ha hagut un error en processar la vostra sol·licitud. Si us plau, torneu-ho a provar més tard o contacteu amb el servei d'assistència.",
                CAST_VOTE_unexpected:
                    "Hi ha hagut un error desconegut en emetre el vot. Si us plau, torneu-ho a provar més tard o contacteu amb el servei d'assistència.",
                CAST_VOTE_timeout:
                    "Error de temps d'espera en emetre el vot. Si us plau, torneu-ho a provar més tard o contacteu amb el servei d'assistència.",
                CAST_VOTE_InsertFailedExceedsAllowedRevotes:
                    "Heu superat el límit de revots. Si us plau, torneu-ho a provar més tard o contacteu amb el servei d'assistència.",
                CAST_VOTE_CheckRevotesFailed:
                    "Heu superat el nombre permès de revots. Si us plau, torneu-ho a provar més tard o contacteu amb el servei d'assistència.",
                CAST_VOTE_CheckVotesInOtherAreasFailed:
                    "Ja heu votat en una altra àrea. Si us plau, torneu-ho a provar més tard o contacteu amb el servei d'assistència.",
                CAST_VOTE_UnknownError:
                    "Hi ha hagut un error desconegut en emetre el vot. Si us plau, torneu-ho a provar més tard o contacteu amb el servei d'assistència.",
                NO_BALLOT_SELECTION:
                    "No es troba l'estat de selecció per a aquesta elecció. Si us plau, assegureu-vos d'haver seleccionat les vostres opcions correctament o contacteu amb el servei d'assistència.",
                NO_BALLOT_STYLE:
                    "L'estil de la papereta no està disponible. Si us plau, contacteu amb el servei d'assistència.",
                NO_AUDITABLE_BALLOT:
                    "No hi ha cap papereta auditable disponible. Si us plau, contacteu amb el servei d'assistència.",
                INCONSISTENT_HASH:
                    "Hi ha hagut un error relacionat amb el procés de hashing de la papereta. L'ID de seguiment: {{ballotId}} no és consistent amb el Hash de la Papereta Auditable: {{auditableBallotHash}}. Si us plau, informeu d'aquest problema al servei d'assistència.",
                ELECTION_EVENT_NOT_OPEN:
                    "L'esdeveniment electoral està tancat. Si us plau, contacteu amb el servei d'assistència.",
                PARSE_ERROR:
                    "Hi ha hagut un error en analitzar la papereta. Si us plau, torneu-ho a provar més tard o contacteu amb el servei d'assistència.",
                DESERIALIZE_AUDITABLE_ERROR:
                    "Hi ha hagut un error en deserialitzar la papereta auditable. Si us plau, torneu-ho a provar més tard o contacteu amb el servei d'assistència.",
                DESERIALIZE_HASHABLE_ERROR:
                    "Hi ha hagut un error en deserialitzar la papereta hashable. Si us plau, torneu-ho a provar més tard o contacteu amb el servei d'assistència.",
                CONVERT_ERROR:
                    "Hi ha hagut un error en convertir la papereta. Si us plau, torneu-ho a provar més tard o contacteu amb el servei d'assistència.",
                SERIALIZE_ERROR:
                    "Hi ha hagut un error en serialitzar la papereta. Si us plau, torneu-ho a provar més tard o contacteu amb el servei d'assistència.",
                UNKNOWN_ERROR:
                    "Hi ha hagut un error. Si us plau, torneu-ho a provar més tard o contacteu amb el servei d'assistència.",
                REAUTH_FAILED:
                    "L'autenticació ha fallat. Si us plau, torneu-ho a provar o contacteu amb el servei d'assistència.",
                SESSION_EXPIRED:
                    "La vostra sessió ha caducat. Si us plau, torneu a començar des del principi.",
                CAST_VOTE_BallotIdMismatch: "L'ID de seguiment no coincideix amb el del vot emès.",
                SESSION_STORAGE_ERROR:
                    "L'emmagatzematge de sessió no està disponible. Si us plau, torneu-ho a provar o contacteu amb el servei d'assistència.",
                PARSE_BALLOT_DATA_ERROR:
                    "S'ha produït un error en analitzar les dades de la papereta. Si us plau, torneu-ho a provar més tard o contacteu amb el servei d'assistència.",
                NOT_VALID_BALLOT_DATA_ERROR:
                    "Les dades de la papereta no són vàlides. Si us plau, torneu-ho a provar més tard o contacteu amb el servei d'assistència.",
                FETCH_DATA_TIMEOUT_ERROR:
                    "Error de temps d'espera en obtenir les dades. Si us plau, torneu-ho a provar més tard o contacteu amb el servei d'assistència.",
                TO_HASHABLE_BALLOT_ERROR:
                    "Error en convertir a papereta hashable. Si us plau, torneu-ho a provar més tard o contacteu amb el servei d'assistència.",
                INTERNAL_ERROR:
                    "S'ha produït un error intern en emetre el vot. Si us plau, torneu-ho a provar més tard o contacteu amb el servei d'assistència.",
            },
            declineToVote: "Declinar votar",
            blankBallot: "Papereta en blanc",
        },
        confirmationScreen: {
            acclamation: {
                title: "Resolt per aclamació",
                description:
                    "Totes les votacions d'aquesta elecció s'han resolt per aclamació, per la qual cosa no s'ha emès cap papereta i no hi ha ID de seguiment.",
                helpDialog: {
                    title: "Informació: Aclamació",
                    content:
                        "Totes les votacions d'aquesta elecció s'han resolt per aclamació: les seves candidatures resulten elegides sense votació. Com que no s'ha emès cap papereta, no hi ha ID de seguiment, comprovant ni codi QR per verificar.",
                    ok: "D'acord",
                },
            },
            title: "El vostre vot ha estat emès",
            description:
                "La vostra papereta va ser emesa correctament. Utilitzeu el codi a continuació per verificar que va ser comptabilitzada",
            blankBallot: {
                description:
                    "La vostra papereta s'ha emès en blanc, que és una elecció vàlida i deliberada.",
            },
            ballotId: "ID de seguiment",
            printButton: "Imprimir",
            finishButton: "Finalitzar",
            remainingElectionsError:
                "No hem pogut comprovar si teniu més eleccions en què votar. Torneu-ho a provar.",
            retryButton: "Tornar-ho a provar",
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
                title: "ID de seguiment",
                content: "No es pot utilitzar el codi, desactivat en mode de demostració.",
                ok: "D'acord",
            },
            ballotIdHelpDialog: {
                title: "Sobre l'ID de seguiment",
                content:
                    "L'ID de seguiment és un codi únic que us permet trobar la vostra papereta a l'urna. No conté informació sobre les vostres seleccions.",
                ok: "D'acord",
            },
            ballotIdDemoHelpDialog: {
                title: "Sobre l'ID de seguiment",
                content:
                    "L'ID de seguiment és un codi que us permet trobar la vostra papereta a l'urna. Aquest ID és únic i no conté informació sobre les vostres seleccions.",
                ok: "D'acord",
            },
            errorDialogPrintBallotReceipt: {
                title: "Error",
                content: "Ha ocorregut un error. Si us plau, intenteu-ho de nou.",
                ok: "Acceptar",
            },
            demoQRText: "El rastrejador de butlletes està deshabilitat en mode de demostració",
        },
        auditScreen: {
            confirmingAudit: "Confirmant que aquesta papereta ja no es pot emetre…",
            auditUnavailable:
                "No s'ha pogut auditar la papereta. No se n'ha revelat el contingut. Torneu-ho a provar.",
            printButton: "Imprimir",
            restartButton: "Iniciar votació",
            title: "Comproveu la vostra papereta",
            description: "Per comprovar la vostra papereta, seguiu els passos a continuació:",
            step1Title: "1. Deseu les dades següents:",
            step1Description:
                "el vostre <b>ID de seguiment</b> a la part superior de la pantalla i la vostra papereta encriptada a continuació",
            step1HelpDialog: {
                title: "Copiar el codi de la papereta",
                content:
                    "Podeu descarregar o copiar el codi de la vostra papereta per verificar que reflecteix correctament les vostres seleccions.",
                ok: "D'acord",
            },
            downloadButton: "Descarregar",
            step2Title: "2. Comproveu la vostra papereta",
            step2Description:
                "Feu clic a <VerifierLink>Comproveu el codi de la vostra papereta</VerifierLink>. S'obrirà en una nova pestanya",
            step2HelpDialog: {
                title: "Com comprovar el codi de la papereta",
                content:
                    "Per comprovar el codi de la vostra papereta, seguiu els passos de la guia. Inclou la descàrrega d'una aplicació d'escriptori per verificar la vostra papereta de forma independent al lloc web.",
                ok: "D'acord",
            },
            bottomWarning:
                "Per motius de seguretat, quan auditeu la vostra papereta, haureu d'invalidar-la. Per continuar amb el procés de votació, feu clic a ‘<b>Iniciar votació</b>’.",
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
                    "No esteu registrat com a votant en aquesta elecció. Si us plau, contacteu amb el servei d'assistència.",
                networkError:
                    "Hi ha hagut un problema de xarxa. Si us plau, torneu-ho a provar més tard o contacteu amb el servei d'assistència.",
                unableToFetchData:
                    "Hi ha hagut un problema en obtenir les dades. Si us plau, torneu-ho a provar més tard o contacteu amb el servei d'assistència.",
                noElectionEvent:
                    "L'esdeveniment electoral no existeix. Si us plau, torneu-ho a provar més tard o contacteu amb el servei d'assistència.",
                ballotStylesEmlError:
                    "Hi ha hagut un error amb la publicació de l'estil de la papereta. Si us plau, torneu-ho a provar més tard o contacteu amb el servei d'assistència.",
                obtainingElectionFromID:
                    "Hi ha hagut un error en obtenir les eleccions associades amb els següents IDs d'eleccions: {{electionIds}}. Si us plau, torneu-ho a provar més tard o contacteu amb el servei d'assistència.",
            },
            alerts: {
                noElections:
                    "No hi ha eleccions en les quals pugueu votar. Això podria ser perquè l'àrea no té cap pregunta associada. Si us plau, torneu-ho a provar més tard o contacteu amb el servei d'assistència.",
                electionEventNotPublished:
                    "L'esdeveniment electoral encara no ha estat publicat. Si us plau, torneu-ho a provar més tard o contacteu amb el servei d'assistència.",
            },
            materialsGate: {
                instructions:
                    "Heu de llegir <MaterialsLink>{{materialsTitle}}</MaterialsLink> abans de poder votar.",
            },
            votingClosedAt: "La votació es va tancar el {{close}} ({{localClose}}).",
            votingClosedOn: "La votació es va tancar el {{close}}.",
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
                invalidLoginHintParametersTitle: "Enllaç de votació no vàlid",
                invalidLoginHintParametersMessage:
                    "Aquest enllaç de votació conté informació d’accés no vàlida. Demaneu un enllaç nou i torneu-ho a provar.",
                certAuthFailedTitle: "Error d'Autenticació amb Certificat",
                certAuthFailedMessage:
                    "No s'ha pogut verificar el vostre certificat. Comproveu que esteu utilitzant un certificat de votant vàlid i torneu-ho a provar.",
            },
        },
        materials: {
            common: {
                label: "Materials de Suport",
                back: "Tornar a la llista de votacions",
                close: "Tancar",
                preview: "Vista prèvia",
                download: "Descarregar",
            },
            mandatory: {
                checkboxLabel: "He llegit els Materials de Suport",
                continueButton: "Continuar",
                error: "Hi ha hagut un problema en registrar la vostra confirmació. Si us plau, torneu-ho a intentar.",
            },
        },
        slates: {
            title: "Candidatures conjuntes",
            description:
                "Una candidatura conjunta és un grup de candidats que es presenten junts. Cada candidat mostra la candidatura a la qual pertany.",
            independent: "Independent",
            contestMembers: "Candidats de {{slate}} per a {{contest}}",
            noCandidate: "Sense candidat",
            coverage: {
                full: "Candidatura completa",
                singleContest: "Només {{contest}}",
                partial: "Candidatura parcial",
                candidates_one: "{{count}} candidat",
                candidates_other: "{{count}} candidats",
                offices_one: "{{count}} càrrec",
                offices_other: "{{count}} càrrecs",
            },
            candidateList: {
                show: "Mostra les candidatures",
                hide: "Amaga les candidatures",
            },
            tabs: {
                label: "Maneres d'omplir la papereta",
                slates: "Triar una llista",
                candidates: "Candidatures individuals",
            },
            review: {
                title: "Les teves seleccions",
                total: "Candidatures seleccionades: {{selected}} de {{seats}}",
                slate: "{{slate}}: {{status}}",
                independent: "Candidatures independents seleccionades: {{count}}",
                note: "El teu vot es registra per a cada candidatura seleccionada. Una llista no és un vot per si mateixa.",
                contestCount: "{{selected}} de {{max}} seleccionades",
                edit: "Editar",
                editLabel: "Editar {{contest}}",
            },
            selection: {
                all: "Les {{total}} seleccionades",
                mixed: "Mixta · {{selected}} de {{total}} seleccionades",
                partly: "Seleccionada en part · {{selected}} de {{total}}",
                selected: "Seleccionada",
                edit: "Editar la selecció",
            },
            apply: {
                button: "Tria aquesta candidatura",
                buttonLabel: "Tria la candidatura {{slate}}",
                chosen: "{{slate}} triada. Candidats seleccionats: {{candidates}}. Conteses: {{contests}}.",
                overMaximum:
                    "{{slate}} no es pot triar: té {{candidates}} candidatures per a {{contest}}, que en permet {{max}}. Podeu continuar triant candidatures individualment.",
                unavailable:
                    "{{slate}} no es pot triar en aquesta papereta. Podeu continuar triant candidatures individualment.",
                replaceDialog: {
                    title: "Vols substituir les teves opcions actuals?",
                    content:
                        "Triar {{slate}} substitueix les teves opcions a les conteses següents. La resta de les teves opcions no canvia.",
                    removed: "Es treu:",
                    added: "Se selecciona en el seu lloc:",
                    ok: "Substitueix les opcions",
                    cancel: "Mantén les meves opcions",
                },
            },
        },
        ballotLocator: {
            title: "Seguiment de la papereta",
            titleResult: "Seguiment de la papereta",
            description: "Confirmeu que la vostra papereta va ser emesa correctament",
            descriptionAudited:
                "Aquesta papereta va ser auditada, per la qual cosa no es va emetre mai. Comproveu que les seleccions descodificades a continuació són les que vau fer.",
            descriptionAuditedBrief:
                "Aquesta papereta va ser auditada, per la qual cosa no es va emetre mai.",
            locate: "Trobeu la vostra papereta",
            locateAgain: "Trobeu una altra papereta",
            found: "El vostre ID de seguiment {{ballotId}} ha estat trobat",
            notFound: "El vostre ID de seguiment {{ballotId}} no ha estat trobat",
            contentDesc: "Aquest és el contingut de la vostra papereta: ",
            wrongFormatBallotId: "Format incorrecte per l'ID de seguiment",
            ballotIdNotFoundAtFilter:
                "No trobat, comproveu que l'ID de seguiment sigui correcte i pertanyi a l'usuari actual.",
            filterByBallotId: "Filtra per ID de seguiment",
            totalBallots: "Paperetes: {{total}}",
            idHelp: "L'ID de seguiment curt, com FTBE-MHRX, o el complet",
            foundShort:
                "Trobada pel seu ID curt. Per estar-ne segur que és la vostra, compareu l'ID de seguiment complet amb el vostre rebut.",
            search: {
                title: "Comproveu la vostra papereta al registre públic",
                intro: "Introduïu l'ID de seguiment de la pantalla de confirmació o del rebut. El vostre navegador el cerca i comprova ell mateix les proves.",
                find: "Cerca",
                invalid: "Un ID de seguiment només té dígits i les lletres de la a a la f.",
                chips: {
                    short: "<b>Curt</b> FTBE-MHRX",
                    full: "<b>Complet</b> 16 grups de quatre",
                    spaces: "Espais opcionals",
                },
                features: {
                    cast: {
                        title: "Paperetes emeses",
                        text: "Comproveu que la vostra papereta és al registre públic, confirmada per testimonis independents i ancorada públicament.",
                    },
                    audited: {
                        title: "Paperetes auditades",
                        text: "Vegeu les seleccions que conté, desxifrades al vostre navegador, i comproveu que són les que vau fer.",
                    },
                    secret: {
                        title: "El vostre vot continua sent secret",
                        text: "Una papereta emesa continua xifrada. El seguiment de paperetes mostra que s'ha comptat, mai com heu votat.",
                    },
                },
            },
            matches: {
                title: "{{total}} paperetes comparteixen l'ID curt <b>{{shortId}}</b>. Trieu la vostra o introduïu l'ID de seguiment complet.",
                cast: "Emesa el {{time}}",
                audited: "Auditada el {{time}}",
            },
            audit: {
                title: "Les vostres seleccions, descodificades",
                note: "El vostre navegador ha desxifrat aquesta papereta auditada amb la clau que ha revelat el portal. Si aquestes no són les seleccions que vau fer, poseu-vos en contacte amb l'autoritat electoral.",
                failed: {
                    title: "Auditoria fallida",
                    text: "La papereta del registre públic no es desxifra amb la clau que ha revelat el portal. No emeteu el vostre vot des d'aquest dispositiu; aviseu l'autoritat electoral.",
                },
                unchecked: {
                    title: "No s'ha pogut comprovar l'auditoria",
                    text: "Aquest dispositiu no ha pogut comprovar aquesta papereta auditada, de manera que no es mostra cap selecció. Torneu-ho a provar més tard, o comproveu el contingut de la papereta al verificador de paperetes.",
                },
            },
            result: {
                receipt: "Rebut de la papereta",
                idComment:
                    "Feu servir l'ID curt o el complet per tornar a trobar aquesta papereta.",
                fullId: "ID de seguiment complet",
                copy: "Copia l'ID de seguiment complet",
                copied: "ID de seguiment copiat",
                tag: {
                    found: "Trobada",
                    auditFailed: "Auditoria fallida",
                },
                record: {
                    title: "Contingut de la vostra papereta",
                    subtitle: "Xifrat, tal com és al registre públic",
                },
                recordEntry: "Entrada del registre",
                save: "Desa el rebut",
                sealLabel: "Estat de la papereta: {{status}}",
                check: "Comprova-ho de manera independent",
                caption: "Comprovat pel vostre navegador, amb codi que qualsevol pot inspeccionar.",
                captionVerified:
                    "Verificat pel vostre navegador, amb codi que qualsevol pot inspeccionar.",
                captionProblem:
                    "Trobat pel vostre navegador, amb codi que qualsevol pot inspeccionar.",
                late: "Normalment vol dir que un node va lent o està fora de línia; no és prova de manipulació. Torneu-ho a comprovar més tard, aquí o en qualsevol altre dispositiu.",
                problem:
                    "El portal indica que aquesta papereta va ser emesa, però el registre públic de l'elecció no té cap entrada per a ella. El vostre navegador ha guardat la prova: la resposta del portal i el punt de control signat del registre. Envieu-la a l'autoritat electoral o a un observador.",
                problemOther:
                    "Les proves signades d'aquesta papereta es contradiuen. Pot ser un error o un atac. El vostre navegador ha guardat la prova. Envieu-la a l'autoritat electoral o a un observador.",
                saveEvidence: "Desa la prova",
                howToReport: "Com denunciar-ho",
                howToReportText:
                    "Deseu el fitxer de la prova i envieu-lo a l'autoritat electoral o a un observador independent. Poden comprovar-lo als seus ordinadors. No conté res sobre com heu votat.",
                noReceipt:
                    "Aquest dispositiu no té el rebut d'aquesta papereta. Comproveu-la al dispositiu amb què vau votar, o comproveu el fitxer del rebut al verificador de paperetes.",
                receiptPending:
                    "Aquest dispositiu conserva un rebut d'aquesta papereta, però no està complet o no s'ha pogut comprovar ara. Torneu-ho a provar més tard, o comproveu el fitxer del rebut al verificador de paperetes.",
                status: {
                    checking: {
                        head: "S'està comprovant…",
                        sub: "El vostre navegador està buscant la papereta al registre públic.",
                    },
                    recorded: {
                        head: "Al registre públic",
                        sub: "El vostre navegador ha trobat la papereta. S'estan comprovant els seus testimonis…",
                    },
                    witnessed: {
                        head: "Confirmada pels testimonis",
                        sub: "S'estan comprovant els ancoratges públics…",
                        subAnchored_one:
                            "Ancorada als nodes públics. On comproven els navegadors arribarà d'aquí a {{count}} minut.",
                        subAnchored_other:
                            "Ancorada als nodes públics. On comproven els navegadors arribarà d'aquí a {{count}} minuts.",
                    },
                    verified: {
                        head: "Verificada",
                        sub: "Al registre públic, confirmada per testimonis independents i ancorada públicament.",
                    },
                    late: {
                        head: "Triga més del previst",
                        sub: "Al registre públic i confirmada pels testimonis, però els seus ancoratges públics van tard.",
                    },
                    problem: {
                        head: "No és al registre públic",
                        sub: "El vostre navegador no ha pogut trobar aquesta papereta al registre públic de l'elecció.",
                    },
                    otherProblem: {
                        head: "Ha fallat una comprovació",
                        sub: "Les proves signades d'aquesta papereta es contradiuen. Mireu el pas que ha fallat.",
                    },
                },
                steps: {
                    record: {
                        title: "Afegida al registre públic",
                        done: "Entrada núm. {{index}} del registre de paperetes de l'elecció. El vostre navegador n'ha comprovat la prova.",
                        failed: "Cap entrada del registre de paperetes de l'elecció té aquest ID de seguiment.",
                        missing: "Falta",
                    },
                    witnesses: {
                        title: "Confirmada pels testimonis",
                        done: "Testimonis independents van signar un registre que la inclou.",
                        count: "{{signed}} de {{total}} · en calen {{needed}}",
                        countOf: "{{signed}} de {{total}}",
                    },
                    hubs: {
                        title_one: "Ancorada a {{count}} node públic",
                        title_other: "Ancorada a {{count}} nodes públics",
                        titleAny: "Ancorada als nodes públics",
                        done: "Els registres compartits també contenen aquest registre: {{hubs}}.",
                    },
                    browser: {
                        title: "Ancorada on comproven els navegadors",
                        done: "Conservada per un node provisional fins que els navegadors comprovin els registres per ells mateixos.",
                    },
                    checking: "El vostre navegador ho està comprovant…",
                    scheduled_one: "D'aquí a {{count}} minut. No cal esperar.",
                    scheduled_other: "D'aquí a {{count}} minuts. No cal esperar.",
                    by: "Abans de {{time}}",
                    late: "Tard",
                    lateHubs: "Els nodes l'haurien d'haver ancorat abans de {{time}}.",
                    lateOther: "Això s'hauria d'haver fet abans de {{time}}.",
                    waiting: "Esperant el pas anterior.",
                    skipped: "No comprovat.",
                    failedStep: "Les proves signades d'aquest pas es contradiuen.",
                    failedTime: "Problema",
                },
            },
            checksAvailableUntil: "Pots comprovar la teva papereta fins al {{date}}.",
            checksEnded: "Les comprovacions van finalitzar el {{date}}.",
            castAt: "Emès el {{date}}",
            steps: {
                lookup: "Trobeu la vostra papereta",
                result: "Resultat",
            },
            titleHelpDialog: {
                title: "Sobre el seguiment de la papereta",
                content:
                    "Aquesta pantalla us permet trobar el vostre vot a partir de l'ID de seguiment. Amb aquest procediment podeu comprovar que la vostra papereta es va emetre correctament i que la papereta registrada coincideix amb la papereta xifrada que vau enviar.",
                ok: "D'acord",
            },
            tabs: {
                logs: "Logs",
                ballotLocator: "Seguiment de la papereta",
            },
            column: {
                statement_kind: "Tipus",
                statement_timestamp: "Marca de temps",
                ballot_id: "ID de seguiment",
            },
        },
        vitrine: {
            ballotId: {
                label: "ID de seguiment",
                full: "ID de seguiment complet",
                yours: "El vostre ID de seguiment:",
                copy: "Copia l'ID de seguiment complet",
                copied: "ID de seguiment copiat",
                showFull: "Mostra l'ID de seguiment complet",
            },
            status: {
                title: "Estat de la papereta",
                cast: "Emesa el {{when}}",
                audited: "Auditada el {{when}}",
                ballotCast: "Vot emès",
                aria: "Estat de la papereta: {{state}}. Mostra els detalls",
                ariaCast: "Vot emès. Estat de la papereta: {{state}}. Mostra els detalls",
                tooltip: "{{state}} · Mostra els detalls",
                tracker: "Obre al seguiment de paperetes",
                save: "Desa el justificant",
                ok: "D'acord",
                tag: {
                    cast: "Emesa",
                    notCast: "No emesa",
                    auditable: "Auditable",
                },
                state: {
                    notcast: "Encara no emesa",
                    checking: "Comprovant…",
                    recorded: "Al registre públic",
                    witnessed: "Confirmada pels testimonis",
                    verified: "Verificada",
                    late: "Tardana",
                    problem: "Problema",
                    auditable: "Auditable",
                    auditfail: "Auditoria fallida",
                },
                head: {
                    late: "Està trigant més del previst",
                    mismatch: "El registre no coincideix",
                    damaged: "El justificant està malmès",
                    noReceipt: "Sense justificant de la vostra papereta",
                    unchecked: "No s'ha pogut comprovar la vostra papereta",
                },
                sub: {
                    notcast:
                        "Aquesta papereta està xifrada en aquest dispositiu i no s'ha enviat res. Quan l'emeteu, el navegador comprova que arriba al registre públic.",
                    checking:
                        "El navegador està comprovant que la vostra papereta és al registre públic.",
                    recorded:
                        "El navegador ha trobat la vostra papereta al registre públic. Ara la confirmen els testimonis.",
                    witnessed:
                        "Testimonis independents han confirmat el registre. Els ancoratges públics arriben en {{minutes}} minuts; no cal esperar.",
                    verified:
                        "La vostra papereta és al registre públic, confirmada per testimonis independents i ancorada públicament.",
                    late: "La vostra papereta és al registre públic i confirmada pels testimonis, però els seus ancoratges públics van tard.",
                    lateStep:
                        "Un pas està trigant més del previst. No és prova de manipulació, i el navegador continua comprovant.",
                    mismatch:
                        "L'entrada #{{index}} del registre públic no és la d'aquest justificant.",
                    mismatchNoEntry: "L'entrada del registre públic no és la d'aquest justificant.",
                    damaged:
                        "No s'ha pogut llegir o verificar una part del justificant o del registre. És una fallada de dades, no una prova de manipulació. El vostre vot s'ha emès.",
                    noReceipt:
                        "Aquest web no ha rebut cap justificant de la vostra papereta. El vostre vot s'ha emès i s'ha avisat els observadors de l'elecció.",
                    unchecked:
                        "Aquest web no ha pogut comprovar el justificant. El vostre vot s'ha emès.",
                    auditable:
                        "Heu auditat aquesta papereta, per la qual cosa no s'ha emès mai. És al registre públic, i el seguiment de paperetes la desxifra per mostrar les seleccions que conté.",
                    auditfail:
                        "La papereta del registre públic no es desxifra amb la clau que ha revelat el portal. No emeteu el vostre vot des d'aquest dispositiu; aviseu l'autoritat electoral.",
                },
                step: {
                    record: {
                        title: "Al registre públic",
                        upcoming: "El navegador troba la papereta al registre públic de l'elecció.",
                    },
                    witnesses: {
                        title: "Confirmada pels testimonis",
                        upcoming: "Testimonis independents signen un registre que la inclou.",
                    },
                    anchors: {
                        title: "Ancorada públicament",
                        upcoming:
                            "Els nodes públics, i els registres que comproven els navegadors, guarden aquest registre.",
                    },
                },
                row: {
                    waiting: "Esperant el pas anterior.",
                    checking: "El navegador està comprovant…",
                    anchoring:
                        "Nodes públics i, després, on comproven els navegadors: en {{minutes}} minuts.",
                    recorded: "El navegador n'ha comprovat la prova.",
                    recordedEntry: "Entrada #{{index}}. El navegador n'ha comprovat la prova.",
                    witnessed:
                        "Prou testimonis independents de la política ({{count}}) han signat un registre que la inclou.",
                    anchoredWaict: "Als nodes públics ({{count}}) i on comproven els navegadors.",
                    anchoredBootstrap:
                        "Als nodes públics ({{count}}) i on comproven els navegadors, per ara mitjançant un node substitut.",
                    anchoredHubs: "Als nodes públics ({{count}}).",
                    lateNow: "Ja hauria d'estar fet.",
                    lateStep: "Hauria d'estar fet a les {{time}}.",
                    lateAnchors: "Els nodes l'haurien d'haver ancorat a les {{time}}.",
                    mismatch:
                        "L'entrada #{{index}} del registre és diferent del vostre justificant.",
                    mismatchNoEntry: "Una entrada del registre és diferent del vostre justificant.",
                    contradiction: "Les seves proves signades es contradiuen.",
                    damaged: "La seva prova està malmesa i no s'ha pogut verificar.",
                    noReceipt: "Aquest web no ha rebut cap justificant.",
                    unchecked: "Aquest web no ha pogut comprovar el justificant.",
                    skipped: "No comprovat, perquè no s'ha pogut completar el pas anterior.",
                    skippedMismatch: "No comprovat, perquè el registre no coincideix.",
                },
                word: {
                    withinSeconds: "En pocs segons",
                    withinMinutes: "En {{minutes}} min",
                    checking: "Comprovant…",
                    late: "Tard",
                    mismatch: "No coincideix",
                    damaged: "Malmès",
                    noReceipt: "Sense justificant",
                    unchecked: "No comprovat",
                    skipped: "—",
                },
                callout: {
                    late: "Normalment vol dir que un node va lent o està fora de línia; no és prova de manipulació.",
                    lateKept:
                        "Aquest dispositiu ho torna a comprovar cada vegada que hi torneu, i el seguiment de paperetes ho pot comprovar des de qualsevol dispositiu.",
                    lateTracker:
                        "El seguiment de paperetes ho pot comprovar des de qualsevol dispositiu.",
                    evidence:
                        "El navegador ha guardat l'evidència: aquest justificant i l'entrada signada del registre. Envieu-la a l'autoritat electoral o a un observador; qualsevol pot comprovar-la.",
                    saveEvidence: "Desa l'evidència",
                },
                caption: {
                    notcast:
                        "No es comprova res fins que l'emeteu. Després, aquesta marca mostra fins on s'ha verificat la vostra papereta.",
                    checking:
                        "El navegador està comprovant el registre públic, amb codi que qualsevol pot inspeccionar.",
                    problem: "Detectat pel navegador, amb codi que qualsevol pot inspeccionar.",
                    auditable: "Desxifrat pel navegador, amb codi que qualsevol pot inspeccionar.",
                    verified: "Verificat pel navegador, amb codi que qualsevol pot inspeccionar.",
                    checked: "Comprovat pel navegador, amb codi que qualsevol pot inspeccionar.",
                    kept: "Aquest dispositiu guarda el justificant i el torna a comprovar a cada visita.",
                },
            },
            booth: {
                title: "Encara no es pot preparar la vostra papereta",
                explanation:
                    "Abans de xifrar la vostra papereta, aquest lloc web comprova que la configuració de l'elecció és la publicada al registre públic de l'elecció. La vostra papereta no s'ha xifrat ni enviat, i les vostres opcions es mantenen.",
                pending:
                    "La configuració publicada encara no està disponible. Torneu-ho a provar d'aquí a un moment.",
                invalid:
                    "No s'ha pogut llegir la configuració publicada. Torneu-ho a provar d'aquí a un moment.",
                failed: "La configuració no coincideix amb la publicada. S'ha avisat els observadors de l'elecció. Poseu-vos en contacte amb el servei d'assistència.",
                retry: "Torna-ho a provar",
                close: "Tanca",
            },
            record: {
                json: "JSON",
                text: "Text",
                fields_one: "{{count}} camp",
                fields_other: "{{count}} camps",
                items_one: "{{count}} element",
                items_other: "{{count}} elements",
                size: {
                    bytes: "{{value}} B",
                    kilobytes: "{{value}} KB",
                },
                copy: "Copia {{file}}",
                copied: "Copiat",
                download: "Descarrega {{file}}",
            },
        },
    },
}

export default catalanTranslation
