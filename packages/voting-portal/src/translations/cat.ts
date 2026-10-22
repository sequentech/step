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
            ballotLocatorTabs: "Seccions del localitzador de paperetes",
            ballotIdLabel: "ID de vot",
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
            copyBallotId: "Copiar l'ID de la papereta",
            ballotIdCopied: "ID de la papereta copiat",
            ballotIdCopyError: "No s'ha pogut copiar l'ID de la papereta",
            reviewScreenHelpDialog: {
                title: "Sobre la pantalla de revisió",
                content:
                    "Aquesta pantalla us permet revisar les vostres seleccions abans d'emetre el vot",
                ok: "D'acord",
            },
            ballotIdHelpDialog: {
                title: "El vostre vot no ha estat emès",
                content:
                    "<p>Aquest és el vostre Localitzador del Vot, però <b>el vostre vot encara no s'ha emès</b>. Si intenteu buscar-lo ara, no apareixerà.</p><p>Mostrem el Localitzador del Vot en aquesta etapa perquè pugueu auditar la papereta xifrada abans d'emetre-la.</p>",
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
                    "Hi ha hagut un error relacionat amb el procés de hashing de la papereta. El BallotId: {{ballotId}} no és consistent amb el Hash de la Papereta Auditable: {{auditableBallotHash}}. Si us plau, informeu d'aquest problema al servei d'assistència.",
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
                CAST_VOTE_BallotIdMismatch:
                    "L'identificador de la papereta no coincideix amb el del vot emès.",
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
                    "Totes les votacions d'aquesta elecció s'han resolt per aclamació, per la qual cosa no s'ha emès cap papereta i no hi ha identificador de papereta per consultar.",
                helpDialog: {
                    title: "Informació: Aclamació",
                    content:
                        "Totes les votacions d'aquesta elecció s'han resolt per aclamació: les seves candidatures resulten elegides sense votació. Com que no s'ha emès cap papereta, no hi ha identificador de papereta, comprovant ni codi QR per verificar.",
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
            ballotId: "Localitzador del Vot",
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
            title: "Trobeu la vostra papereta",
            titleResult: "Resultats de la cerca de la vostra papereta",
            description: "Confirmeu que la vostra papereta va ser emesa correctament",
            locate: "Trobeu la vostra papereta",
            locateAgain: "Trobeu una altra papereta",
            found: "El vostre ID de Papereta {{ballotId}} ha estat trobat",
            notFound: "El vostre ID de Papereta {{ballotId}} no ha estat trobat",
            ambiguous:
                "Més d'una de les vostres paperetes coincideix amb {{ballotId}}. Utilitzeu l'ID complet de la papereta.",
            contentDesc: "Aquest és el contingut de la vostra papereta: ",
            wrongFormatBallotId: "Format incorrecte per l'ID de la Papereta",
            ballotIdNotFoundAtFilter:
                "No trobat, comproveu que l'ID de la Papereta sigui correcte i pertanyi a l'usuari actual.",
            filterByBallotId: "Filtra per ID de la Papereta",
            totalBallots: "Paperetes: {{total}}",
            checksAvailableUntil: "Pots comprovar la teva papereta fins al {{date}}.",
            checksEnded: "Les comprovacions van finalitzar el {{date}}.",
            castAt: "Emès el {{date}}",
            steps: {
                lookup: "Trobeu la vostra papereta",
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
                ballot_id: "ID de la Papereta",
            },
        },
    },
}

export default catalanTranslation
