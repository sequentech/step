// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//

import englishTranslation, {TranslationType} from "./en"

// SPDX-License-Identifier: AGPL-3.0-only
const galegoTranslation: TranslationType = {
    translations: {
        ballotPrivacy: {
            ...englishTranslation.translations.ballotPrivacy,
            title: "Todas as papeletas",
            subtitle:
                "Todas as papeletas enviadas nesta eleición, unha soa vez, no seu último paso. Os votantes aparecen cun pseudónimo en lugar dun nome.",
            own: "A túa papeleta está no rexistro",
            copy: "Copiar o ID de seguimento",
            ballotIdHelp: "Sobre o teu ID de seguimento",
            stageTime: "{{stage}} {{time}}",
            noMatchHelp:
                "Comproba que non haxa erros ao escribir. Un ID de seguimento ten un guión e polo menos 8 caracteres, como FTBE-MHRX. Un pseudónimo ten dous puntos, como K7QM:2XWD.",
            copied: "ID de seguimento copiado",
            copyFailed: "Non se puido copiar o ID de seguimento. Selecciona o código para copialo.",
            search: "Buscar un ID de seguimento ou un pseudónimo",
            placeholder: "FTBE-MHRX ou K7QM:2XWD",
            matchesBoth:
                "Coinciden {{tracking}} por ID de seguimento · {{pseudonym}} por pseudónimo",
            ballotId: "ID de seguimento",
            lastStep: "Último paso",
            time: "Hora",
            trackingId: "ID de seguimento",
            trackingIdHelp: "Calculado a partir da papeleta cifrada e da clave do votante.",
            noMatch: "Ningún ID de seguimento nin pseudónimo comeza por {{prefix}}",
            noBallots: "Aínda non se enviou ningunha papeleta.",
            invalidPrefix:
                "Escribe polo menos 8 caracteres dun ID de seguimento con guión, como FTBE-MHRX, ou ata 8 dun pseudónimo con dous puntos, como K7QM:2XWD. Os códigos usan os díxitos do 0 ao 9 e letras maiúsculas, sen I, L, O nin U.",
            policyId:
                "Este é o ID da política desta elección. O ID de seguimento dunha papeleta está no teu recibo.",
            stages: {
                received: {
                    label: "Recibida",
                    title: "Papeleta recibida",
                    help: "A urna asinou un recibo desta papeleta e o rexistro público prometeu engadila. Non se conta a non ser que o seu votante a emita.",
                    castState: "Aínda non emitida",
                },
                cast: {
                    label: "Emitida",
                    title: "Papeleta emitida",
                    help: "A urna confirmou que o votante emitiu esta papeleta.",
                    castState: "Superada",
                },
                audited: {
                    label: "Auditada",
                    title: "Papeleta auditada",
                    help: "O seu votante abriuna para comprobala. Unha papeleta auditada nunca se emite nin se conta.",
                    castState: "Nunca emitida",
                },
            },
            checkLabels: {
                ...englishTranslation.translations.ballotPrivacy.checkLabels,
                ballotId: "ID de seguimento",
            },
            checkDescriptions: {
                ...englishTranslation.translations.ballotPrivacy.checkDescriptions,
                ballotId:
                    "O ID de seguimento calcúlase a partir da papeleta cifrada e da clave do votante.",
            },
        },
        startup: {
            error: "Non se puido iniciar o portal de votación. Téntao de novo.",
            retry: "Tentar de novo",
        },
        contactChannel: {
            change: "Método de contacto",
            updated: "Cambiouse o seu contacto. Os códigos e os avisos envíanse agora alí.",
            cancelled: "O seu contacto non se cambiou.",
            failed: "Non puidemos cambiar como contactamos con vostede. Ténteo de novo máis tarde ou contacte co soporte.",
            close: "Pechar",
        },
        common: {
            goBack: "Volver",
            showMore: "Mostrar Máis",
            showLess: "Mostrar Menos",
        },
        audioInstructions: {
            screens: {
                "election-chooser":
                    "Esta páxina mostra as eleccións nas que pode votar. Use a tecla Tabulador para pasar dunha elección á seguinte. Cada elección ten un botón para comezar a votar. Prema Intro nese botón para abrir a súa papeleta.",
                "start":
                    "Esta páxina explica como votar. Primeiro elixe as súas opcións, despois revísaas e por último emite o seu voto. Use a tecla Tabulador para chegar ao botón que inicia a votación e prema Intro. Se se mostra unha declaración, marque antes a súa casa coa barra espazadora.",
                "ballot":
                    "Esta é a súa papeleta. Cada pregunta é un grupo de opcións, e a pregunta indica cantas pode elixir. Use a tecla Tabulador para moverse entre as opcións e a barra espazadora para marcar ou desmarcar unha. Cando remate, vaia ao botón para continuar, ao final da páxina, e prema Intro para revisar as súas opcións. Tamén hai un botón para borrar todas as súas opcións. Non se emite nada ata que o confirme na páxina de revisión.",
                "review":
                    "Esta páxina mostra as opcións que elixiu. Compróbeas con atención. Para cambiar algo, use o botón para editar a súa papeleta. Para emitir o seu voto, vaia ao botón para emitir a papeleta e prema Intro. O identificador da súa papeleta está ao principio da páxina. Pode copialo para localizar a súa papeleta máis adiante.",
                "confirmation":
                    "O seu voto foi emitido. Esta páxina mostra o identificador da súa papeleta, co que pode comprobar que se rexistrou. Pode copialo ou gardar o seu recibo en PDF. Cando remate, use o botón para finalizar, que vai xusto despois do que garda o recibo.",
                "audit":
                    "Esta páxina permítelle auditar a súa papeleta en lugar de emitila. Mostra a información que pode comprobar co verificador de papeletas. Unha papeleta auditada non se emite, así que volva despois á súa papeleta para votar.",
                "ballot-locator":
                    "Esta páxina permítelle comprobar que unha papeleta se rexistrou. Escriba ou pegue un identificador de papeleta no campo de texto e prema Intro. O resultado aparece debaixo do campo.",
                "support-materials":
                    "Esta páxina mostra documentos que lle axudan a votar. Use a tecla Tabulador para moverse entre eles e prema Intro no botón dun documento para abrilo. Use o botón para volver á lista de eleccións.",
            },
        },
        a11y: {
            skipToContent: "Ir ao contido principal",
            helpAbout: "Axuda sobre {{topic}}",
            previewMaterial: "Vista previa de {{title}}",
            ballotsTable: "Papeletas",
            ballotLocatorTabs: "Seccións do seguimento da papeleta",
            ballotIdLabel: "ID de seguimento",
            votingProgress: "Progreso da votación",
            stepOf: "Paso {{current}} de {{total}}",
            selectUpTo_one: "Seleccione ata {{count}} opción",
            selectUpTo_many: "Seleccione ata {{count}} opcións",
            selectUpTo_other: "Seleccione ata {{count}} opcións",
            selectExactly_one: "Seleccione {{count}} opción",
            selectExactly_many: "Seleccione {{count}} opcións",
            selectExactly_other: "Seleccione {{count}} opcións",
            selectBetween: "Seleccione entre {{min}} e {{max}} opcións",
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
            electionList: "Papeletas",
            ballot: "Papeleta",
            review: "Revisión",
            confirmation: "Confirmar",
            audit: "Auditoría",
        },
        footer: {
            poweredBy: "Desenvolvido por <1></1>",
        },
        contest: {
            acclamation: {
                description:
                    "Esta pregunta resolveuse por aclamación. As súas candidaturas resultan elixidas sen votación, polo que non se pode seleccionar ningunha opción nin se rexistra ningún voto.",
            },
        },
        votingScreen: {
            backButton: "Volver",
            reviewButton: "Seguinte",
            clearButton: "Limpar selección",
            ballotHelpDialog: {
                title: "Sobre a pantalla da papeleta",
                content:
                    "Esta pantalla mostra a pregunta na que es elixible para votar. Activa a caixa á dereita para seleccionar un Candidato/Resposta. Para restablecer, fai clic en “<b>Limpar selección</b>”; para continuar, fai clic en “<b>Seguinte</b>”.",
                ok: "Aceptar",
            },
            nonVotedDialog: {
                title: "Voto inválido ou en branco",
                content:
                    "Algunhas das túas respostas farán que a papeleta sexa inválida ou quede en branco en unha ou máis preguntas.",
                ok: "Volver e revisar",
                continue: "Continuar",
                cancel: "Cancelar",
            },
            warningDialog: {
                title: "Revisa a túa papeleta",
                content:
                    "A túa papeleta contén seleccións que poden necesitar a túa atención (como seleccionar menos opcións das permitidas). A túa papeleta é válida e contarase tal como se enviou.",
                ok: "Voltar e revisar",
                continue: "Continuar",
                cancel: "Cancelar",
            },
            blankBallotDialog: {
                title: "Non seleccionou ningún candidato",
                content:
                    "Non fixo ningunha selección. A súa papeleta emitirase como papeleta en branco, o cal é unha elección válida e deliberada e contabilizarase como tal.",
                continue: "Continuar",
                cancel: "Cancelar",
            },
        },
        startScreen: {
            startButton: "Comezar a votar",
            declineToVoteButton: "Absterse de votar",
            declineToVoteDialog: {
                title: "Confirmar abstención de voto",
                content:
                    "Seguro que queres absterte de votar?<br />Irás directamente á revisión e o teu estado de participación gardarase como <b>Abstívose de votar</b>.",
                continue: "Absterse de votar",
                cancel: "Cancelar",
            },
            instructionsTitle: "Como votar",
            instructionsDescription: "Siga estes pasos para votar:",
            step1Title: "1. Selecciona as túas opcións",
            step1Description:
                "Elixe os teus candidatos e responde a cada pregunta. Podes editar a túa papeleta ata estar listo.",
            step2Title: "2. Revisa a túa papeleta",
            step2Description:
                "Encriptaremos a túa papeleta e mostrarémosche unha revisión final. Recibirás un ID de seguimento único.",
            step3Title: "3. Emite a túa papeleta",
            step3Description:
                "Emite a túa papeleta para que quede rexistrada, ou auditala para confirmar que foi encriptada correctamente.",
        },
        reviewScreen: {
            reloaded: {
                title: "A túa papeleta non se emitiu",
                text: "Ao recargar a páxina borrouse a clave dun só uso que a emite. Revisa as túas opcións e envía de novo a papeleta. A papeleta que enviaches queda no rexistro como recibida e nunca se conta.",
                back: "Volver á miña papeleta",
            },
            castRecovery: {
                uncertain:
                    "Non se confirmou o resultado do envío. Pode tentar de novo a mesma solicitude ou comprobar o ID de seguimento antes de comezar de novo.",
                checkBallot: "Comprobar papeleta",
            },
            acclamation: {
                title: "Resolto por aclamación",
                helpDialog: {
                    title: "Información: Aclamación",
                    content:
                        "Esta pantalla mostra o que se resolveu por aclamación. Como non se puido seleccionar ningunha opción, non se emite ningunha papeleta e non hai nada que verificar despois.",
                    ok: "Aceptar",
                },
                description:
                    "Revisa o que se resolveu por aclamación nesta elección. Non se emitirá ningunha papeleta.",
                finishButton: "Rematar",
            },
            title: "Revisa a túa papeleta",
            description:
                "Fai clic en “<b>Editar papeleta</b>” para cambiar as túas seleccións, “<b>Emitir papeleta</b>” para confirmar, ou “<b>Comproba a papeleta</b>” para auditala.",
            descriptionNoAudit:
                "Fai clic en “<b>Editar papeleta</b>” para cambiar as túas seleccións, ou “<b>Emitir papeleta</b>” para confirmar.",
            backButton: "Editar papeleta",
            castBallotButton: "Emitir papeleta",
            auditButton: "Comproba a papeleta",
            copyBallotId: "Copiar o ID de seguimento",
            ballotIdCopied: "ID de seguimento copiado",
            ballotIdCopyError: "Non se puido copiar o ID de seguimento",
            reviewScreenHelpDialog: {
                title: "Sobre a pantalla de revisión",
                content:
                    "Esta pantalla permíteche revisar as túas seleccións antes de emitir a papeleta.",
                ok: "Aceptar",
            },
            ballotIdHelpDialog: {
                title: "O voto non foi emitido",
                content:
                    "<p>Este é o teu ID de seguimento, pero <b>o teu voto aínda non foi emitido</b>. Se tentas rastrexar a papeleta, non a atoparás.</p><p>A razón pola que mostramos o ID de seguimento nesta fase é permitirche auditar a corrección da papeleta encriptada antes de emitila.</p>",
                ok: "Entendo que o meu voto non foi emitido",
                cancel: "Cancelar",
            },
            auditBallotHelpDialog: {
                title: "Queres auditar a papeleta?",
                content:
                    "<p>Ten en conta que auditar a túa papeleta anularaa, polo que terás que reiniciar o proceso de votación. O proceso de auditoría permíteche verificar que a túa papeleta está correctamente codificada, pero implica pasos técnicos avanzados. Recomendamos continuar só se estás seguro das túas habilidades técnicas. Se só queres emitir a túa papeleta, fai clic en <u>Cancelar</u> para volver á pantalla de revisión.</p>",
                ok: "Si, quero descartar a miña papeleta para auditala",
                cancel: "Cancelar",
            },
            auditConfirm: {
                title: "Auditar esta papeleta en lugar de emitila?",
                text: "O portal publica como cifrou <Id>{{id}}</Id>, para que poidas comprobar nun nodo que contén o que elixiches.",
                never: "Nunca se emite nin se conta: o seu segredo amosaría como votaches.",
                again: "Despois votas de novo, cun novo ID de seguimento.",
                cancel: "Cancelar",
                audit: "Auditar a papeleta",
            },
            confirmCastVoteDialog: {
                title: "Estás seguro de que queres emitir o teu voto?",
                content: "Tras confirmar, o teu voto será emitido.",
                ok: "Si, quero emitir o meu voto",
                cancel: "Cancelar",
            },
            confirmCastBlankBallotDialog: {
                title: "Está seguro de que quere emitir unha papeleta en branco?",
                content:
                    "Non seleccionou ningún candidato. Unha vez confirme, a súa papeleta emitirase en branco.",
                ok: "Si, quero emitir a miña papeleta en branco",
                cancel: "Cancelar",
            },
            unfilledContestsDialog: {
                title: "Hai seleccións sen completar",
                content:
                    "Pode deixar seleccións en branco ou escoller menos opcións das permitidas. Revise o seguinte antes de continuar.",
                selected: "{{selected}} de {{max}} seleccionadas",
                nothingSelected: "Sen selección",
                ok: "Continuar con estas seleccións",
                cancel: "Revisar seleccións",
            },
            error: {
                NETWORK_ERROR:
                    "Houbo un problema de rede. Inténtao de novo máis tarde ou contacta co soporte para obter axuda.",
                UNABLE_TO_FETCH_DATA:
                    "Houbo un problema ao obter os datos. Inténtao de novo máis tarde ou contacta co soporte para obter axuda.",
                LOAD_ELECTION_EVENT:
                    "Non se pode cargar o evento de eleccións. Inténtao de novo máis tarde.",
                CAST_VOTE:
                    "Houbo un erro descoñecido ao emitir o voto. Inténtao de novo máis tarde ou contacta co soporte para obter axuda.",
                CAST_VOTE_AreaNotFound:
                    "Houbo un erro ao emitir o voto: área non atopada. Inténtao de novo máis tarde ou contacta co soporte para obter axuda.",
                CAST_VOTE_CheckStatusFailed:
                    "A elección non permite emitir o voto. A elección pode estar pechada, arquivada ou podes estar intentando votar fóra do período de gracia.",
                CAST_VOTE_InternalServerError:
                    "Produciuse un erro interno ao emitir o voto. Inténtao de novo máis tarde ou contacta co soporte para obter axuda.",
                CAST_VOTE_QueueError:
                    "Houbo un problema ao procesar o teu voto. Inténtao de novo máis tarde ou contacta co soporte para obter axuda.",
                CAST_VOTE_Unauthorized:
                    "Non estás autorizado para emitir un voto. Contacta co soporte para obter axuda.",
                CAST_VOTE_ElectionEventNotFound:
                    "Non se puido atopar o evento de eleccións. Inténtao de novo máis tarde ou contacta co soporte para obter axuda.",
                CAST_VOTE_ElectoralLogNotFound:
                    "Non se puido atopar o rexistro de votación. Contacta co soporte para obter axuda.",
                CAST_VOTE_CheckPreviousVotesFailed:
                    "Produciuse un erro ao comprobar o estado do teu voto. Inténtao de novo máis tarde ou contacta co soporte para obter axuda.",
                CAST_VOTE_GetClientCredentialsFailed:
                    "Non se puideron verificar as túas credenciais. Inténtao de novo máis tarde ou contacta co soporte para obter axuda.",
                CAST_VOTE_GetAreaIdFailed:
                    "Produciuse un erro ao verificar a túa área de votación. Inténtao de novo máis tarde ou contacta co soporte para obter axuda.",
                CAST_VOTE_GetTransactionFailed:
                    "Produciuse un erro ao procesar o teu voto. Inténtao de novo máis tarde ou contacta co soporte para obter axuda.",
                CAST_VOTE_DeserializeBallotFailed:
                    "Produciuse un erro ao ler a túa papeleta. Inténtao de novo máis tarde ou contacta co soporte para obter axuda.",
                CAST_VOTE_DeserializeContestsFailed:
                    "Produciuse un erro ao ler as túas seleccións. Inténtao de novo máis tarde ou contacta co soporte para obter axuda.",
                CAST_VOTE_PokValidationFailed:
                    "Non se puido validar o teu voto. Inténtao de novo máis tarde ou contacta co soporte para obter axuda.",
                CAST_VOTE_UuidParseFailed:
                    "Produciuse un erro ao procesar a túa solicitude. Inténtao de novo máis tarde ou contacta co soporte para obter axuda.",
                CAST_VOTE_unexpected:
                    "Produciuse un erro descoñecido ao emitir o voto. Inténtao de novo máis tarde ou contacta co soporte para obter axuda.",
                CAST_VOTE_timeout:
                    "Erro de tempo de espera ao emitir o voto. Por favor, inténteo de novo máis tarde ou contacte co soporte para obter axuda.",
                CAST_VOTE_InsertFailedExceedsAllowedRevotes:
                    "Superou o límite de renovacións de voto. Por favor, inténteo de novo máis tarde ou contacte co soporte para obter axuda.",
                CAST_VOTE_CheckRevotesFailed:
                    "Superou o número permitido de renovacións de voto. Por favor, inténteo de novo máis tarde ou contacte co soporte para obter axuda.",
                CAST_VOTE_CheckVotesInOtherAreasFailed:
                    "Xa votou noutra área. Por favor, inténteo de novo máis tarde ou contacte co soporte para obter axuda.",
                CAST_VOTE_UnknownError:
                    "Produciuse un erro descoñecido ao emitir o voto. Inténtao de novo máis tarde ou contacta co soporte para obter axuda.",
                NO_BALLOT_SELECTION:
                    "O estado de selección para esta elección non está presente. Asegúrate de que seleccionaches as túas eleccións correctamente ou contacta co soporte.",
                NO_BALLOT_STYLE: "O estilo da papeleta non está dispoñible. Contacta co soporte.",
                NO_AUDITABLE_BALLOT: "Non hai papeleta auditable dispoñible. Contacta co soporte.",
                INCONSISTENT_HASH:
                    "Houbo un erro relacionado co proceso de hash da papeleta. O ID de seguimento: {{ballotId}} non é consistente co hash da Papeleta Auditable: {{auditableBallotHash}}. Informa deste problema ao soporte.",
                NOT_RECEIVED:
                    "O rexistro público aínda non contén a túa papeleta, polo que non se enviou. Volve á túa papeleta e téntao de novo.",
                ELECTION_EVENT_NOT_OPEN: "O evento de eleccións está pechado. Contacta co soporte.",
                PARSE_ERROR:
                    "Houbo un erro ao analizar a papeleta. Inténtao de novo máis tarde ou contacta co soporte para obter axuda.",
                DESERIALIZE_AUDITABLE_ERROR:
                    "Houbo un erro ao deserializar a papeleta auditable. Inténtao de novo máis tarde ou contacta co soporte para obter axuda.",
                DESERIALIZE_HASHABLE_ERROR:
                    "Houbo un erro ao deserializar a papeleta hashable. Inténtao de novo máis tarde ou contacta co soporte para obter axuda.",
                CONVERT_ERROR:
                    "Houbo un erro ao converter a papeleta. Inténtao de novo máis tarde ou contacta co soporte para obter axuda.",
                SERIALIZE_ERROR:
                    "Houbo un erro ao serializar a papeleta. Inténtao de novo máis tarde ou contacta co soporte para obter axuda.",
                UNKNOWN_ERROR:
                    "Houbo un erro. Inténtao de novo máis tarde ou contacta co soporte para obter axuda.",
                REAUTH_FAILED:
                    "A autenticación fallou. Inténtao de novo ou contacta co soporte para obter axuda.",
                SESSION_EXPIRED:
                    "A túa sesión expirou. Por favor, comeza de novo dende o principio.",
                CAST_VOTE_BallotIdMismatch: "O ID de seguimento non coincide co do voto emitido.",
                SESSION_STORAGE_ERROR:
                    "O almacenamento de sesión non está dispoñible. Por favor, inténteo de novo ou contacte co soporte.",
                PARSE_BALLOT_DATA_ERROR:
                    "Houbo un erro ao analizar os datos da papeleta. Por favor, inténteo de novo máis tarde ou contacte co soporte para obter axuda.",
                NOT_VALID_BALLOT_DATA_ERROR:
                    "Os datos da papeleta non son válidos. Por favor, inténteo de novo máis tarde ou contacte co soporte para obter axuda.",
                FETCH_DATA_TIMEOUT_ERROR:
                    "Erro de tempo de espera ao obter os datos. Por favor, inténteo de novo máis tarde ou contacte co soporte para obter axuda.",
                TO_HASHABLE_BALLOT_ERROR:
                    "Erro ao converter a papeleta hashable. Por favor, inténteo de novo máis tarde ou contacte co soporte para obter axuda.",
                INTERNAL_ERROR:
                    "Houbo un erro interno ao emitir o voto. Por favor, inténteo de novo máis tarde ou contacte co soporte para obter axuda.",
            },
            declineToVote: "Absterse de votar",
            blankBallot: "Papeleta en branco",
        },
        confirmationScreen: {
            acclamation: {
                title: "Resolto por aclamación",
                description:
                    "Todas as preguntas desta elección resolvéronse por aclamación, polo que non se emitiu ningunha papeleta e non hai ID de seguimento.",
            },
            title: "O teu voto foi emitido",
            blankBallot: {
                description:
                    "A súa papeleta emitiuse en branco, o cal é unha elección válida e deliberada.",
            },
            ballotId: "ID de seguimento",
            finishButton: "Rematar",
            saveReceiptButton: "Gardar o recibo (PDF)",
            receipt: "Recibo",
            keepTitle: "Garde o seu recibo",
            keepLead:
                "Este sitio web non garda ningunha copia. Gárdeo agora para comprobar a súa papeleta máis adiante, en calquera dispositivo.",
            castEmailNote: "Tamén lle enviamos unha copia por correo electrónico.",
            qrCaption: "Escanéeo para comprobala noutro dispositivo",
            remainingElectionsError:
                "Non puidemos comprobar se tes máis eleccións nas que votar. Téntao de novo.",
            retryButton: "Tentar de novo",
            demoPrintDialog: {
                title: "Imprimindo papeleta",
                content: "Impresión desactivada en modo de demostración",
                ok: "Aceptar",
            },
            demoBallotUrlDialog: {
                title: "ID de seguimento",
                content: "Non se pode usar o código, desactivado en modo de demostración.",
                ok: "Aceptar",
            },
            ballotIdHelpDialog: {
                title: "Sobre o ID de seguimento",
                content:
                    "O ID de seguimento é un código que permite atopar a túa papeleta na urna, este ID é único e non contén información sobre as túas seleccións.",
                ok: "Aceptar",
            },
            ballotIdDemoHelpDialog: {
                title: "Sobre o ID de seguimento",
                content:
                    "O ID de seguimento é un código que permite atopar a túa papeleta na urna, este ID é único e non contén información sobre as túas seleccións.",
                ok: "Aceptar",
            },
            errorDialogPrintBallotReceipt: {
                title: "Erro",
                content: "Produciuse un erro, por favor inténteo de novo",
                ok: "OK",
            },
            demoQRText: "O rastrexador de papeletas está desactivado en modo de demostración",
        },
        auditScreen: {
            confirmingAudit: "Confirmando que esta papeleta xa non se pode emitir…",
            auditUnavailable:
                "Non se puido auditar a papeleta. Non se revelou o seu contido. Téntao de novo.",
            printButton: "Imprimir",
            restartButton: "Comezar a votar",
            title: "Comproba a túa Papeleta",
            description: "Segue os pasos abaixo para comprobar a túa papeleta:",
            step1Title: "1. Descarga ou copia a seguinte información",
            step1Description:
                "O teu <b>ID de seguimento</b> aparece na parte superior da pantalla e a túa papeleta encriptada abaixo:",
            step1HelpDialog: {
                title: "Copia a Papeleta Encriptada",
                content:
                    "Podes descargar ou copiar a túa papeleta encriptada para auditala e verificar que o contido encriptado contén as túas seleccións.",
                ok: "Aceptar",
            },
            downloadButton: "Descargar",
            step2Title: "2. Verifica a túa papeleta",
            step2Description:
                "<VerifierLink>Accede ao verificador de papeletas</VerifierLink>, abrirase unha nova pestana no teu navegador.",
            step2HelpDialog: {
                title: "Tutorial de auditoría da papeleta",
                content:
                    "Para auditar a túa papeleta, necesitarás seguir os pasos mostrados no tutorial, o que inclúe a descarga dunha aplicación de escritorio usada para verificar a papeleta encriptada de forma independente do sitio web.",
                ok: "Aceptar",
            },
            bottomWarning:
                "Por razóns de seguridade, ao auditar a túa papeleta, esta debe ser anulada. Para continuar co proceso de votación, necesitas facer clic en ‘<b>Comezar a votar</b>’ abaixo.",
            voteAgain: "Votar de novo",
            check: {
                title: "Comproba a túa papeleta auditada",
                lead: "Esta papeleta abriuse en lugar de emitirse, así que nunca se contará. Comproba nun nodo que contén o que elixiches e despois vota de novo.",
                label: "Comprobación dunha papeleta auditada",
                encrypted: {
                    title: "Cifrada neste dispositivo",
                    text: "Co seu ID de seguimento, antes de que decidises auditala.",
                },
                opened: {
                    title: "Aberta no rexistro público",
                    text: "O portal publicou o segredo co que a cifrou. O rexistro marca a papeleta como Auditada.",
                    publishingText: "O portal está a publicar o segredo co que a cifrou.",
                    failedTitle: "Non aberta no rexistro público",
                    failedText:
                        "O portal non puido publicar o segredo co que a cifrou, así que ningún nodo pode comprobar aínda esta papeleta. A papeleta segue auditada e non se contará. Téntao de novo ou volve votar.",
                },
                compare: {
                    title: "Compárala nun nodo",
                    text: "O nodo descífraa no teu navegador e amosa as opcións que contén. Só ti sabes o que elixiches.",
                },
                entry: "entrada #{{index}}",
                hubsLabel: "Nodos que comproban papeletas, segundo a política da elección",
                runBy: "Xestionado por {{party}}",
                hubLabel:
                    "{{name}}, xestionado por {{party}}: comproba esta papeleta no seu propio sitio (ábrese nunha lapela nova)",
                authorityName: "Verificador de papeletas da elección",
                runByAuthority: "Xestionado pola autoridade electoral",
                authorityLabel:
                    "Verificador de papeletas da elección, xestionado pola autoridade electoral: comproba esta papeleta no seu propio sitio (ábrese nunha lapela nova)",
                footnote:
                    "Todos os nodos da política da elección que comproban papeletas, nunha orde aleatoria cada vez. Cada un ábrese no seu propio sitio co ID de seguimento na ligazón.",
                footnoteAuthority:
                    "Ningún nodo da política da elección comproba papeletas aínda, así que este é o verificador de papeletas da propia autoridade electoral. Ábrese co ID de seguimento na ligazón.",
                trackerText:
                    "Ningún nodo da política da elección comproba papeletas aínda. O rastrexador de papeletas pode amosala: descifra a papeleta no teu navegador.",
                trackerLink: "Ábrea no rastrexador de papeletas",
                tip: "É máis sólido desde outro dispositivo, como o teu móbil: abre o sitio de calquera nodo e pega o ID de seguimento.",
                tipTracker:
                    "É máis sólido desde outro dispositivo, como o teu móbil: abre alí o rastrexador de papeletas e pega o ID de seguimento.",
                why: "Por que auditar?",
                faq: {
                    why: {
                        question: "Por que auditar?",
                        answer: "O teu voto cífrase no teu dispositivo, así que ninguén pode lelo, nin sequera ti. Auditar permíteche comprobar que o portal cifrou o que elixiches.",
                    },
                    catch: {
                        question: "Por que detecta o engano?",
                        answer: "O portal cifra antes de que decidas auditar, así que non pode saber que papeletas se comprobarán. Se cambiase votos, calquera que audite podería descubrilo.",
                    },
                    spent: {
                        question: "Por que non pode contar esta papeleta?",
                        answer: "Co segredo publicado, a papeleta amosa como votaches. Para manter o teu voto en segredo, unha papeleta auditada nunca se emite.",
                    },
                    hub: {
                        question: "Por que nun nodo?",
                        answer: "Un portal que cambiase o teu voto tamén podería amosarche unha comprobación falsa. Un nodo é outro sitio; a papeleta e o seu segredo saen do rexistro público, non desta páxina.",
                    },
                },
                helpDialog: {
                    title: "Comproba a túa papeleta auditada",
                    content:
                        "<p>Auditar abre unha papeleta en lugar de emitila. O portal publica o segredo co que cifrou as túas opcións, así que calquera pode descifrar a papeleta e ver o que contén.</p><p>Comproba nun nodo que contén o que elixiches. Despois vota de novo: a túa nova papeleta ten un novo ID de seguimento, e é a que conta.</p>",
                    ok: "Aceptar",
                },
            },
        },
        electionSelectionScreen: {
            title: "Papeletas",
            description: "Selecciona a papeleta na que queres votar",
            chooserHelpDialog: {
                title: "Sobre a lista de papeletas",
                content:
                    "Esta pantalla amosa a lista de papeletas nas que podes votar. Só poderás acceder á papeleta se o período de votación está aberto.",
                ok: "OK",
            },
            noResults: "Sen papeletas por agora.",
            resultsButton: "Ver resultados",
            demoDialog: {
                title: "Cabina de votación de demostración",
                content:
                    "Estás entrando nunha cabina de votación de demostración. <strong>O teu voto non será emitido.</strong> Só para fins de demostración.",
                ok: "Entendo que o meu voto non será emitido",
            },
            errors: {
                noVotingArea:
                    "Non estás rexistrado como votante nesta elección. Por favor, contacta co soporte.",
                networkError:
                    "Houbo un problema de rede. Inténteo de novo máis tarde ou contacte co soporte para obter asistencia.",
                unableToFetchData:
                    "Houbo un problema ao recuperar os datos. Inténteo de novo máis tarde ou contacte co soporte para obter asistencia.",
                noElectionEvent:
                    "O evento electoral non existe. Inténteo de novo máis tarde ou contacte co soporte para obter asistencia.",
                ballotStylesEmlError:
                    "Houbo un erro co estilo de papeleta publicado. Inténteo de novo máis tarde ou contacte co soporte para obter asistencia.",
                obtainingElectionFromID:
                    "Houbo un erro ao obter as eleccións asociadas cos seguintes IDs de elección: {{electionIds}}. Inténteo de novo máis tarde ou contacte co soporte para obter asistencia.",
            },
            alerts: {
                noElections:
                    "Non hai eleccións nas que poidas votar. Isto pode deberse a que a área non ten ningunha pregunta asociada. Inténteo de novo máis tarde ou contacte co soporte para obter asistencia.",
                electionEventNotPublished:
                    "O evento electoral aínda non foi publicado. Inténteo de novo máis tarde ou contacte co soporte para obter asistencia.",
            },
            materialsGate: {
                instructions:
                    "Debes ler <MaterialsLink>{{materialsTitle}}</MaterialsLink> antes de poder votar.",
            },
            votingClosedAt: "A votación pechou o {{close}} ({{localClose}}).",
            votingClosedOn: "A votación pechou o {{close}}.",
        },
        errors: {
            encoding: {
                notEnoughChoices: "Non hai suficientes opcións para decodificar",
                writeInChoiceOutOfRange: "Opción de escritura libre fóra de rango: {{index}}",
                writeInNotEndInZero: "A escritura libre non remata en 0",
                bytesToUtf8Conversion:
                    "Erro ao converter a escritura libre de bytes a cadea UTF-8: {{errorMessage}}",
                ballotTooLarge: "A papeleta é máis grande do esperado",
            },
            explicit: {
                notAllowed:
                    "A papeleta está marcada como explicitamente inválida, pero a pregunta non o permite",
                alert: "Esta selección contarase como un voto inválido",
            },
            page: {
                oopsWithStatus: "Vaia! {{status}}",
                oopsWithoutStatus: "Vaia! Erro inesperado",
                somethingWrong: "Algo saiu mal.",
                invalidLoginHintParametersTitle: "Ligazón de votación non válida",
                invalidLoginHintParametersMessage:
                    "Esta ligazón de votación contén información de acceso non válida. Solicite unha nova ligazón e ténteo de novo.",
                certAuthFailedTitle: "Erro de Autenticación con Certificado",
                certAuthFailedMessage:
                    "Non foi posible verificar o seu certificado. Comprobe que está a usar un certificado de votante válido e ténteo de novo.",
            },
        },
        materials: {
            common: {
                label: "Materiais de apoio",
                back: "Voltar á lista de papeletas",
                close: "Pechar",
                preview: "Previsualizar",
                download: "Descargar",
            },
            mandatory: {
                checkboxLabel: "Lin os Materiais de apoio",
                continueButton: "Continuar",
                error: "Houbo un problema ao rexistrar a túa confirmación. Inténtao de novo.",
            },
        },
        slates: {
            title: "Candidaturas conxuntas",
            description:
                "Unha candidatura conxunta é un grupo de candidatos que se presentan xuntos. Cada candidato mostra a candidatura á que pertence.",
            independent: "Independente",
            contestMembers: "Candidatos de {{slate}} para {{contest}}",
            noCandidate: "Sen candidato",
            coverage: {
                full: "Candidatura completa",
                singleContest: "Só {{contest}}",
                partial: "Candidatura parcial",
                candidates_one: "{{count}} candidato",
                candidates_other: "{{count}} candidatos",
                offices_one: "{{count}} cargo",
                offices_other: "{{count}} cargos",
            },
            candidateList: {
                show: "Amosar candidaturas",
                hide: "Agochar candidaturas",
            },
            tabs: {
                label: "Formas de cubrir a súa papeleta",
                slates: "Elixir unha lista",
                candidates: "Candidaturas individuais",
            },
            review: {
                title: "As túas seleccións",
                total: "Candidaturas seleccionadas: {{selected}} de {{seats}}",
                slate: "{{slate}}: {{status}}",
                independent: "Candidaturas independentes seleccionadas: {{count}}",
                note: "O teu voto rexístrase para cada candidatura seleccionada. Unha lista non é un voto por si mesma.",
                contestCount: "{{selected}} de {{max}} seleccionadas",
                edit: "Editar",
                editLabel: "Editar {{contest}}",
            },
            selection: {
                all: "As {{total}} seleccionadas",
                mixed: "Mixta · {{selected}} de {{total}} seleccionadas",
                partly: "Seleccionada en parte · {{selected}} de {{total}}",
                selected: "Seleccionada",
                edit: "Editar a selección",
            },
            apply: {
                button: "Elixir esta candidatura",
                buttonLabel: "Elixir a candidatura {{slate}}",
                chosen: "{{slate}} elixida. Candidatos seleccionados: {{candidates}}. Contendas: {{contests}}.",
                overMaximum:
                    "{{slate}} non se pode elixir: ten {{candidates}} candidaturas para {{contest}}, que permite {{max}}. Pode seguir elixindo candidaturas individualmente.",
                unavailable:
                    "{{slate}} non se pode elixir nesta papeleta. Pode seguir elixindo candidaturas individualmente.",
                replaceDialog: {
                    title: "Substituír as túas opcións actuais?",
                    content:
                        "Elixir {{slate}} substitúe as túas opcións nas contendas seguintes. O resto das túas opcións non cambia.",
                    removed: "Quítase:",
                    added: "Selecciónase no seu lugar:",
                    ok: "Substituír opcións",
                    cancel: "Manter as miñas opcións",
                },
            },
        },
        ballotLocator: {
            title: "Seguimento da papeleta",
            titleResult: "Seguimento da papeleta",
            description: "Verifica que a túa Papeleta foi correctamente enviada",
            descriptionAudited:
                "Esta papeleta foi auditada, polo que nunca se emitiu. Comproba que as seleccións descodificadas a continuación son as que fixeches.",
            descriptionAuditedBrief: "Esta papeleta foi auditada, polo que nunca se emitiu.",
            locate: "Busca a túa Papeleta",
            locateAgain: "Buscar outra Papeleta",
            found: "O teu ID de seguimento {{ballotId}} foi atopado",
            notFound: "O teu ID de seguimento {{ballotId}} non foi atopado",
            contentDesc: "Este é o contido da túa Papeleta:",
            wrongFormatBallotId: "Formato incorrecto para o ID de seguimento",
            ambiguous: "Varias das túas papeletas comezan por {{ballotId}}. Escolle unha:",
            ambiguousMany:
                "Varias das túas papeletas comezan por {{ballotId}}. Escribe máis do ID de seguimento.",
            ballotIdNotFoundAtFilter:
                "Non atopado, comproba que o ID de seguimento sexa correcto e pertenza a este usuario.",
            filterByBallotId: "Filtrar por ID de seguimento",
            totalBallots: "Papeletas: {{total}}",
            idHelp: "O ID de seguimento curto, como FTBE-MHRX, ou o completo",
            policyId:
                "Este é o ID da política desta elección. O ID de seguimento dunha papeleta está no teu recibo.",
            foundShort:
                "Atopada polo seu ID curto. Para estares seguro de que é a túa, compara o ID de seguimento completo co teu recibo.",
            search: {
                title: "Comproba a túa papeleta no rexistro público",
                intro: "Introduce o ID de seguimento da túa pantalla de confirmación ou do teu recibo. O teu navegador búscao e comproba as probas por si mesmo.",
                find: "Buscar",
                invalid: "Un ID de seguimento só ten díxitos e letras, nunca a U.",
                chips: {
                    short: "<b>Curto</b> FTBE-MHRX",
                    full: "<b>Completo</b> 13 grupos de catro",
                    spaces: "Espazos opcionais",
                },
                features: {
                    cast: {
                        title: "Papeletas emitidas",
                        text: "Comproba que a túa papeleta está no rexistro público, confirmada por testemuñas e ancorada publicamente.",
                    },
                    audited: {
                        title: "Papeletas auditadas",
                        text: "Mira as seleccións que contén, descifradas no teu navegador, e comproba que son as que fixeches.",
                    },
                    secret: {
                        title: "O teu voto segue sendo secreto",
                        text: "Unha papeleta emitida segue cifrada. O rastrexador mostra que se contou, nunca como votaches.",
                    },
                },
            },
            matches: {
                cast: "Emitida o {{time}}",
                audited: "Auditada o {{time}}",
            },
            shortId: {
                head: {
                    one: "1 papeleta comeza por {{start}}",
                    other: "{{count}} papeletas comezan por {{start}}",
                    atLeast: "{{count}} ou máis papeletas comezan por {{start}}",
                },
                sub: {
                    few: "Cal é a túa? Compara o grupo resaltado co teu recibo, ou escríbeo abaixo.",
                    one: "Ábrea e compara o seu ID de seguimento completo co teu recibo.",
                    many: "Son máis das que mostramos. Escribe o terceiro grupo do teu recibo para atopar a túa.",
                },
                why: {
                    few: "Un ID curto atopa papeletas. Só o ID de seguimento completo do teu recibo di cal é a túa.",
                    many: "Tantas adoita significar que alguén creou papeletas para que coincidan con este ID curto adrede. A túa segue no rexistro: acertar tamén tres grupos requiriría uns mil trillóns de intentos.",
                },
                third: "Terceiro grupo do teu recibo",
                thirdOr: "Ou escribe o terceiro grupo do teu recibo",
                find: "Buscar",
                serverSees:
                    "Os servidores do rexistro non puideron facer esta busca sen saber que papeleta buscas, así que se lle pediu ao servidor da elección este ID exacto e puido velo.",
            },
            audit: {
                kicker: "Auditoría da papeleta",
                title: "Comproba que a túa papeleta se cifrou como elixiches",
                note: "Esta é a papeleta que auditaches, descodificada polo teu navegador. Só ti sabes o que elixiches, así que só ti podes facer esta comprobación.",
                steps: {
                    label: "Como funciona a comprobación",
                    encrypted: {
                        title: "Cifrada no teu dispositivo",
                        text: "O portal cifrou as túas opcións e deulle á papeleta o seu ID de seguimento, antes de que decidiras auditala.",
                    },
                    decoded: {
                        title: "Descodificada aquí",
                        text: "Ao auditar, o portal revelou o seu segredo. O teu navegador descodificou a papeleta con el e comparouna co rexistro público.",
                    },
                    compare: {
                        title: "Comparas ti",
                        text: "Mira as opcións de abaixo. Son as que fixeches?",
                    },
                },
                choices: "Opcións descodificadas desta papeleta",
                question: "Coinciden co que elixiches?",
                yes: "Si, coinciden",
                no: "Non, hai algo diferente",
                change: "Cambiar a miña resposta",
                match: {
                    title: "A túa papeleta cifrouse como elixiches",
                    text: "Esta papeleta auditada xa está gastada: nunca se emitiu e nunca se contará. Volve e vota. O teu voto recibirá un ID de seguimento novo, e se queres podes auditar outra vez antes.",
                    vote: "Volver e votar",
                },
                mismatch: {
                    title: "Non votes neste dispositivo",
                    text: "O que se cifrou non é o que elixiches, así que este dispositivo ou o portal de votación poden ter sido manipulados. O teu navegador gardou a proba: a papeleta cifrada do rexistro público, o segredo que revelou o portal e o que descodifica.",
                    report: "Avisa a autoridade electoral",
                    reportNoContact: "Avisa a autoridade electoral.",
                    fromPolicy: "Da política publicada da elección, non desta páxina.",
                    fromSettings: "Dos axustes da elección.",
                    then: "Despois vota desde outro dispositivo.",
                    save: "Gardar a proba",
                    email: "Escribir á autoridade",
                    emailSubject: "A miña papeleta auditada non se descodifica nas miñas opcións",
                    emailBody:
                        "Auditei unha papeleta nesta elección e as opcións que descodificou o meu navegador non son as que elixín.\n\nID de seguimento: {{id}}\n\nGardei a proba nun ficheiro e podo enviala.",
                },
                how: "Como funciona?",
                faq: {
                    why: {
                        question: "Por que auditar?",
                        answer: "O teu voto cífrase no teu dispositivo, así que ninguén pode lelo, nin sequera ti. Auditar permíteche comprobar que o portal cifrou o que elixiches antes de votar de verdade.",
                    },
                    catch: {
                        question: "Por que detecta o engano?",
                        answer: "O portal cifra antes de que decidas auditar, así que non pode saber que papeletas se comprobarán. Se cambiase votos, calquera votante que audite podería descubrilo.",
                    },
                    spent: {
                        question: "Por que non pode contar esta papeleta?",
                        answer: "Co segredo revelado, a papeleta amosa como votaches. Para manter o teu voto en segredo, unha papeleta auditada nunca se emite.",
                    },
                    differ: {
                        question: "E se non coinciden?",
                        answer: "Non votes nese dispositivo. Garda a proba e avisa a autoridade electoral, cuxo contacto sae da política da elección. Despois vota desde outro dispositivo.",
                    },
                    tip: "Para a comprobación máis sólida, abre esta páxina nun dispositivo distinto daquel co que votaches, por exemplo o teu móbil. Un dispositivo que cambiase a túa papeleta tamén podería cambiar o que che amosa aquí.",
                },
                failed: {
                    title: "Auditoría fallida",
                    text: "A papeleta do rexistro público non se descifra coa clave que revelou o portal. Non emitas o teu voto desde este dispositivo; avisa á autoridade electoral.",
                },
                unchecked: {
                    title: "Non se puido comprobar a auditoría",
                    text: "Este dispositivo non puido comprobar esta papeleta auditada, polo que non se mostra ningunha selección. Téntao de novo máis tarde, ou comproba o contido da papeleta no verificador de papeletas.",
                },
            },
            result: {
                receipt: "Recibo da papeleta",
                idComment: "Usa o ID curto ou o completo para volver atopar esta papeleta.",
                fullId: "ID de seguimento completo",
                copy: "Copiar o ID de seguimento completo",
                copied: "ID de seguimento copiado",
                tag: {
                    found: "Atopada",
                    auditFailed: "Auditoría fallida",
                },
                record: {
                    title: "Contido da túa papeleta",
                    subtitle: "Cifrado, tal como está no rexistro público",
                },
                recordEntry: "Entrada do rexistro",
                save: "Gardar o recibo",
                sealLabel: "Estado da papeleta: {{status}}",
                check: "Comprobalo de forma independente",
                caption:
                    "Comprobado polo teu navegador, con código que calquera pode inspeccionar.",
                captionVerified:
                    "Verificado polo teu navegador, con código que calquera pode inspeccionar.",
                captionProblem:
                    "Atopado polo teu navegador, con código que calquera pode inspeccionar.",
                late: "Normalmente significa que un nodo vai lento ou está sen conexión; non é proba de manipulación. Volve comprobalo máis tarde, aquí ou noutro dispositivo.",
                problem:
                    "O portal indica que esta papeleta foi emitida, pero o rexistro público da elección non ten ningunha entrada para ela. O teu navegador gardou a proba: a resposta do portal e o punto de control asinado do rexistro. Envíaa á autoridade electoral ou a un observador.",
                problemOther:
                    "As probas asinadas desta papeleta contradínse entre si. Pode ser un fallo ou un ataque. O teu navegador gardou a proba. Envíaa á autoridade electoral ou a un observador.",
                saveEvidence: "Gardar a proba",
                howToReport: "Como denuncialo",
                howToReportText:
                    "Garda o ficheiro da proba e envíao á autoridade electoral ou a un observador. Poden comprobalo nos seus propios ordenadores. Non contén nada sobre como votaches.",
                noReceipt:
                    "Aquí non se puido comprobar ningún recibo desta papeleta. Comproba o ficheiro do recibo que gardaches no verificador de papeletas, ou téntao de novo máis tarde.",
                status: {
                    checking: {
                        head: "Comprobando…",
                        sub: "O teu navegador está buscando a papeleta no rexistro público.",
                    },
                    recorded: {
                        head: "No rexistro público",
                        sub: "O teu navegador atopou a papeleta. Comprobando as súas testemuñas…",
                    },
                    witnessed: {
                        head: "Confirmada por testemuñas",
                        sub: "Comprobando os ancoraxes públicos…",
                        subAnchored_one:
                            "Ancorada en nodos públicos. Onde comproban os navegadores chegará en {{count}} minuto.",
                        subAnchored_other:
                            "Ancorada en nodos públicos. Onde comproban os navegadores chegará en {{count}} minutos.",
                    },
                    verified: {
                        head: "Verificada",
                        sub: "No rexistro público, confirmada por testemuñas e ancorada publicamente.",
                    },
                    late: {
                        head: "Tarda máis do esperado",
                        sub: "No rexistro público e confirmada por testemuñas, pero os seus ancoraxes públicos van tarde.",
                    },
                    problem: {
                        head: "Non está no rexistro público",
                        sub: "O teu navegador non puido atopar esta papeleta no rexistro público da elección.",
                    },
                    otherProblem: {
                        head: "Fallou unha comprobación",
                        sub: "As probas asinadas desta papeleta contradínse entre si. Mira o paso que fallou.",
                    },
                },
                steps: {
                    record: {
                        title: "Engadida ao rexistro público",
                        done: "Entrada n.º {{index}} do rexistro de papeletas da elección. O teu navegador comprobou a súa proba.",
                        failed: "Ningunha entrada do rexistro de papeletas da elección ten este ID de seguimento.",
                        missing: "Falta",
                    },
                    witnesses: {
                        title: "Confirmada por testemuñas",
                        done: "Testemuñas asinaron un rexistro que a inclúe.",
                        count: "{{signed}} de {{total}} · fan falta {{needed}}",
                        countOf: "{{signed}} de {{total}}",
                    },
                    hubs: {
                        title_one: "Ancorada en {{count}} nodo público",
                        title_other: "Ancorada en {{count}} nodos públicos",
                        titleAny: "Ancorada en nodos públicos",
                        done: "Rexistros compartidos tamén gardan esta entrada: {{hubs}}.",
                    },
                    browser: {
                        title: "Ancorada onde comproban os navegadores",
                        done: "Gardada por un nodo provisional ata que os navegadores comproben os rexistros por si mesmos.",
                    },
                    checking: "O teu navegador está comprobando…",
                    scheduled_one: "En {{count}} minuto. Non fai falta esperar.",
                    scheduled_other: "En {{count}} minutos. Non fai falta esperar.",
                    by: "Antes de {{time}}",
                    late: "Tarde",
                    lateHubs: "Os nodos deberían telo ancorado antes de {{time}}.",
                    lateOther: "Isto debería terse feito antes de {{time}}.",
                    waiting: "Agardando o paso anterior.",
                    skipped: "Sen comprobar.",
                    failedStep: "As probas asinadas deste paso contradínse entre si.",
                    failedTime: "Problema",
                },
            },
            checksAvailableUntil: "Podes comprobar a túa papeleta ata o {{date}}.",
            checksEnded: "As comprobacións remataron o {{date}}.",
            castAt: "Emitido o {{date}}",
            steps: {
                lookup: "Busca a túa Papeleta",
                result: "Resultado",
            },
            titleHelpDialog: {
                title: "Sobre o seguimento da papeleta",
                content:
                    "Esta pantalla permíteche atopar o teu voto a partir do ID de seguimento. Con este procedemento podes comprobar que a túa papeleta se emitiu correctamente e que a papeleta rexistrada coincide coa papeleta cifrada que enviaches.",
                ok: "Aceptar",
            },
            tabs: {
                logs: "Logs",
                ballotLocator: "Seguimento da papeleta",
            },
            column: {
                statement_kind: "Tipo",
                statement_timestamp: "Marca de tempo",
                ballot_id: "ID de seguimento",
            },
        },
        vitrine: {
            hubProblems: {
                title: {
                    one: "Un hub lista un problema nesta elección",
                    other: "Lístanse {{count}} problemas nesta elección",
                },
                listedBy: "Lístao {{hubs}}",
                proof: "Ver a proba",
                note: "Este aviso é unha comodidade deste sitio, e un listado é só unha referencia: abre a proba para comprobala no teu navegador. Os hubs tamén cho comunican por correo, nos seus propios sitios e na extensión, o faga ou non este sitio.",
                kind: {
                    splitView: "Dúas versións do rexistro",
                    brokenPromise: "Falta unha entrada prometida",
                    breaksRules: "Unha entrada incumpre as regras",
                    twoPolicies: "Dúas políticas",
                    unloggedCode: "Código que non está rexistrado",
                    other: "Un problema rexistrado por un hub",
                },
            },
            ballotId: {
                label: "ID de seguimento",
                full: "ID de seguimento completo",
                yours: "O teu ID de seguimento:",
                copy: "Copiar o ID de seguimento completo",
                copied: "ID de seguimento copiado",
                copyText: "Copiar",
                copiedText: "Copiado",
                showFull: "Mostrar o ID de seguimento completo",
                selected: "Seleccionado. Preme Ctrl+C, ou ⌘C nun Mac, para copialo.",
                copyByHand:
                    "O teu navegador non o copiou. Abre o ID de seguimento completo para seleccionalo.",
            },
            elsewhere: {
                title: "Búscaa noutro sitio",
                count_one: "{{count}} explorador",
                count_other: "{{count}} exploradores",
                introBallot:
                    "Os exploradores de terceiros len por conta propia o rexistro público. Cada un debería atopar a túa papeleta, coas mesmas comprobacións, sen esta web.",
                introEntry:
                    "Os exploradores de terceiros len por conta propia o rexistro público. Cada un debería amosar esta entrada tal como aparece aquí.",
                foot: "A lista vén da política da elección. Calquera explorador de Vitrine pode abrir o mesmo rexistro: dálle o ID de seguimento completo.",
                name: "Explorador de {{name}}",
                operatorName: "Explorador da elección",
                newTab: "Ábrese nunha pestana nova",
                who: {
                    monitor: "Xestionado por {{party}}, un monitor citado na política da elección",
                    hub: "Xestionado por {{party}}, un hub de ancoraxe citado na política da elección",
                    operator: "Xestionado pola autoridade electoral, que xestiona esta elección",
                },
            },
            status: {
                title: "Estado da papeleta",
                audit: "Auditar a papeleta",
                auditNote:
                    "Non estás seguro de que o portal cifrase ben as túas opcións? Audita esta papeleta: o portal revela como a cifrou para que o comprobes. Unha papeleta auditada nunca pode emitirse, así que despois votas de novo.",
                cast: "Emitida o {{when}}",
                audited: "Auditada o {{when}}",
                ballotCast: "Voto emitido",
                aria: "Estado da papeleta: {{state}}. Mostrar detalles",
                ariaCast: "Voto emitido. Estado da papeleta: {{state}}. Mostrar detalles",
                tooltip: "{{state}} · Mostrar detalles",
                tracker: "Abrir no rastrexador de papeletas",
                save: "Gardar o xustificante",
                ok: "Aceptar",
                tag: {
                    cast: "Emitida",
                    notCast: "Non emitida",
                    auditable: "Auditable",
                    audited: "Auditada",
                },
                state: {
                    notcast: "Aínda sen emitir",
                    checking: "Comprobando…",
                    recorded: "No rexistro público",
                    witnessed: "Confirmada por testemuñas",
                    verified: "Verificada",
                    late: "Con atraso",
                    problem: "Problema",
                    auditable: "Auditable",
                    auditfail: "Auditoría fallida",
                },
                head: {
                    late: "Tarda máis do previsto",
                    mismatch: "O rexistro non coincide",
                    damaged: "O xustificante está danado",
                    noReceipt: "Sen xustificante da túa papeleta",
                    unchecked: "Non se puido comprobar a túa papeleta",
                },
                sub: {
                    notcast:
                        "Esta papeleta está cifrada neste dispositivo e non se enviou nada. Cando a emitas, o teu navegador comprobará que chega ao rexistro público.",
                    checking:
                        "O teu navegador está a comprobar que a túa papeleta está no rexistro público.",
                    recorded:
                        "O teu navegador atopou a túa papeleta no rexistro público. Agora confírmana as testemuñas.",
                    witnessed:
                        "Testemuñas confirmaron o rexistro. As ancoraxes públicas chegan en {{minutes}} minutos; non hai que agardar.",
                    verified:
                        "A túa papeleta está no rexistro público, confirmada por testemuñas e ancorada publicamente.",
                    late: "A túa papeleta está no rexistro público e confirmada polas testemuñas, pero as súas ancoraxes públicas van con atraso.",
                    lateStep:
                        "Un paso tarda máis do previsto. Non é proba de manipulación, e o teu navegador segue comprobando.",
                    mismatch:
                        "A entrada n.º {{index}} do rexistro público non é a deste xustificante.",
                    mismatchNoEntry: "A entrada do rexistro público non é a deste xustificante.",
                    damaged:
                        "Non se puido ler ou verificar unha parte do xustificante ou do rexistro. É un fallo de datos, non unha proba de manipulación. O teu voto emitiuse.",
                    noReceipt:
                        "Este sitio web non recibiu ningún xustificante da túa papeleta. O teu voto emitiuse e avisouse aos observadores da elección.",
                    unchecked:
                        "Este sitio web non puido comprobar o xustificante. O teu voto emitiuse.",
                    auditable:
                        "Auditaches esta papeleta, polo que nunca se emitiu. Está no rexistro público, e o rastrexador de papeletas descífraa para mostrar as seleccións que contén.",
                    auditfail:
                        "A papeleta do rexistro público non se descifra coa clave que revelou o portal. Non emitas o teu voto desde este dispositivo; avisa á autoridade electoral.",
                },
                step: {
                    record: {
                        title: "No rexistro público",
                        upcoming:
                            "O teu navegador atopa a papeleta no rexistro público da elección.",
                    },
                    witnesses: {
                        title: "Confirmada por testemuñas",
                        upcoming: "Testemuñas asinan un rexistro que a inclúe.",
                    },
                    anchors: {
                        title: "Ancorada publicamente",
                        upcoming:
                            "Os nodos públicos, e os rexistros que comproban os navegadores, gardan ese rexistro.",
                    },
                },
                row: {
                    waiting: "Agardando polo paso anterior.",
                    checking: "O teu navegador está a comprobar…",
                    anchoring:
                        "Nodos públicos e, despois, onde comproban os navegadores: en {{minutes}} minutos.",
                    recorded: "O teu navegador comprobou a súa proba.",
                    recordedEntry: "Entrada n.º {{index}}. O teu navegador comprobou a súa proba.",
                    witnessed:
                        "Un número suficiente das testemuñas da política ({{count}}) asinou un rexistro que a inclúe.",
                    anchoredWaict:
                        "Nos nodos públicos ({{count}}) e onde comproban os navegadores.",
                    anchoredBootstrap:
                        "Nos nodos públicos ({{count}}) e onde comproban os navegadores, por agora mediante un nodo substituto.",
                    anchoredHubs: "Nos nodos públicos ({{count}}).",
                    anchoredOneHub: "Ancorada en 1 nodo, xestionado por {{party}}.",
                    anchoredOneHubPlatform: "Ancorada en 1 nodo, xestionado pola plataforma.",
                    lateNow: "Xa debería estar feito.",
                    lateStep: "Debería estar feito ás {{time}}.",
                    lateAnchors: "Os nodos deberían telo ancorado ás {{time}}.",
                    mismatch: "A entrada n.º {{index}} do rexistro é distinta do teu xustificante.",
                    mismatchNoEntry: "Unha entrada do rexistro é distinta do teu xustificante.",
                    contradiction: "As súas probas asinadas contradicíense.",
                    damaged: "A súa proba está danada e non se puido verificar.",
                    noReceipt: "Este sitio web non recibiu ningún xustificante.",
                    unchecked: "Este sitio web non puido comprobar o xustificante.",
                    skipped: "Non comprobado, porque non se puido completar o paso anterior.",
                    skippedMismatch: "Non comprobado, porque o rexistro non coincide.",
                },
                word: {
                    withinSeconds: "En poucos segundos",
                    withinMinutes: "En {{minutes}} min",
                    checking: "Comprobando…",
                    late: "Con atraso",
                    mismatch: "Non coincide",
                    damaged: "Danado",
                    noReceipt: "Sen xustificante",
                    unchecked: "Non comprobado",
                    skipped: "—",
                },
                callout: {
                    late: "Adoita significar que un nodo vai lento ou está fóra de liña; non é proba de manipulación. Volve comprobalo máis tarde co teu xustificante, no rastrexador de papeletas ou nun nodo, desde calquera dispositivo.",
                    evidence:
                        "O teu navegador gardou a evidencia: este xustificante e a entrada asinada do rexistro. Envíaa á autoridade electoral ou a un observador; calquera pode comprobala.",
                    saveEvidence: "Gardar a evidencia",
                },
                caption: {
                    notcast:
                        "Non se comproba nada ata que a emitas. Despois, esta marca mostra ata onde se verificou a túa papeleta.",
                    checking:
                        "O teu navegador está a comprobar o rexistro público, con código que calquera pode inspeccionar.",
                    problem:
                        "Detectado polo teu navegador, con código que calquera pode inspeccionar.",
                    auditable:
                        "Descifrado polo teu navegador, con código que calquera pode inspeccionar.",
                    verified:
                        "Verificado polo teu navegador, con código que calquera pode inspeccionar.",
                    checked:
                        "Comprobado polo teu navegador, con código que calquera pode inspeccionar.",
                    keep: "Garda o teu xustificante para volver comprobalo máis tarde, no rastrexador de papeletas ou nun nodo.",
                },
            },
            signers: {
                witnesses: {open: "Quen asinou", close: "Agochar quen asinou"},
                hubs: {open: "Que nodos", close: "Agochar que nodos"},
                runBy: "Xestionado por {{party}}",
                runByAlso: "Xestionado por {{party}}, que tamén xestiona {{hubs}}",
                signed: "Asinou ás {{time}}",
                signedNoTime: "Asinou",
                notYet: "Aínda non",
                late: "Con atraso",
                anchored: "Ancorado ás {{time}}",
                anchoredNoTime: "Ancorado",
                within: "En {{minutes}} min",
                check: "Comprobar no seu sitio",
                checkWitness:
                    "Comprobar no seu sitio: testemuña {{name}} (ábrese nunha pestana nova)",
                checkHub: "Comprobar no seu sitio: nodo {{name}} (ábrese nunha pestana nova)",
                linksNote:
                    "«Comprobar no seu sitio» abre o explorador do propio asinante, que comproba alí esa sinatura.",
                standIn: {
                    name: "Onde comproban os navegadores",
                    who: "Un nodo substituto, por agora",
                },
                rule: {
                    witnesses:
                        "A política esixe que asinen {{needed}} das súas {{total}} testemuñas. Asinaron {{signed}}.",
                    witnessesAlone:
                        "A política esixe que asinen {{needed}} das súas {{total}} testemuñas, e ningunha parte xestiona tantas. Asinaron {{signed}}.",
                    witnessesCount: "Asinaron {{signed}} das {{total}} testemuñas da política.",
                    hubs: "Os {{count}} nodos deben ancorar o rexistro, e polo menos {{independent}} deben estar xestionados por alguén distinto da autoridade electoral.",
                    hubsPlatform:
                        "Os {{count}} nodos deben ancorar o rexistro, e polo menos {{independent}} deben estar xestionados por alguén que non sexa nin a autoridade electoral nin a plataforma.",
                    hubsAll: "Os {{count}} nodos deben ancorar o rexistro.",
                    oneHub: "A política nomea un só nodo, xestionado por {{party}}.",
                    oneHubPlatform: "A política nomea un só nodo, xestionado pola plataforma.",
                },
            },
            updates: {
                optional: "Opcional",
                title: "Recibe actualizacións por correo sobre esta papeleta",
                description:
                    "Cando se confirme e cando se conte, ou se se rexistra un problema. Escolle calquera nodo: cada un comproba o rexistro por si mesmo.",
                listLabel: "Nodos da política desta elección",
                runBy: "Xestionado por {{party}}",
                hubLabel:
                    "{{name}}, xestionado por {{party}}: recibir actualizacións por correo no seu propio sitio (ábrese nunha pestana nova)",
                footnote:
                    "Os nodos que nomea a política da elección, en orde aleatoria cada vez. Cada un ábrese no seu propio sitio co teu recibo na ligazón; non se comparte nada ata que escollas que seguir alí.",
                footnoteNoReceipt:
                    "Os nodos que nomea a política da elección, en orde aleatoria cada vez. Cada un ábrese no seu propio sitio; non se comparte nada ata que escollas que seguir alí.",
            },
            booth: {
                title: "Aínda non se pode preparar a súa papeleta",
                explanation:
                    "Antes de cifrar a súa papeleta, este sitio web comproba que a configuración da elección é a publicada no rexistro público da elección. A súa papeleta non se cifrou nin se enviou, e as súas opcións mantéñense.",
                pending:
                    "A configuración publicada aínda non está dispoñible. Ténteo de novo nun momento.",
                invalid: "Non se puido ler a configuración publicada. Ténteo de novo nun momento.",
                failed: "A configuración non coincide coa publicada. Avisouse aos observadores da elección. Póñase en contacto co soporte.",
                retry: "Tentar de novo",
                close: "Pechar",
            },
            record: {
                json: "JSON",
                text: "Texto",
                fields_one: "{{count}} campo",
                fields_other: "{{count}} campos",
                items_one: "{{count}} elemento",
                items_other: "{{count}} elementos",
                size: {
                    bytes: "{{value}} B",
                    kilobytes: "{{value}} KB",
                },
                copy: "Copiar {{file}}",
                copied: "Copiado",
                download: "Descargar {{file}}",
            },
        },
    },
}

export default galegoTranslation
