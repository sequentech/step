// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {TranslationType} from "./en"

const spanishTranslation: TranslationType = {
    translations: {
        language: "Español",
        welcome: "Let's start: Import auditable ballot..",
        breadcrumbSteps: {
            select: "Seleccionar un Verificador",
            import: "Importar Datos",
            verify: "Verificar",
            finish: "Terminar",
        },
        electionEventBreadcrumbSteps: {
            created: "Creado",
            keys: "Claves",
            publish: "Publicar",
            started: "Iniciado",
            ended: "Finalizado",
            results: "Resultados",
        },
        a11y: {
            closeDialog: "Cerrar diálogo",
            languageSelector: "Idioma: {{language}}",
            dismissMessage: "Descartar mensaje",
            ballotIdHelp: "Acerca de su ID de voto",
            loading: "Cargando",
            severity: {
                error: "Error",
                warning: "Advertencia",
                success: "Correcto",
                info: "Información",
            },
            selectList: "Seleccionar toda la lista",
            preferenceLabel: "Preferencia",
            writeInFor: "Nombre del candidato por escrito",
        },
        candidate: {
            moreInformationLink: "Más información",
            writeInsPlaceholder: "Teclee aquí el candidato por escrito",
            blankVote: "Voto en blanco",
            preferential: {
                position: "Posición",
                none: "Ninguna",
                ordinals: {
                    first: "º",
                    second: "º",
                    third: "º",
                    other: "º",
                },
            },
        },
        homeScreen: {
            title: "Verificador de Voto Sequent",
            description1:
                "El verificador de voto se usa cuando el votante elige auditar la boleta en la cabina de votación. La verificación debe tomar de 1 a 2 minutos.",
            description2:
                "El verificador de voto le permite al votante asegurarse de que el voto cifrado capture correctamente las selecciones realizadas en la cabina de votación. Permitir realizar esta verificación se denomina verificabilidad de transmisión según lo previsto y evita errores y actividades maliciosas durante el cifrado del voto.",
            descriptionMore: "Más información",
            startButton: "Selecciona fichero",
            dragDropOption: "O arrastre el fichero aquí",
            importErrorDescription:
                "Hubo un problema al importar el voto auditable. ¿Elegiste el archivo correcto?",
            importErrorMoreInfo: "Más información",
            importErrorTitle: "Error",
            useSampleText: "¿No tiene un voto verificable?",
            useSampleLink: "Use un voto verificable de ejemplo",
        },
        confirmationScreen: {
            title: "Verificador de Voto Sequent",
            topDescription1:
                "En base a la información del voto auditable importado, calculamos que:",
            topDescription2: "Si este ID de voto es mostrado en la Cabina de Votación:",
            bottomDescription1:
                "Su voto fue cifrado correctamente. Ahora puede cerrar esta ventana y volver a la Cabina de Votación.",
            bottomDescription2:
                "Si no coinciden, haga clic aquí para obtener más información sobre los posibles motivos y las acciones que puede tomar.",
            ballotChoicesDescription: "Y sus selecciones de voto son:",
            helpAndFaq: "Ayuda y Preguntas Frecuentes",
            backButton: "Atrás",
            markedInvalid: "Voto explícitamente marcado inválido",
        },
        ballotSelectionsScreen: {
            statusModal: {
                title: "Estado",
                content:
                    "El panel de estado te da información sobre las verificaciones realizadas.",
                ok: "OK",
            },
        },
        footer: {
            poweredBy: "Funciona con <sequent />",
        },
        errors: {
            encoding: {
                notEnoughChoices: "No hay suficientes opciones para decodificar",
                writeInChoiceOutOfRange: "Opción de voto escrita fuera de rango: {{index}}",
                writeInNotEndInZero: "Opción de voto escrita no finaliza en 0",
                writeInCharsExceeded:
                    "Opción de voto escrita excede el número de caracters por {{numCharsExceeded}} caracteres. Requiere arreglo.",
                bytesToUtf8Conversion:
                    "Error convirtiendo bytes de opción de voto escrita a cadena UTF-8: {{errorMessage}}",
                ballotTooLarge: "Voto más grande de lo esperado",
            },
            implicit: {
                selectedMax:
                    "Sobrevoto: El número de opciones seleccionadas {{numSelected}} es mayor que el máximo {{max}}",
                selectedMin:
                    "El número de opciones seleccionadas {{numSelected}} es menor que el máximo {{min}}",
                maxSelectionsPerType:
                    "El número de opciones seleccionadas {{numSelected}} para la lista {{type}} es mayor que el máximo {{max}}",
                underVote:
                    "Subvoto: El número de opciones seleccionadas {{numSelected}} es menor que el máximo permitido de {{max}}",
                overVoteDisabled:
                    "Máximo alcanzado: Has seleccionado el máximo de {{numSelected}} opciones. Para cambiar tu selección, por favor, desmarca primero otra opción.",
                blankVote: "Voto en Blanco: 0 opciones seleccionadas",
                preferenceOrderWithGaps:
                    "¡Voto inválido! El orden de preferencia tiene uno o más huecos.",
                duplicatedPosition:
                    "¡Voto inválido! La misma posición fue seleccionada para dos o más candidatos.",
            },
            explicit: {
                notAllowed:
                    "Voto marcado explícitamente como inválido pero la pregunta no lo permite",
                alert: "La selección marcada será considerada voto inválido.",
            },
            configuration: {
                multipleExplicitInvalidCandidates:
                    "Configuración de voto inválida: el concurso define {{count}} candidatos explícitamente inválidos, pero solo se permite uno.",
                multipleExplicitBlankCandidates:
                    "Configuración de voto inválida: el concurso define {{count}} candidatos de voto en blanco explícito, pero solo se permite uno.",
            },
        },
        ballotHash: "Su Localizador de Voto: {{ballotId}}",
        version: {
            header: "Versión:",
        },
        hash: {
            header: "Hash:",
        },
        logout: {
            buttonText: "Cerrar sesión",
            modal: {
                title: "¿Estás seguro de que quieres cerrar sesión?",
                content:
                    "Está a punto de cerrar esta aplicación. Esta acción no se puede deshacer.",
                ok: "OK",
                close: "Cerrar",
            },
        },
        stories: {
            openDialog: "Abrir Diálogo",
        },
        dragNDrop: {
            firstLine: "Arrastrar y soltar ficheros o",
            browse: "Cargar fichero",
            format: "Formatos soportados: txt",
            importError: "No se pudo importar este archivo. Inténtalo de nuevo.",
        },
        selectElection: {
            electionWebsite: "Sitio web electoral",
            countdown:
                "La elección comienza en {{years}} años, {{months}} meses, {{weeks}} semanas, {{days}} días, {{hours}} horas, {{minutes}} minutos, {{seconds}} segundos",
            openElection: "Abierta",
            closedElection: "Cerrada",
            voted: "Votado",
            notVoted: "No votado",
            resultsButton: "Resultados de Votación",
            voteButton: "Haga click para Votar",
            openDate: "Abierta: ",
            closeDate: "Cerrada: ",
            ballotLocator: "Localiza tu voto",
        },
        header: {
            profile: "Perfil",
            welcome: "Bienvenido/a,<br><span>{{name}}</span>",
            session: {
                title: "Su sesión está a punto de expirar.",
                timeLeft: "Le quedan {{time}} para emitir su voto.",
                timeLeftMinutesAndSeconds: "{{timeLeftInMinutes}} minutos y {{time}} segundos",
                timeLeftSeconds: "{{timeLeft}} segundos",
            },
        },
        problems: {
            noProblems: "Nada que señalar. Esto se importaría.",
            errors_one: "{{count}} error",
            errors_other: "{{count}} errores",
            errorsExplained:
                "Esto no se podrá importar hasta que se corrija cada uno de estos puntos.",
            warnings_one: "{{count}} aviso",
            warnings_other: "{{count}} avisos",
            warningsExplained:
                "Esto se importará. Cada uno de estos puntos es algo que probablemente no era lo que se quería.",
            warningsStrict: "El modo estricto está activado, así que estos impiden construir.",
            messages: {
                area: {
                    "cycle": {
                        lead: "Circunscripciones unas dentro de otras",
                        text: "Circunscripciones unas dentro de otras — «{{area}}» forma parte de un ciclo de circunscripciones madre, que bloquearía el Portal de Administración.",
                    },
                    "duplicate-name": {
                        lead: "Dos circunscripciones repetidas",
                        text: "Dos circunscripciones repetidas — «{{name}}» es a la vez «{{first}}» y «{{second}}». El CSV de votantes resuelve por nombre, así que los votantes acabarían en la que el importador encontrara primero.",
                    },
                    "early-voting-unknown": {
                        lead: "Voto anticipado desconocido",
                        text: "Voto anticipado desconocido — «{{value}}» no es válido para una circunscripción. La plataforma acepta {{allowed}}.",
                    },
                    "empty-ballot": {
                        lead: "Circunscripción con papeleta vacía",
                        text: "Circunscripción con papeleta vacía — «{{area}}» no vota en ninguna pregunta, ni propia ni heredada de la circunscripción que la contiene, así que sus votantes verían una papeleta vacía.",
                    },
                    "inside-itself": {
                        lead: "Circunscripción dentro de sí",
                        text: "Circunscripción dentro de sí — no puede ser su propia madre.",
                    },
                    "no-identifier": {
                        lead: "Circunscripción sin identificador",
                        text: "Circunscripción sin identificador — todas necesitan uno.",
                    },
                    "no-name": {
                        lead: "Circunscripción sin nombre",
                        text: "Circunscripción sin nombre — el CSV de votantes identifica la circunscripción por nombre, no por id, así que en una sin nombre no se puede poner a nadie.",
                    },
                    "parent-missing": {
                        lead: "Falta la circunscripción madre",
                        text: "Falta la circunscripción madre — la circunscripción está dentro de otra que no está en este archivo.",
                    },
                    "parent-unknown": {
                        lead: "Circunscripción madre desconocida",
                        text: "Circunscripción madre desconocida — nada tiene el identificador «{{parent}}».",
                    },
                },
                ballot: {
                    "no-elections": {
                        lead: "Sin elecciones",
                        text: "Sin elecciones — un evento electoral necesita al menos una.",
                    },
                },
                bundle: {
                    "duplicate-id": {
                        lead: "Identificador repetido",
                        text: "Identificador repetido — {{id}} en {{kind}} también lo usa {{previous}}.",
                    },
                    "event-no-encryption": {
                        lead: "Sin protocolo de cifrado",
                        text: "Sin protocolo de cifrado — el evento electoral no dice cómo se cifran las papeletas.",
                    },
                    "event-no-id": {
                        lead: "Evento sin id",
                        text: "Evento sin id — el evento electoral de este archivo no tiene identificador.",
                    },
                    "no-areas": {
                        lead: "Sin circunscripciones",
                        text: "Sin circunscripciones — no se puede dar papeleta a ningún votante hasta que el evento tenga al menos una circunscripción.",
                    },
                    "no-elections": {
                        lead: "Sin elecciones",
                        text: "Sin elecciones — un evento electoral necesita al menos una elección.",
                    },
                    "no-tenant": {
                        lead: "Sin tenant",
                        text: "Sin tenant — el archivo no dice a qué tenant pertenece.",
                    },
                    "tenant-not-uuid": {
                        lead: "El tenant no es un identificador",
                        text: "El tenant no es un identificador — «{{value}}» no es un id de tenant válido.",
                    },
                },
                candidate: {
                    "contest-missing": {
                        lead: "Falta la pregunta de la candidatura",
                        text: "Falta la pregunta de la candidatura — la candidatura pertenece a una pregunta que no está en este archivo.",
                    },
                    "no-contest": {
                        lead: "Candidatura sin pregunta",
                        text: "Candidatura sin pregunta — toda candidatura debe pertenecer a una pregunta.",
                    },
                    "picture-mismatch": {
                        lead: "La imagen apunta a otro sitio",
                        text: "La imagen apunta a otro sitio — la papeleta muestra «{{url}}», que no nombra el documento «{{document}}» que la acompaña. Tras importar, cada uno apuntaría a un archivo distinto.",
                    },
                    "picture-not-shown": {
                        lead: "Imagen que nunca se muestra",
                        text: "Imagen que nunca se muestra — una candidatura nombra una imagen a la que no apunta ninguna entrada de la papeleta, así que se subiría y nunca se vería.",
                    },
                    "picture-unrecorded": {
                        lead: "Imagen sin registrar",
                        text: "Imagen sin registrar — la imagen de una candidatura aparece en la papeleta y nada registra qué documento es, así que después no se podrá cambiar ni quitar en la plataforma.",
                    },
                },
                census: {
                    "column-not-declared": {
                        lead: "Columna del censo no declarada",
                        text: "Columna del censo no declarada — {{message}}",
                    },
                    "no-area-column": {
                        lead: "El censo no nombra ninguna",
                        text: "El censo no nombra ninguna — esta elección tiene circunscripciones pero todos los votantes recibirían la papeleta por defecto. Si la división debe aplicarse, el censo necesita una columna de circunscripción.",
                    },
                    "note": {
                        lead: "Sobre el censo",
                        text: "Sobre el censo — {{message}}",
                    },
                    "unreadable": {
                        lead: "Censo ilegible",
                        text: "Censo ilegible — {{message}}",
                    },
                    "unreadable-member": {
                        lead: "Censo del zip ilegible",
                        text: "Censo del zip ilegible — {{message}}",
                    },
                },
                channels: {
                    "early-voting-closed": {
                        lead: "Voto anticipado desactivado",
                        text: "Voto anticipado desactivado — {{areas}} permiten el voto anticipado y el evento no abre ese canal, así que el ajuste no hace nada.",
                    },
                    "early-voting-no-area": {
                        lead: "Voto anticipado sin circunscripciones",
                        text: "Voto anticipado sin circunscripciones — el voto anticipado está abierto y ninguna circunscripción lo permite, así que el periodo anticipado no tendría votantes.",
                    },
                    "kiosk-client": {
                        lead: "El quiosco necesita su propio cliente",
                        text: "El quiosco necesita su propio cliente — votar en quiosco requiere un cliente de autenticación con el nombre del habitual terminado en «-kiosk», y este archivo no lo crea.",
                    },
                    "none-open": {
                        lead: "Ninguna forma de votar",
                        text: "Ninguna forma de votar — todos los canales de votación están desactivados, así que nadie puede votar.",
                    },
                    "telephone-elsewhere": {
                        lead: "Voto telefónico, después",
                        text: "Voto telefónico, después — el voto por teléfono se configura en la pestaña IVR del evento tras importar; nada de ello viene en este archivo.",
                    },
                    "unknown": {
                        lead: "Forma de votar desconocida",
                        text: "Forma de votar desconocida — nada lee «{{name}}». Las formas de votar que la plataforma entiende son {{allowed}}.",
                    },
                },
                contacts: {
                    none: {
                        lead: "Sin puntos de contacto",
                        text: "Sin puntos de contacto — el día de la elección es a quien se llama.",
                    },
                },
                contest: {
                    "algorithm-unknown": {
                        lead: "Método de recuento desconocido",
                        text: "Método de recuento desconocido — «{{value}}» no es un algoritmo de recuento. La plataforma acepta {{allowed}}.",
                    },
                    "area-unknown": {
                        lead: "Circunscripción desconocida",
                        text: "Circunscripción desconocida — nada tiene el identificador «{{area}}».",
                    },
                    "cap-invalid": {
                        lead: "Límite de selección imposible",
                        text: "Límite de selección imposible — {{cap}} no es un número de selecciones.",
                    },
                    "cap-never-applies": {
                        lead: "Límite de selección que nunca se aplica",
                        text: "Límite de selección que nunca se aplica — un límite de {{cap}} por tipo nunca se aplica en una pregunta en la que se pueden elegir {{max}} en total.",
                    },
                    "chooses-more-than-available": {
                        lead: "Más opciones que candidaturas",
                        text: "Más opciones que candidaturas — un votante puede elegir {{max}} entre {{available}} candidaturas.",
                    },
                    "chooses-more-than-offered": {
                        lead: "Más opciones que candidaturas",
                        text: "Más opciones que candidaturas — un votante puede elegir hasta {{chosen}} pero solo hay {{offered}} entre las que elegir.",
                    },
                    "columns-invalid": {
                        lead: "Diseño imposible",
                        text: "Diseño imposible — {{columns}} columnas no es un diseño.",
                    },
                    "columns-too-many": {
                        lead: "Demasiadas columnas",
                        text: "Demasiadas columnas — {{columns}} columnas serán ilegibles en un móvil, que es como vota la mayoría.",
                    },
                    "count-missing": {
                        lead: "Falta un número en la pregunta",
                        text: "Falta un número en la pregunta — la pregunta necesita {{field}}.",
                    },
                    "count-negative": {
                        lead: "Número negativo",
                        text: "Número negativo — {{field}} vale {{value}}, y un recuento no puede ser menor que cero.",
                    },
                    "election-missing": {
                        lead: "Falta la elección de la pregunta",
                        text: "Falta la elección de la pregunta — la pregunta pertenece a una elección que no está en este archivo.",
                    },
                    "elects-more-than-available": {
                        lead: "Más puestos que candidaturas",
                        text: "Más puestos que candidaturas — la pregunta elige {{winners}} de {{available}} candidaturas.",
                    },
                    "elects-more-than-chosen": {
                        lead: "Elige más de lo permitido",
                        text: "Elige más de lo permitido — la votación elige {{winners}} pero un votante solo puede elegir {{chosen}}.",
                    },
                    "elects-more-than-standing": {
                        lead: "Más puestos que candidaturas",
                        text: "Más puestos que candidaturas — la votación elige {{winners}} de un total de {{standing}}.",
                    },
                    "elects-nobody": {
                        lead: "No elige a nadie",
                        text: "No elige a nadie — la votación no tiene ganadores.",
                    },
                    "max-votes-below-one": {
                        lead: "Nada que votar",
                        text: "Nada que votar — un votante puede elegir menos de una candidatura.",
                    },
                    "min-above-max": {
                        lead: "Mínimo por encima del máximo",
                        text: "Mínimo por encima del máximo — un votante debe elegir al menos {{min}} pero puede elegir como mucho {{max}}.",
                    },
                    "no-candidates": {
                        lead: "Sin candidaturas",
                        text: "Sin candidaturas — todavía no hay nadie en esta pregunta.",
                    },
                    "no-candidates-in-bundle": {
                        lead: "Sin candidaturas",
                        text: "Sin candidaturas — la pregunta no tiene candidaturas, así que nadie puede votar en ella.",
                    },
                    "on-no-ballot": {
                        lead: "Pregunta en ninguna papeleta",
                        text: "Pregunta en ninguna papeleta — ninguna circunscripción incluye esta pregunta en su papeleta, así que nadie puede votar en ella.",
                    },
                    "policy-not-text": {
                        lead: "Regla de papeleta que no es texto",
                        text: "Regla de papeleta que no es texto — {{key}} debería ser texto, y es {{value}}.",
                    },
                    "policy-unknown": {
                        lead: "Regla de papeleta desconocida",
                        text: "Regla de papeleta desconocida — «{{value}}» no es un valor válido de {{key}}. La plataforma acepta {{allowed}}.",
                    },
                    "ranked-counted-unranked": {
                        lead: "Papeleta ordenada contada sin orden",
                        text: "Papeleta ordenada contada sin orden — una pregunta preferencial se cuenta con «{{algorithm}}», que ignora el orden que dan los votantes.",
                    },
                    "tie-breaking-unknown": {
                        lead: "Regla de desempate desconocida",
                        text: "Regla de desempate desconocida — «{{value}}» no es una política de desempate. La plataforma acepta {{allowed}}.",
                    },
                    "unranked-counted-ranked": {
                        lead: "Papeleta simple contada por orden",
                        text: "Papeleta simple contada por orden — una pregunta no preferencial se cuenta con «{{algorithm}}», que necesita papeletas ordenadas.",
                    },
                    "voting-type-unknown": {
                        lead: "Tipo de votación desconocido",
                        text: "Tipo de votación desconocido — «{{value}}» no es un tipo de votación. La plataforma acepta {{allowed}}.",
                    },
                    "write-in-slots-not-allowed": {
                        lead: "Huecos de escritura no permitidos",
                        text: "Huecos de escritura no permitidos — hay {{count}} huecos para escribir candidaturas en una pregunta que no las permite, lo que pone opciones sin nombre en la papeleta.",
                    },
                    "write-ins-no-slot": {
                        lead: "Candidaturas escritas sin hueco",
                        text: "Candidaturas escritas sin hueco — se permiten candidaturas escritas y la pregunta no tiene hueco para ellas, así que el votante no tiene dónde escribir un nombre.",
                    },
                },
                delivery: {
                    "no-importable": {
                        lead: "Sin archivo importable",
                        text: "Sin archivo importable — este zip tiene un plan pero no el archivo que importa el Portal de Administración, así que no contiene el censo ni los archivos que nombra.",
                    },
                },
                election: {
                    "channels-differ": {
                        lead: "Elección y evento no coinciden",
                        text: "Elección y evento no coinciden — {{channels}} difiere del evento, así que los controles de inicio de esta elección no coincidirían.",
                    },
                    "grace-disallowed": {
                        lead: "Periodo de gracia desactivado",
                        text: "Periodo de gracia desactivado — hay {{seconds}} segundos de gracia configurados y no se permite periodo de gracia, así que la votación cierra a la hora límite.",
                    },
                    "grace-negative": {
                        lead: "Periodo de gracia negativo",
                        text: "Periodo de gracia negativo — {{value}} segundos no es una duración.",
                    },
                    "grace-zero": {
                        lead: "Periodo de gracia sin tiempo",
                        text: "Periodo de gracia sin tiempo — se permite un periodo de gracia y dura cero segundos, así que no hay ninguno.",
                    },
                    "no-contests": {
                        lead: "Sin preguntas",
                        text: "Sin preguntas — nadie vota en esta elección.",
                    },
                    "revotes-negative": {
                        lead: "Número de votos imposible",
                        text: "Número de votos imposible — {{value}} no es un número de veces que se pueda votar.",
                    },
                    "setting-unknown": {
                        lead: "Ajuste de la elección desconocido",
                        text: "Ajuste de la elección desconocido — «{{value}}» no es un valor válido de {{key}}. La plataforma acepta {{allowed}}.",
                    },
                    "spoil-without-revote": {
                        lead: "Anular sin segunda oportunidad",
                        text: "Anular sin segunda oportunidad — un votante puede desechar una papeleta emitida y no tiene otro intento para sustituirla.",
                    },
                },
                event: {
                    "no-identifier": {
                        lead: "Sin identificador",
                        text: "Sin identificador — cada id generado se deriva de él, así que sin uno nada puede construirse dos veces igual.",
                    },
                    "no-name": {
                        lead: "Sin nombre",
                        text: "Sin nombre — los votantes lo ven encima de la papeleta, y pasa a ser el título de la página de acceso.",
                    },
                    "setting-unknown": {
                        lead: "Ajuste del evento desconocido",
                        text: "Ajuste del evento desconocido — «{{value}}» no es un valor válido de {{key}}. La plataforma acepta {{allowed}}.",
                    },
                },
                file: {
                    "cannot-decrypt": {
                        lead: "No se pudo descifrar",
                        text: "No se pudo descifrar — el archivo está cifrado y no se ha abierto. Comprueba la contraseña.",
                    },
                    "checksum-mismatch": {
                        lead: "No es el archivo esperado",
                        text: "No es el archivo esperado — su SHA-256 no coincide con el indicado, así que no es el archivo que se pretendía. Revisa la suma de comprobación o vuelve a subir el archivo.",
                    },
                    "duplicate-name": {
                        lead: "Nombre de archivo repetido",
                        text: "Nombre de archivo repetido — «{{file}}» nombra dos cosas distintas. Los archivos viajan identificados por nombre, así que uno reemplazaría al otro en silencio.",
                    },
                    "missing": {
                        lead: "Falta un archivo",
                        text: "Falta un archivo — se nombra «{{file}}» y nada aquí lo contiene. Una entrada vacía en el archivo hace fallar la importación en vez de perder un fichero.",
                    },
                    "not-a-bundle": {
                        lead: "No es una exportación de evento electoral",
                        text: "No es una exportación de evento electoral — el archivo se ha leído pero no tiene la forma de una: {{reason}}",
                    },
                    "not-json": {
                        lead: "Archivo ilegible",
                        text: "Archivo ilegible — no es una exportación de evento electoral que la plataforma pueda leer: {{reason}}",
                    },
                    "unreadable-archive": {
                        lead: "Archivo comprimido ilegible",
                        text: "Archivo comprimido ilegible — {{reason}}",
                    },
                    "unused": {
                        lead: "Archivo sin usar",
                        text: "Archivo sin usar — se ha aportado «{{file}}» y nada lo nombra, así que viajaría en la entrega y no se le mostraría a nadie.",
                    },
                    "version-incompatible": {
                        lead: "Exportado con otra versión",
                        text: "Exportado con otra versión — el archivo viene de la versión {{found}}, que la versión {{current}} no puede importar.",
                    },
                },
                identifier: {
                    duplicate: {
                        lead: "Identificador repetido",
                        text: "Identificador repetido — «{{identifier}}» ya lo usa {{first}}. Los identificadores son únicos en todo el evento electoral, así que el segundo reemplaza al primero en vez de añadirse.",
                    },
                },
                ivr: {
                    "language-not-spoken": {
                        lead: "Idioma no disponible por teléfono",
                        text: "Idioma no disponible por teléfono — la llamada solo habla inglés, francés y español, así que no se ofrece {{languages}} a quien llama.",
                    },
                    "missing-prompts": {
                        lead: "Faltan mensajes de la llamada",
                        text: "Faltan mensajes de la llamada — {{prompts}} no tienen texto en «{{language}}», y el sistema telefónico rechaza todas las llamadas hasta que lo tengan.",
                        lead_one: "Falta un mensaje de la llamada",
                        text_one:
                            "Falta un mensaje de la llamada — {{prompts}} no tiene texto en «{{language}}», y el sistema telefónico rechaza todas las llamadas hasta que lo tenga.",
                    },
                },
                labels: {
                    "in-use": {
                        lead: "Etiquetas de permiso en uso",
                        text: "Etiquetas de permiso en uso — {{labels}}. Todo lo que lleva una etiqueta queda oculto a quien no la tiene, así que quien importe esto necesita una de ellas en su propia cuenta o el Portal de Administración le mostrará una lista vacía.",
                    },
                },
                languages: {
                    "default-not-offered": {
                        lead: "Idioma por defecto no ofrecido",
                        text: "Idioma por defecto no ofrecido — «{{chosen}}» no está entre {{offered}}, así que los votantes recibirían el primero.",
                    },
                    "detection-unknown": {
                        lead: "Detección de idioma desconocida",
                        text: "Detección de idioma desconocida — «{{policy}}» no es una política que la plataforma conozca.",
                    },
                    "none": {
                        lead: "Sin idiomas",
                        text: "Sin idiomas — la papeleta recurre al inglés, que es una red de seguridad y no una elección.",
                    },
                },
                link: {
                    "area-missing": {
                        lead: "Papeleta de una circunscripción inexistente",
                        text: "Papeleta de una circunscripción inexistente — una pregunta se pone en la papeleta de una circunscripción que no está en este archivo.",
                    },
                    "contest-missing": {
                        lead: "Papeleta con una pregunta inexistente",
                        text: "Papeleta con una pregunta inexistente — la papeleta de una circunscripción incluye una pregunta que no está en este archivo.",
                    },
                },
                logo: {
                    "file-and-link": {
                        lead: "Dos logotipos",
                        text: "Dos logotipos — este plan tiene a la vez un logotipo subido («{{file}}») y un enlace («{{url}}»). Se usa el archivo; el enlace se ignora.",
                    },
                    "file-missing": {
                        lead: "Falta el logotipo",
                        text: "Falta el logotipo — «{{file}}» se nombra como logotipo y no se ha aportado. Pon el archivo junto al libro con exactamente ese nombre.",
                    },
                    "no-bytes": {
                        lead: "Logotipo vacío",
                        text: "Logotipo vacío — «{{file}}» se nombra como logotipo y no tiene contenido. Una entrada vacía hace fallar la importación en vez de perder una imagen.",
                    },
                },
                material: {
                    "empty-document": {
                        lead: "Material con documento vacío",
                        text: "Material con documento vacío — se importaría como un enlace a nada. Si el material no tiene archivo, omite el documento por completo.",
                    },
                    "file-missing": {
                        lead: "Falta el archivo del material",
                        text: "Falta el archivo del material — «{{file}}» se nombra aquí y no se ha aportado. Pon el archivo junto al libro con exactamente ese nombre.",
                    },
                    "file-unused": {
                        lead: "Archivo de material sin usar",
                        text: "Archivo de material sin usar — se ha aportado «{{file}}» y ninguna fila lo nombra, así que se subiría y no se le mostraría a nadie.",
                    },
                    "no-identifier": {
                        lead: "Material sin identificador",
                        text: "Material sin identificador — el id de su documento se deriva de él, así que sin uno el archivo no puede emparejarse con la fila.",
                    },
                    "tab-off": {
                        lead: "Pestaña de materiales desactivada",
                        text: "Pestaña de materiales desactivada — hay {{count}} materiales de apoyo en este archivo y la pestaña que los muestra está desactivada, así que los votantes nunca los verían.",
                    },
                    "wrong-event": {
                        lead: "Material de otro evento",
                        text: "Material de otro evento — este material de apoyo pertenece a un evento electoral distinto.",
                    },
                },
                messages: {
                    "weekly-no-time": {
                        lead: "Una repetición sin hora",
                        text: "Una repetición sin hora — un mensaje se repite cada semana pero no dice a qué hora, así que quien lo envíe tiene que elegir una que nadie escribió.",
                    },
                },
                passwords: {
                    "column-already-there": {
                        lead: "Contraseñas por partida doble",
                        text: "Contraseñas por partida doble — el censo ya tiene una columna de contraseña, así que las generadas serían una segunda respuesta a la misma pregunta. Quita la columna o desactiva la generación.",
                    },
                    "no-characters": {
                        lead: "Contraseñas sin caracteres",
                        text: "Contraseñas sin caracteres — las contraseñas generadas necesitan al menos un tipo de carácter con el que formarse.",
                    },
                    "no-seed": {
                        lead: "Contraseñas sin semilla",
                        text: "Contraseñas sin semilla — la semilla es lo que hace que reconstruir produzca las mismas contraseñas y no otras nuevas.",
                    },
                },
                plan: {
                    "not-a-plan": {
                        lead: "Plan no válido",
                        text: "Plan no válido — el archivo no se ha podido leer como tal: {{error}}",
                    },
                    "saved-by-newer-version": {
                        lead: "Guardado con otra versión",
                        text: "Guardado con otra versión — {{saved}} frente a {{supported}}. Abrirlo aquí descartaría en silencio lo que esa versión añadiera.",
                    },
                    "unreadable": {
                        lead: "Plan ilegible",
                        text: "Plan ilegible — {{error}}",
                    },
                },
                schedule: {
                    "closes-before-opens": {
                        lead: "Cierra antes de abrirse",
                        text: "Cierra antes de abrirse — la votación nunca estaría abierta.",
                    },
                    "crosses-daylight-saving": {
                        lead: "Cruza un cambio de hora",
                        text: "Cruza un cambio de hora — la ventana dura una hora más o una hora menos de lo que indican los relojes.",
                    },
                    "key-ceremony-not-first": {
                        lead: "Ceremonia de claves tardía",
                        text: "Ceremonia de claves tardía — la clave de elección tiene que existir antes de que se pueda cifrar un voto con ella.",
                    },
                    "tally-ceremony-too-early": {
                        lead: "Recuento demasiado pronto",
                        text: "Recuento demasiado pronto — contaría votos que aún no se han emitido.",
                    },
                    "window-incomplete": {
                        lead: "Ventana de votación incompleta",
                        text: "Ventana de votación incompleta — el periodo habrá que abrirlo o cerrarlo a mano en el Portal de Administración.",
                    },
                },
                threshold: {
                    "above-trustees": {
                        lead: "Umbral demasiado alto",
                        text: "Umbral demasiado alto — se exigen {{threshold}} de {{trustees}} personas depositarias, lo que no se puede cumplir. La clave se generaría y el resultado no podría descifrarse nunca.",
                    },
                    "one": {
                        lead: "Umbral de uno",
                        text: "Umbral de uno — cualquier persona depositaria puede abrir el recuento sola, diga lo que diga la lista: la misma garantía que tener una sola, y ninguna frente a esa persona.",
                    },
                    "zero": {
                        lead: "Umbral de cero",
                        text: "Umbral de cero — el recuento podría abrirse sin ninguna persona depositaria.",
                    },
                },
                trustee: {
                    "email-malformed": {
                        lead: "Correo no válido",
                        text: "Correo no válido — «{{email}}» no lo parece.",
                    },
                    "no-email": {
                        lead: "Depositaria sin correo",
                        text: "Depositaria sin correo — es como se la invita a la ceremonia de claves.",
                    },
                    "no-name": {
                        lead: "Depositaria sin nombre",
                        text: "Depositaria sin nombre — el importador resuelve por nombre quiénes son los miembros de la ceremonia de claves, contra las ya provisionadas en el tenant, y un nombre vacío se convierte en silencio en un miembro que no existe.",
                    },
                },
                trustees: {
                    "none": {
                        lead: "Sin personas depositarias",
                        text: "Sin personas depositarias — una clave de elección necesita al menos dos que la custodien. Con una sola, esa persona puede descifrar todas las papeletas por su cuenta, que es justo la garantía que aporta el cifrado.",
                    },
                    "only-one": {
                        lead: "Una sola persona depositaria",
                        text: "Una sola persona depositaria — una clave de elección necesita al menos dos que la custodien. Con una sola, esa persona puede descifrar todas las papeletas por su cuenta, que es justo la garantía que aporta el cifrado.",
                    },
                },
                voter: {
                    "area-unknown": {
                        lead: "Circunscripción desconocida",
                        text: "Circunscripción desconocida — nada se llama «{{area}}». Los votantes se emparejan con su circunscripción por nombre, así que esta persona no recibiría papeleta. Copia el nombre en vez de volver a escribirlo.",
                    },
                    "duplicate-username": {
                        lead: "Nombre de usuario repetido",
                        text: "Nombre de usuario repetido — «{{username}}» es también la fila {{first}}. Dos votantes que lo comparten se convierten en una sola cuenta, y esta reemplazaría a la otra sin decirlo.",
                    },
                    "no-area": {
                        lead: "Votante sin circunscripción",
                        text: "Votante sin circunscripción — la circunscripción decide qué papeleta recibe cada votante, y la construcción rechaza una fila del censo sin ella.",
                    },
                    "no-username": {
                        lead: "Votante sin nombre de usuario",
                        text: "Votante sin nombre de usuario — es con lo que accede y de lo que se deriva su cuenta.",
                    },
                },
                voters: {
                    "duplicate-column": {
                        lead: "Columna repetida",
                        text: "Columna repetida — dos columnas indican «{{column}}». Deja solo una.",
                    },
                    "unreadable-row": {
                        lead: "Fila ilegible",
                        text: "Fila ilegible — no se ha podido leer la fila {{row}}: {{reason}}",
                    },
                    "vote-weight-misspelled": {
                        lead: "Columna de peso mal escrita",
                        text: "Columna de peso mal escrita — no se reconoce «{{column}}». La columna se escribe exactamente «{{expected}}»; si no, cada votante contaría con peso 1.",
                    },
                    "vote-weight-not-a-number": {
                        lead: "El peso del voto no es un número",
                        text: "El peso del voto no es un número — «{{value}}» en la fila {{row}} debe ser un número entero entre 1 y {{max}}.",
                    },
                    "vote-weight-out-of-range": {
                        lead: "Peso del voto fuera de rango",
                        text: "Peso del voto fuera de rango — {{value}} en la fila {{row}} debe estar entre {{min}} y {{max}}.",
                    },
                },
            },
        },
    },
}

export default spanishTranslation
