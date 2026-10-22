// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import englishTranslation, {TranslationType} from "./en"

const spanishTranslation: TranslationType = {
    translations: {
        ballotPrivacy: englishTranslation.translations.ballotPrivacy,
        startup: {
            error: "No se pudo iniciar el portal de votación. Inténtelo de nuevo.",
            retry: "Intentar de nuevo",
        },
        contactChannel: {
            change: "Método de contacto",
            updated: "Se cambió su contacto. Los códigos y avisos ahora se envían allí.",
            cancelled: "Su contacto no se cambió.",
            failed: "No pudimos cambiar cómo le contactamos. Inténtelo más tarde o contacte con soporte.",
            close: "Cerrar",
        },
        common: {
            goBack: "Regresar",
            showMore: "Mostrar más",
            showLess: "Mostrar menos",
        },
        audioInstructions: {
            screens: {
                "election-chooser":
                    "Esta página muestra las elecciones en las que puede votar. Use la tecla Tabulador para pasar de una elección a la siguiente. Cada elección tiene un botón para empezar a votar. Pulse Intro en ese botón para abrir su papeleta.",
                "start":
                    "Esta página explica cómo votar. Primero elige sus opciones, después las revisa y por último emite su voto. Use la tecla Tabulador para llegar al botón que inicia la votación y pulse Intro. Si se muestra una declaración, marque antes su casilla con la barra espaciadora.",
                "ballot":
                    "Esta es su papeleta. Cada pregunta es un grupo de opciones, y la pregunta indica cuántas puede elegir. Use la tecla Tabulador para moverse entre las opciones y la barra espaciadora para marcar o desmarcar una. Cuando termine, vaya al botón para continuar, al final de la página, y pulse Intro para revisar sus opciones. También hay un botón para borrar todas sus opciones. No se emite nada hasta que lo confirme en la página de revisión.",
                "review":
                    "Esta página muestra las opciones que ha elegido. Compruébelas con atención. Para cambiar algo, use el botón para editar su papeleta. Para emitir su voto, vaya al botón para emitir la papeleta y pulse Intro. El identificador de su papeleta está al principio de la página. Puede copiarlo para localizar su papeleta más adelante.",
                "confirmation":
                    "Su voto se ha emitido. Esta página muestra el identificador de su papeleta, con el que puede comprobar que se ha registrado. Puede copiarlo o imprimir esta página. Cuando termine, use el botón del final de la página para finalizar.",
                "audit":
                    "Esta página le permite auditar su papeleta en lugar de emitirla. Muestra la información que puede comprobar con el verificador de papeletas. Una papeleta auditada no se emite, así que vuelva después a su papeleta para votar.",
                "ballot-locator":
                    "Esta página le permite comprobar que una papeleta se ha registrado. Escriba o pegue un identificador de papeleta en el campo de texto y pulse Intro. El resultado aparece debajo del campo.",
                "support-materials":
                    "Esta página muestra documentos que le ayudan a votar. Use la tecla Tabulador para moverse entre ellos y pulse Intro en el botón de un documento para abrirlo. Use el botón para volver a la lista de elecciones.",
            },
        },
        a11y: {
            skipToContent: "Saltar al contenido principal",
            helpAbout: "Ayuda sobre {{topic}}",
            previewMaterial: "Vista previa de {{title}}",
            ballotsTable: "Papeletas",
            ballotLocatorTabs: "Secciones del localizador de papeletas",
            ballotIdLabel: "ID de voto",
            votingProgress: "Progreso de la votación",
            stepOf: "Paso {{current}} de {{total}}",
            selectUpTo_one: "Seleccione hasta {{count}} opción",
            selectUpTo_many: "Seleccione hasta {{count}} opciones",
            selectUpTo_other: "Seleccione hasta {{count}} opciones",
            selectExactly_one: "Seleccione {{count}} opción",
            selectExactly_many: "Seleccione {{count}} opciones",
            selectExactly_other: "Seleccione {{count}} opciones",
            selectBetween: "Seleccione entre {{min}} y {{max}} opciones",
        },
        candidatesList: {
            collapseToggle: "Alternar lista {{listTitle}}",
            showCandidates: "Mostrar candidatos",
            hideCandidates: "Ocultar candidatos",
            selectedCandidates_one: "{{count}} candidato seleccionado",
            selectedCandidates_many: "{{count}} candidatos seleccionados",
            selectedCandidates_other: "{{count}} candidatos seleccionados",
            expandAll: "Expandir todo",
            collapseAll: "Contraer todo",
        },
        breadcrumbSteps: {
            electionList: "Votaciones",
            ballot: "Papeleta",
            review: "Revisión",
            confirmation: "Confirmar",
            audit: "Auditar",
        },
        footer: {
            poweredBy: "Funciona con <1></1>",
        },
        contest: {
            acclamation: {
                description:
                    "Esta votación se ha resuelto por aclamación. Sus candidaturas resultan elegidas sin votación, por lo que no se puede seleccionar ninguna opción ni se registra ningún voto.",
            },
        },
        votingScreen: {
            backButton: "Atrás",
            reviewButton: "Siguiente",
            clearButton: "Limpiar selecciones",
            ballotHelpDialog: {
                title: "Sobre esta pantalla",
                content:
                    "Esta pantalla muestra las preguntas en las que usted es elegible para votar. Puede hacer su selección activando la casilla a la derecha del Candidato/Respuesta. Para restablecer sus selecciones, haga clic en el botón “<b>Limpiar selecciones</b>”, para pasar al siguiente paso, haga clic en el botón “<b>Siguiente</b>”.",
                ok: "OK",
            },
            nonVotedDialog: {
                title: "Su voto es inválido o está en blanco",
                content:
                    "Algunas de sus respuestas harán que la papeleta en una o más preguntas sea inválida o en blanco.",
                ok: "Revisar selección",
                continue: "Continuar",
                cancel: "Cancelar",
            },
            warningDialog: {
                title: "Revise su papeleta",
                content:
                    "Su papeleta contiene selecciones que pueden necesitar su atención (como seleccionar menos opciones de las permitidas). Su papeleta es válida y se contará tal como se ha enviado.",
                ok: "Volver y revisar",
                continue: "Continuar",
                cancel: "Cancelar",
            },
            blankBallotDialog: {
                title: "No ha seleccionado ningún candidato",
                content:
                    "No ha realizado ninguna selección. Su papeleta se emitirá como papeleta en blanco, lo cual es una elección válida y deliberada y se contabilizará como tal.",
                continue: "Continuar",
                cancel: "Cancelar",
            },
        },
        startScreen: {
            startButton: "Empezar a votar",
            declineToVoteButton: "Declinar votar",
            declineToVoteDialog: {
                title: "Confirmar declinación de voto",
                content:
                    "¿Está seguro de que desea declinar votar?<br />Irá directamente a la revisión y su estado de participación se guardará como <b>Ha declinado votar</b>.",
                continue: "Declinar votar",
                cancel: "Cancelar",
            },
            instructionsTitle: "Cómo votar",
            instructionsDescription: "Siga estos pasos para emitir su voto",
            step1Title: "1. Haga sus selecciones",
            step1Description:
                "Elija a sus candidatos preferidos y responda cada pregunta de la papeleta según aparezca. Puede cambiar sus selecciones en cualquier momento antes de emitir su voto",
            step2Title: "2. Revise sus selecciones",
            step2Description:
                "Cuando esté satisfecho con sus selecciones, cifraremos su papeleta de forma segura y le mostraremos una revisión final. También recibirá un ID de seguimiento único como referencia",
            step3Title: "3. Emita su papeleta",
            step3Description:
                "Cuando esté listo, emita su papeleta para que quede registrada oficialmente. O elija auditar primero para confirmar que fue correctamente capturada y cifrada",
        },
        reviewScreen: {
            castRecovery: {
                uncertain:
                    "No se ha confirmado el resultado del envío. Puede reintentar la misma solicitud o comprobar el identificador de la papeleta antes de empezar de nuevo.",
                checkBallot: "Comprobar papeleta",
            },
            acclamation: {
                title: "Resuelto por aclamación",
                helpDialog: {
                    title: "Información: Aclamación",
                    content:
                        "Esta pantalla muestra lo que se ha resuelto por aclamación. Como no se ha podido seleccionar ninguna opción, no se emite ninguna papeleta y no hay nada que verificar después.",
                    ok: "OK",
                },
                description:
                    "Revise lo que se ha resuelto por aclamación en esta elección. No se emitirá ninguna papeleta.",
                finishButton: "Finalizar",
            },
            title: "Revise su voto",
            description:
                "Para realizar cambios en sus selecciones, haga clic en el botón “<b>Editar su voto</b>”, para confirmar sus selecciones, haga clic en el botón “<b>Enviar voto</b>” debajo, y para auditar su papeleta haga clic en el botón “<b>Auditar papeleta</b>” debajo.",
            descriptionNoAudit:
                "Para realizar cambios en sus selecciones, haga clic en el botón “<b>Editar su voto</b>”, para confirmar sus selecciones, haga clic en el botón “<b>Enviar voto</b>” debajo.",
            backButton: "Editar su voto",
            castBallotButton: "Enviar voto",
            auditButton: "Auditar papeleta",
            copyBallotId: "Copiar el ID de la papeleta",
            ballotIdCopied: "ID de la papeleta copiado",
            ballotIdCopyError: "No se pudo copiar el ID de la papeleta",
            reviewScreenHelpDialog: {
                title: "Sobre la pantalla de revisión",
                content: "Esta pantalla le permite revisar sus selecciones antes de emitir su voto",
                ok: "OK",
            },
            ballotIdHelpDialog: {
                title: "Su voto no ha sido emitido",
                content:
                    "<p>Este es su Localizador del Voto, pero <b>su voto aún no ha sido emitido</b>. Si intenta buscarlo ahora, no aparecerá.</p><p>Mostramos el Localizador del Voto en esta etapa para que pueda auditar la papeleta cifrada antes de emitirla.</p>",
                ok: "Entiendo que mi voto no ha sido emitido",
                cancel: "Cancelar",
            },
            auditBallotHelpDialog: {
                title: "¿Quiere auditar su papeleta?",
                content:
                    "<p>Auditar su papeleta la invalidará y tendrá que reiniciar el proceso de votación. Continúe solo si se siente cómodo con los pasos técnicos avanzados. De lo contrario, haga clic en <u>Cancelar</u> para volver.</p>",
                ok: "Sí, descartar mi papeleta para auditarla",
                cancel: "Cancelar",
            },
            confirmCastVoteDialog: {
                title: "¿Está seguro de que quiere emitir su voto?",
                content: "Una vez que confirme, su voto será emitido.",
                ok: "Sí, quiero emitir mi voto",
                cancel: "Cancelar",
            },
            confirmCastBlankBallotDialog: {
                title: "¿Está seguro de que desea emitir una papeleta en blanco?",
                content:
                    "No ha seleccionado ningún candidato. Una vez confirme, su papeleta se emitirá en blanco.",
                ok: "Sí, quiero emitir mi papeleta en blanco",
                cancel: "Cancelar",
            },
            unfilledContestsDialog: {
                title: "Hay selecciones sin completar",
                content:
                    "Puede dejar selecciones en blanco o elegir menos opciones de las permitidas. Revise lo siguiente antes de continuar.",
                selected: "{{selected}} de {{max}} seleccionadas",
                nothingSelected: "Sin selección",
                ok: "Continuar con estas selecciones",
                cancel: "Revisar selecciones",
            },
            error: {
                NETWORK_ERROR:
                    "Hubo un problema de red. Por favor, inténtelo de nuevo más tarde o contacte con el soporte para obtener ayuda.",
                UNABLE_TO_FETCH_DATA:
                    "Hubo un problema al recuperar los datos. Por favor, inténtelo de nuevo más tarde o contacte con el soporte para obtener ayuda.",
                LOAD_ELECTION_EVENT:
                    "No se puede cargar el evento electoral. Por favor, inténtelo de nuevo más tarde.",
                CAST_VOTE:
                    "Ha ocurrido un error desconocido al emitir el voto. Por favor, inténtelo de nuevo más tarde o contacte con el soporte para obtener ayuda.",
                CAST_VOTE_CheckStatusFailed:
                    "La elección no permite emitir el voto. La elección puede estar cerrada, archivada o quizá esté intentando votar fuera del período de gracia.",
                CAST_VOTE_AreaNotFound:
                    "Ha ocurrido un error al emitir el voto: área no encontrada. Por favor, inténtelo de nuevo más tarde o contacte con el soporte para obtener ayuda.",
                CAST_VOTE_InternalServerError:
                    "Ha ocurrido un error interno al emitir el voto. Por favor, inténtelo de nuevo más tarde o contacte con el soporte para obtener ayuda.",
                CAST_VOTE_QueueError:
                    "Ha ocurrido un problema al procesar su voto. Por favor, inténtelo de nuevo más tarde o contacte con el soporte para obtener ayuda.",
                CAST_VOTE_Unauthorized:
                    "No está autorizado para emitir un voto. Por favor, contacte con el soporte para obtener ayuda.",
                CAST_VOTE_ElectionEventNotFound:
                    "No se pudo encontrar el evento electoral. Por favor, inténtelo de nuevo más tarde o contacte con el soporte para obtener ayuda.",
                CAST_VOTE_ElectoralLogNotFound:
                    "No se pudo encontrar su registro de votación. Por favor, contacte con el soporte para obtener ayuda.",
                CAST_VOTE_CheckPreviousVotesFailed:
                    "Ha ocurrido un error al verificar su estado de votación. Por favor, inténtelo de nuevo más tarde o contacte con el soporte para obtener ayuda.",
                CAST_VOTE_GetClientCredentialsFailed:
                    "No se pudieron verificar sus credenciales. Por favor, inténtelo de nuevo más tarde o contacte con el soporte para obtener ayuda.",
                CAST_VOTE_GetAreaIdFailed:
                    "Ha ocurrido un error al verificar su área de votación. Por favor, inténtelo de nuevo más tarde o contacte con el soporte para obtener ayuda.",
                CAST_VOTE_GetTransactionFailed:
                    "Ha ocurrido un error al procesar su voto. Por favor, inténtelo de nuevo más tarde o contacte con el soporte para obtener ayuda.",
                CAST_VOTE_DeserializeBallotFailed:
                    "Ha ocurrido un error al leer su papeleta. Por favor, inténtelo de nuevo más tarde o contacte con el soporte para obtener ayuda.",
                CAST_VOTE_DeserializeContestsFailed:
                    "Ha ocurrido un error al leer sus selecciones. Por favor, inténtelo de nuevo más tarde o contacte con el soporte para obtener ayuda.",
                CAST_VOTE_PokValidationFailed:
                    "No se pudo validar su voto. Por favor, inténtelo de nuevo más tarde o contacte con el soporte para obtener ayuda.",
                CAST_VOTE_UuidParseFailed:
                    "Ha ocurrido un error al procesar su solicitud. Por favor, inténtelo de nuevo más tarde o contacte con el soporte para obtener ayuda.",
                CAST_VOTE_unexpected:
                    "Ha ocurrido un error desconocido al emitir el voto. Por favor, inténtelo de nuevo más tarde o contacte con el soporte para obtener ayuda.",
                CAST_VOTE_timeout:
                    "Error de tiempo de espera al emitir el voto. Por favor, inténtelo de nuevo más tarde o contacte con el soporte para obtener ayuda.",
                CAST_VOTE_InsertFailedExceedsAllowedRevotes:
                    "Ha superado el límite de revotos. Por favor, inténtelo de nuevo más tarde o contacte con el soporte para obtener ayuda.",
                CAST_VOTE_CheckRevotesFailed:
                    "Ha superado el número permitido de revotos. Por favor, inténtelo de nuevo más tarde o contacte con el soporte para obtener ayuda.",
                CAST_VOTE_CheckVotesInOtherAreasFailed:
                    "Ya ha votado en otra área. Por favor, inténtelo de nuevo más tarde o contacte con el soporte para obtener ayuda.",
                CAST_VOTE_UnknownError:
                    "Ha ocurrido un error desconocido al emitir el voto. Por favor, inténtelo de nuevo más tarde o contacte con el soporte para obtener ayuda.",
                NO_BALLOT_SELECTION:
                    "El estado de selección para esta elección no está presente. Asegúrese de haber seleccionado correctamente sus opciones o contacte con el soporte.",
                NO_BALLOT_STYLE:
                    "El estilo de la papeleta no está disponible. Por favor, contacte con el soporte.",
                NO_AUDITABLE_BALLOT:
                    "No hay una papeleta verificable disponible. Por favor, contacte con el soporte.",
                INCONSISTENT_HASH:
                    "Hubo un error relacionado con el proceso de hash de la papeleta. El BallotId: {{ballotId}} no es coherente con el Hash de la Papeleta Verificable: {{auditableBallotHash}}. Por favor, informe de este problema al soporte.",
                ELECTION_EVENT_NOT_OPEN:
                    "El evento electoral está cerrado. Por favor, contacte con el soporte.",
                PARSE_ERROR:
                    "Hubo un error al analizar la papeleta. Por favor, inténtelo de nuevo más tarde o contacte con el soporte para obtener ayuda.",
                DESERIALIZE_AUDITABLE_ERROR:
                    "Hubo un error al deserializar la papeleta verificable. Por favor, inténtelo de nuevo más tarde o contacte con el soporte para obtener ayuda.",
                DESERIALIZE_HASHABLE_ERROR:
                    "Hubo un error al deserializar la papeleta hashable. Por favor, inténtelo de nuevo más tarde o contacte con el soporte para obtener ayuda.",
                CONVERT_ERROR:
                    "Hubo un error al convertir la papeleta. Por favor, inténtelo de nuevo más tarde o contacte con el soporte para obtener ayuda.",
                SERIALIZE_ERROR:
                    "Hubo un error al serializar la papeleta. Por favor, inténtelo de nuevo más tarde o contacte con el soporte para obtener ayuda.",
                UNKNOWN_ERROR:
                    "Hubo un error. Por favor, inténtelo de nuevo más tarde o contacte con el soporte para obtener ayuda.",
                REAUTH_FAILED:
                    "La autenticación ha fallado. Por favor, inténtelo de nuevo o contacte con el soporte para obtener ayuda.",
                SESSION_EXPIRED:
                    "Su sesión ha expirado. Por favor, inténtelo de nuevo desde el principio.",
                CAST_VOTE_BallotIdMismatch:
                    "El identificador de la papeleta no coincide con el del voto emitido.",
                SESSION_STORAGE_ERROR:
                    "El almacenamiento de sesión no está disponible. Por favor, inténtelo de nuevo o contacte con el soporte.",
                PARSE_BALLOT_DATA_ERROR:
                    "Hubo un error al analizar los datos de la papeleta. Por favor, inténtelo de nuevo más tarde o contacte con el soporte para obtener ayuda.",
                NOT_VALID_BALLOT_DATA_ERROR:
                    "Los datos de la papeleta no son válidos. Por favor, inténtelo de nuevo más tarde o contacte con el soporte para obtener ayuda.",
                FETCH_DATA_TIMEOUT_ERROR:
                    "Error de tiempo de espera al obtener los datos. Por favor, inténtelo de nuevo más tarde o contacte con el soporte para obtener ayuda.",
                TO_HASHABLE_BALLOT_ERROR:
                    "Error al convertir a papeleta hashable. Por favor, inténtelo de nuevo más tarde o contacte con el soporte para obtener ayuda.",
                INTERNAL_ERROR:
                    "Hubo un error interno al emitir el voto. Por favor, inténtelo de nuevo más tarde o contacte con el soporte para obtener ayuda.",
            },
            declineToVote: "Declinar votar",
            blankBallot: "Papeleta en blanco",
        },
        confirmationScreen: {
            acclamation: {
                title: "Resuelto por aclamación",
                description:
                    "Todas las votaciones de esta elección se han resuelto por aclamación, por lo que no se ha emitido ninguna papeleta y no hay identificador de papeleta que consultar.",
                helpDialog: {
                    title: "Información: Aclamación",
                    content:
                        "Todas las votaciones de esta elección se han resuelto por aclamación: sus candidaturas resultan elegidas sin votación. Como no se ha emitido ninguna papeleta, no hay identificador de papeleta, comprobante ni código QR que verificar.",
                    ok: "OK",
                },
            },
            title: "Su voto ha sido emitido",
            description:
                "Su papeleta fue emitida correctamente. Use el código a continuación para verificar que fue contabilizada",
            blankBallot: {
                description:
                    "Su papeleta se emitió en blanco, lo cual es una elección válida y deliberada.",
            },
            ballotId: "Localizador del Voto",
            printButton: "Imprimir",
            finishButton: "Finalizar",
            remainingElectionsError:
                "No pudimos comprobar si tiene más elecciones en las que votar. Vuelva a intentarlo.",
            retryButton: "Reintentar",
            verifyCastTitle: "Compruebe que su voto fue emitido",
            verifyCastDescription:
                "Puede verificar en cualquier momento que su papeleta fue emitida correctamente usando el código QR a continuación",
            confirmationHelpDialog: {
                title: "Sobre la pantalla de confirmación",
                content:
                    "Esta pantalla confirma que su voto fue emitido correctamente. La información aquí le permite verificar que su papeleta fue almacenada en la urna, tanto durante el periodo de votación como después de su cierre",
                ok: "OK",
            },
            demoPrintDialog: {
                title: "Impresión de la papeleta de votación",
                content: "La impresión está desactivada en modo de demostración",
                ok: "Aceptar",
            },
            demoBallotUrlDialog: {
                title: "Rastreador de Papeletas",
                content: "No se puede usar el código, deshabilitado en modo de demostración.",
                ok: "OK",
            },
            ballotIdHelpDialog: {
                title: "Sobre el Localizador del Voto",
                content:
                    "El Localizador del Voto es un código único que le permite encontrar su papeleta en la urna. No contiene información sobre sus selecciones.",
                ok: "OK",
            },
            ballotIdDemoHelpDialog: {
                title: "Sobre el Localizador del Voto",
                content:
                    "La identificación de la papeleta es un código que le permite encontrar su papeleta en la urna. Este identificador es único y no contiene información sobre sus selecciones.",
                ok: "Aceptar",
            },
            errorDialogPrintBallotReceipt: {
                title: "Error",
                content: "Ha ocurrido un error. Por favor, inténtelo de nuevo.",
                ok: "Aceptar",
            },
            demoQRText: "El rastreador de papeletas está deshabilitado en modo de demostración",
        },
        auditScreen: {
            confirmingAudit: "Confirmando que esta papeleta ya no se puede emitir…",
            auditUnavailable:
                "No se ha podido auditar la papeleta. Su contenido no se ha revelado. Inténtelo de nuevo.",
            printButton: "Imprimir",
            restartButton: "Iniciar votación",
            title: "Compruebe su papeleta",
            description: "Para comprobar su papeleta, siga los pasos a continuación:",
            step1Title: "1. Guarde los siguientes datos:",
            step1Description:
                "su <b>Localizador del Voto</b> en la parte superior de la pantalla y su papeleta encriptada a continuación",
            step1HelpDialog: {
                title: "Copiar código de la papeleta",
                content:
                    "Puede descargar o copiar el código de su papeleta para verificar que refleja correctamente sus selecciones.",
                ok: "OK",
            },
            downloadButton: "Descargar",
            step2Title: "2. Compruebe su papeleta",
            step2Description:
                "Haga clic en <VerifierLink>Compruebe el código de su papeleta</VerifierLink>. Se abrirá en una nueva pestaña",
            step2HelpDialog: {
                title: "Cómo comprobar el código de la papeleta",
                content:
                    "Para comprobar el código de su papeleta, siga los pasos de la guía. Incluye la descarga de una aplicación de escritorio para verificar su papeleta de forma independiente al sitio web.",
                ok: "OK",
            },
            bottomWarning:
                "Por motivos de seguridad, cuando audite su papeleta, deberá invalidarla. Para continuar con el proceso de votación, haga clic en ‘<b>Iniciar votación</b>’.",
        },
        electionSelectionScreen: {
            title: "Lista de Votaciones",
            description: "Seleccione la papeleta en la que desea votar",
            chooserHelpDialog: {
                title: "Sobre la Lista de Votaciones",
                content:
                    "Esta pantalla muestra la lista de papeletas a las que puede acceder. Pueden estar abiertas, programadas o cerradas. Solo puede votar en las que están abiertas",
                ok: "OK",
            },
            noResults: "No hay elecciones por ahora.",
            resultsButton: "Ver resultados",
            demoDialog: {
                title: "Cabina de votación de demostración",
                content:
                    "Está entrando en una cabina de votación de demostración. <strong>Su voto no será registrado.</strong> Esta cabina es solo para demostración.",
                ok: "Entiendo que mi voto no será registrado",
            },
            errors: {
                noVotingArea:
                    "No está registrado como votante en esta elección. Por favor, contacte con el soporte.",
                networkError:
                    "Hubo un problema de red. Por favor, inténtelo de nuevo más tarde o contacte con el soporte para obtener ayuda.",
                unableToFetchData:
                    "Hubo un problema al obtener los datos. Por favor, inténtelo de nuevo más tarde o contacte con el soporte para obtener ayuda.",
                noElectionEvent:
                    "El evento electoral no existe. Por favor, inténtelo de nuevo más tarde o contacte con el soporte para obtener ayuda.",
                ballotStylesEmlError:
                    "Hubo un error con la publicación del estilo de la papeleta. Por favor, inténtelo de nuevo más tarde o contacte con el soporte para obtener ayuda.",
                obtainingElectionFromID:
                    "Hubo un error al obtener las elecciones asociadas con los siguientes IDs de elecciones: {{electionIds}}. Por favor, inténtelo de nuevo más tarde o contacte con el soporte para obtener ayuda.",
            },
            alerts: {
                noElections:
                    "No hay elecciones en las que pueda votar. Esto podría deberse a que el área no tiene ninguna pregunta asociada. Por favor, inténtelo de nuevo más tarde o contacte con el soporte para obtener ayuda.",
                electionEventNotPublished:
                    "El evento electoral aún no ha sido publicado. Por favor, inténtelo de nuevo más tarde o contacte con el soporte para obtener ayuda.",
            },
            materialsGate: {
                instructions:
                    "Debe leer <MaterialsLink>{{materialsTitle}}</MaterialsLink> antes de poder votar.",
            },
            votingClosedAt: "La votación se cerró el {{close}} ({{localClose}}).",
            votingClosedOn: "La votación se cerró el {{close}}.",
        },
        errors: {
            encoding: {
                notEnoughChoices: "No hay suficientes opciones para decodificar",
                writeInChoiceOutOfRange: "Opción de escritura libre fuera de rango: {{index}}",
                writeInNotEndInZero: "La escritura libre no termina en 0",
                bytesToUtf8Conversion:
                    "Error al convertir la escritura libre de bytes a cadena UTF-8: {{errorMessage}}",
                ballotTooLarge: "La papeleta es más grande de lo esperado",
            },
            explicit: {
                notAllowed:
                    "La papeleta está marcada como explícitamente inválida, pero la pregunta no lo permite",
                alert: "Esta selección se contará como un voto inválido",
            },
            page: {
                oopsWithStatus: "¡Vaya! {{status}}",
                oopsWithoutStatus: "¡Vaya! Error Inesperado",
                somethingWrong: "Algo salió mal.",
                invalidLoginHintParametersTitle: "Enlace de votación no válido",
                invalidLoginHintParametersMessage:
                    "Este enlace de votación contiene información de acceso no válida. Solicite un nuevo enlace e inténtelo de nuevo.",
                certAuthFailedTitle: "Error de Autenticación con Certificado",
                certAuthFailedMessage:
                    "No se ha podido verificar su certificado. Compruebe que está usando un certificado de votante válido e inténtelo de nuevo.",
            },
        },
        materials: {
            common: {
                label: "Materiales de Soporte",
                back: "Volver a la lista de votaciones",
                close: "Cerrar",
                preview: "Vista previa",
                download: "Descargar",
            },
            mandatory: {
                checkboxLabel: "He leído los Materiales de Soporte",
                continueButton: "Continuar",
                error: "Hubo un problema al registrar su confirmación. Por favor, inténtelo de nuevo.",
            },
        },
        slates: {
            title: "Candidaturas conjuntas",
            description:
                "Una candidatura conjunta es un grupo de candidatos que se presentan juntos. Cada candidato muestra la candidatura a la que pertenece.",
            independent: "Independiente",
            contestMembers: "Candidatos de {{slate}} para {{contest}}",
            noCandidate: "Sin candidato",
            coverage: {
                full: "Candidatura completa",
                singleContest: "Solo {{contest}}",
                partial: "Candidatura parcial",
                candidates_one: "{{count}} candidato",
                candidates_other: "{{count}} candidatos",
                offices_one: "{{count}} cargo",
                offices_other: "{{count}} cargos",
            },
            candidateList: {
                show: "Mostrar candidaturas",
                hide: "Ocultar candidaturas",
            },
            tabs: {
                label: "Formas de rellenar su papeleta",
                slates: "Elegir una lista",
                candidates: "Candidaturas individuales",
            },
            review: {
                title: "Tus selecciones",
                total: "Candidaturas seleccionadas: {{selected}} de {{seats}}",
                slate: "{{slate}}: {{status}}",
                independent: "Candidaturas independientes seleccionadas: {{count}}",
                note: "Tu voto se registra para cada candidatura seleccionada. Una lista no es un voto por sí misma.",
                contestCount: "{{selected}} de {{max}} seleccionadas",
                edit: "Editar",
                editLabel: "Editar {{contest}}",
            },
            selection: {
                all: "Las {{total}} seleccionadas",
                mixed: "Mixta · {{selected}} de {{total}} seleccionadas",
                partly: "Seleccionada en parte · {{selected}} de {{total}}",
                selected: "Seleccionada",
                edit: "Editar la selección",
            },
            apply: {
                button: "Elegir esta candidatura",
                buttonLabel: "Elegir la candidatura {{slate}}",
                chosen: "{{slate}} elegida. Candidatos seleccionados: {{candidates}}. Contiendas: {{contests}}.",
                overMaximum:
                    "{{slate}} no se puede elegir: tiene {{candidates}} candidaturas para {{contest}}, que permite {{max}}. Puede seguir eligiendo candidaturas individualmente.",
                unavailable:
                    "{{slate}} no se puede elegir en esta papeleta. Puede seguir eligiendo candidaturas individualmente.",
                replaceDialog: {
                    title: "¿Reemplazar tus opciones actuales?",
                    content:
                        "Elegir {{slate}} reemplaza tus opciones en las contiendas siguientes. El resto de tus opciones no cambia.",
                    removed: "Se quita:",
                    added: "Se selecciona en su lugar:",
                    ok: "Reemplazar opciones",
                    cancel: "Mantener mis opciones",
                },
            },
        },
        ballotLocator: {
            title: "Encuentre su papeleta",
            titleResult: "Resultados de su búsqueda de Papeleta",
            description: "Confirme que su papeleta fue emitida correctamente",
            locate: "Encuentre su papeleta",
            locateAgain: "Encuentre otra papeleta",
            found: "Su ID de Papeleta {{ballotId}} ha sido encontrado",
            notFound: "Su ID de Papeleta {{ballotId}} no fue encontrado",
            ambiguous:
                "Más de una de sus papeletas coincide con {{ballotId}}. Use el ID de papeleta completo.",
            contentDesc: "Este es el contenido de su papeleta: ",
            wrongFormatBallotId: "Formato incorrecto para el ID de la Papeleta",
            ballotIdNotFoundAtFilter:
                "No encontrado, compruebe que el ID de la papeleta sea correcto y pertenezca a este usuario.",
            filterByBallotId: "Filtrar por ID de Papeleta",
            totalBallots: "Papeletas: {{total}}",
            checksAvailableUntil: "Puedes comprobar tu papeleta hasta el {{date}}.",
            checksEnded: "Las comprobaciones finalizaron el {{date}}.",
            castAt: "Emitido el {{date}}",
            steps: {
                lookup: "Encuentre su papeleta",
                result: "Resultado",
            },
            titleHelpDialog: {
                title: "Sobre el Buscador de Papeletas",
                content:
                    "El Buscador de Papeletas le permite introducir su ID de Papeleta para localizar su voto y confirmar que fue registrado correctamente.",
                ok: "OK",
            },
            tabs: {
                logs: "Logs",
                ballotLocator: "Localizador de Papeletas",
            },
            column: {
                statement_kind: "Tipo",
                statement_timestamp: "Marca de tiempo",
                ballot_id: "ID de Papeleta",
            },
        },
    },
}

export default spanishTranslation
