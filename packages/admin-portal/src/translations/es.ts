// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import { TranslationType } from "./en"

const spanishTranslation: TranslationType = {
    translations: {
        breadcrumbSteps: {
            electionList: "Lista de Votaciones",
            ballot: "Ballot",
            review: "Review",
            confirmation: "Confirmation",
            audit: "Auditar",
        },
        votingScreen: {
            backButton: "Back",
            reviewButton: "Next",
            ballotHelpDialog: {
                title: "Información: Pantalla de votación",
                content:
                    "Esta pantalla muestra la votación en la que usted es elegible para votar. Puede seleccionar su sección activando la casilla de la derecha Candidato/Respuesta. Para restablecer sus selecciones, haga clic en el botón “<b>Borrar selección</b>”, para pasar al siguiente paso, haga clic en el botón “<b>Siguiente</b>”.",
                ok: "OK",
            },
        },
        startScreen: {
            startButton: "Empezar a votar",
            instructionsTitle: "Instrucciones",
            instructionsDescription: "Seguirá estos pasos al emitir tu voto:",
            step1Title: "1. Seleccione su opción de voto",
            step1Description:
                "Seleccione sus opciones de voto que se presentan una a una. Configurará así las preferencias de su papeleta.",
            step2Title: "2. Revise su papeleta",
            step2Description:
                "Una vez ha elegido sus preferencias, procederemos a cifrarlas y obtendrá un localizador. Le mostraremos el contenido de su papeleta para que pueda revisarla.",
            step3Title: "3. Envíe su voto",
            step3Description:
                "Puede enviar su voto a la urna electrónica para que sea debidamente registrado.",
        },
        reviewScreen: {
            title: "Review your ballot",
            description:
                "To make changes in your selections, click “<b>Change selection</b>” button, to confirm your selections, click “<b>Submit Ballot</b>” button bellow, and to audit your ballot click the “<b>Audit the Ballot</b>” button bellow. Please note than once you submit your ballot, you have voted and you will not be issued another ballot for this election.",
            backButton: "Edit ballot",
            castBallotButton: "Cast your ballot",
            auditButton: "Audit ballot",
            reviewScreenHelpDialog: {
                title: "Información: Pantalla de revisión",
                content:
                    "Esta pantalla le permite revisar sus selecciones antes de emitir su voto.",
                ok: "OK",
            },
            ballotIdHelpDialog: {
                title: "Voto no emitido",
                content:
                    "<p>Está a punto de copiar el Localizador del Voto, pero <b>su voto aún no se ha emitido</b>. Si intenta buscar el Localizador del Voto, no lo encontrará.</p><p>La razón por la que mostramos el Localizador del Voto en este momento es para que pueda auditar la corrección del voto cifrado antes de emitirlo. Si esa es la razón por la que desea copiar el Localizador del Voto, proceda a copiarlo y luego audite su voto.</p>",
                ok: "Acepto que mi voto NO ha sido emitido",
                cancel: "Cancelq4",
            },
            auditBallotHelpDialog: {
                title: "¿Realmente quieres Auditar tu papeleta?",
                content:
                    "<p>La auditoría de la papeleta lo invalidará y tendrás que iniciar el proceso de votación de nuevo si deseas emitir tu voto. El proceso de auditoría de la papeleta permite verificar que está codificada correctamente. Hacer este proceso requiere que unos conocimientos técnicos importantes, por lo que no se recomienda si no sabes lo que estás haciendo.</p><p><b>Si lo que desea es emitir su voto, en <u>Cancelar</u> para volver a la pantalla de revisión de votación.</b></p>",
                ok: "Si, quiero INVALIDAR mi papeleta para AUDITARLA",
                cancel: "Cancelar",
            },
        },
        confirmationScreen: {
            title: "Su voto ha sido emitido",
            description:
                "El código de confirmación que aparece a continuación verifica que <b>su voto se ha emitido correctamente</b>. Puede utilizar este código para verificar que su voto ha sido contabilizado.",
            ballotId: "Localizador del Voto",
            printButton: "Imprimir",
            finishButton: "Finalizar",
            verifyCastTitle: "Compruebe que su voto ha sido emitido",
            verifyCastDescription:
                "Puede comprobar en todo momento que su papeleta se ha emitido correctamente utilizando el siguiente código QR:",
            confirmationHelpDialog: {
                title: "Información: Pantalla de confirmación",
                content:
                    "Esta pantalla muestra que su voto se ha emitido correctamente. La información proporcionada en esta página le permite verificar que la papeleta ha sido almacenada en la urna , este proceso puede ser ejecutado en cualquier momento durante el periodo de votación y después de que la elección haya sido cerrada.",
                ok: "OK",
            },
            ballotIdHelpDialog: {
                title: "Información: Localizador del Voto",
                content:
                    "El Localizador del Voto de papeleta es un código que le permite encontrar su papeleta en la urna, este Localizador es único y no contiene información sobre sus selecciones.",
                ok: "OK",
            },
        },
        auditScreen: {
            printButton: "Imprimir",
            restartButton: "Iniciar votación",
            title: "Audite su Papeleta",
            description: "Para verificar su papeleta deberá seguir los siguientes pasos:",
            step1Title: "1. Descargue o copie la siguiente información",
            step1Description:
                "Tu <b>Localizador del Voto</b> que aparece en la parte superior de la pantalla y tu papeleta encriptada a continuación:",
            step1HelpDialog: {
                title: "Copiar el Voto Cifrado",
                content:
                    "Puede descargar o copiar su Voto Cifrado para auditarlo y verificar que el contenido encriptado contiene sus selecciones.",
                ok: "OK",
            },
            downloadButton: "Descargar",
            step2Title: "2. Siga los pasos de este tutorial",
            step2Description:
                '(<a href="https://github.com/sequentech/new-ballot-verifier/blob/main/README.md">haga click aquí</a>, se abrirá una nueva pestaña en su navegador)',
            step2HelpDialog: {
                title: "Tutorial sobre la Auditoría del Voto",
                content:
                    "Para auditar su voto deberá seguir los pasos indicados en el tutorial, que incluyen la descarga de una aplicación de escritorio utilizada para verificar el voto cifrado independientemente del sitio web.",
                ok: "OK",
            },
            bottomWarning:
                "Por motivos de seguridad, cuando audite su papeleta, deberá invalidarla. Para continuar con el proceso de votación, haga clic en ‘<b>Iniciar votación/b>’.",
        },
        electionSelectionScreen: {
            title: "Lista de Votaciones",
            description: "Seleccione la votación que desea votar",
            chooserHelpDialog: {
                title: "Información: Lista de Votaciones",
                content:
                    'Bienvenido a la cabina de votación, esta pantalla muestra la lista de elecciones en las que puede emitir su voto. Las elecciones que aparecen en esta lista pueden estar abiertas a votación, programadas o cerradas. Sólo podrá acceder a la votación si el periodo de votación está abierto. En el caso de que una elección esté cerrada y su administrador electoral haya publicado el resultado, verá un botón "Resultado electoral" que le llevará a la página pública de resultados.',
                ok: "OK",
            },
        },
        areas: {
            common: {
                title: "Áreas",
                subTitle: "Configuración de Área.",
            },
            createAreaSuccess: "Área creadas",
            createAreaError: "Error creando área",
            sequent_backend_area_contest: "Area de Contiendas",
        },
        electionTypeScreen: {
            common: {
                title: 'Tipo de Elección',
                subtitle: 'Configuración del Tipo de Elección',
                onlineVoting: 'Votación en Línea',
                kioskVoting: 'Votación en Quiosco'
            },
            create: {
                title: 'Crear Tipo de Elección'
            },
            edit: {
                title: 'Editar Tipo de Elección'
            },
            tabs: {
                votingChannels: 'CANALES DE VOTACIÓN',
                electionTypes: 'TIPOS DE ELECCIÓN',
                communications: 'COMUNICACIÓN',
                languages: 'IDIOMAS'
            }
        },
        electionEventScreen: {
            new: {
                subtitle: "Crear Evento Electoral",
            },
            common: {
                title: "Evento Electoral",
                subtitle: "Configuración del Evento de Elección.",
                contest: "Contiendas    ",
            },
            edit: {
                general: "General",
                dates: "Fechas",
                language: "Idiomas",
                allowed: "Canales de Voto Permitidos",
            },
            field: {
                name: "Nombre",
                alias: "Alias",
                description: "Descripción",
                startDateTime: "Fecha y hora de inicio",
                endDateTime: "Fecha y hora de finalización",
                language: "Idioma",
                votingChannels: "Canales de Voto",
            },
            error: {
                endDate: "La fecha de finalización debe ser posterior a la fecha de inicio",
            },
            voters: {
                title: "Votantes",
            },
            createElectionEventSuccess: "Evento de Elección creado",
            createElectionEventError: "Error creando Evento de Elección",
            stats: {
                elegibleVoters: "Votantes elegibles",
                elections: "Elecciones",
                areas: "Áreas",
                sentEmails: "Emails enviados",
                sentSMS: "SMS enviados",
                calendar: {
                    title: "Calendario",
                    scheduled: "Programado",
                },
            },
        },
        electionScreen: {
            common: {
                subtitle: "Configuración de la elección.",
            },
            edit: {
                general: "General",
                dates: "Fechas",
                language: "Idioma",
                allowed: "Canales de Voto Permitidos",
                default: "Por defecto",
                receipts: "Comprobantes",
                image: "Imagen",
                advanced: "Configuración Avanzada",
            },
            field: {
                name: "Nombre",
                language: "Idioma",
                votingChannels: "Canales de Voto",
                startDateTime: "Fecha y hora de inicio",
                endDateTime: "Fecha y hora de finalización",
                alias: "Alias",
                description: "Descripción",
            },
            error: {
                endDate: "La fecha de finalización debe ser posterior a la fecha de inicio",
                fileError: "Error al cargar el archivo",
            },
            createElectionEventSuccess: "Creada la elección",
            createElectionEventError: "Error Creando la elección",
        },
        tenantScreen: {
            common: {
                title: "Cliente",
            },
            new: {
                subtitle: "Crear Cliente",
            },
            createSuccess: "Cliente creado",
            createError: "Error creando cliente",
        },
        usersAndRolesScreen: {
            common: {
                title: "Usuarios y Roles",
                subtitle: "Configuración general",
            },
            users: {
                title: "Usuarios",
            },
            roles: {
                title: "Roles",
                edit: {
                    title: "Información de Rol",
                    subtitle: "Ver y editar Rol",
                },
            },
            permissions: {
                "tenant-create": "Create Tenant",
                "tenant-read": "Read Tenant",
                "tenant-write": "Edit Tenant",
                "election-event-create": "Create Election Event",
                "election-event-read": "Read Election Event",
                "election-event-write": "Edit Election Event",
                "voter-create": "Create Voter",
                "voter-read": "Read Voter",
                "voter-write": "Edit Voter",
                "user-create": "Create User",
                "user-read": "Read User",
                "user-write": "Edit User",
                "user-permission-create": "Create User Permission",
                "user-permission-read": "Read User Permission",
                "user-permission-write": "Edit User Permission",
                "role-create": "Create Role",
                "role-read": "Read Role",
                "role-write": "Edit Role",
                "role-assign": "Assign Role",
                "communication-template-create": "Create Communication Template",
                "communication-template-read": "Read Communication Template",
                "communication-template-write": "Edit Communication Template",
                "notification-read": "Read Notification",
                "notification-write": "Edit Notification",
                "notification-send": "Send Notification",
                "area-read": "Read Area",
                "area-write": "Edit Area",
                "election-state-write": "Edit Election State",
                "election-type-create": "Create Election Type",
                "election-type-read": "Read Election Type",
                "election-type-write": "Edit Election Type",
                "voting-channel-read": "Read Voting Channel",
                "voting-channel-write": "Edit Voting Channel",
                "trustee-create": "Create Trustee",
                "trustee-read": "Read Trustee",
                "trustee-write": "Edit Trustee",
                "tally-read": "Read Tally",
                "tally-start": "Start Tally",
                "tally-write": "Edit Tally",
                "tally-results-read": "Read Tally Results",
                "publish-read": "Read Publish",
                "publish-write": "Edit Publish",
                "logs-read": "Read Logs",
                "keys-read": "Read Keys",
            },
        },
        common: {
            label: {
                add: "Añadir",
                create: "Crear",
                delete: "Borrar",
                cancel: "Cancelar",
                edit: "Editar",
                save: "Guardar",
                close: "Cerrar",
                back: "Atrás",
                next: "Siguiente",
                warning: "Aviso",
                json: "Vista previa",
            },
            message: {
                delete: "¿Estás seguro de que quieres eliminar este elemento?",
            },
            language: {
                es: "Español",
                en: "Inglés",
            },
            channel: {
                online: "En línea",
                kiosk: "Kiosco",
            },
        },
        sideMenu: {
            electionEvents: "Procesos Electorales",
            search: "Buscar",
            usersAndRoles: "Usuarios y Roles",
            settings: "Configuracion",
            communicationTemplates: "Plantillas de Comunicación",
            active: "Activos",
            archived: "Archivados",
            addResource: {
                addElectionEvent: "Agregar un evento electoral",
                addElection: "Agregar una elección",
                addContest: "Agregar un concurso",
                addCandidate: "Agregar un candidato",
            },
        },
        candidateScreen: {
            common: {
                subtitle: "Configuración de candidatos.",
            },
            edit: {
                general: "General",
                type: "Tipo",
                image: "Imagen",
            },
            field: {
                name: "Nombre",
                alias: "Alias",
                description: "Descripción",
            },
            options: {
                "candidate": "Candidato",
                "option": "Opción",
                "write-in": "Voto por Escrito",
                "open-list": "Lista Abierta",
                "closed-list": "Lista Cerrada",
                "semi-open-list": "Lista Semiabierta",
                "invalid-vote": "Voto Inválido",
                "blank-vote": "Voto en Blanco",
            },
            error: {},
            createCandidateSuccess: "Candidato creado",
            createCandidateError: "Error creating candidato",
        },
        contestScreen: {
            common: {
                subtitle: "Configuración de contienda.",
            },
            edit: {
                general: "General",
                type: "Tipo",
                image: "Imagen",
                system: "Sistema de votación de papeletas",
                design: "Diseño de la papeleta",
                reorder: "Reordernar candidatos",
            },
            field: {
                name: "Nombre",
                alias: "Alias",
                description: "Descripción",
            },
            options: {
                "no-preferential": "Sin Preferencia",
                "plurality-at-large": "Mayoría Plural",
                "random-asnwers": "Respuestas Aleatorias",
                "custom": "Personalizado",
                "alphabetical": "Alfabético",
            },
            error: {},
            createContestSuccess: "Contienda creado",
            createContestError: "Error creating contienda",
        },
    },
}

export default spanishTranslation
