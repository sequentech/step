// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {TranslationType} from "./en"

const spanishTranslation: TranslationType = {
    translations: {
        404: {
            title: "Página no encontrada",
            subtitle: "La página que busca no existe",
        },
        homeScreen: {
            step1: "Paso 1: Importe su papeleta electoral.",
            description1:
                "Para continuar, por favor importe los datos de las papeletas encriptadas proporcionados en el Portal de Votación:",
            importBallotHelpDialog: {
                title: "Información: Importe su papeleta electoral",
                ok: "OK",
                content:
                    "Para continuar, por favor importe los datos de las papeletas encriptadas proporcionados en el Portal de Votación.",
            },
            step2: "Paso 2: Inserte su ID de seguimiento.",
            description2:
                "Pegue el ID de seguimiento completo del Portal de Votación: el botón de copiar junto a su ID de seguimiento lo copia.",
            ballotIdHelpDialog: {
                title: "Información: Su ID de seguimiento",
                ok: "OK",
                content:
                    "Pegue el ID de seguimiento completo proporcionado en el Portal de Votación. El botón de copiar que hay junto a él lo copia.",
            },
            startButton: "Seleccione fichero",
            dragDropOption: "O arrastre el fichero aquí",
            importErrorDescription:
                "Hubo un problema al importar el voto auditable. ¿Eligió el archivo correcto?",
            importErrorMoreInfo: "Más información",
            importErrorTitle: "Error",
            ciphertextErrorTitle: "La verificación del voto ha fallado",
            ciphertextErrorDescription:
                "El texto cifrado de este voto auditable no es el cifrado del texto en claro y la aleatoriedad que contiene. No se puede confiar en el voto.",
            verificationErrorTitle: "La verificación del voto ha fallado",
            unpublishedStyleErrorDescription:
                "Este voto auditable no corresponde a ninguna papeleta publicada para este evento electoral, por lo que no se puede verificar.",
            unavailableStyleErrorDescription:
                "No se pudieron cargar las papeletas publicadas para este evento electoral, por lo que este voto auditable aún no se puede verificar. Por favor, inténtelo de nuevo.",
            styleErrorDescription:
                "El estilo de voto de este voto auditable no es el publicado para la elección: su clave pública o sus contiendas y candidaturas difieren, o no está publicado. No se puede confiar en el voto.",
            legacyBallotWarning:
                "Este voto auditable usa un formato anterior cuyo ID de seguimiento no cubre el estilo de voto. Su ID de seguimiento no puede confirmar que se cifró con la clave publicada de la elección.",
            useSampleLink: "Use voto de ejemplo",
            nextButton: "Continuar",
            fileUploaded: "Cargado",
            receiptTitle: "Recibo de transparencia",
            receiptDescription:
                "Esta elección publica un recibo de cada papeleta emitida. Compruebe el suyo: que está en el registro público, sin cambios e igual para todos.",
            receiptButton: "Comprobar un recibo",
        },
        confirmationScreen: {
            ballotIdTitle: "ID de seguimiento",
            ballotIdDescription:
                "A continuación, el sistema muestra el ID de seguimiento de la papeleta descodificada y el que proporcionaste.",
            ballotIdError: "No coincide con el ID de seguimiento decodificado.",
            decodedBallotId: "ID de seguimiento decodificado",
            decodedBallotIdHelpDialog: {
                title: "Información: ID de seguimiento decodificado",
                ok: "OK",
                content:
                    "Este es el ID de seguimiento extraído del fichero de la Papeleta Auditable descodificada que proporcionaste.",
            },
            yourBallotId: "El ID de seguimiento que proporcionaste",
            userBallotIdHelpDialog: {
                title: "Información: El ID de seguimiento que proporcionaste",
                ok: "OK",
                content:
                    "Este es el ID de seguimiento que escribiste en el anterior paso y que recogiste de la Cabina de Votación.",
            },
            backButton: "Atrás",
            printButton: "Imprimir",
            finishButton: "Verificado",
            verifySelectionsTitle: "Verifique sus selecciones en la papeleta",
            verifySelectionsDescription:
                "Las siguientes selecciones de la papeleta han sido descodificadas de la papeleta que importó. Por favor, revíselas y asegúrese de que coincidan con las selecciones que hizo en el Portal de Votación. Si sus selecciones no coinciden, por favor, contacte con las autoridades electorales...",
            verifySelectionsHelpDialog: {
                title: "Información: Verifique sus selecciones en la papeleta",
                ok: "OK",
                content:
                    "Las siguientes selecciones de la papeleta han sido descodificadas de la papeleta que importó. Por favor, revíselas y asegúrese de que coincidan con las selecciones que hizo en el Portal de Votación. Si sus selecciones no coinciden, por favor, contacte con las autoridades electorales...",
            },
            markedInvalid: "Voto explícitamente marcado inválido",
            points_one: "({{count}} Punto)",
            points_many: "({{count}} Puntos)",
            points_other: "({{count}} Puntos)",
            contestNotFound: "Pregunta no encontrada: {{contestId}}",
            declineToVote: "Se abstuvo de votar",
            blankBallot: "Papeleta en blanco",
            acclamationDescription:
                "Esta votación se ha resuelto por aclamación. Sus candidaturas resultan elegidas sin votación, por lo que no se puede seleccionar ninguna opción ni se registra ningún voto.",
        },
        receiptScreen: {
            title: "Comprobar un recibo de transparencia",
            description:
                "Un recibo de transparencia muestra que su papeleta está en el registro público de la elección, sin cambios e igual para todos. Esta página lo comprueba en su navegador, con la política de confianza que publicó la autoridad electoral.",
            loading: "Cargando el comprobador de recibos…",
            loadFailed:
                "No se ha podido cargar el comprobador de recibos. Recargue la página para volver a intentarlo.",
            receipt: {
                title: "Su recibo",
                description:
                    "Abra el archivo del recibo que guardó al votar, o pegue el recibo, su enlace o su referencia (el texto de su código QR).",
                fileFormat: "Los archivos de recibo terminan en .vitrine-receipt",
                pasteLabel: "Recibo, enlace o referencia",
                pastePlaceholder: "Péguelo aquí",
                open: "Abrir",
                another: "Comprobar otro recibo",
                problems: {
                    "empty": "Pegue primero un recibo, su enlace o su referencia.",
                    "too-large": "Es demasiado grande para ser un recibo.",
                    "bad-link":
                        "Este enlace no contiene un recibo que se pueda leer. Abra el archivo del recibo.",
                    "bad-reference":
                        "Esta referencia está incompleta o dañada. Cópiela de nuevo o abra el archivo del recibo.",
                    "bad-chain": "Este recibo está dañado: no se puede leer.",
                    "not-https":
                        "El enlace indica que la política está en una dirección que no es segura (https), así que no se usa.",
                    "wrong-box":
                        "Esto es una política de confianza, no un recibo. Abra primero el recibo.",
                    "unrecognised":
                        "Esto no es un recibo, ni el enlace o la referencia de un recibo.",
                    "unreadable": "No se ha podido leer el archivo.",
                },
            },
            policy: {
                title: "Política de confianza",
                description:
                    "Los recibos se comprueban con la política de confianza que publicó la autoridad electoral. Indíquela aquí, con su texto o su dirección https, tal como figura en el anuncio oficial de la elección.",
                fromLink:
                    "El enlace del recibo indica dónde está su política de confianza. Antes de fijarla en este dispositivo, compare los cuatro primeros grupos de su ID de política con el anuncio oficial de la elección.",
                compareGroups: "Compare los cuatro primeros grupos, en negrita, con el anuncio. Las herramientas comparan los 13.",
                inputLabel: "Política de confianza o su dirección",
                inputPlaceholder: "Pegue la política o su dirección https",
                load: "Cargar la política",
                openFile: "Abrir un archivo de política",
                loading: "Cargando la política de confianza…",
                pin: "Coinciden: fijar esta política",
                reject: "No coinciden",
                rejected:
                    "No use esta política. Avise a la autoridad electoral e indique la política del anuncio oficial de la elección.",
                pinned: "Comprobado con la política de confianza que fijó en este dispositivo.",
                unpin: "Olvidar esta política",
                draft: "Esto es un borrador de la política, no la política sellada, así que no se puede fijar. Su ID de borrador:",
                invalid: "Esto no es una política de confianza válida.",
                otherPolicy: "Esta política de confianza es válida, pero el recibo es de otra.",
                ambiguous:
                    "Más de una política fijada coincide con esta referencia, así que no se usa. Abra el archivo del recibo.",
                problems: {
                    "empty": "Pegue primero la política de confianza o su dirección.",
                    "too-large": "Es demasiado grande para ser una política de confianza.",
                    "not-https": "Solo se usan direcciones que empiezan por https.",
                    "wrong-box": "Esto es un recibo, no una política de confianza.",
                    "unrecognised": "Esto no es una política de confianza ni su dirección.",
                    "unreadable": "No se ha podido leer el archivo.",
                },
                fetchProblems: {
                    "not-https": "Solo se usan direcciones que empiezan por https.",
                    "unreachable":
                        "No se ha podido cargar la política de confianza. Compruebe la dirección o pegue la política.",
                    "status":
                        "El servidor no ha devuelto la política de confianza (estado {{status}}).",
                    "too-large": "Es demasiado grande para ser una política de confianza.",
                    "not-text": "La dirección no ha devuelto una política de confianza.",
                },
            },
            ballot: {
                title: "Su papeleta",
                description:
                    "El recibo solo se muestra como suyo para la papeleta que emitió. Pegue el ID de seguimiento completo que le mostró el portal de votación.",
                election: "ID de la elección",
                electionPlaceholder: "Elija la elección",
                event: "ID del evento electoral",
                check: "Comprobar el recibo",
                change: "Cambiar el ID de seguimiento",
                lookedFor: "ID de seguimiento buscado",
                fullId: "ID de seguimiento completo",
                stage: "Paso de la papeleta",
                stages: {received: "Recibida", cast: "Emitida", audited: "Auditada"},
            },
            result: {
                title: "Resultado",
                rounds: {
                    log: "Comprobado con los registros de la elección. En la próxima media hora, esta página también consultará las réplicas de los registros, los hubs de anclaje y los monitores, en un momento aleatorio, para que no puedan saber cuándo lo comprobó.",
                    full: "Comprobado con los registros de la elección, sus réplicas, los hubs de anclaje y los monitores.",
                    offline:
                        "Los registros de esta elección son privados, así que el recibo se ha comprobado con lo que contiene.",
                },
                checkEverything: "Comprobarlo todo ahora",
                checkAgain: "Volver a comprobar",
                retry: "Reintentar",
                problems: {
                    "policy-mismatch": "Esta referencia es de otra política de confianza.",
                    "unknown-log":
                        "La política de confianza no tiene ningún registro con este nombre.",
                    "quiet-period":
                        "El registro no sirve pruebas durante su periodo de silencio. Vuelva a intentarlo después de {{time}}.",
                    "not-found":
                        "El registro aún no tiene esta entrada. Si votó hace poco, vuelva a intentarlo más tarde.",
                    "entry-mismatch":
                        "La entrada del registro en esta posición no es la de la referencia.",
                    "private-reference":
                        "Los registros de esta elección son privados, así que aquí no se puede comprobar una referencia. Abra el archivo del recibo.",
                    "failed": "No se ha podido comprobar el recibo. Vuelva a intentarlo.",
                },
                evidenceSaved:
                    "Se ha guardado el archivo del recibo. Contiene los objetos firmados que se contradicen: un auditor puede comprobarlo, y crear con él el archivo de pruebas, con el verificador de Vitrine.",
                advice: {
                    failed: "Una comprobación ha fallado: hay datos firmados que se contradicen entre sí o con la política de confianza. Guarde las pruebas y entréguelas, con el archivo del recibo, a la autoridad electoral y a los observadores que nombra la política de confianza.",
                    late: "Algo que ya debería estar aún no está. Puede que solo sea lentitud: vuelva a comprobarlo más tarde y, si sigue con retraso, avise a la autoridad electoral. Guarde el archivo del recibo.",
                },
            },
            status: {
                checking: {
                    head: "Comprobando…",
                    sub: "Su navegador está comprobando que su papeleta esté en el registro público.",
                },
                recorded: {
                    head: "En el registro público",
                    sub: "Su navegador ha encontrado su papeleta en el registro público. A continuación la confirman los testigos.",
                },
                witnessed: {
                    head: "Confirmado por testigos",
                    sub: "Testigos independientes han confirmado el registro. A continuación se consulta a los hubs públicos, en la próxima media hora.",
                    subLater:
                        "Testigos independientes han confirmado el registro. Los anclajes públicos llegan en menos de 10 minutos; no hace falta esperar.",
                },
                verified: {
                    head: "Verificada",
                    sub: "Su papeleta está en el registro público, confirmada por testigos independientes y anclada públicamente.",
                },
                late: {
                    head: "Tarda más de lo esperado",
                    sub: "Su papeleta está en el registro público y confirmada por testigos, pero sus anclajes públicos van con retraso.",
                    subOther: "Algunos de los pasos de abajo tardan más de lo esperado.",
                },
                problem: {
                    head: "El registro no coincide",
                    sub: "La entrada n.º {{index}} del registro público no es la de este recibo.",
                    subOther:
                        "El registro público, este recibo y su ID de seguimiento no coinciden entre sí.",
                },
                damaged: {
                    head: "No se ha podido comprobar",
                    sub: "Algunos datos parecen dañados: el recibo o la respuesta del servidor. Esto no demuestra que nadie haya hecho trampa.",
                },
                evidence:
                    "Su navegador ha guardado las pruebas: este recibo y la entrada firmada del registro. Envíelas a la autoridad electoral o a un observador; cualquiera puede comprobarlas.",
                saveEvidence: "Guardar las pruebas",
                howToReport: "Cómo denunciarlo",
                captionChecking:
                    "Su navegador está comprobando el registro público, con código que cualquiera puede inspeccionar.",
                captionChecked:
                    "Comprobado por su navegador, con código que cualquiera puede inspeccionar.",
                captionProblem:
                    "Detectado por su navegador, con código que cualquiera puede inspeccionar.",
                rows: {
                    record: "En el registro público",
                    witnesses: "Confirmado por testigos",
                    anchors: "Anclado públicamente",
                    recordDone: "Entrada n.º {{index}}. Su navegador ha comprobado su prueba.",
                    recordDoneNoIndex: "Su navegador ha comprobado su prueba.",
                    witnessesDone:
                        "{{signed}} de {{total}} testigos independientes han firmado; la política exige {{needed}}.",
                    witnessesDoneAny: "Testigos independientes la han firmado.",
                    hubs_one: "En {{count}} hub público",
                    hubs_many: "En {{count}} hubs públicos",
                    hubs_other: "En {{count}} hubs públicos",
                    browser: ", y donde comprueban los navegadores",
                    standIn: ", mediante un hub suplente por ahora",
                    unused: "No lo usa la política de esta elección.",
                    checking: "Su navegador está comprobando…",
                    anchorsActive: "Se consulta en la próxima media hora, en un momento aleatorio.",
                    anchorsPending: "Aún no está anclada. Vuelva a comprobarlo en unos minutos.",
                    waiting: "Esperando al paso anterior.",
                    lateAnchors: "Los hubs deberían haberla anclado a las {{time}}.",
                    lateOther: "Ya debería estar hecho.",
                    recordFailed: "La entrada n.º {{index}} del registro difiere de su recibo.",
                    failed: "No coincide con su recibo y su ID de seguimiento.",
                    damaged: "No se ha podido comprobar: los datos parecen dañados.",
                    skipped: "No comprobado, porque el registro no coincide.",
                    skippedDamaged: "No comprobado, porque un paso anterior no se pudo comprobar.",
                    statusChecking: "Comprobando…",
                    withinSeconds: "En segundos",
                    within10: "En 10 min",
                    within30: "En 30 min",
                    statusLate: "Con retraso",
                    statusMismatch: "No coincide",
                    statusDamaged: "Dañado",
                },
            },
        },
        footer: {
            poweredBy: "Funciona con <1></1>",
        },
        errors: {
            encoding: {
                notEnoughChoices: "No hay suficientes opciones para decodificar",
                writeInChoiceOutOfRange: "Opción de voto escrita fuera de rango: {{index}}",
                writeInNotEndInZero: "Opción de voto escrita no finaliza en 0",
                bytesToUtf8Conversion:
                    "Error convirtiendo bytes de opción de voto escrita a cadena UTF-8: {{errorMessage}}",
                ballotTooLarge: "Voto más grande de lo esperado",
            },
            explicit: {
                notAllowed:
                    "Voto marcado explícitamente como inválido pero la pregunta no lo permite",
            },
        },
        ballotIdField: {
            label: "ID de seguimiento",
            placeholder: "Pegue su ID de seguimiento completo",
            help: "Los espacios son opcionales.",
            shortError:
                "Ese es el ID corto. Pegue el ID de seguimiento completo: la comprobación lo compara entero.",
            formatError:
                "Revise el ID de seguimiento: tiene 64 caracteres, dígitos y letras de la a a la f.",
        },
        auditedScreen: {
            title: "Papeleta",
            tag: "Auditada",
            loading: "Comprobando la papeleta en el registro público…",
            retry: "Reintentar",
            header: {
                strip: {
                    withOperator:
                        "Un centro de transparencia, gestionado por {{operator}}. No es el sitio web de la elección y nunca le pide votar, iniciar sesión ni pagar.",
                    noOperator:
                        "Un centro de transparencia. No es el sitio web de la elección y nunca le pide votar, iniciar sesión ni pagar.",
                },
                subline: {
                    withOperator: "Gestionado por {{operator}} · un centro Vitrine",
                    noOperator: "Un centro Vitrine",
                },
                unnamed: "Centro de transparencia",
            },
            trackingId: {
                full: "ID de seguimiento completo",
                copy: "Copiar el ID de seguimiento completo",
                copied: "ID de seguimiento copiado",
                copyText: "Copiar",
                copiedText: "Copiado",
                selected: "Seleccionado. Pulsa Ctrl+C, o ⌘C en Mac, para copiarlo.",
            },
            checks: {
                title: "Comprobado en su navegador",
                record: {
                    title: "En el registro público",
                    text: "Entrada n.º {{index}}, auditada a las {{time}}. Nunca emitida.",
                    waiting: "Buscando su auditoría en el registro de la elección.",
                    failed: "La prueba del registro para esta auditoría no es válida.",
                },
                secret: {
                    title: "El secreto corresponde a la papeleta",
                    text: "Al volver a cifrar con él se obtiene la misma papeleta, byte a byte.",
                    waiting: "Se comprueba cuando se haya comprobado el registro.",
                    failed: "Al volver a cifrar con él no se obtiene esta papeleta.",
                },
                decoded: {
                    title: "Descodificada",
                    text: "Las opciones de la izquierda, según el diseño de papeleta publicado de la elección.",
                    waiting: "Solo cuando el secreto corresponda.",
                    failed: "No se pudo descodificar.",
                },
            },
            note: {
                named: "Con código que cualquiera puede inspeccionar. {{hub}} solo lo almacena y lo sirve, y nunca pregunta qué eligió. Su respuesta se queda en este navegador.",
                unnamed:
                    "Con código que cualquiera puede inspeccionar. Este centro solo lo almacena y lo sirve, y nunca pregunta qué eligió. Su respuesta se queda en este navegador.",
            },
            states: {
                "bad-link": {
                    title: "Este enlace no está completo",
                    text: "Necesita el ID de seguimiento completo de la papeleta y el ID de política de la elección. Ábralo de nuevo desde el sitio de votación.",
                },
                "no-policy": {
                    title: "No se encontró la política de la elección",
                    text: "Esta página solo comprueba una papeleta con la política que indica el enlace, y no pudo obtenerla de la elección. Inténtelo más tarde.",
                },
                "other-election": {
                    title: "Este enlace es de otra elección",
                    text: "La política que indica el enlace no cubre esta elección, así que la papeleta no se puede comprobar con ella.",
                },
                "not-found": {
                    title: "No hay ninguna papeleta con este ID de seguimiento",
                    text: "El registro público de la elección no tiene ninguna papeleta con este ID de seguimiento. Compruebe que el enlace está completo o inténtelo de nuevo en un minuto.",
                },
                "cast": {
                    title: "Esta papeleta se emitió, no se auditó",
                    text: "Solo se puede descodificar una papeleta auditada. Una papeleta emitida sigue cifrada, así que nadie ve cómo se votó, ni aquí ni en ningún otro sitio.",
                },
                "pending": {
                    title: "Esta papeleta aún no está auditada",
                    text: "El registro público tiene esta papeleta, pero aún no su auditoría. Inténtelo de nuevo en un minuto.",
                },
                "no-content": {
                    title: "La papeleta auditada aún no está publicada",
                    text: "El registro público tiene la auditoría de esta papeleta, pero no la papeleta y su secreto, así que aún no se puede descodificar aquí. Inténtelo más tarde.",
                },
                "unavailable": {
                    title: "No se pudo comprobar la papeleta",
                    text: "Esta página no pudo acceder al registro público de la elección. Inténtelo más tarde.",
                },
                "record-failed": {
                    title: "El registro no prueba esta auditoría",
                    text: "El registro de la elección dio una prueba de la auditoría de esta papeleta que no es válida, así que no se descodifica nada. Avise a la autoridad electoral.",
                },
                "mismatch": {
                    title: "Auditoría fallida",
                    text: "La papeleta del registro público no es la que indica su ID de seguimiento, o el secreto que publicó el portal no la reproduce. No vote en el dispositivo desde el que auditó y avise a la autoridad electoral.",
                },
                "unchecked": {
                    title: "No se pudo comprobar la auditoría",
                    text: "Esta página no pudo comprobar esta papeleta auditada, por lo que no se muestra ninguna opción. Inténtelo más tarde o en otro dispositivo.",
                },
            },
            compare: {
                kicker: "Auditoría de la papeleta",
                title: "Compruebe que su papeleta se cifró como usted eligió",
                note: "Esta es la papeleta que auditó, descodificada por su navegador. Solo usted sabe lo que eligió, así que solo usted puede hacer esta comprobación.",
                steps: {
                    label: "Cómo funciona la comprobación",
                    encrypted: {
                        title: "Cifrada en su dispositivo",
                        text: "El portal cifró sus opciones y dio a la papeleta su ID de seguimiento, antes de que usted decidiera auditarla.",
                    },
                    decoded: {
                        title: "Descodificada en su navegador",
                        text: "La papeleta y el secreto que publicó el portal vienen del registro público. Esta página descodificó una con el otro.",
                    },
                    compare: {
                        title: "Usted compara",
                        text: "Mire las opciones de abajo. ¿Son las que eligió?",
                    },
                },
                choices: "Opciones descodificadas de esta papeleta",
                question: "¿Coinciden con lo que eligió?",
                yes: "Sí, coinciden",
                no: "No, hay algo distinto",
                change: "Cambiar mi respuesta",
                match: {
                    title: "Su papeleta se cifró como eligió",
                    text: "Esta papeleta auditada ya está gastada: nunca se emitió y nunca se contará. Vuelva y vote. Su voto recibirá un ID de seguimiento nuevo, y si lo desea puede auditar otra vez antes.",
                    back: "Volver al sitio de votación",
                },
                mismatch: {
                    title: "No vote en este dispositivo",
                    text: "Lo que se cifró no es lo que eligió, así que este dispositivo o el portal de votación pueden haber sido manipulados. Su navegador guardó la prueba: la papeleta cifrada del registro público, el secreto que reveló el portal y lo que descodifica.",
                    report: "Avise a la autoridad electoral",
                    reportNoContact: "Avise a la autoridad electoral.",
                    fromPolicy: "De la política publicada de la elección, no de esta página.",
                    fromSettings: "De los ajustes de la elección.",
                    then: "Después vote desde otro dispositivo.",
                    save: "Guardar la prueba",
                    email: "Escribir a la autoridad",
                    emailSubject: "Mi papeleta auditada no se descodifica en mis opciones",
                    emailBody:
                        "Audité una papeleta en esta elección y las opciones que descodificó mi navegador no son las que elegí.\n\nID de seguimiento: {{id}}\n\nHe guardado la prueba en un archivo y puedo enviarla.",
                },
                how: "¿Cómo funciona?",
                faq: {
                    why: {
                        question: "¿Por qué auditar?",
                        answer: "Su voto se cifra en su dispositivo, así que nadie puede leerlo, ni siquiera usted. Auditar le permite comprobar que el portal cifró lo que eligió antes de votar de verdad.",
                    },
                    catch: {
                        question: "¿Por qué detecta el engaño?",
                        answer: "El portal cifra antes de que usted decida auditar, así que no puede saber qué papeletas se comprobarán. Si cambiara votos, cualquier votante que audite podría descubrirlo.",
                    },
                    spent: {
                        question: "¿Por qué no puede contar esta papeleta?",
                        answer: "Con su secreto revelado, la papeleta muestra cómo votó usted. Para mantener su voto en secreto, una papeleta auditada nunca se emite.",
                    },
                    differ: {
                        question: "¿Y si no coinciden?",
                        answer: "No vote en ese dispositivo. Guarde la prueba y avise a la autoridad electoral, cuyo contacto sale de la política de la elección. Después vote desde otro dispositivo.",
                    },
                    tip: "Para la comprobación más sólida, abra esta página en un dispositivo distinto de aquel con el que votó, por ejemplo su móvil. Un dispositivo que cambiara su papeleta también podría cambiar lo que le muestra aquí.",
                },
            },
        },
    },
}

export default spanishTranslation
