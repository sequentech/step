// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import englishTranslation, {TranslationType} from "./en"

const spanishTranslation: TranslationType = {
    translations: {
        configurationPackage: {
            tab: "Paquete de configuración",
            title: "Importar un paquete de configuración",
            description:
                "Aplica una entrega completa exportada por Election Architect: configuración y ajustes del cliente, roles, administradores, autoridades y el evento electoral con su censo. Nada cambia hasta que se aplica la vista previa.",
            tenant: "Cliente: {{tenantId}}",
            notPermitted:
                "Importar un paquete de configuración requiere tenant-write, user-write, role-write, trustee-create, trustee-write y election-event-create en este cliente.",
            requirements: "El paquete debe incluir",
            censusRequired: "El censo del evento electoral",
            sections: {
                "tenant-configuration": "Configuración del cliente",
                "tenant-settings": "Ajustes del cliente",
                "role-permissions": "Permisos de roles",
                "admin-accounts": "Cuentas de administración",
                "trustees": "Autoridades",
                "election-event": "Evento electoral",
            },
            upload: "Subir paquete",
            uploadTitle: "Subir un paquete de configuración",
            uploadSubtitle: "El zip de la entrega, desde un medio extraíble",
            uploadParagraph: "Primero se previsualiza; la subida por sí sola no aplica nada.",
            documentId: "O el id de un documento ya subido",
            preview: "Vista previa",
            apply: "Aplicar",
            blocked:
                "La vista previa encontró bloqueos: corrige el paquete o el cliente y vuelve a previsualizar.",
            noTask: "La plataforma no devolvió ninguna tarea.",
            mode: {preview: "Vista previa", apply: "Aplicación"},
            task: "Tarea {{taskId}}",
            running: "En curso…",
            toChange: "Solo vista previa, nada ha cambiado: {{count}} sección(es) cambiarían.",
            outcome: {
                running: "En curso",
                previewed: "Previsualizado: no se cambió nada",
                applied: "Aplicado: el cliente coincide con el paquete",
                refused:
                    "Rechazado antes de que esta ejecución aplicara nada; las secciones que aplicó una ejecución anterior siguen aplicadas, según su estado",
                failed: "Falló a mitad: las secciones anteriores pueden estar aplicadas, según su estado",
                unknown:
                    "Desconocido: una sección no pudo resolverse y no se envió nada más; compruébela antes de reintentar",
            },
            columns: {section: "Sección", state: "Estado", task: "Tarea", details: "Detalles"},
            reconciled: "conciliado",
            blocker: "Bloqueado",
            needsApply: "Cambiaría",
            members: "Miembros",
            history: "Aplicaciones anteriores",
            applyRechecks:
                "Aplicar vuelve a previsualizar el paquete en la plataforma antes de cambiar nada, y se detiene ante cualquier bloqueo.",
            earlierApplied: "Aplicado por una ejecución anterior",
            earlierTasks: "Tareas iniciadas para esta sección: {{tasks}}",
            inFlight: "Hay un envío sin resolver (tarea {{taskId}})",
            stoppedWithoutRecord:
                "Detenido sin registro: esta ejecución o una anterior pueden haber aplicado secciones; previsualice de nuevo para ver el estado del cliente",
            lastLog: "Último mensaje: {{message}}",
            unreadable:
                "No se puede leer la tarea ({{reason}}). Puede seguir en curso: compruebe las aplicaciones anteriores o deje de seguirla",
            notFound: "no encontrada en este cliente",
            stopFollowing: "Dejar de seguir",
            mayHaveTask:
                "Se desconoce el resultado de la petición, así que puede haberse creado una tarea: compruebe las aplicaciones anteriores antes de reintentar.",
            shaMismatch:
                "El paquete previsualizado no es el del SHA-256 introducido ({{expected}}); no se puede aplicar.",
            confirmTitle: "¿Aplicar este paquete?",
            confirmBody:
                "Esto aplica el paquete previsualizado al cliente {{tenantId}}. La plataforma lo vuelve a previsualizar primero y se detiene ante cualquier bloqueo.",
            cancel: "Cancelar",
            proposedChanges: "Lo que cambiaría al aplicar",
            shaMalformed:
                "El SHA-256 introducido no tiene 64 caracteres hexadecimales, así que el paquete no se ha previsualizado. Introduzca el resumen exacto o déjelo en blanco.",
            nothingToChange: "Nada: el cliente ya coincide con el paquete.",
        },
        ballotPrivacy: englishTranslation.translations.ballotPrivacy,
        readiness: {
            close: "Cerrar",
            action: "Verificar preparación",
            title: "Preparación electoral",
            description:
                "Compare este evento oficial con un ensayo finalizado. Si la verificación es satisfactoria, se registra evidencia firmada en el historial del evento oficial.",
            testedEvent: "Evento probado",
            archived: "Archivado",
            noEvents: "No hay otros eventos disponibles",
            retry: "Reintentar",
            eventsError: "No se pudieron cargar los eventos.",
            run: "Ejecutar verificación",
            running: "Verificando preparación…",
            error: "No se pudo completar la verificación. Revise el historial antes de reintentar; la respuesta puede haberse perdido tras registrar la evidencia.",
            downloadError: "No se pudo descargar el informe. Inténtelo de nuevo.",
            success:
                "Preparación verificada. Se registró evidencia firmada en el historial del evento oficial.",
            failed: "La preparación no es válida. Resuelva las verificaciones fallidas y repita. No se registró ninguna entrada de preparación.",
            download: "Descargar informe JSON",
            snapshot:
                "Este informe es una instantánea. Repita tras cambios relevantes; no bloquea el evento ni sustituye la aprobación operativa.",
            results: "Resultados de la verificación",
            condition: "Condición comprobada por el servidor",
            noDetail: "El servidor no dio más detalle sobre esta comprobación.",
            reasons: "Por qué falló",
            noArea: "ninguna área o un área desconocida",
            eventRoles: {
                official: "oficial",
                tested: "de prueba",
                both: "ambos",
            },
            scope: {
                event: "El evento {{event}}",
                election: "La elección {{election}}",
            },
            certification: {
                declared_sha256_missing: "no se indicó el SHA-256",
                declared_sha256_mismatch: "el SHA-256 indicado no coincide con el del archivo",
                no_counts: "no tiene votantes por área",
                blank_area: "hay votantes sin área",
                invalid_sha256: "su SHA-256 no es válido",
            },
            findings: {
                same_event: "El evento oficial y el de prueba son el mismo evento.",
                configuration_missing:
                    "El evento {{event}} no se importó desde un paquete de configuración firmado.",
                configuration_incomplete:
                    "La configuración firmada del evento oficial no tiene un valor válido para «{{field}}».",
                external_id_mismatch:
                    "Los eventos proceden de configuraciones distintas: {{official}} (oficial) y {{tested}} (de prueba).",
                content_mismatch:
                    "El contenido firmado es distinto: SHA-256 {{official}} (oficial) y {{tested}} (de prueba).",
                approvers_mismatch:
                    "Los aprobadores verificados son distintos: {{only_official}} solo en el evento oficial y {{only_tested}} solo en el de prueba.",
                areas_mismatch: "{{area}} solo está en el contenido firmado del evento {{event}}.",
                voter_import_missing:
                    "El evento {{event}} no tiene una importación certificada de votantes.",
                voter_import_in_progress:
                    "Todavía hay una importación de votantes en curso en el evento {{event}}.",
                voter_file_not_certified:
                    "La última importación de votantes del evento {{event}} no está certificada: {{reason}}.",
                voter_receipt_not_signed:
                    "El registro del evento {{event}} no tiene una entrada firmada que corresponda a la importación de votantes {{import_id}}.",
                registry_count_changed:
                    "El evento {{event}} tiene ahora {{current}} votantes en {{area}}, pero su importación certificada tiene {{imported}}.",
                voter_area_not_configured:
                    "Los votantes certificados del evento {{event}} incluyen {{area}}, que no está en su contenido firmado.",
                voter_file_mismatch:
                    "Los archivos certificados de votantes son distintos: SHA-256 {{official}} (oficial) y {{tested}} (de prueba).",
                voter_count_mismatch:
                    "Los archivos certificados difieren en {{area}}: {{official}} votantes (oficial) y {{tested}} (de prueba).",
                ballots_present: "El evento oficial tiene {{count}} papeletas almacenadas.",
                received_ballots_present:
                    "El evento oficial tiene {{count}} papeletas recibidas para revisión.",
                tally_sessions_present: "El evento oficial tiene {{count}} sesiones de recuento.",
                status_missing: "{{scope}} no tiene estado de votación.",
                voting_started: "{{scope}}: el canal {{channel}} está en {{status}}.",
                voting_dates_recorded:
                    "{{scope}}: el canal {{channel}} tiene fechas de votación registradas.",
                no_elections: "El evento {{event}} no tiene elecciones.",
                publication_missing: "El evento {{event}} no tiene ninguna publicación activa.",
                publication_mismatch:
                    "Los {{design_count}} diseños de papeleta publicados del evento {{event}} no coinciden con todos los diseños de su configuración firmada.",
                publication_empty:
                    "Los diseños de papeleta publicados del evento {{event}} no se pudieron comprobar con una configuración firmada.",
                channels_open: "El evento de prueba todavía tiene canales de votación sin cerrar.",
                seal_at_close_off: "El evento de prueba no sella las urnas al cierre.",
                no_expected_boxes: "La publicación del evento de prueba no espera ninguna urna.",
                boxes_unsealed:
                    "La elección {{election}} del evento de prueba tiene {{unsealed}} urnas sin sellar.",
            },
        },
        plugins: {
            title: "Complementos",
            description:
                "Los complementos se incluyen en esta versión. Al activar uno se añaden su elemento de menú y sus permisos para este inquilino; al desactivarlo se oculta y se conservan sus datos y permisos.",
            enabled: "Activado",
            disabled: "Desactivado",
            compatible: "Compatible",
            incompatible: "Los datos guardados requieren una versión más reciente",
            api: "API de complementos {{major}}",
            permissions: "Permisos",
            adminGroup: "se conceden al grupo admin al activarlo",
            enable: "Activar",
            disable: "Desactivar",
            confirmEnableTitle: "¿Activar {{name}}?",
            confirmDisableTitle: "¿Desactivar {{name}}?",
            confirmEnable:
                "{{name}} aparece en el menú de los usuarios con sus permisos. Sus permisos se crean en este inquilino y se conceden al grupo admin.",
            confirmDisable:
                "{{name}} se oculta y sus solicitudes se rechazan. Se conservan sus datos, historial y permisos, y no cambia nada más en el inquilino.",
            confirm: "Confirmar",
            noPermission: "Cambiar los complementos requiere el permiso tenant-write.",
            retry: "Reintentar",
            loading: "Cargando complementos…",
            signIn: "Inicia sesión en el inquilino seleccionado para usar sus complementos.",
            unavailable: "Este complemento no forma parte de esta versión.",
            incompatibleRoute:
                "Los datos guardados de este complemento requieren una versión más reciente. Sus datos no han cambiado.",
            disabledRoute:
                "{{name}} no está activado para este inquilino. Un administrador del inquilino puede activarlo en Ajustes → Complementos.",
            forbidden: "No tienes permiso para usar {{name}}.",
        },
        projects: {
            title: "Proyectos",
            subtitle:
                "Planifica el trabajo del inquilino en una línea de tiempo. Los proyectos nunca cambian los calendarios electorales.",
            cancel: "Cancelar",
            retry: "Reintentar",
            loading: "Cargando…",
            list: {
                create: "Nuevo proyecto",
                import: "Importar",
                emptyTitle: "Aún no hay proyectos",
                emptyHelp: "Crea un proyecto o importa un archivo CSV exportado de otro proyecto.",
                emptyViewer: "No se ha creado ningún proyecto en este inquilino.",
                active_one: "Activos ({{count}})",
                active_other: "Activos ({{count}})",
                archived_one: "Archivados ({{count}})",
                archived_other: "Archivados ({{count}})",
                name: "Nombre",
                dates: "Fechas",
                tasks: "Tareas",
                timezone: "Zona horaria",
                updated: "Último cambio",
                updatedBy: "{{at}} · {{actor}}",
                archivedChip: "Archivado",
                noneInTab: "No hay proyectos aquí.",
            },
            create: {
                title: "Nuevo proyecto",
                name: "Nombre del proyecto",
                timezone: "Zona horaria",
                timezoneHelp:
                    "Todas las fechas del proyecto se introducen y muestran en esta zona horaria IANA.",
                submit: "Crear proyecto",
            },
            import: {
                title: "Importar un proyecto",
                help: "Importar siempre crea un proyecto nuevo. Usa un archivo CSV en UTF-8 con las columnas {{columns}}, tal como se exporta de un proyecto, con un desfase UTC en cada fecha. No se guarda nada hasta que creas el proyecto.",
                choose: "Elegir archivo CSV",
                noFile: "Ningún archivo elegido",
                fileLimit: "El archivo debe ocupar como máximo 1 MiB.",
                invalidFile: "El archivo no es texto UTF-8 válido.",
                valid_one: "{{count}} tarea está lista para importar.",
                valid_other: "{{count}} tareas están listas para importar.",
                invalid_one:
                    "{{count}} de {{total}} filas tiene errores. Corrige el archivo y vuelve a elegirlo.",
                invalid_other:
                    "{{count}} de {{total}} filas tienen errores. Corrige el archivo y vuelve a elegirlo.",
                previewTable: "Vista previa de la importación",
                line: "Línea",
                result: "Comprobación",
                rowValid: "Lista",
                commit: "Crear proyecto",
            },
            detail: {
                timezone: "Todas las fechas y horas están en {{timezone}}.",
                export: "Exportar",
                archive: "Archivar",
                restore: "Restaurar",
                addTask: "Añadir tarea",
                viewOnly: "Solo lectura. Cambiar tareas requiere el permiso projects-write.",
                archivedNotice: "Este proyecto está archivado. Restáuralo para cambiar sus tareas.",
            },
            notices: {
                saved: "Se ha guardado «{{title}}».",
                moved: "Se ha guardado «{{title}}»: {{start}} – {{end}}.",
                deleted: "Se ha eliminado «{{title}}».",
                archived: "Proyecto archivado.",
                restored: "Proyecto restaurado.",
            },
            panel: {
                details: "Detalles de la tarea",
                edit: "Editar tarea",
                add: "Añadir tarea",
                close: "Cerrar los detalles de la tarea",
                hint: "Selecciona una tarea en la línea de tiempo para ver aquí sus detalles.",
                noTasks: "Este proyecto aún no tiene tareas.",
            },
            conflict: {
                body: "Otra persona ha cambiado esta tarea ({{actor}}, {{at}}). Tus cambios siguen aquí. Revisa su versión antes de volver a guardar.",
                load: "Cargar su versión",
            },
            unsaved: {
                body: "Tienes cambios sin guardar. Guárdalos o descártalos para {{action}}.",
                keep: "Seguir editando",
                discard: "Descartar cambios",
                actions: {
                    open: "abrir «{{title}}»",
                    add: "añadir una tarea nueva",
                    leave: "volver a los proyectos",
                    close: "cerrar la tarea",
                    state: "cambiar el proyecto",
                },
            },
            deleteDialog: {
                title: "¿Eliminar la tarea?",
                body: "«{{title}}» se elimina del proyecto. Su historial de cambios se conserva.",
                confirm: "Eliminar",
            },
            history: {
                task: "Historial de cambios",
                project: "Historial del proyecto",
                empty: "Aún no hay cambios registrados.",
                more: "Mostrar cambios anteriores",
                by: "{{actor}} · {{at}}",
                actions: {
                    create: "Creado {{subject}}",
                    update: "Cambiado {{subject}}",
                    delete: "Eliminado {{subject}}",
                    import_one: "Importada {{count}} tarea",
                    import_other: "Importadas {{count}} tareas",
                    export: "Exportado como CSV",
                    other: "{{action}}",
                },
            },
            editor: {
                title: "Título",
                type: "Tipo",
                status: "Estado",
                start: "Inicio",
                end: "Fin",
                at: "Fecha y hora",
                notes: "Notas",
                order: "Orden",
                orderHelp: "Las tareas se ordenan por orden y después por inicio.",
                timezoneNote: "Las horas están en {{timezone}}.",
                offset: "UTC{{offset}}",
                localTime: {
                    invalid: "Introduce una fecha y hora entre 1900 y 2200.",
                    skipped:
                        "Esta hora no existe en {{timezone}}: los relojes se la saltan. Elige otra hora.",
                    repeated:
                        "Esta hora ocurre dos veces en {{timezone}}. Elige a cuál te refieres.",
                },
                chooseInstant: "Qué vez",
                instantChoice: "En UTC{{offset}}",
                saving: "Guardando…",
                toFix_one: "{{count}} campo por corregir",
                toFix_other: "{{count}} campos por corregir",
                save: "Guardar",
                delete: "Eliminar",
            },
            validation: {
                title: "Introduce un título de como máximo 200 caracteres.",
                offset: "Introduce fechas válidas.",
                range: "El fin debe ser posterior al inicio.",
                milestone: "Un hito no tiene fin.",
                notes: "Las notas deben tener como máximo 16.000 caracteres.",
                order: "El orden debe ser un número entero de 0 a 1.000.000.",
            },
            status: {
                not_started: "Sin empezar",
                in_progress: "En curso",
                done: "Hecha",
            },
            type: {
                task: "Tarea",
                milestone: "Hito",
            },
            units: {
                days_one: "{{count}} día",
                days_other: "{{count}} días",
                hours_one: "{{count}} h",
                hours_other: "{{count}} h",
                minutes_one: "{{count}} min",
                minutes_other: "{{count}} min",
            },
            timeline: {
                title: "Línea de tiempo",
                tasks_one: "{{count}} tarea",
                tasks_other: "{{count}} tareas",
                empty: "Aún no hay tareas.",
                editHelp:
                    "Señala o enfoca una tarea para ver un resumen; selecciónala para editarla abajo. Arrastra una barra o usa ← → para moverla 24 horas; añade Mayús para cambiar el fin.",
                viewHelp:
                    "Señala o enfoca una tarea para ver un resumen. Selecciónala para ver sus detalles abajo.",
                touchEditHelp:
                    "Toca una tarea para editarla abajo, o arrastra una barra para moverla días completos.",
                touchViewHelp: "Toca una tarea para ver sus detalles abajo.",
                taskLabel: "{{title}}, de {{start}} a {{end}}, {{status}}",
                milestoneLabel: "{{title}}, hito el {{at}}, {{status}}",
                previewEdit: "Haz clic o pulsa Intro para editar",
                previewView: "Haz clic o pulsa Intro para ver los detalles",
            },
        },
        philippinePassport: "Pasaporte filipino",
        seamanBook: "Libro de marinero",
        philSysID: "Identificación PhilSys",
        iBP: "Colegio de Abogados Integrado de Filipinas (IBP)",
        driversLicense: "Licencia de conducir",
        loading: "Cargando...",
        loadingDataProvider: "Cargando proveedor de datos...",
        tallySheetImport: {
            title: "Importaciones de actas de escrutinio",
            subtitle:
                "Importa archivos de actas de escrutinio ES&S o CSV, previsualiza las urnas generadas y apruébalas antes de crear actas de escrutinio.",
            createTitle: "Importar actas de escrutinio",
            detailTitle: "Importación de acta de escrutinio",
            empty: "Todavía no hay importaciones de actas de escrutinio.",
            emptyBody:
                "Empieza importando un archivo ES&S Enhanced XML o CSV canónico para este evento electoral.",
            sourceFormat: {
                ESS_ENHANCED_XML: "ES&S Enhanced XML",
                CANONICAL_CSV: "CSV canónico",
            },
            channel: {
                PAPER: "Papel",
                POSTAL: "Postal",
                IN_PERSON: "Presencial",
            },
            table: {
                created: "Creado",
                createdBy: "Creado por",
                file: "Archivo",
                format: "Formato",
                channel: "Canal",
                status: "Estado",
                labels: "Etiquetas",
                annotations: "Anotaciones",
                actions: "Acciones",
            },
            summary: {
                imported: "Importadas",
                changed: "Modificadas",
                new: "Nuevas",
                unchanged: "Sin cambios",
                conflicted: "En conflicto",
                errors: "Errores",
            },
            status: {
                PENDING_REVIEW: "Pendiente de revisión",
                APPROVED: "Aprobada",
                DISAPPROVED: "Desaprobada",
                FAILED_VALIDATION: "Validación fallida",
                CONFLICTED: "En conflicto",
                NEW: "Nueva",
                CHANGED: "Modificada",
                UNCHANGED: "Sin cambios",
            },
            fields: {
                format: "Formato",
                channel: "Canal",
                supportedFormats: "Formatos admitidos: XML, CSV",
                generatedTallySheet: "Acta de escrutinio generada",
                sourceCandidates: "IDs de candidatos de origen",
                none: "Ninguno",
            },
            actions: {
                create: "Importar actas de escrutinio",
                review: "Revisar",
                source: "Origen",
                cancel: "Cancelar",
                preview: "Previsualizar",
                save: "Guardar importación",
                approve: "Aprobar",
                disapprove: "Desaprobar",
                close: "Cerrar",
                openExisting: "Abrir existente",
            },
            notifications: {
                selectFile: "Selecciona un archivo de importación antes de previsualizarlo",
                duplicateSource:
                    "Este hash de archivo de origen ya aparece en una importación de acta de escrutinio anterior.",
                uploadUrlError: "No se pudo crear la URL de subida",
                uploadError: "No se pudo subir el archivo de importación",
                previewEmpty: "La respuesta de la previsualización estaba vacía",
                previewError: "No se pudo previsualizar la importación",
                importEmpty: "La respuesta de la importación estaba vacía",
                created: "Se ha creado la importación del acta de escrutinio",
                createError: "No se pudo crear la importación",
                reviewEmpty: "La respuesta de la revisión estaba vacía",
                conflicted: "La importación tiene conflictos de línea base desactualizados",
                approved: "Se ha aprobado la importación",
                disapproved: "Se ha desaprobado la importación",
                reviewError: "No se pudo revisar la importación",
                sourceUrlError: "No se pudo crear la URL de descarga de origen",
                sourceDownloadError: "No se pudo descargar el archivo de origen",
            },
            pagination: {
                range: "{{rangeStart}}-{{rangeEnd}} de {{total}}",
                previous: "Anterior",
                next: "Siguiente",
            },
        },
        reconciliation: {
            menuButton: "Sincr. votantes ext.",
            categories: {
                VOTED_INTERNET: "Votó por Internet",
                VOTED_OTHER_CHANNEL: "Votó por otro canal",
                DISABLED_DELETE_CALL: "Votante deshabilitado",
                DELETION_REVERTED: "Eliminación revertida",
                PROFILE_UPDATE: "Perfil actualizado",
                VOTER_ADDED: "Votante añadido",
                REENABLED: "Votante rehabilitado",
                VOTED_UNMARKED: "Votante desmarcado como votante",
                ROW_FAILURE: "Fallo de fila",
            },
            table: {
                voterId: "ID de Votante",
                field: "Campo",
                category: "Categoría",
                currentValue: "Valor actual",
                newValue: "Valor nuevo",
                reason: "Motivo",
                rowLabel: "Fila",
                noDifferences: "No se encontraron diferencias - los sistemas están sincronizados.",
            },
            wizard: {
                title: "Sincronización de reconciliación externa",
                subtitle: "Sincroniza la lista de votantes con el sistema externo",
                drop: {
                    description:
                        "Suelta el archivo de reconciliación generado por el sistema externo - ambos diffs (lado externo y lado Sequent) se calculan automáticamente y se muestran en tablas separadas.",
                    fileFormatLabel: "Archivo CSV",
                    uploading: "Subiendo {{fileName}} y calculando ambos diffs...",
                },
                review: {
                    fileSummary: "{{fileName}} - Secuencia {{sequence}}, generado {{generatedAt}}",
                    rowFailuresWarning:
                        "{{count}} fila(s) no se pudieron reconciliar de forma segura y quedan excluidas de ambos diffs - ver detalles abajo.",
                    noDifferences: "No hay diferencias - ambos sistemas ya están sincronizados.",
                    diffOnlyDifferences:
                        "Esta es una comprobación de convergencia de una secuencia ya aplicada. Las diferencias se muestran para su seguimiento, pero esta ronda no se puede volver a aplicar.",
                    externalDiffTitle: "Diff externo",
                    sequentDiffTitle: "Diff de Sequent",
                    downloadExternalPatch: "Descargar parche externo",
                    externalDiffCaption:
                        "Descarga el parche y entrégaselo al sistema externo fuera de esta herramienta. Cuando lo aplique y genere el siguiente archivo de reconciliación, haz clic en 'Atrás' y suelta ese archivo - 'Aplicar' se habilita cuando esta tabla esté vacía.",
                    noExternalDifferences: "No hay diferencias del lado externo.",
                    sequentDiffCaption:
                        "Aplica los cambios directamente a Sequent haciendo clic en 'Aplicar' - no se genera archivo de parche para estos.",
                    noSequentDifferences: "No hay diferencias del lado de Sequent.",
                },
                applying: {
                    inProgress: "Aplicando cambios del lado de Sequent...",
                    rowFailures:
                        "{{count}} fila(s) quedaron excluidas de esta ronda y requieren seguimiento manual - ver detalles abajo.",
                    rowFailuresTruncated:
                        "Se muestran los primeros {{shown}} de {{count}} fallos de fila. Resuelve la causa común y vuelve a intentarlo para ver los fallos restantes.",
                    success: "Todos los cambios del lado de Sequent se aplicaron correctamente.",
                },
                actions: {
                    cancel: "Cancelar",
                    back: "Atrás",
                    apply: "Aplicar",
                    next: "Siguiente",
                    startOver: "Empezar de nuevo",
                    close: "Cerrar",
                },
                confirm: {
                    title: "Confirmar cambios de reconciliación",
                    categoriesNote:
                        "Las categorías resaltadas en naranja ({{categories}}) afectan el estado de voto o deshabilitan votantes.",
                    applyChanges: "Aplicar cambios",
                    continue: "Continuar",
                },
                summary: {
                    votedOtherChannel: "marca {{count}} votante(s) como votado(s) por otro canal",
                    disabled: "deshabilita {{count}} votante(s)",
                    reenabled: "rehabilita {{count}} votante(s)",
                    votedUnmarked: "desmarca {{count}} votante(s) como votante",
                    profileUpdated: "actualiza {{count}} perfil(es)",
                    voterAdded: "añade {{count}} votante(s)",
                    prefix: "Esto aplicará cambios que {{parts}}.",
                    empty: "No hay cambios del lado de Sequent para aplicar.",
                },
                notifications: {
                    envelopeLoadError:
                        "No se pudo cargar el diff de reconciliación - inténtalo de nuevo.",
                    generateFailed:
                        "No se pudo calcular el diff de reconciliación - consulta el widget de tareas para más detalles.",
                    applyFailed:
                        "No se pudieron aplicar los cambios del lado de Sequent - consulta el widget de tareas para más detalles.",
                    uploadUrlError: "No se pudo obtener una URL de subida",
                    generateTaskError: "No se pudo iniciar la tarea de diff de reconciliación",
                    uploadError: "No se pudo subir el archivo de reconciliación",
                    applyTaskError: "No se pudo iniciar la tarea de aplicación",
                    applyError: "No se pudieron aplicar los cambios de reconciliación",
                },
            },
        },
        tenantLogs: {
            explanation: {
                action: "Acción: {{action}}",
                outcome: {
                    succeeded: "Aceptada (HTTP {{status}})",
                    denied: "Denegada (HTTP {{status}})",
                    failed: "Fallida (HTTP {{status}})",
                },
                by: "Pedida por {{username}} ({{userId}})",
                detail: "{{name}}: {{value}}",
                board: "Bitácora: {{board}}",
                publicKey: "Clave pública de la bitácora: {{key}}",
            },
        },
        logsScreen: {
            configurationPackage: englishTranslation.translations.logsScreen.configurationPackage,
            readiness: {
                passed: "Superado",
                failed: "Fallido",
                checkResult: "{{outcome}}: {{description}}",
                checkResultWithMessage:
                    "{{outcome}}: {{description}} Condición registrada: «{{message}}»",
                description: {
                    VotersImported: "Archivo de votantes importado.",
                    ReadinessVerified:
                        "Preparación del evento oficial verificada frente al evento probado.",
                },
                checks: {
                    configuration:
                        "Ambos eventos tienen el mismo contenido aprobado, aprobadores verificados y áreas.",
                    certified_voters:
                        "Ambos eventos coinciden en el archivo certificado de votantes y los recuentos actuales por área.",
                    official_zero:
                        "El evento oficial nunca ha abierto la votación y no tiene votos ni sesiones de recuento.",
                    publication: "La publicación oficial coincide con su configuración firmada.",
                    tested_finished:
                        "Los canales de votación del evento probado están cerrados y todas las urnas previstas están selladas.",
                },
                fields: {
                    report: "Informe de preparación",
                    passed: "Superado",
                    checked_at: "Fecha de comprobación",
                    evidence: "Evidencia",
                    official: "Evento oficial",
                    tested: "Evento probado",
                    official_event_id: "ID del evento oficial",
                    tested_event_id: "ID del evento probado",
                    event_id: "ID del evento",
                    election_id: "ID de elección",
                    configuration: "Configuración",
                    external_id: "ID externo",
                    revision: "Revisión",
                    content_sha256: "SHA-256 del contenido",
                    manifest_sha256: "SHA-256 del manifiesto",
                    approvers: "Aprobadores verificados",
                    areas: "Áreas",
                    voters: "Importación de votantes",
                    import_id: "ID de importación",
                    document_id: "ID del documento",
                    file_name: "Nombre del archivo",
                    sha256: "SHA-256 del archivo",
                    declared_sha256: "SHA-256 declarado",
                    counts_by_area: "Votantes por área",
                    imported_at: "Fecha de importación",
                    voter_log_delivered: "Importación firmada registrada",
                    current_voter_counts: "Votantes actuales por área",
                    voter_import_in_progress: "Importación de votantes activa",
                    status: "Estado",
                    elections: "Elecciones",
                    ballot_count: "Votos almacenados",
                    received_ballot_count: "Votos recibidos en revisión",
                    tally_session_count: "Sesiones de recuento",
                    publication: "Publicación",
                    publication_id: "ID de publicación",
                    publication_ids: "ID de las publicaciones",
                    matches_manifest: "Coincide con el manifiesto firmado",
                    design_count: "Diseños publicados",
                    seal_at_close: "Sellar al cierre",
                    expected_boxes: "Urnas previstas",
                    sealed_boxes: "Urnas selladas",
                    all_channels_closed: "Todos los canales cerrados",
                    locked_down: "Bloqueado",
                    log_entry_id: "ID de entrada del registro",
                },
            },
            noPermissions: "No tienes permiso para acceder a las bitácoras.",
            title: "Bitácoras",
            subtitle:
                "Todo lo que se hace en este evento electoral. Los inicios de sesión y los cambios fuera de él están en Configuración > Bitácoras.",
            empty: "No se encontraron entradas.",
            tenant: {
                subtitle:
                    "Inicios de sesión, cuentas del personal y roles, configuración y eventos electorales de esta organización. Cada evento electoral guarda su propia bitácora en su pestaña Bitácoras.",
                noPermissions: "No tienes permiso para ver esta bitácora.",
            },
            entry: {
                title: "Entrada de bitácora {{id}}",
                statementKind: "Tipo de declaración",
                eventType: "Tipo de evento",
                logType: "Tipo de registro",
                when: "Cuándo",
                utc: "{{time}} UTC",
                who: "Quién",
                userId: "ID de usuario {{id}}",
                role: "Rol {{role}}",
                unknownAccount: "Cuenta desconocida (no se registra el nombre escrito)",
                madeIn: "Hecho en",
                changed: "Cambiado",
                objectId: "ID {{id}}",
                actionLabel: "Acción",
                details: "Detalles",
                origin: {
                    "hasura": "Portal de administración (a través de Hasura)",
                    "harvest": "Portal de administración (a través de Harvest)",
                    "windmill": "Portal de administración (una tarea de Windmill)",
                    "keycloak-console":
                        "Consola de administración de Keycloak, fuera del portal de administración",
                    "service-account":
                        "Cuenta de servicio de Step, por un cambio hecho en el portal de administración",
                },
                action: {
                    create: "Creado",
                    update: "Actualizado",
                    delete: "Eliminado",
                    import: "Importado",
                    export: "Exportado",
                    archive: "Archivado",
                    unarchive: "Desarchivado",
                    enable: "Activado",
                    disable: "Desactivado",
                    reset_password: "Contraseña restablecida",
                    grant: "Concedido",
                    revoke: "Revocado",
                },
                objectType: {
                    tenant: "Configuración del inquilino",
                    tenant_log: "Bitácora del inquilino",
                    staff_account: "Cuenta del personal",
                    role: "Rol",
                    template: "Plantilla",
                    trustee: "Fideicomisario",
                    election_type: "Tipo de elección",
                    election_event: "Evento electoral",
                    election: "Elección",
                    contest: "Contienda",
                    candidate: "Candidato",
                    area: "Área",
                },
                values: {
                    title: "Valores anteriores y nuevos",
                    field: "Campo",
                    old: "Valor anterior",
                    new: "Valor nuevo",
                    none: "(ninguno)",
                    secret: "Cambiado. Los valores secretos nunca se registran.",
                    yes: "Sí",
                    no: "No",
                    sha: "SHA-256 {{hash}}",
                },
                field: {
                    tenant: {
                        slug: "Nombre corto",
                        isActive: "Activo",
                        logsTimezone: "Zona horaria de la bitácora del inquilino (Idiomas)",
                        languages: "Idiomas",
                        enabledLanguages: "Idiomas habilitados (Idiomas)",
                        defaultLanguage: "Idioma predeterminado (Idiomas)",
                        languageDetection: "Detección del idioma (Idiomas)",
                        translations: "Traducciones (Localización)",
                        sms: "SMS",
                        mail: "Correo electrónico",
                        gapiKey: "Clave de la API de Google (Integraciones)",
                        gapiEmail: "Correo de la API de Google (Integraciones)",
                        helpLinks: "Enlaces de ayuda (Apariencia)",
                        displayName: "Nombre de la organización (Apariencia)",
                        votingCountries: "Países bloqueados para votar (Países)",
                        enrollCountries: "Países bloqueados para inscribirse (Países)",
                        logoUrl: "Logotipo (Apariencia)",
                        css: "CSS personalizado (Apariencia)",
                        votingChannels: "Canales de votación",
                        onlineVoting: "Voto en línea (Canales de votación)",
                        kioskVoting: "Voto en quiosco (Canales de votación)",
                        telephoneVoting: "Voto telefónico (Canales de votación)",
                        earlyVoting: "Voto anticipado (Canales de votación)",
                        paperVoting: "Voto en papel (Canales de votación)",
                    },
                },
                signIn: {
                    title: "Inicio de sesión y seguridad de la cuenta",
                    event: "Evento de Keycloak",
                    error: "Error",
                    authentication: "Autenticación",
                    credential: "Credencial",
                    failures: "Fallos seguidos",
                    client: "Cliente",
                    ipAddress: "Dirección IP",
                },
                keycloak: {
                    title: "Cambio en Keycloak",
                    account: "Cuenta",
                    operation: "Operación",
                    appliesTo: "Se aplica a",
                    resource: "Recurso",
                    newValue: "Valor nuevo",
                    error: "Error",
                },
                importedFile: {
                    title: "El archivo importado",
                    declared: "Suma de comprobación declarada",
                    sha256: "SHA-256 del archivo",
                    declaredSha256: "SHA-256 declarado",
                    document: "Documento",
                    checksum: {
                        undeclared: "No se declaró",
                        matches: "Coincide con el archivo",
                        mismatch: "No coincide con el archivo",
                    },
                    counts: {
                        count: "Filas",
                        elections: "Elecciones",
                        contests: "Contiendas",
                        candidates: "Candidatos",
                        areas: "Áreas",
                        area_contests: "Áreas por contienda",
                        sections: "Secciones de configuración",
                    },
                },
                deletedEvent: {
                    title: "La bitácora del evento electoral eliminado",
                    lastEntry: "Última entrada",
                    stateHash: "Hash de estado",
                    logDatabase: "Base de datos de la bitácora",
                    status: "Estado",
                    retired:
                        "Descargada y conservada hasta que se borre según la política de conservación",
                    task: "Tarea",
                    systemKey: "La clave de sistema de la bitácora",
                },
                exported: {
                    title: "Exportación",
                    entries: "Entradas",
                    sha256: "SHA-256 del archivo",
                    document: "Documento",
                    scope: {
                        TENANT_LOG: "Toda la bitácora del inquilino",
                        ACCOUNT: "Una cuenta",
                        ACCOUNT_WITH_EVENTS:
                            "Una cuenta, con sus entradas en cada evento electoral",
                        ALL: "Todo el evento electoral",
                        ELECTION: "Una elección",
                        AREA: "Un área",
                    },
                },
                writer: {
                    title: "Escritor de la bitácora",
                    release: "Versión",
                    lastEntry: "Última entrada publicada",
                },
                signatures: {
                    title: "Bitácora y firmas",
                    log: "Bitácora",
                    tenantLog: "Bitácora del inquilino",
                    eventLog: "Bitácora del evento electoral",
                    signedBy: "Firmado por",
                    adminKey: "Clave de administración de {{name}}",
                    systemOnly: "Solo la clave de sistema de la bitácora",
                    systemSignature: "Firma del sistema",
                    publicKey: "Clave de administración publicada",
                },
            },
            column: {
                id: "ID",
                statement_kind: "Tipo de declaración",
                created: "Creado",
                statement_timestamp: "Marca de tiempo de declaración",
                message: "Mensaje",
                user_id: "ID de usuario",
                username: "Nombre de Usuario",
                sender_pk: "Clave primaria del remitente",
                log_type: "Tipo de registro",
                event_type: "Tipo de evento",
                description: "Descripción",
                version: "Versión",
            },
            auditDialog: {
                title: "Auditar el registro electoral",
                confirm: "Auditar",
                description:
                    "La auditoría comprueba cada entrada del registro con su registro Merkle y verifica los puntos de control publicados. El resultado se guarda como una tarea.",
            },
            exportdialog: {
                description:
                    "Por favor, confirma que deseas ejecutar esta acción; puede tardar un tiempo en completarse.",
                title: "Exportar registros",
                from: "Desde",
                to: "Hasta",
                timeZone: "Zona horaria",
                format: "Formato",
                csv: "CSV",
                pdf: "PDF",
                sql: "SQL",
                scope: {
                    label: "Alcance",
                    tenant: {
                        TENANT_LOG: "Toda la bitácora del inquilino",
                        ACCOUNT: "Una cuenta",
                    },
                    event: {
                        ALL: "Todo el evento electoral",
                        ELECTION: "Una elección",
                        AREA: "Un área",
                        ACCOUNT: "Una cuenta",
                    },
                },
                username: "Nombre de usuario",
                election: "Elección",
                area: "Área",
                withElectionEvents: "Incluir las entradas de esta cuenta en cada evento electoral",
                rowsNameLog: "Las filas de los eventos electorales indican su bitácora.",
                recorded: "La propia exportación queda registrada en esta bitácora.",
                zoneNote:
                    "Cada fila conserva su hora en UTC (ISO 8601) y en {{abbr}}, con el nombre de la zona horaria. El intervalo de fechas incluye ambos extremos, en {{abbr}}.",
                zoneNotePdf:
                    "El PDF muestra cada hora en {{abbr}}. El intervalo de fechas incluye ambos extremos, en {{abbr}}.",
                rowZones: "La zona horaria de la elección de cada fila",
                zoneNoteRows:
                    "Cada fila conserva su hora en UTC (ISO 8601) y en la zona horaria de su elección, con el nombre de la zona horaria. El intervalo de fechas incluye ambos extremos, en {{abbr}}.",
                zoneNoteRowsPdf:
                    "El PDF muestra cada hora en la zona horaria de su elección. El intervalo de fechas incluye ambos extremos, en {{abbr}}.",
            },
            filter: {
                createdFrom: "Creado desde",
                createdTo: "hasta",
                statementTimestampFrom: "Marca de tiempo de la declaración desde",
                statementTimestampTo: "Marca de tiempo de la declaración hasta",
                timeZone: "Zona horaria",
            },
            scheduledOutcome: {
                outcome: {
                    "waiting-for-initialization": "Esperando la inicialización",
                    "runs": "se ejecuta",
                    "runs-unsigned": "se ejecuta sin firmas",
                    "refused": "se rechaza",
                },
                check: {
                    "initialization": "La inicialización requerida está incompleta",
                    "voting-close": "La votación no puede abrirse después del plazo de cierre",
                    "needs-signatures": "firmas necesarias",
                    "covered": "en la configuración firmada",
                    "unsigned-close": "cierre sin firmas",
                    "stricter-copy": "configuración actual o publicada",
                    "defaults": "aún no hay nada publicado",
                },
                changed: "Ahora {{after}} (antes: {{before}}).",
                result: "Resultado: {{outcome}}.",
                deciding: "Comprobación decisiva: {{check}}. {{value}}",
                authorizedBy: "Autorizado por la configuración {{code}}.",
                nextStep: "Siguiente paso: {{step}}",
                reason: {
                    "ballot-box-seal-policy":
                        "Se mantiene cerrada: con Sellar al cierre, la votación que se ha cerrado sigue cerrada.",
                    "never-opened-kept-open":
                        "No hay nada que cerrar según la programación: el Puesto nunca se abrió, así que se queda como está.",
                },
            },
            actions: {
                csv: "Exportar en CSV",
                pdf: "Exportar en PDF",
                audit: "Auditar",
            },
            main: {
                title: "Bitácora de Base de Datos Principal",
            },
            iam: {
                title: "Bitácora de Base de Datos de IAM",
            },
            ballotBoxSeal: {
                sealHash: "Hash del sello: {{hash}}",
                counted: "{{counted}} de {{inBox}} papeletas contadas.",
                notCounted_one:
                    "La otra papeleta fue sustituida por una papeleta posterior del votante, descartada o emitida por un votante no elegible.",
                notCounted_other:
                    "Las otras {{count}} papeletas fueron sustituidas por una papeleta posterior del votante, descartadas o emitidas por un votante no elegible.",
                closeRequest: "Cerrada por la solicitud de Cerrar la votación {{request}}.",
                noCloseRequest:
                    "Cerrada sin una solicitud de Cerrar la votación (Detener votación o el cierre programado).",
                failedReason: "Motivo: {{reason}}",
                failedLocked: "La urna sigue bloqueada y no está sellada: es una incidencia.",
                verifiedCounted: "{{counted}} papeletas contadas a partir del sello.",
                tallySession: "Sesión de recuento {{session}}.",
                differs: "Diferencias: {{differs}}",
            },
        },
        tasksScreen: {
            noPermissions: "No tienes permiso para acceder a los registros.",
            title: "Ejecución de tareas",
            subtitle: "Información sobre las tareas ejecutadas",
            taskInformation: "Información de la tarea",
            status: "estado: {{status}}",
            ok: "Aceptar",
            column: {
                id: "Índice",
                name: "Nombre de la tarea",
                type: "Tipo",
                execution_status: "Estado",
                start_at: "Hora de inicio",
                end_at: "Hora de finalización",
                executed_by_user: "Ejecutor",
                annotations: "Anotaciones",
                labels: "Etiquetas",
                logs: "Registros",
            },
            tasksExecution: {
                DELETE_TENANT: "Eliminar organización",
                PUBLISH_BALLOT: "Publicar papeleta",
                VOTER_INFORMATION_LETTER: "Carta de información para el votante",
                EXPORT_MONITORING_DATA: "Exportar Datos de Monitorización",
                AUDIT_ELECTORAL_LOG: "Auditar registro electoral",
                EXPORT_ELECTION_EVENT: "Exportar evento electoral",
                CREATE_ELECTION_EVENT: "Crear Evento Electoral",
                IMPORT_ELECTION_EVENT: "Importar evento electoral",
                IMPORT_USERS: "Importar usuarios",
                EDIT_USER: "Editar votante",
                IMPORT_CANDIDATES: "Importar candidatos",
                EXPORT_VOTERS: "Exportar votantes",
                CREATE_TRANSMISSION_PACKAGE: "Crear paquete de transmisión",
                EXPORT_BALLOT_PUBLICATION: "Exportar Publicación de Boleta",
                EXPORT_ACTIVITY_LOGS_REPORT: "Exportar Informe de Registros de Actividad",
                EXPORT_TENANT_LOGS: "Exportar Bitácora del Inquilino",
                GENERATE_REPORT: "Generar Reporte",
                GENERATE_TRANSMISSION_REPORT: "Generar Informe de Transmisión",
                EXPORT_TRUSTEES: "Exportar Autoridades",
                EXPORT_APPLICATION: "Exportar Solicitudes",
                EXPORT_TENANT_CONFIG: "Exportar Configuración del Cliente",
                IMPORT_TENANT_CONFIG: "Importar Configuración del Cliente",
                APPLY_CONFIGURATION_PACKAGE: "Aplicar Paquete de Configuración",
                RENDER_DOCUMENT_PDF: "Renderizar el documento PDF",
                CREATE_TENANT: "Crear Inquilino",
                EXPORT_TEMPLATES: "Exportar Plantillas",
                IMPORT_TEMPLATES: "Importar Plantillas",
                DELETE_ELECTION_EVENT: "Eliminar evento electoral",
                DELETE_VOTERS: "Eliminar votantes",
                PREPARE_PUBLICATION_PREVIEW: "Preparar la vista previa de la publicación",
                EXPORT_TALLY_RESULTS_XLSX: "Exportar los resultados del escrutinio en formato XLSX",
                EXPORT_CERTIFICATE_AUTHORITIES: "Exportar autoridades de certificación",
                PUBLISH_RESULTS_WEBSITE: "Publicar sitio web de resultados",
            },
            documentAccess: {
                title: "Acceso al documento",
                sensitivityNotice:
                    "Información confidencial. Comparta esta contraseña únicamente con el destinatario previsto.",
                passwordLabel: "Contraseña para abrir el PDF cifrado",
                showPassword: "Mostrar contraseña",
                copyPassword: "Copiar contraseña",
                passwordCopied: "Contraseña copiada",
                passwordError: "No se pudo recuperar la contraseña del PDF",
                copyError: "No se pudo copiar la contraseña",
                guidance:
                    "La contraseña se carga únicamente después de elegir Mostrar contraseña. Una vez cargada, aparecerá aquí un campo de solo lectura con la opción de copiar.",
            },
            widget: {
                taskTitle: "Tarea: {{title}}",
                viewTask: "Ver Tarea",
                downloadDocument: "Descargar Archivo",
                downloadHashManifest: "Manifiesto de hashes",
            },
            exportTasksExecution: {
                success: "La exportación se completó con éxito",
                error: "Error al exportar la Ejecución de Tareas",
            },
        },
        areas: {
            common: {
                title: "Áreas",
                subTitle: "Configuración de Área.",
                deleteError: "Error borrando Área",
            },
            createAreaSuccess: "Área creada",
            updateAreaSuccess: "Área actualizada",
            createAreaError: "Error creando área",
            sequent_backend_area_contest: "Preguntas del Área",
            empty: {
                header: "No hay Áreas aún.",
                action: "Crear un Área",
            },
            formImputs: {
                allowEarlyVoting: "Permitir Votación Temprana",
            },
        },
        integrationsScreen: {
            common: {
                gapiKey: "Clave de Cuenta de Servicio de Google Calendar",
                gapiEmail: "Correo de Autenticación de Google Calendar",
                gapiKeyHelper:
                    "La clave guardada no se muestra. Pegue una clave nueva para reemplazarla.",
                gapiKeySaved: "Clave de Cuenta de Servicio de Google Calendar guardada",
            },
            errors: {
                invalidGapiKey:
                    "Formato de Clave de Cuenta de Servicio de Google Calendar inválido",
                saveGapiKey: "No se pudo guardar la Clave de Cuenta de Servicio de Google Calendar",
            },
        },
        lookAndFeelScreen: {
            common: {
                helpLinks: "Enlaces de Ayuda",
                logoUrl: "URL del logotipo",
                css: "CSS Personalizado",
                displayName: "Nombre visible",
                displayNameHelp:
                    "El nombre de la organización en los mensajes que la mencionan. Vacío: el nombre corto del inquilino.",
            },
            errors: {
                invalidHelpLinks: "Formato de Enlaces de Ayuda inválido",
            },
        },
        electionTypeScreen: {
            noPermissions: "No tienes permiso para acceder a la configuración.",
            common: {
                title: "Tipo de Elección",
                subtitle: "Configuración del Tipo de Elección",
                onlineVoting: "Votación en Línea",
                kioskVoting: "Votación en Quiosco",
                telephoneVoting: "Votación Telefónica",
                settingTitle: "Configuración",
                settingSubtitle: "Ajustes generales",
                createNew: "Crear un Tipo de Elección",
                emptyHeader: "No hay Tipos de Elección aún.",
                emptyBody: "¿Quieres crear uno?",
            },
            create: {
                title: "Crear Tipo de Elección",
            },
            edit: {
                title: "Editar Tipo de Elección",
            },
            tabs: {
                votingChannels: "CANALES DE VOTACIÓN",
                electionTypes: "TIPOS DE ELECCIÓN",
                languages: "IDIOMAS",
                localization: "LOCALIZACIÓN",
                integrations: "INTEGRACIONES",
                lookAndFeel: "PERSONILIZACIÓN DE APARIENCIA",
                schedules: "EVENTOS PROGRAMADOS",
                trustees: "AUTORIDADES",
                BackupRestore: "COPIA DE SEGURIDAD / RESTAURAR",
                logs: "BITÁCORAS",
            },
        },
        trusteesSettingsScreen: {
            common: {
                emptyHeader: "Aún no hay fideicomisarios.",
                createNew: "Crear fideicomisario",
                title: "Fideicomisario",
                subtitle: "Configuración del fideicomisario",
                emptyBody: "¿Quieres crear uno?",
            },
            create: {
                title: "Crear fideicomisario",
            },
            edit: {
                title: "Editar fideicomisario",
            },
        },
        scheduleScreen: {
            noPermissions: "No tienes permiso para acceder a la configuración.",
            createScheduleSuccess: "Fecha creada",
            createScheduleError: "Error creando fecha",
            deleteScheduleSuccess: "Fecha borrada",
            deleteScheduleError: "Error borrando fecha",
            common: {
                title: "Programado",
                subtitle: "Configuración de fechas",
                createNew: "Crear fecha",
                emptyHeader: "Aún no hay fechas.",
                emptyBody: "¿Quieres crear uno?",
            },
            create: {
                title: "Crear fecha",
                selectSchedule:
                    "Seleccione un horario de la lista predefinida o escriba uno personalizado",
            },
            edit: {
                title: "Editar fecha",
            },
            eventTypes: {
                SYSTEM_LOCKDOWN_FOR_INTERNET_VOTING_SETTINGS:
                    "Bloqueo del sistema para la finalización de la configuración de votación por Internet",
                START_PRE_REGISTRATION_OVCS: "Inicio de la preinscripción para OVCS",
                END_PRE_REGISTRATION_OVCS: "Fin de la preinscripción para OVCS",
                START_TEST_VOTING_PERIOD: "Inicio del período de votación de prueba",
                END_TEST_VOTING_PERIOD: "Fin del período de votación de prueba",
                START_INTERNET_VOTING_PERIOD: "Inicio del período de votación por Internet",
                END_INTERNET_VOTING_PERIOD: "Fin del período de votación por Internet",
                LAB_TEST: "Prueba de laboratorio",
                FIELD_TEST: "Prueba de campo",
                MOCK_ELECTIONS: "Elecciones simuladas",
                FTS: "FTS",
            },
        },
        dashboard: {
            voteByDay: "Votos por día",
            votesOverTime: "Votos a lo largo del tiempo",
            timeResolution: "Resolución temporal",
            timeRange: "Intervalo de tiempo",
            minute: "Minuto",
            hour: "Hora",
            day: "Día",
            votersByChannels: "Votantes por canales",
            voterLoginURL: "URL de inicio de sesión de votantes",
            voterEnrollURL: "URL de inscripción de votantes",
            voterEnrollKioskURL: "Kiosk URL de inscripción de votantes",
            ballotBoxes: {
                show: "Mostrar urnas",
                loadError:
                    "No se pudieron leer los sellos de las urnas. Recargue la página o compruebe la conexión con el servidor.",
                title: "Urnas",
                sealing:
                    "La votación se cerró a las {{closed}}. Las urnas se sellan cuando termina el periodo de gracia, a las {{deadline}}.",
                sealed: "La votación se cerró a las {{closed}}. Las urnas están selladas: no se puede añadir, cambiar ni borrar ninguna papeleta.",
                failed: "La votación se cerró a las {{closed}}. No se pudo sellar una urna: sigue bloqueada y la incidencia figura en los registros.",
                closedBySignatures:
                    "Cerrada por {{names}} con sus certificados, código de firma {{code}}.",
                closedByUser: "Cerrada por {{username}}.",
                closedBySchedule: "Cerrada por el cierre programado de la votación.",
                column: {
                    area: "Área",
                    status: "Estado",
                    inTheBox: "En la urna",
                    counted: "Contadas",
                    sealedAt: "Sellada",
                    sealHash: "Hash del sello",
                    record: "Acta de sellado",
                },
                status: {
                    open: "Abierta",
                    sealing: "Se sella a las {{time}}",
                    publishing: "Sellada, publicándose",
                    sealed: "Sellada",
                    failed: "Sin sellar: incidencia",
                    due: "Sellándose ahora",
                    overdue: "Sellado con retraso",
                },
                help: {
                    publishing:
                        "La urna está bloqueada y su sello está en el tablón de anuncios. Su acta de sellado se está publicando.",
                    counted:
                        "Papeletas que cuentan: la última papeleta válida de cada votante elegible. Las demás de la urna fueron sustituidas por una papeleta posterior del votante, descartadas o emitidas por un votante no elegible.",
                },
                copyHash: "Copiar el hash del sello",
                copied: "Hash del sello copiado",
                copyError: "No se pudo copiar el hash del sello",
                notYet: "Todavía no",
                openRecord: "Abrir el acta de sellado de {{area}}",
                downloadRecord: "Descargar el acta de sellado de {{area}}",
                recordRestricted:
                    "Restringida: pídala a un administrador que pueda descargar documentos.",
                recordError: "No se pudo descargar el acta de sellado. Inténtelo de nuevo.",
                recordMissing:
                    "Falta el documento del acta de sellado: notifíquelo como incidencia.",
                beforeClose: "La urna de cada área se sella cuando se cierra la votación.",
                notStarted:
                    "La votación aún no se ha abierto. La urna de cada área se sella cuando se cierra la votación.",
                openOn: "La votación está abierta en: {{channels}}. La urna de cada área se sella cuando se cierra la votación.",
                paused: "La votación está en pausa. La urna de cada área se sella cuando se cierra la votación.",
                holding_one:
                    "El canal {{channels}} está habilitado y sin cerrar: deténgalo para sellar las urnas.",
                holding_other:
                    "Los canales {{channels}} están habilitados y sin cerrar: deténgalos para sellar las urnas.",
                sealingNow: "La votación se cerró a las {{closed}}. Las urnas se están sellando.",
                sealingPastGrace:
                    "La votación se cerró a las {{closed}}. El periodo de gracia terminó a las {{deadline}}; las urnas se están sellando.",
                why: {
                    due: "Sellándose: tarda hasta un minuto.",
                    channelOpen:
                        "El canal {{channel}} sigue habilitado y sin cerrar: deténgalo para sellar la urna.",
                    channelNotEnabled:
                        "El canal {{channel}} no está cerrado ni habilitado en esta elección: deténgalo para sellar la urna.",
                    channelHasBallots:
                        "El canal {{channel}} tiene papeletas en esta urna y no está cerrado: deténgalo para sellar la urna.",
                    datafixVotes_one:
                        "Hay {{count}} voto en curso en Datafix: la urna se sella cuando se resuelva.",
                    datafixVotes_other:
                        "Hay {{count}} votos en curso en Datafix: la urna se sella cuando se resuelvan.",
                    sequencer_one:
                        "Hay {{count}} papeleta que aún no está en el tablón de anuncios: la urna se sella cuando el secuenciador la añada.",
                    sequencer_other:
                        "Hay {{count}} papeletas que aún no están en el tablón de anuncios: la urna se sella cuando el secuenciador las añada.",
                    stale: "Último intento a las {{time}}: puede que el proceso de sellado no esté en marcha. Compruebe Beat y el worker de sellado.",
                    notTried:
                        "Aún no se ha intentado: puede que el proceso de sellado no esté en marcha. Compruebe Beat y el worker de sellado.",
                    errorCategory: {
                        board: "El último intento no pudo acceder al tablón de anuncios; se reintenta cada minuto.",
                        census: "El último intento no pudo leer el censo de votantes; se reintenta cada minuto.",
                        keystore:
                            "El último intento no pudo obtener la clave de firma; se reintenta cada minuto.",
                        storage:
                            "El último intento no pudo subir el acta de sellado al almacenamiento de archivos; se reintenta cada minuto.",
                        settings:
                            "El último intento no pudo leer la configuración de la elección; se reintenta cada minuto.",
                        other: "El último intento falló; se reintenta cada minuto. El registro del servicio tiene los detalles.",
                        ballots:
                            "El último intento encontró una papeleta que aún no se puede leer o que sigue en curso; se reintenta cada minuto.",
                        database:
                            "El último intento no pudo completarse en la base de datos; se reintenta cada minuto.",
                    },
                },
                failure: {
                    ballotIdMismatch: "Una papeleta no coincide con su ID de papeleta.",
                    missingContent: "Una papeleta no tiene contenido o no tiene ID de papeleta.",
                    unreadable: "No se puede leer una papeleta.",
                    inProgress: "Una papeleta sigue en curso.",
                    alreadyOnBoard: "Ya hay un sello de esta urna en el tablón de anuncios.",
                    noBoard: "El evento electoral no tiene tablón de anuncios.",
                    unknownChannel: "Una papeleta tiene un canal de votación desconocido.",
                    unrecorded: "Una papeleta no tiene registro de voto en el tablón de anuncios.",
                },
                incident: {
                    title_one: "No se pudo sellar {{count}} urna",
                    title_other: "No se pudieron sellar {{count}} urnas",
                    body: "Esto es una incidencia: cada una de estas urnas sigue bloqueada y no se puede incluir en un recuento. Siga el procedimiento para un sellado fallido.",
                    line: "{{election}}, {{area}}: {{reason}}",
                },
            },
            ipAddress: {
                emptyState: "Aún no hay votos.",
                title: "Direcciones IP",
                ip: "IP",
                country: "País",
                VoteCount: "Número de votos",
                ElectionName: "Nombre de la elección",
                VotersId: "Identificadores de votantes",
            },
        },
        electionEventScreen: {
            dataUnavailable: "Los datos no están disponibles para este evento electoral.",
            noAccessibleTabs:
                "No tienes acceso a ninguna pestaña disponible de este evento electoral.",

            common: {
                subtitle: "Configuración del Evento Electoral.",
                showMore: "Mostrar más",
                showLess: "Mostrar menos",
                adminPortal: "Portal de Administración",
                allowPublishAfterLockdown:
                    "Permitir la publicación del evento electoral solo después del bloqueo",
                reset: "Restablecer filtro personalizado",
            },
            edit: {
                general: "General",
                dates: "Fechas",
                customUrls: "Prefixo de URLs personalizados",
                votingPeriod: "Período de votación",
                language: "Idiomas",
                allowed: "Canales de Voto Permitidos",
                materials: "Materiales de Soporte",
                ballotReceipts: "Recibos de papeleta",
                ballotDesign: "Diseño de la Papeleta",
                templates: "Plantillas",
                reorder: "Reordenar elecciones",
                advancedConfigurations: "Voting Portal Countdown Policy",
                importCandidates: "Importar Candidatos",
                custom_filters: "Filtros personalizados",
                voter_authentication: "Autenticación de votantes",
                realm_attributes: "Atributos del realm de Keycloak",
                realm_attributes_load_error: "Error al cargar los atributos del realm de Keycloak",
                realm_attributes_update_error:
                    "Error al actualizar los atributos del realm de Keycloak",
                realm_attributes_not_loaded:
                    "Los atributos del realm de Keycloak no se han cargado; los cambios no se guardaron",
                password_policy: "Política de contraseñas",
                password_policy_load_error:
                    "Error al cargar la política de contraseñas de Keycloak",
                password_policy_update_error:
                    "Error al actualizar la política de contraseñas de Keycloak",
                password_policy_not_loaded:
                    "La política de contraseñas de Keycloak no se ha cargado; los cambios no se guardaron",
            },
            customUrls: {
                login: "Inicio de sesión",
                enrollment: "Inscripción",
            },
            localization: {
                emptyHeader: "No se han establecido idiomas para el evento",
                selectLanguage: "Seleccionar idioma",
                notify: {
                    success: "Localización actualizada con éxito",
                    error: "La actualización de la localización ha fallado",
                    duplicateKey: "Ya existe una sustitución con esta clave y ámbito del portal.",
                    invalidDateTimeFormat:
                        "Formato de fecha/hora no válido. Usa los tokens yyyy, MM, dd, HH, mm, ss (p. ej. dd/MM/yyyy HH:mm).",
                    invalidTimeZoneText: "Este texto debe conservar {{placeholders}}.",
                },
                common: {
                    title: "Localización",
                    subTitle: "Configuración de la localización",
                },
                labels: {
                    key: "Clave",
                    scope: "Ámbito del portal",
                    value: "Valor",
                },
                scopes: {
                    legacy: "Heredado ({{portal}})",
                    global: "Global",
                    votingPortal: "Portal de votación",
                    ballotVerifier: "Verificador de papeletas",
                    resultsPortal: "Portal de resultados",
                    adminPortal: "Portal de administración",
                    templates: "Informes y mensajes",
                },
            },
            field: {
                passwordPolicy: {
                    minimumLength: "Longitud mínima",
                    maximumLength: "Longitud máxima",
                    includeUppercase: "Incluir letras mayúsculas",
                    includeLowercase: "Incluir letras minúsculas",
                    includeDigits: "Incluir dígitos",
                    includeSpecialCharacters: "Incluir caracteres especiales",
                    help: {
                        minimumLength:
                            "El número mínimo de caracteres requeridos para la contraseña.",
                        maximumLength:
                            "El número máximo de caracteres permitidos para la contraseña.",
                        includeUppercase:
                            "La contraseña debe incluir al menos una letra mayúscula.",
                        includeLowercase:
                            "La contraseña debe incluir al menos una letra minúscula.",
                        includeDigits: "La contraseña debe incluir al menos un dígito.",
                        includeSpecialCharacters:
                            "La contraseña debe incluir al menos un carácter especial.",
                    },
                    notConfigured:
                        "No hay ninguna política de contraseñas configurada. Al guardar se aplicarán los valores predeterminados que se muestran a continuación.",
                    errors: {
                        lengthRange:
                            "Las longitudes de la contraseña deben ser números enteros entre 1 y 256.",
                        minimumExceedsMaximum:
                            "La longitud mínima no puede superar la longitud máxima.",
                        characterClassRequired:
                            "Seleccione al menos una clase de caracteres para la contraseña.",
                    },
                },
                name: "Nombre",
                alias: "Alias",
                description: "Descripción",
                startDateTime: "Fecha y hora de inicio",
                endDateTime: "Fecha y hora de finalización",
                language: "Idioma",
                votingChannels: "Canales de Voto",
                materialActivated: "Materiales de Soporte activados",
                supportMaterialsPolicy: {
                    label: "Política de Materiales de Soporte",
                    helperText:
                        "La opción Obligatorio para Votar requiere que los votantes abran cada Material de Soporte y confirmen que lo han leído antes de poder votar.",
                    options: {
                        off: "Desactivado",
                        optional: "Opcional",
                        mandatory_for_voting: "Obligatorio para Votar",
                    },
                },
                materialTitle: "Titulo",
                materialSubTitle: "Subtitulo",
                logoUrl: "URL del logotipo",
                userVerification:
                    "Puede introducir una plantilla personalizada que se utilizará para verificar manualmente a los votantes",
                redirectFinishUrl: "URL de redirección al finalizar",
                kioskRedirectFinishUrl: "URL de redirección al finalizar del quiosco",
                css: "CSS personalizado",
                skipElectionList: "Saltar pantalla para escoger elección",
                showUserProfile: "Mostrar perfil de usuario",
                ballotReceipts: {
                    checksPeriod: {
                        policyLabel: "Periodo de comprobación de los votos emitidos",
                        helper: "Durante cuánto tiempo los votantes pueden localizar su voto emitido e imprimir su recibo en el Portal de Votación.",
                        options: {
                            "unlimited": "Sin límite",
                            "until-date": "Hasta una fecha",
                        },
                    },
                    checksAvailableUntil: "Comprobaciones disponibles hasta ({{timezone}})",
                    checksAvailableUntilRequired:
                        "Introduce la fecha y hora hasta la que se pueden comprobar los votos.",
                },
                voterAccessibilitySettingsPolicy: {
                    policyLabel: "Ajustes de accesibilidad del votante",
                    options: {
                        disabled: "Ocultar los ajustes de accesibilidad",
                        enabled:
                            "Ofrecer ajustes de tamaño de texto, contraste, espaciado y movimiento",
                    },
                },
                instructionsStepPolicy: {
                    policyLabel: "Instrucciones en la barra de pasos",
                    options: {
                        "merged-with-ballot": "Dentro del paso Papeleta",
                        "own-step": "Un paso propio",
                    },
                },
                audioInstructionsPolicy: {
                    policyLabel: "Instrucciones en audio",
                    options: {
                        "disabled": "Sin instrucciones en audio",
                        "recorded": "Solo grabaciones subidas",
                        "recorded-or-synthesized":
                            "Grabaciones subidas, o la voz del navegador donde no haya",
                    },
                },
                showCastVoteLogs: {
                    policyLabel: "Mostrar logs de votación",
                    options: {
                        "show-logs-tab": "Mostrar Tab de logs de votación",
                        "hide-logs-tab": "Ocultar Tab de logs de votación",
                    },
                },
                lockdownState: {
                    since: "Bloqueado desde {{date}}.",
                    scheduledEnd: "Fin del bloqueo programado para {{date}}.",
                    endScheduled: "Hay un fin del bloqueo programado.",
                    noScheduledEnd: "No hay un fin del bloqueo programado.",
                    banner: "Este evento electoral está bloqueado.",
                    policyLabel: "Estado de Confinamiento",
                    helperText:
                        "Programe el inicio o el fin del período de confinamiento para cambiar este estado.",
                    options: {
                        "locked-down": "Confinado",
                        "not-locked-down": "No Confinado",
                    },
                },
                ballotBoxSealPolicy: {
                    policyLabel: "Política de sellado de urnas",
                    helperText:
                        "Cuando se cierra la votación, se sella la urna de cada área: un hash firmado de sus papeletas se publica en el tablón de anuncios, no se puede añadir, cambiar ni borrar ninguna papeleta, y la votación no puede volver a empezar.",
                    locked: "No se puede cambiar una vez abierta la votación.",
                    options: {
                        "seal-at-close": "Sellar al cierre",
                        "do-not-seal": "No sellar",
                    },
                    checking: "Comprobando si la votación se ha abierto…",
                    lockedUnknown:
                        "Bloqueada: no se pudieron leer las elecciones, así que no se sabe si la votación se ha abierto.",
                    lockedOpened: "Bloqueada: la votación se ha abierto en {{names}}.",
                    lockedEvent: "Bloqueada: la votación se ha abierto en este evento electoral.",
                    refused:
                        "La Política de sellado de urnas no se puede cambiar una vez abierta la votación.",
                    settingLocked: "Con Sellar al cierre, el sello depende de esta configuración.",
                    settingRefused:
                        "Esta configuración no se puede cambiar una vez abierta la votación: con Sellar al cierre, el sello depende de ella.",
                    boardRefused:
                        "El tablón de anuncios del evento electoral no puede cambiar una vez abierta la votación: con Sellar al cierre, los sellos se publican en él.",
                },
                ballotBoxSealRecordPolicy: {
                    policyLabel: "Acta de sellado de urnas",
                    options: {
                        restricted: "Restringida",
                        public: "Pública",
                    },
                    help: {
                        restricted:
                            "Solo los administradores pueden descargarla; compártala con los observadores.",
                        public: "Cualquiera con los identificadores del evento puede descargarla, sin iniciar sesión; muestra cómo contó cada papeleta.",
                    },
                },
                decodedBallots: {
                    policyLabel:
                        "Incluir papeletas decodificadas en la base de datos de resultados",
                    options: {"included": "Incluir", "not-included": "No incluir"},
                },
                fileIntegrityPolicy: {
                    policyLabel: "Integridad del archivo de votantes",
                    helperText:
                        "Exigir el SHA-256 certificado de forma independiente antes de importar votantes.",
                    options: {
                        OPTIONAL: "SHA-256 opcional",
                        REQUIRED_SHA256: "SHA-256 obligatorio",
                    },
                },
                contestEncryptionPolicy: {
                    options: {
                        "single-contest": "Pregunta única",
                        "multiple-contests": "Varias preguntas",
                    },
                    policyLabel: "Política de cifrado de pregunta",
                },
                votingPortalDateTimeFormat: {
                    policyLabel: "Formato de fecha y hora del portal de votación",
                    helperText:
                        'Se aplica a todo el evento. Para anularlo por idioma, añade la clave "votingPortalDateTimeFormat" en la pestaña Localización usando los tokens yyyy, MM, dd, HH, mm, ss (p. ej. dd/MM/yyyy HH:mm). Consulta la documentación para más detalles.',
                    options: {
                        "legacy-gb-24h": "GB heredado 24 h (dd/MM/yyyy HH:mm, 24h)",
                        "iso-local": "ISO local (yyyy-MM-dd HH:mm)",
                        "us-12h": "EE. UU. 12 h (MM/dd/yyyy h:mm AM/PM)",
                        "locale-medium": "Locale Medium (fecha media, hora corta)",
                        "date-only": "Date Only (sin hora)",
                        "custom": "Formato personalizado",
                    },
                    customFormat: {
                        label: "Formato de fecha y hora personalizado",
                        helperText:
                            "Usa los tokens yyyy, MM, dd, HH, mm, ss (p. ej. dd/MM/yyyy HH:mm). Cualquier otro carácter se muestra literalmente.",
                        invalid:
                            "Formato no válido. Usa al menos uno de los tokens yyyy, MM, dd, HH, mm, ss.",
                    },
                },
                countDownPolicyOptions: {
                    NO_COUNTDOWN: "Sin cuenta regresiva",
                    COUNTDOWN: "Cuenta regresiva",
                    COUNTDOWN_WITH_ALERT: "Cuenta regresiva con alerta",
                    sectionTitle: "Portal de votación",
                    policyLabel: "Política de cuenta regresiva del portal de votación",
                    coundownSecondsLabel:
                        "tiempo en segundos antes de la expiración para mostrar la cuenta regresiva",
                    alertSecondsLabel:
                        "tiempo en segundos antes de la expiración para mostrar la alerta de cierre de sesión",
                },
                voterSigningPolicy: {
                    "policyLabel": "Política de Firma de Votantes",
                    "no-signature": "Sin firma",
                    "with-signature": "Con firma",
                },
                receiptsPolicy: {
                    "policyLabel": "Recibos firmados por la urna",
                    "disabled": "Desactivado",
                    "signed-by-ballot-box": "Firmados por la urna",
                    "helperText":
                        "Cuando está activado, la urna almacena y firma cada papeleta en la pantalla de revisión, y el votante ve un ID de papeleta solo cuando la urna la ha recibido. Los votantes firman sus papeletas. Vuelva a publicar las papeletas después de cambiarlo.",
                    "lockedHelperText": "No se puede cambiar una vez iniciada la votación.",
                },
                VoterCertificatePolicy: {
                    policyLabel: "Política de certificado digital del votante",
                    enabled: "Habilitado",
                    disabled: "Deshabilitado",
                },
                enrollment: {
                    policyLabel: "Inscripción",
                    options: {
                        enabled: "Habilitado",
                        disabled: "Deshabilitado",
                    },
                },
                otp: {
                    policyLabel: "OTP",
                    options: {
                        enabled: "Habilitado",
                        disabled: "Deshabilitado",
                    },
                },
                ceremoniesPolicy: {
                    policyLabel: "Política de ceremonias de llaves/recuento",
                    options: {
                        "automated-ceremonies": "Permitir ceremonias automáticas",
                        "manual-ceremonies": "Ceremonias manuales",
                    },
                },
                automaticRecountPolicy: {
                    policyLabel: "Recuento automático tras aprobar una importación",
                    options: {
                        enabled: "Habilitado",
                        disabled: "Deshabilitado",
                    },
                },
                weightedVotingPolicy: {
                    policyLabel: "Política de Votación Ponderada",
                    options: {
                        "areas-weighted-voting": "Votación Ponderada por Áreas",
                        "voters-weighted-voting": "Votación Ponderada por Votantes",
                        "disabled-weighted-voting": "Votación Ponderada Deshabilitada",
                    },
                    noDelegated:
                        "La Votación Ponderada por Votantes no puede combinarse con el Voto Delegado",
                    noDecodedBallots:
                        "La Votación Ponderada por Votantes no puede combinarse con la inclusión de papeletas descifradas en los resultados",
                },
                delegatedVotingPolicy: {
                    policyLabel: "Política de Votación Delegada",
                    options: {
                        enabled: "Habilitado",
                        disabled: "Deshabilitado",
                    },
                },
                batchAnonymityPolicy: {
                    policyLabel: "Política de Anonimato de Lotes",
                    helperText:
                        "Rechazar detiene el recuento cuando el lote de papeletas de un área tiene menos de cinco votantes, o difiere del lote que otro recuento ya publicó para la misma área y pregunta.",
                    options: {
                        warn: "Advertir",
                        refuse: "Rechazar",
                    },
                },
                languageDetectionPolicy: {
                    policyLabel: "Política de detección de idioma",
                    options: {
                        "browser-detect": "Detectar desde el navegador",
                        "force-default": "Forzar predeterminado",
                    },
                },
            },
            error: {
                endDate: "La fecha de finalización debe ser posterior a la fecha de inicio",
                noResult: "Todavía no hay ningún evento electoral",
                startDate: "La fecha de inicio debe ser en el futuro",
                endDateInvalid: "La fecha de finalización debe estar en el futuro",
            },
            voters: {
                title: "Votantes",
            },
            createElectionEventSuccess: "Evento Electoral creado",
            createElectionEventError: "Error creando Evento Electoral",
            ivr: {
                tabs: {
                    config: "Configuración",
                    blacklist: "Lista de bloqueo",
                    prompts: "Locuciones",
                    emulator: "Emulador",
                },
                common: {
                    saveSuccess: "Guardado correctamente",
                    saveError: "No se pudo guardar",
                    deleteSuccess: "Eliminado correctamente",
                    deleteError: "No se pudo eliminar",
                },
                config: {
                    configuredPhone: "Número de teléfono configurado",
                    infoMsg:
                        "Configure el flujo del IVR y sus propiedades a continuación. Para más detalles, póngase en contacto con Sequent.",
                },
                prompts: {
                    emptyMsg: "Aún no se ha creado ningún mensaje",
                    infoMsg:
                        "Configure los mensajes utilizados por el IVR. Los mensajes de anuncio son obligatorios, y los mensajes del sistema se pueden sobrescribir para los idiomas deseados. Se admite SSML, incluso para mezclar idiomas.",
                    editorTitle: "Mensaje",
                    editorSubtitle: "Configuración del mensaje",
                },
                blacklist: {
                    columns: {
                        phone: "Número de teléfono",
                        reason: "Motivo",
                        createdAt: "Creado el",
                        createdBy: "Creado por",
                        createdBefore: "Creado antes",
                        createdAfter: "Creado después",
                    },
                    emptyMsg: "No hay entradas en la lista de bloqueo",
                    infoMsg:
                        "Configure la lista de bloqueo del IVR. Las llamadas de estos números serán desconectadas automáticamente por el sistema.",

                    noFilterMatch: "Ninguna entrada coincide con los filtros indicados",
                    phoneRequired: "El número de teléfono es obligatorio",
                },
                emulator: {
                    infoMsg:
                        "Seleccione un área y las elecciones deseadas para probar la sesión de IVR.",
                    apiStatus: {
                        unavailable: "El sistema del emulador no está disponible en su entorno",
                        loading: "Cargando el sistema del emulador",
                        error: "Error al cargar el sistema del emulador",
                    },
                    hints: {
                        title: "Sugerencias",
                        publishRequired:
                            "Cualquier cambio realizado en las elecciones, preguntas o candidatos debe publicarse primero para que esté disponible. En el emulador solo se utilizarán los estilos de boleta publicados más recientemente para el área correspondiente.",
                        eventChangesImmediate:
                            "Los cambios realizados en el evento electoral, como la configuración de IVR o las modificaciones de los mensajes, están disponibles inmediatamente al reiniciar la sesión del emulador.",
                        credentials:
                            'El identificador de votante y el PIN válidos son "123" y "123".',
                    },
                    sendDtmf: "Enviar entrada DTMF",
                    keypadInput: "Entrada del teclado",
                    sendTimeout: "Enviar tiempo de espera",
                    disconnected: "Desconectado",
                    startSession: "Iniciar nueva sesión",
                    endSession: "Finalizar la sesión",
                    noStylesFound:
                        "No se encontraron estilos de boleta publicados que coincidan con sus selecciones",
                    inputPlaceholder: "Pulse {{keys}} (tiempo de espera={{timeout}} s)",
                    inputPlaceholderOr: "o",
                    inputPlaceholderAnyKeys:
                        "Introduzca hasta {{maxDigits}} dígitos (cualquier dígito, tiempo de espera={{timeout}} s)",
                    blacklistCaller: "Bloquear a la persona que llama",
                    elections: "Elecciones",
                    area: "Área",
                },
            },
            stats: {
                elegibleVoters: "Votantes Elegibles",
                voters: "Votantes",
                elections: "Elecciones",
                contests: "Preguntas",
                areas: "Áreas",
                sentEmails: "Emails Enviados",
                sentSMS: "SMS Enviados",
                calendar: {
                    title: "Calendario",
                    scheduled: "Programado",
                },
            },
            keys: {
                createNew: "Crear Ceremonia de Claves",
                emptyHeader: "Ninguna Ceremonia de Claves aún.",
                statusLabel: "Estado",
                waitingKeys: "Esperando a la Generación de Claves..",
                started: "Iniciada en",
                actions: {
                    participate: "Participar en la ceremonia de claves",
                    view: "Ver la ceremonia de claves",
                },
                breadCrumbs: {
                    configure: "Configurar",
                    ceremony: "Ceremonia",
                    created: "Finalizado",
                    start: "Comienzo",
                    status: "Estado",
                    download: "Descargar",
                    check: "Comprobar",
                    success: "Finalizado",
                },
                notify: {
                    participateNow:
                        "Ha sido invitado a participar a una Ceremonia de Claves. Por favor <1>haz clic abajo en la acción de llave de la ceremonia</1> para participar.",
                },
            },
            tabs: {
                dashboard: "Panel de Control",
                monitoring: "Monitoreo",
                data: "Datos",
                ivr: "IVR",
                localization: "Localización",
                voters: "Votantes",
                areas: "Áreas",
                keys: "Claves",
                tally: "Recuento",
                tallySheetImports: "Importación de actas",
                publish: "Publicar",
                logs: "Logs",
                tasks: "Tareas",
                events: "Eventos Programados",
                notifications: "Notificaciones",
                reports: "Informe",
                approvals: "Aprobaciones",
                cas: "Certificados",
            },
            tally: {
                emptyHeader: "Ningún Recuento aún.",
                title: "Recuento del Evento Electoral",
                elections: "Elecciones",
                electionNumber: "Número de Elecciones",
                trustees: "Custodios de claves",
                status: "Estado",
                permissionLabels: "Etiquetas de Permisos",
                tallyType: {
                    label: "Tipo de Conteo",
                    ELECTORAL_RESULTS: "Resultados Electorales",
                    INITIALIZATION_REPORT: "Resultados de Inicialización",
                },
                create: {
                    title: "Crear Recuento",
                    subtitle: "Crear un nuevo Recuento para este Evento Electoral",
                    createTallyButton: "Iniciar la Ceremonia de Recuento",
                    createInitializationReportButton: "Crear Informe de Inicialización",
                    error: {
                        create: "Error creando Recuento",
                    },
                    success: "Recuento creado",
                },
                logs: {
                    noLogs: "No hay registros disponibles",
                },
                notify: {
                    noKeysTally:
                        "La Ceremonia de Recuento no puede comenzar hasta que la Ceremonia de Claves se haya completado con éxito.",
                    noPublication:
                        "La Ceremonia de Escrutinio no puede comenzar hasta que crees una publicación en la pestaña Publicar.",
                    participateNow:
                        "Ha sido invitado a participar a una Ceremonia de Recuento. Por favor <1>haz clic abajo en la acción de llave de la ceremonia</1> para participar.",
                    startDisabled:
                        "No puedes continuar con la ceremonia porque no se han seleccionado elecciones o las elecciones no están publicadas.",
                    ceremonyDisabled:
                        "No puedes continuar con la ceremonia porque la sesión de recuento no está conectada o el inicio de la ceremonia no está permitido.",
                },
            },
            importAreas: {
                title: "Importar Áreas",
                subtitle: "Importar datos de áreas",
                areaParagraph:
                    "Importar áreas utilizando un archivo de hoja de cálculo en formato de valores separados por comas (CSV).",
                importSuccess: "Áreas Importadas Exitosamente",
                importError: "Error al importar Áreas",
                upsert: "Insertar o actualizar áreas",
            },
            import: {
                eetitle: "Importar Evento Electoral",
                eesubtitle: "Importar datos del Evento Electoral",
                title: "Importar votantes",
                subtitle: "Importar votantes al Evento Electoral",
                voters: "Votantes",
                votersParagraph:
                    "Importa votantes usando una hoja de cálculo en formato Comma Separated Values (CSV) format. Descarga un ejemplo de fichero de importación CSV aquí.",
                electionEventParagraph: "Importa Eventos Electorales usando un fichero JSON.",
                elections: "Elecciones",
                areas: "Áreas",
                sha: "Verificación de Integridad (SHA 256)",
                cancel: "Cancelar",
                import: "Importar",
                fileUploadSuccess: "Fichero subido al servidor - pero no importado aún",
                fileUploadError: "Error subiendo el fichero",
                importVotersSuccess: "Importación de Votantes lanzada en segundo plano con éxito.",
                importVotersError: "Error Importando Votantes.",
                importElectionEventSuccess: "Evento electoral importado correctamente",
                importElectionEventError: "Error al importar el evento electoral",
                shaRequiredHelp:
                    "Introduzca el SHA-256 hexadecimal certificado de 64 caracteres. Es obligatorio para importar.",
                shaDialog: {
                    ok: "Sí, Importar Sin Verificación de Integridad",
                    cancel: "Volver",
                    title: "Importar Sin Verificación de Integridad?",
                    description:
                        "No ingresó el campo Verificación de integridad (SHA-256). Confirme que está importando el archivo correcto y que desea importarlo.",
                },
                passwordDialog: {
                    title: "Contraseña de Descifrado",
                    description: "Introduce la contraseña para descifrar el archivo",
                    label: "Contraseña",
                    copyPassword: "Copiar Contraseña",
                    ok: "Aceptar",
                },
            },
            export: {
                title: "Exportar Evento Electoral",
                subtitle:
                    "La exportación puede ser una operación larga. ¿Estás seguro de que deseas exportar los registros?",
                encryptWithPassword: "Cifrar con Contraseña",
                passwordForcedNote:
                    "El archivo se protegerá con contraseña de todos modos: los informes, las solicitudes y los datos del tablón se cifran siempre. Marca la casilla para incluir también los campos secretos de votante descifrados.",
                includeVoters: "Incluir Votantes",
                activityLogs: "Registros de Actividad",
                bulletinBoard: "Tablón de Anuncios",
                publications: "Publicaciones",
                s3Files: "Archivos S3",
                scheduledEvents: "Eventos Programados",
                exportSuccess: "Evento Electoral exportado con éxito",
                exportError: "Error al exportar el Evento Electoral",
                passwordTitle: "Contraseña",
                passwordDescription: "Contraseña para descifrar el archivo:",
                copiedSuccess: "Nakopya ang password sa clipboard",
                copiedError: "Error al copiar la contraseña",
                reports: "Informes",
                applications: "Aplicaciones",
                tally: "Recuento",
                certificates: "Certificados",
            },
            taskNotification:
                "{{action}} ha comenzado. Puedes ver su estado en la tabla de Ejecución de Tareas.",
        },
        electionScreen: {
            common: {
                title: "Elección",
                subtitle: "Configuración de la elección.",
                fileLoaded: "Archivo cargado",
                noPermission: "No tienes permiso para acceder a esta elección.",
            },
            edit: {
                general: "General",
                dates: "Fechas",
                votingPeriod: "Período de votación",
                language: "Idioma",
                allowed: "Canales de Voto Permitidos",
                default: "Por defecto",
                defaultLang: "Idioma predeterminado",
                receipts: "Comprobantes",
                image: "Imagen",
                advanced: "Configuración Avanzada",
                numAllowedVotes: "Número de votos permitidos",
                reorder: "Reordenar preguntas",
                castVoteConfirm: "Modal de Confirmación de Voto",
                gracePeriodPolicy: "Política de período de gracia",
                allowTallyPolicy: "Permitir Recuento",
                permissionLabel: "Etiqueta de permiso",
                custom_filters: "Filtros personalizados",
            },
            field: {
                name: "Nombre",
                language: "Idioma",
                votingChannels: "Canales de Voto",
                startDateTime: "Fecha y hora de inicio",
                endDateTime: "Fecha y hora de finalización",
                startDateTimeWithTimezone: "Fecha y Hora de inicio ({{timezone}})",
                endDateTimeWithTimezone: "Fecha y Hora de finalización ({{timezone}})",
                scheduledOpening: "Apertura Programada",
                scheduledClosing: "Cierre Programado",
                alias: "Alias",
                description: "Descripción",
                securityConfirmationHtml: "Confirmación de seguridad HTML",
                ivrPrompt: "Mensaje IVR",
                externalId: "ID externo",
            },
            securityConfirmationPolicy: {
                label: "Política de la casilla de confirmación de seguridad",
                none: "Ninguno",
                mandatory: "Obligatorio",
            },
            error: {
                endDate: "La fecha de finalización debe ser posterior a la fecha de inicio",
                fileError: "Error al cargar el archivo",
                fileLoaded: "Archivo cargado",
                startDate: "La fecha de inicio debe ser en el futuro",
                endDateInvalid: "La fecha de finalización debe estar en el futuro",
            },
            createElectionEventSuccess: "Creada la elección",
            createElectionEventError: "Error Creando la elección",
            tabs: {
                dashboard: "Panel de Control",
                monitoring: "Monitoreo",
                data: "Datos",
                voters: "Votantes",
                publish: "Publicar",
                logs: "Logs",
                approvals: "Aprobaciones",
                tallySheets: "Hojas de Recuento",
            },
            gracePeriodPolicy: {
                "label": "Política de período de gracia",
                "no-grace-period": "Sin período de gracia",
                "grace-period-without-alert": "Período de gracia sin alerta",
                "gracePeriodSecs": "Tiempo de gracia en segundos",
            },
            allowTallyPolicy: {
                "allowed": "Permitido",
                "disallowed": "No permitido",
                "requires-voting-period-end": "Permitir cuando termine el periodo de votación",
            },
            initializeReportPolicy: {
                "label": "Inicializar Política de Reportes",
                "not-required": "No Requerido",
                "required": "Requerido",
            },
            castVoteGoldLevelPolicy: {
                label: "Política de autenticación de nivel oro",
                options: {
                    "gold-level": "Autenticación de nivel oro",
                    "no-gold-level": "Sin autenticación de nivel oro",
                },
            },
            slates: {
                title: "Candidaturas",
                configuration: "Configuración de candidaturas (JSON)",
                helper: "Candidaturas con nombre y los candidatos que cada una presenta en cada contienda. Déjalo vacío para una elección sin candidaturas.",
                loading:
                    "Las contiendas y los candidatos de la elección aún se están cargando. Inténtalo de nuevo en un momento.",
                mobileCandidateLists: {
                    label: "Listas de candidatos en móvil",
                    helper: "Cómo aparece inicialmente la lista de candidatos de cada candidatura en el móvil. El votante siempre puede abrirla o cerrarla.",
                    options: {
                        collapsed: "Contraídas",
                        expanded: "Desplegadas",
                    },
                },
            },
            startScreenTitlePolicy: {
                label: "Política de título de pantalla de inicio",
                options: {
                    "election": "Elección",
                    "election-event": "Evento Electoral",
                },
            },
            consolidatedReportPolicy: {
                label: "Política de informe consolidado",
                options: {
                    "generate": "Generar",
                    "do-not-generate": "No generar",
                },
            },
            declineToVotePolicy: {
                label: "Política de abstención de voto",
                options: {
                    enabled: "Habilitado",
                    disabled: "Deshabilitado",
                },
            },
            blankBallotsPolicy: {
                label: "Política de papeletas en blanco",
                options: {
                    enabled: "Habilitado",
                    disabled: "Deshabilitado",
                },
            },
            votingScreenBackPolicy: {
                label: "Política del botón Atrás de la pantalla de votación",
                options: {
                    "election-selection-screen": "Ir a la pantalla de selección de elecciones",
                    "start-screen": "Ir a la pantalla de inicio de la elección",
                },
            },
        },
        tenantScreen: {
            common: {
                title: "Inquilinos",
            },
            new: {
                subtitle: "Crear nuevo inquilino",
            },
            createSuccess: "Inquilino creado",
            createError: "Error al crear el inquilino",
        },
        usersAndRolesScreen: {
            noPermissions: "No tienes permiso para acceder a los usuarios o roles.",
            common: {
                title: "Usuarios y Roles",
                subtitle: "Configuración general",
                mobileNumber: "Móvil",
            },
            editPassword: {
                passwordPolicyViolation:
                    "La contraseña no cumple la Política de contraseñas de este evento electoral. Revise la política en Datos del evento electoral e introduzca una contraseña válida.",
                passwordPolicyRules: {
                    minimumLength: "La longitud mínima de la contraseña es {{count}}.",
                    maximumLength: "La longitud máxima de la contraseña es {{count}}.",
                    uppercase: "Caracteres en mayúscula requeridos: {{count}}.",
                    lowercase: "Caracteres en minúscula requeridos: {{count}}.",
                    digits: "Dígitos requeridos: {{count}}.",
                    specialCharacters: "Caracteres especiales requeridos: {{count}}.",
                },
                label: "Cambiar contraseña",
                temporatyLabel: "Temporal",
                temporatyInfo:
                    "Si está habilitado, el usuario deberá cambiar la contraseña en el próximo inicio de sesión.",
            },
            users: {
                title: "Usuarios",
                subtitle: "Ver y editar datos del usuario",
                review: {
                    title: "Revisar cambios",
                    subtitle: "Confirma estas actualizaciones antes de enviarlas.",
                    confirm: "Confirmar cambios",
                    noChanges: "No hay cambios que revisar",
                    field: "Campo",
                    currentValue: "Valor actual",
                    newValue: "Valor nuevo",
                },
                edit: {
                    title: "Información de Usuario",
                    subtitle: "Ver y editar Usuario",
                },
                create: {
                    title: "Usuario",
                    subtitle: "Crear usuario",
                },
                fields: {
                    "has_voted": "Ha votado",
                    "support_materials_viewed": "Materiales de Soporte vistos",
                    "vote-weight": "Peso del voto",
                    "voted-channel": "Canal de voto",
                    "disable-comment": "Comentario de deshabilitación",
                    "username": "Nombre de Usuario",
                    "first_name": "Nombre",
                    "last_name": "Apellido",
                    "email": "Email",
                    "enabled": "Habilitado",
                    "emailVerified": "Email Verificado",
                    "groups": "Grupos",
                    "attributes": "Atributos",
                    "area": "Área",
                    "password": "Contraseña",
                    "savePassword": "Guardar contraseña",
                    "repeatPassword": "Repetir Contraseña",
                    "passwordMismatch": "Las contraseñas deben de coincidir",
                    "passwordLengthValidate": "La contraseña debe tener al menos 8 caracteres",
                    "passwordUppercaseValidate":
                        "La contraseña debe contener al menos una letra mayúscula",
                    "passwordLowercaseValidate":
                        "La contraseña debe contener al menos una letra minúscula",
                    "passwordDigitValidate": "La contraseña debe contener al menos un dígito",
                    "passwordSpecialCharValidate":
                        "La contraseña debe contener al menos un carácter especial",
                    "trustee": "Actuar como fideicomisario",
                    "permissionLabel": "Etiqueta de permiso",
                    "authorized-election-ids": "Elecciones",
                },
                delete: {
                    body: "¿Estás seguro que quieres borrar este usuario?",
                    bulkBody: "¿Estás seguro que quieres borrar los usuarios seleccionados?",
                    bulkBodySelected:
                        "¿Borrar los {{count}} usuarios seleccionados? Esta acción no se puede deshacer.",
                    bulkBodyChoose:
                        "Hay {{count}} usuarios seleccionados. En su lugar, puedes borrar todos los usuarios que coinciden con los filtros actuales, que pueden ser más. Esta acción no se puede deshacer.",
                    okSelected: "Borrar {{count}} seleccionados",
                    okAllMatching: "Borrar todos los que coinciden",
                },
                notifications: {
                    exportError: "Error al exportar usuarios",
                    deleteError: "Error borrando usuario",
                    deleteSuccess: "Usuario borrado",
                    multipleDeleteSuccess: "Usuarios borrados",
                },
            },
            voters: {
                voterInformationLetter: {
                    label: "Carta de información para el votante",
                    generate: "Generar",
                    confirmation:
                        "¿Generar una Carta de información para este votante? Se asignará una nueva contraseña y se incluirá en un PDF cifrado.",
                    generationStarted: "Se inició la generación de la Carta de información",
                    generationError: "No se pudo generar la Carta de información",
                    policyNotConfigured:
                        "La Política de contraseñas no está configurada. Configúrela en Datos del evento electoral antes de generar una carta.",
                    policyMinimumLengthMissing:
                        "La Política de contraseñas debe incluir una longitud mínima antes de generar una carta.",
                    policyCharacterClassMissing:
                        "La Política de contraseñas debe incluir al menos una clase de caracteres antes de generar una carta.",
                },
                title: "Votantes",
                subtitle: "Ver y editar datos del votante",
                secretAttribute: {
                    storedPlaceholder: "Valor cifrado almacenado",
                    reveal: "Mostrar",
                    hide: "Ocultar",
                    revealError: "No se pudo mostrar el campo cifrado del votante",
                    includeInExport: "Incluir campos secretos descifrados del votante",
                    exportWarning:
                        "Exportación sensible: el CSV descargado contendrá estos campos en texto plano.",
                    clear: "Borrar",
                    add: "Añadir valor",
                    remove: "Eliminar valor",
                },
                review: {
                    title: "Revisar cambios",
                    subtitle: "Confirma estas actualizaciones antes de enviarlas.",
                    confirm: "Confirmar cambios",
                    noChanges: "No hay cambios que revisar",
                    field: "Campo",
                    currentValue: "Valor actual",
                    newValue: "Valor nuevo",
                },
                logs: {
                    label: "Registros del usuario",
                },
                emptyHeader: "Ningún votante aún.",
                askCreate: "¿Quieres crear uno?",
                create: {
                    title: "Votante",
                    subtitle: "Crear votante",
                },
                manualVerification: {
                    label: "Verificar manualmente",
                    verify: "Verificar manualmente al votante",
                    body: "Verifique manualmente a este votante. Obtendrá un PDF con un enlace de código QR que le permite al votante iniciar sesión omitiendo el KYC en línea.",
                    noEmailOrPhone:
                        "Este votante no puede ser verificado manualmente porque no tiene una dirección de correo electrónico o un número de teléfono asociado a él.",
                },
                errors: {
                    editError: "Error editando votante",
                    editErrorReason: "Error editando votante: {{reason}}",
                    editSuccess: "Votante editado",
                    createError: "Error creando votante",
                    createErrorReason: "Error creando votante: {{reason}}",
                    createSuccess: "Votante creado",
                    attribute: {
                        invalidNamed: 'Se rechazó "{{field}}": {{constraint}}',
                        fieldsToCorrect: "Algunos campos deben corregirse antes de guardar",
                        hintBetween: "Entre {{min}} y {{max}} caracteres",
                        hintMin: "Al menos {{min}} caracteres",
                        hintMax: "Como máximo {{max}} caracteres",
                        andMore: "y {{count}} más",
                        invalidLength: '"{{field}}" debe tener entre {{min}} y {{max}} caracteres',
                        tooShort: '"{{field}}" debe tener al menos {{min}} caracteres',
                        tooLong: '"{{field}}" debe tener como máximo {{max}} caracteres',
                        required: '"{{field}}" es obligatorio',
                        invalidEmail:
                            '"{{field}}" debe ser una dirección de correo electrónico válida',
                        invalidFormat: '"{{field}}" no tiene el formato esperado',
                        invalid: '"{{field}}" tiene un valor no válido',
                    },
                    createPasswordError: "Votante creado, pero no se pudo establecer su contraseña",
                    createPasswordErrorReason:
                        "Votante creado, pero no se pudo establecer su contraseña: {{reason}}",
                },
                delete: {
                    body: "¿Estás seguro que quieres borrar este votante?",
                    bulkBody: "¿Estás seguro que quieres borrar los votantes seleccionados?",
                    bulkBodySelected:
                        "¿Borrar los {{count}} votantes seleccionados? Esta acción no se puede deshacer.",
                    bulkBodyChoose:
                        "Hay {{count}} votantes seleccionados. En su lugar, puedes borrar todos los votantes que coinciden con los filtros actuales, que pueden ser más. Esta acción no se puede deshacer.",
                    okSelected: "Borrar {{count}} seleccionados",
                    okAllMatching: "Borrar todos los que coinciden",
                },
                notifications: {
                    exportError: "Error al exportar votantes",
                    deleteError: "Error borrando votante",
                    deleteSuccess: "Votante borrado",
                    multipleDeleteSuccess: "Votantes borrado",
                    manualVerificationError: "Error verificando manualmente al votante",
                    manualVerificationSuccess:
                        "Verificado exitosamente manualmente al votante, descargar PDF..",
                },
            },
            roles: {
                title: "Roles",
                edit: {
                    title: "Información de Rol",
                    subtitle: "Ver y editar Rol",
                },
                create: {
                    title: "Rol",
                    subtitle: "Crear rol",
                },
                errors: {
                    createError: "Error creando rol",
                    createSuccess: "Rol creado",
                },
                fields: {
                    name: "Nombre",
                },
                delete: {
                    body: "¿Estás seguro que quieres borrar este rol?",
                },
                notifications: {
                    deleteError: "Error borrando rol",
                    deleteSuccess: "Rol borrado",
                    permissionEditError: "Error editando permiso",
                    permissionEditSuccess: "Permiso editado",
                },
            },
            permissions: {
                "voter-information-letter": "Generar Carta de información para el votante",
                "admin-user": "Administración",
                "admin-dashboard-view": "Vista del Panel de Administración",
                "monitoring-view": "Ver Paneles de Monitorización",
                "monitoring-configure": "Configurar Paneles de Monitorización",
                "election-event-signatures-tab": "Pestaña Firmas del Evento Electoral",
                "signing-rules-read": "Firmas: ver acciones protegidas",
                "signing-rules-write": "Firmas: editar acciones protegidas",
                "signing-certificates-read": "Firmas: ver certificados",
                "signing-issuers-write": "Firmas: importar y eliminar emisores de confianza",
                "signing-checks-write": "Firmas: editar comprobaciones de certificados",
                "signing-certificates-register": "Firmas: registrar certificados",
                "signing-certificates-revoke": "Firmas: revocar certificados",
                "signing-requests-read": "Firmas: ver solicitudes",
                "signing-requests-cancel": "Firmas: cancelar solicitudes",
                "signing-requests-export": "Firmas: exportar solicitudes",
                "sign-initialize-voting": "Firmar: inicializar la votación",
                "sign-open-voting": "Firmar: abrir la votación",
                "sign-close-voting": "Firmar: cerrar la votación",
                "sign-generate-election-returns": "Firmar: generar actas electorales",
                "sign-generate-reports": "Firmar: generar otros informes electorales",
                "sign-transmit-results": "Firmar: transmitir resultados",
                "sign-approve-voter": "Firmar: aprobar manualmente a un votante",
                "sign-approve-configuration": "Firmar: aprobar una versión de configuración",
                "sign-key-ceremony": "Firmar: confirmar un fragmento de clave",
                "sign-tally-key": "Firmar: aportar un fragmento de clave",
                "application-export": "Exportación de Aplicaciones",
                "application-import": "Importación de Aplicaciones",
                "tenant-create": "Crear Inquilino",
                "tenant-read": "Leer Inquilino",
                "tenant-write": "Editar Inquilino",
                "tenant-delete": "Eliminar Inquilino",
                "election-event-create": "Crear Evento Electoral",
                "election-event-read": "Leer Evento Electoral",
                "election-event-write": "Editar Evento Electoral",
                "keycloak-realm-attributes-read": "Leer atributos del realm de Keycloak",
                "keycloak-realm-attributes-write": "Editar atributos del realm de Keycloak",
                "election-event-delete": "Eliminar Evento Electoral",
                "voter-create": "Crear Votante",
                "voter-read": "Leer Votante",
                "voter-write": "Editar Votante",
                "voter-secret-attribute-read": "Mostrar Campos Secretos del Votante",
                "voter-secret-attribute-write": "Editar Campos Secretos del Votante",
                "user-create": "Crear Usuario",
                "user-read": "Leer Usuario",
                "user-write": "Editar Usuario",
                "user-permission-create": "Crear Permiso de Usuario",
                "user-permission-read": "Leer Permiso de Usuario",
                "user-permission-write": "Editar Permiso de Usuario",
                "role-create": "Crear Rol",
                "role-read": "Leer Rol",
                "role-write": "Editar Rol",
                "role-assign": "Asignar Rol",
                "communication-template-create": "Crear Plantilla de Comunicación",
                "communication-template-read": "Leer Plantilla de Comunicación",
                "communication-template-write": "Editar Plantilla de Comunicación",
                "notification-read": "Leer Notificación",
                "notification-write": "Editar Notificación",
                "notification-send": "Enviar Notificación",
                "area-read": "Leer Área",
                "area-write": "Editar Área",
                "election-state-write": "Editar Estado de Elección",
                "election-type-create": "Crear Tipo de Elección",
                "election-type-read": "Leer Tipo de Elección",
                "election-type-write": "Editar Tipo de Elección",
                "voting-channel-read": "Leer Canal de Votación",
                "voting-channel-write": "Editar Canal de Votación",
                "trustee-create": "Crear Fideicomisario",
                "trustee-read": "Leer Fideicomisario",
                "trustee-write": "Editar Fideicomisario",
                "tally-read": "Leer Recuento",
                "tally-start": "Iniciar Recuento",
                "tally-write": "Editar Recuento",
                "tally-results-read": "Leer Resultados de Recuento",
                "publish-read": "Leer Publicación",
                "publish-write": "Editar Publicación",
                "publish-results-read": "Leer Publicación de Resultados",
                "publish-results-write": "Editar Publicación de Resultados",
                "logs-read": "Leer Registros",
                "electoral-log-audit": "Auditar Registro Electoral",
                "electoral-log-console-read": "Explorar Registro Electoral",
                "electoral-log-console-query": "Consultar Registro Electoral",
                "electoral-log-personal-data-read": "Leer Datos Personales del Registro Electoral",
                "task-queues-read": "Leer Colas de Tareas",
                "task-queues-write": "Gestionar Colas de Tareas",
                "task-queues-query": "Consultar Colas de Tareas",
                "tasks-read": "Leer la Ejecución de Tareas",
                "keys-read": "Leer Claves",
                "document-upload": "Subir Documentos",
                "document-download": "Descargar Documentos",
                "document-password-read": "Leer contraseñas de documentos",
                "tally-sheet-create": "Crear Acta de Recuento",
                "tally-sheet-import-create": "Crear importación de actas de recuento",
                "tally-sheet-import-review": "Revisar importación de actas de recuento",
                "tally-sheet-import-view": "Ver importación de actas de recuento",
                "tally-recount-execute": "Ejecutar recuento de resultados",
                "trustee-ceremony": "Ceremonia de Fideicomisario",
                "tally-sheet-review": "Revisar hoja de recuento",
                "tally-sheet-view": "Ver Acta de Recuento",
                "admin-ceremony": "Administrar Ceremonia de Claves",
                "tally-sheet-delete": "Borrar Acta de Recuento",
                "document-read": "Leer Documentos",
                "document-write": "Editar Documentos",
                "support-material-read": "Leer Materiales de Soporte",
                "support-material-write": "Editar Materiales de Soporte",
                "miru-create": "Crear Miru",
                "miru-download": "Descargar Miru",
                "miru-send": "Enviar Miru",
                "miru-sign": "Firmar Miru",
                "contest-write": "Editar Pregunta",
                "contest-read": "Leer Pregunta",
                "candidate-write": "Editar Candidatos",
                "candidate-read": "Leer Candidatos",
                "permission-label-write": "Editar la etiqueta de permiso",
                "scheduled-event-write": "Editar Eventos Programados",
                "contest-create": "Crear Pregunta",
                "contest-delete": "Eliminar Pregunta",
                "candidate-create": "Crear Candidato",
                "candidate-delete": "Eliminar Candidato",
                "election-create": "Crear Elección",
                "election-read": "Leer Elección",
                "election-write": "Editar Elección",
                "election-delete": "Eliminar Elección",
                "election-event-archive": "Archivar Evento Electoral",
                "election-data-tab": "Ver Datos de la Elección",
                "election-event-areas-tab": "Ver Áreas del Evento Electoral",
                "election-event-data-tab": "Ver Datos del Evento Electoral",
                "election-event-keys-tab": "Ver Claves del Evento Electoral",
                "election-event-logs-tab": "Ver Registros del Evento Electoral",
                "election-event-publish-tab": "Ver Publicación del Evento Electoral",
                "election-event-reports-tab": "Ver Informes del Evento Electoral",
                "election-event-scheduled-tab": "Ver Programación del Evento Electoral",
                "election-event-tally-tab": "Ver Escrutinio del Evento Electoral",
                "election-event-tasks-tab": "Ver Tareas del Evento Electoral",
                "election-event-voters-tab": "Ver Votantes del Evento Electoral",
                "election-publish-tab": "Ver Publicación de la Elección",
                "election-voters-tab": "Ver Votantes de la Elección",
                "report-write": "Editar Informes",
                "report-read": "Leer Informes",
                "users-menu": "Ver usuarios y roles",
                "settings-menu": "Ver configuración",
                "templates-menu": "Ver plantillas",
                "settings-election-types-tab": "Ver configuración de tipos de elección",
                "settings-voting-channels-tab": "Ver configuración de canales de votación",
                "settings-templates-tab": "Ver configuración de plantillas",
                "settings-languages-tab": "Ver configuración de idiomas",
                "settings-localization-tab": "Ver configuración de localización",
                "settings-look-feel-tab": "Ver configuración de apariencia",
                "settings-trustees-tab": "Ver configuración de fideicomisarios",
                "settings-countries-tab": "Ver configuración de países",
                "voter-import": "Importar Votante",
                "ee-voters-columns": "Ver Columnas de Votantes del Evento Electoral",
                "voter-manually-verify": "Verificar Votante Manualmente",
                "ee-voters-logs": "Ver Registros de Votantes del Evento Electoral",
                "voter-export": "Exportar Votante",
                "ee-voters-filters": "Ver Filtros de Votantes del Evento Electoral",
                "voter-delete": "Eliminar Votante",
                "voter-change-password": "Cambiar la Contraseña del Votante",
                "election-event-localization-selector":
                    "Selector de Localización del Evento Electoral",
                "localization-create": "Crear Localización",
                "localization-read": "Leer Localización",
                "localization-write": "Editar Localización",
                "localization-delete": "Eliminar Localización",
                "area-create": "Crear Área",
                "area-delete": "Eliminar Área",
                "area-export": "Exportar Área",
                "area-import": "Importar Área",
                "area-upsert": "Insertar o Actualizar Área",
                "election-event-areas-columns": "Columnas de las Áreas del Evento Electoral",
                "election-event-areas-filters": "Filtros de las Áreas del Evento Electoral",
                "election-event-tasks-back-button": "Volver a las Tareas del Evento Electoral",
                "election-event-tasks-columns": "Columnas de las Tareas del Evento Electoral",
                "election-event-tasks-filters": "Filtros de las Tareas del Evento Electoral",
                "task-export": "Exportar Tareas",
                "application-read": "Leer Aplicación",
                "application-write": "Editar Aplicación",
                "approval-matrix-write": "Editar la Matriz de Aprobación",
                "readiness-check": "Verificar la preparación electoral",
                "forensic-read": "Leer datos forenses de red aislados",
                "voter-pseudonym-resolve":
                    "Resolver un seudónimo de votante con un motivo auditado",
                "logs-export": "Exportar Registros",
                "tenant-logs-read": "Leer Bitácora del Inquilino",
                "tenant-logs-export": "Exportar Bitácora del Inquilino",
                "election-event-logs-columns": "Columnas de los Registros del Evento Electoral",
                "election-events-logs-filters": "Filtros de los Registros del Evento Electoral",
                "election-event-scheduled-event-columns":
                    "Columnas de los Eventos Programados del Evento Electoral",
                "scheduled-event-create": "Crear Evento Programado",
                "scheduled-event-delete": "Eliminar Evento Programado",
                "election-event-reports-columns": "Columnas de los Informes del Evento Electoral",
                "report-create": "Crear Informe",
                "report-delete": "Eliminar Informe",
                "report-generate": "Generar Informe",
                "report-preview": "Vista Previa del Informe",
                "monitor-authenticated-voters": "Monitoreo de Votantes Autenticados",
                "monitor-all-approve-disapprove-voters":
                    "Leer Monitoreo de Votantes Aprobados y Rechazados",
                "monitor-automatic-approve-disapprove-voters":
                    "Leer Monitoreo de Aprobaciones y Rechazos Automáticos",
                "monitor-manually-approve-disapprove-voters":
                    "Leer Monitoreo de Aprobaciones y Rechazos Manuales",
                "monitor-enrolled-overseas-voters":
                    "Leer Monitoreo de Votantes Inscritos en el Extranjero",
                "monitor-posts-already-closed-voting":
                    "Leer Monitoreo de Publicaciones con Votación Cerrada",
                "monitor-posts-already-generated-election-results":
                    "Leer Monitoreo de Publicaciones con Resultados Generados",
                "monitor-posts-already-opened-voting":
                    "Leer Monitoreo de Publicaciones con Votación Abierta",
                "monitor-posts-already-started-counting-votes":
                    "Leer Monitoreo de Publicaciones que Comenzaron a Contar Votos",
                "monitor-posts-initialized-the-system":
                    "Leer Monitoreo de Publicaciones que Inicializaron el Sistema",
                "monitor-posts-started-voting":
                    "Leer Monitoreo de Publicaciones que Comenzaron a Votar",
                "monitor-posts-transmitted-results":
                    "Leer Monitoreo de Publicaciones que Transmitieron Resultados",
                "monitor-voters-voted-test-election":
                    "Leer Monitoreo de Votantes en Elección de Prueba",
                "monitor-voters-who-voted": "Leer Monitoreo de Votantes que Votaron",
                "election-event-publish-preview":
                    "Vista Previa de la Publicación del Evento Electoral",
                "election-event-publish-back-button":
                    "Volver a la Publicación del Evento Electoral",
                "election-event-publish-columns": "Columnas de la Publicación del Evento Electoral",
                "election-event-publish-filters": "Filtros de la Publicación del Evento Electoral",
                "publish-create": "Crear Publicación",
                "publish-regenerate": "Regenerar Publicación",
                "publish-export": "Exportar Publicación",
                "publish-start-voting": "Iniciar Votación",
                "publish-pause-voting": "Pausar Votación",
                "publish-stop-voting": "Detener Votación",
                "publish-changes": "Publicar Cambios",
                "election-event-publish-view": "Ver Publicación del Evento Electoral",
                "election-event-keys-columns": "Columnas de las Claves del Evento Electoral",
                "create-ceremony": "Crear Ceremonia",
                "export-ceremony": "Exportar Ceremonia",
                "election-event-tally-columns": "Columnas del Recuento del Evento Electoral",
                "election-event-tally-back-button": "Volver al Recuento del Evento Electoral",
                "transmition-ceremony": "Ceremonia de Transmisión",
                "admin-ip-address-view": "Ver Dirección IP",
                "election-approvals-tab": "Ver Aprobaciones de la Elección",
                "election-event-approvals-tab": "Ver Aprobaciones del Evento Electoral",
                "election-ip-address-view": "Ver Dirección IP de la Elección",
                "election-dashboard-tab": "Ver Dashboard de la Elección",
                "trustees-export": "Exportar Fideicomisarios",
                "user-import": "Importar Usuarios",
                "voter-voted-edit": "Editar a los votantes que votaron",
                "voter-email-tlf-edit": "Editar email/teléfono de los votantes",
                "cloudflare-write": "Editar las reglas de bloqueo por país en Cloudflare",
                "transmission-report-generate": "Generar Informe de Transmisión",
                "google-meet-link": "Generar Enlace de Google Meet",
                "service-account": "Cuenta de servicio",
                "datafix-account": "Cuenta de corrección de datos",
                "gold": "Oro",
                "silver": "Plata",
                "election-event-ivr-tab": "Ver IVR del evento electoral",
                "election-event-cas-tab": "Ver CAS del evento electoral",
                "ca-read": "Consultar autoridades de certificación",
                "ca-write": "Editar autoridades de certificación",
                "generate-preview": "Generar vista previa",
                "preview-read": "Consultar vista previa",
                "tally-resolution-submit": "Enviar resolución del escrutinio",
                "phone-blacklist-read": "Consultar lista negra de teléfonos",
                "phone-blacklist-create": "Crear entradas en la lista negra de teléfonos",
                "phone-blacklist-update": "Editar entradas de la lista negra de teléfonos",
                "phone-blacklist-delete": "Eliminar entradas de la lista negra de teléfonos",
                "election-event-voter-list-reconciliation":
                    "Conciliar la lista de votantes del evento electoral",
                "messaging-account-read": "Ver cuentas de mensajería",
                "messaging-account-write": "Gestionar cuentas de mensajería",
                "messaging-config-write": "Configurar la mensajería del evento electoral",
                "messaging-config-read": "Ver la mensajería del evento electoral",
            },
        },
        generalSettingsScreen: {
            body: "Active los idiomas en el sistema. Solo los idiomas activados aquí estarán disponibles para eventos electorales.",
        },
        eventsScreen: {
            title: "Eventos Programados",
            subtitle:
                "Gestiona la configuración de la ejecución automática de eventos como el inicio o fin del período de votación.",
            messages: {
                createSuccess: "Evento Programado creado con éxito",
                createError: "Error al crear el Evento Programado",
                editSuccess: "Evento Programado editado con éxito",
                editError: "Error al editar el Evento Programado",
                onlineWithEarlyVoting:
                    "Una programación de inicio no puede abrir a la vez el voto en línea y el voto anticipado: el voto anticipado debe empezar antes que el voto en línea.",
            },
            eventType: {
                label: "Tipo",
                ALLOW_INIT_REPORT: "Permitir informe de inicialización",
                START_VOTING_PERIOD: "Inicio del Período de Votación",
                END_VOTING_PERIOD: "Fin del Período de Votación",
                ALLOW_VOTING_PERIOD_END: "Permitir fin del periodo de votación",
                START_ENROLLMENT_PERIOD: "Inicio del Período de Inscripción",
                END_ENROLLMENT_PERIOD: "Fin del Período de Inscripción",
                START_LOCKDOWN_PERIOD: "Inicio del Período de Bloqueo de Datos Censales",
                END_LOCKDOWN_PERIOD: "Fin del Período de Bloqueo de Datos Censales",
                ALLOW_TALLY: "Permitir Recuento",
                START_READINESS_TEST: "Iniciar prueba de preparación electoral",
                END_READINESS_TEST: "Finalizar prueba de preparación electoral",
                START_FINAL_TESTING: "Iniciar pruebas finales y bloqueo",
                END_FINAL_TESTING: "Finalizar pruebas finales y bloqueo",
                START_TEST_VOTING: "Iniciar votación de prueba",
                END_TEST_VOTING: "Finalizar votación de prueba",
            },
            warning: {
                votingWindowDays:
                    "El periodo de votación de {{election}} abarca {{days}} días locales (del {{start_local}} al {{end_local}}, {{time_zone}}); la regla pide {{expected}}.",
                finalTestingLeadTime:
                    "Las pruebas finales de {{election}} empiezan el {{final_testing_local}}, menos de {{minimum_days}} días antes de que se abra la votación el {{voting_start_local}} ({{time_zone}}).",
                closeBeforeOpen:
                    "La votación de {{election}} se cierra antes o en el momento de abrirse ({{start_local}} a {{end_local}}, {{time_zone}}).",
                shortLastDay:
                    "El último día de votación de {{election}} tiene {{hours}} horas, menos de {{minimum_hours}}: la votación se cierra el {{end_local}} ({{time_zone}}).",
            },
            election: {
                label: "Elección",
            },
            empty: {
                header: "No hay Eventos Programados todavía.",
                body: "¿Quieres crear uno?",
                button: "Crear Evento Programado",
            },
            create: {
                title: "Crear Evento Programado",
                subtitle: "Crea una nueva configuración de Evento Programado.",
            },
            edit: {
                title: "Editar Evento Programado",
                subtitle: "Edita la configuración del Evento Programado.",
                delete: "¿Estás seguro de que quieres eliminar este Evento Programado?",
            },
            fields: {
                electionId: "Elección",
                eventProcessor: "Tipo",
                stoppedAt: "Detenido En",
                scheduledDate: "Programado En",
            },
        },
        reportsScreen: {
            title: "Informes",
            subtitle: "Generar informes para los eventos electorales",
            messages: {
                createSuccess: "Informe creado con éxito",
                createError: "Error al crear el informe",
                submitError: "Error al enviar el Informe",
                updateSuccess: "Informe actualizado con éxito",
                passwordMismatch:
                    "Las contraseñas no coinciden. Asegúrate de que ambos campos contengan la misma contraseña.",
                incorectPassword: "Contraseña incorrecta",
                decryptFileTitle: "Descifrar archivo",
                decryptInstructions:
                    "1. '-in': La ruta al archivo cifrado. \n2. '-out': La ruta donde se guardará el archivo descifrado. \n3. '-pass': La contraseña utilizada para cifrar el archivo. \n",
                encryptSuccess: "Configuración de cifrado del informe exitosa",
                encryptError: "Error al configurar el cifrado del informe",
            },
            reportType: {
                BALLOT_RECEIPT: "Recibo de Voto",
                ELECTORAL_RESULTS: "Resultados Electorales",
                MANUAL_VERIFICATION: "Verificación Manual",
                PARTICIPATION_REPORT: "Informe de Participación",
                STATISTICAL_REPORT: "Informe Estadístico",
                OVCS_EVENTS: "Monitoreo de Votación en el Extranjero - Eventos OVCS",
                AUDIT_LOGS: "Registros de Auditoría",
                ACTIVITY_LOG: "Registros de Actividad",
                STATUS: "Estado",
                OVCS_INFORMATION: "Información de OVCS",
                OVERSEAS_VOTERS: "Lista de Votantes en el Extranjero",
                OV_USERS_WHO_VOTED: "Lista de Votantes en el Extranjero que Votaron",
                OV_WITH_VOTING_STATUS: "Lista de Votantes en el Extranjero con Estado de Votación",
                OVCS_STATISTICS: "Monitoreo de Votación en el Extranjero - Estadísticas OVCS",
                PRE_ENROLLED_OV_BUT_DISAPPROVED:
                    "Lista de Votantes en el Extranjero que se Preinscribieron pero Fueron Rechazados",
                PRE_ENROLLED_OV_SUBJECT_TO_MANUAL_VALIDATION:
                    "Lista de Votantes en el Extranjero que se Preinscribieron pero Requieren Validación Manual",
            },
            reportEncryptionPolicy: {
                title: "Política de cifrado",
                UNENCRYPTED: "Sin cifrar",
                CONFIGURED_PASSWORD: "Contraseña configurada",
            },
            empty: {
                header: "No hay informes todavía.",
                body: "¿Quieres crear uno?",
                button: "Crear informe",
            },
            create: {
                title: "Crear informe",
                subtitle: "Crear una nueva configuración de informe.",
            },
            edit: {
                title: "Editar informe",
                subtitle: "Editar configuración del informe.",
                delete: "¿Estás seguro de que quieres eliminar este informe?",
            },
            fields: {
                electionId: "Elección",
                template: "Plantilla",
                reportType: "Tipo de informe",
                repeatable: "Repetible",
                cronExpression: "Expresión Cron",
                emailRecipients: "Destinatarios de correo electrónico",
                emailRecipientsPlaceholder: "Escribe el correo electrónico y presiona Enter",
            },
            delete: {
                body: "¿Estás seguro de que quieres eliminar este informe?",
            },
            actions: {
                generate: "Generar",
                delete: "Eliminar",
                edit: "Editar",
                preview: "Vista previa",
            },
        },
        googleMeet: {
            title: "Generar Enlace de Google Meet",
            generateButton: "Google Meet",
            meetingTitle: "Título de la Reunión",
            description: "Descripción (Opcional)",
            startDate: "Fecha de Inicio",
            startTime: "Hora de Inicio",
            duration: "Duración (minutos)",
            attendeeEmails: "Emails de los Participantes",
            attendeeEmailHelp: "Emails separados por comas para los participantes de la reunión",
            note: "Nota: Esto creará un evento de calendario en tu Google Calendar con un enlace de Google Meet. Necesitarás iniciar sesión en tu cuenta de Google.",
            success: "¡Enlace de Google Meet Generado Exitosamente!",
            copy: "Copiar al portapapeles",
            copied: "¡Enlace copiado al portapapeles!",
            instructions:
                "Comparte este enlace con los participantes para unirse a la reunión. El evento de calendario ha sido añadido a tu Google Calendar.",
            generating: "Generando...",
            generate: "Generar Enlace de Meet",
        },
        common: {
            export: "La exportación puede ser un proceso largo. ¿Estás seguro que quieres exportar?",
            resources: {
                electionEvent: "Evento Electoral",
                election: "Elección",
                contest: "Pregunta",
                candidate: "Candidato",
                noResult: {
                    askCreate: "¿Quieres crear una?",
                },
            },
            label: {
                add: "Añadir",
                actions: "Acciones",
                create: "Crear",
                delete: "Borrar",
                archive: "Archivar",
                unarchive: "Desarchivar",
                cancel: "Cancelar",
                edit: "Editar",
                yes: "Sí",
                no: "No",
                save: "Guardar",
                close: "Cerrar",
                back: "Atrás",
                next: "Siguiente",
                warning: "Aviso",
                json: "Vista previa",
                noResult: "Sin resultados",
                import: "Importar",
                export: "Exportar",
                loadingData: "Cargando datos ...",
                exportFormat: "Exportar en formato {{format}} - Resultados de '{{item}}'",
                allResults: "del evento electoral",
                globalAreaResults: "de todas las áreas",
                title: "Título",
                subtitle: "Subtítulo",
                kind: "Tipo de archivo",
                filter: "Filtros Personalizados",
                approve: "Aprobar",
                continue: "Continuar",
                logout: "Cerrar Sesión",
                selectTenant: "Seleccionar Inquilino",
                processing: "Procesando...",
                tenantName: "Nombre del Inquilino",
            },
            language: {
                es: "Español",
                en: "Inglés",
                fr: "Francés",
                cat: "Valenciano",
                tl: "Tagalo",
                gl: "Galego",
                nl: "Holandes",
                eu: "Euskera",
            },
            channel: {
                online: "En línea",
                kiosk: "Kiosco",
                early_voting: "Votación anticipada",
                telephone: "Votación telefónica",
                other: "Otros",
            },
            message: {
                delete: "¿Estás seguro que quieres borrar este elemento?",
                continueOrLogout: "¿Quieres continuar o cerrar sesión?",
            },
        },
        createResource: {
            electionEvent: "Crear un Evento Electoral",
            election: "Crear una Elección",
            contest: "Crear una Pregunta",
            candidate: "Crear un Candidato",
        },
        importResource: {
            electionEvent: "Importar un Evento Electoral",
            election: "Importar una Elección",
            contest: "Importar una Pregunta",
            candidate: "Importar un Candidato",
            ImportHashMismatch: "Los hashes no coinciden. Fallo en la comprobación de integridad.",
        },
        sideMenu: {
            electionEvents: "Procesos Electorales",
            search: "Buscar",
            usersAndRoles: "Usuarios y Roles",
            electoralLogConsole: "Registro Electoral",
            taskQueues: "Colas de Tareas",
            logs: "Bitácora",
            settings: "Configuracion",
            help: "Ayuda",
            templates: "Plantillas",
            active: "Activos",
            archived: "Archivados",
            addResource: {
                electionEvent: "Crear un Evento Electoral",
                election: "Crear una Elección",
                contest: "Crear una Pregunta",
                candidate: "Crear un Candidato",
            },
            menuActions: {
                archive: {
                    electionEvent: "Archivar este Evento Electoral",
                },
                unarchive: {
                    electionEvent: "Desarchivar este Evento Electoral",
                    election: "Desarchivar esta Elección",
                    contest: "Desarchivar esta Pregunta",
                    candidate: "Desarchivar este Candidato",
                },
                remove: {
                    electionEvent: "Eliminar este Evento Electoral",
                    election: "Eliminar esta Elección",
                    contest: "Eliminar esta Pregunta",
                    candidate: "Eliminar este Candidato",
                },
                messages: {
                    confirm: {
                        archive: "¿Está seguro de que desea archivar este elemento?",
                        unarchive: "¿Está seguro de que desea desarchivar este elemento?",
                        delete: "¿Está seguro de que desea eliminar este elemento?",
                        sealsUnknown:
                            "No se pudieron comprobar los sellos de las urnas: si tiene urnas selladas, se rechazará el borrado.",
                        archiveNamed: '¿Está seguro de que desea archivar "{{name}}"?',
                        unarchiveNamed: '¿Está seguro de que desea desarchivar "{{name}}"?',
                        deleteNamed: '¿Está seguro de que desea eliminar "{{name}}"?',
                    },
                    notification: {
                        success: {
                            archive: "El elemento ha sido archivado",
                            unarchive: "El elemento ha sido desarchivado",
                            delete: "El elemento ha sido eliminado",
                            reloading: "Espere. La página se recargara en unos momentos",
                        },
                        error: {
                            archive: "Error al intentar archivar este elemento",
                            unarchive: "Error al intentar desarchivar este elemento",
                            delete: "Error al intentar eliminar este elemento",
                            deleteSealedElection:
                                "Esta elección tiene urnas selladas y no se puede borrar. Archive su evento electoral en su lugar.",
                            deleteSealedEvent:
                                "Este evento electoral tiene urnas selladas y no se puede borrar. Archívelo en su lugar.",
                            deleteMaybeSealed:
                                "Error al intentar borrar este elemento. Si tiene urnas selladas, no se puede borrar.",
                        },
                    },
                },
            },
        },
        candidateScreen: {
            common: {
                subtitle: "Configuración de candidatos.",
            },
            edit: {
                externalId: "ID externo",
                general: "General",
                type: "Tipo",
                image: "Imagen",
                isDisabled: "Deshabilitado",
                isExplicitInvalid: "Voto Inválido",
                isExplicitBlank: "Voto en Blanco",
                isCategoryList: "Lista",
                isWriteIn: "Voto por Escrito",
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
            invalidVotePosition: {
                label: "Posición del Voto Inválido",
                null: "Ninguno (Por defecto)",
                top: "Arriba",
                bottom: "Abajo",
            },
            error: {},
            createCandidateSuccess: "Candidato creado",
            createCandidateError: "Error creating candidato",
        },
        contestScreen: {
            common: {
                subtitle: "Configuración de pregunta.",
            },
            edit: {
                externalId: "ID externo",
                general: "General",
                type: "Tipo",
                image: "Imagen",
                system: "Sistema de votación de papeletas",
                design: "Diseño de la papeleta",
                reorder: "Reordernar candidatos",
                policies: "Políticas",
            },
            field: {
                name: "Nombre",
                alias: "Alias",
                description: "Descripción",
            },
            options: {
                "non-preferential": "Sin Preferencia",
                "plurality-at-large": "Mayoría Plural",
                "instant-runoff": "Segunda Vuelta Instantánea",
                "random": "Aleatorias",
                "external-procedure": "Procedimiento externo",
                "custom": "Personalizado",
                "alphabetical": "Alfabético",
            },
            tieBreakingPolicy: {
                label: "Política de desempate",
            },
            auditButtonConfig: {
                "label": "Opciones de visualización del botón de auditoría",
                "show": "Mostrar",
                "not-show": "No mostrar",
                "show-in-help": "Mostrar en el cuadro de diálogo de ayuda",
            },
            underVotePolicy: {
                "label": "Política de Subvoto",
                "allowed": "Permitido",
                "warn-only-in-review": "Advertir en Revisión",
                "warn": "Advertir",
                "warn-and-alert": "Advertir y Alertar",
                "warn-and-confirm-in-review": "Advertir y Confirmar en Revisión",
            },
            invalidVotePolicy: {
                "label": "Política de Voto Inválido",
                "allowed": "Permitido",
                "warn": "Advertencia",
                "warn-invalid-implicit-and-explicit": "Advertir Inválidos Implícitos y Explícitos",
                "not-allowed": "No Permitido",
                "allowed-with-exclusive-explicit": "Permitido con Voto Inválido Exclusivo",
            },
            candidatesIconCheckboxPolicy: {
                "label": "Forma del icono de la casilla de verificación de los candidatos",
                "square-checkbox": "Casilla de verificación cuadrada",
                "round-checkbox": "Casilla de verificación redonda",
            },
            checkableListPolicy: {
                "allow-selecting-candidates-and-lists": "Candidatos Y Listas",
                "allow-selecting-candidates": "Sólo Candidatos",
                "allow-selecting-lists": "Sólo Listas",
                "disabled": "Deshabilitado",
            },
            collapsibleListsPolicy: {
                "label": "Listas plegables",
                "disabled": "Deshabilitado",
                "enabled-expanded": "Habilitado (empieza expandido)",
                "enabled-collapsed": "Habilitado (empieza contraído)",
            },
            blankVotePolicy: {
                "label": "Política de voto en blanco",
                "allowed": "Permitido",
                "warn-only-in-review": "Advertir en Revisión",
                "warn": "Advertir",
                "not-allowed": "No permitido",
            },
            overVotePolicy: {
                "label": "Política de voto excesivo",
                "allowed": "Permitido",
                "allowed-with-msg": "Permitido con un mensaje de warning",
                "allowed-with-msg-and-alert": "Permitido con un mensaje de warning y alerta",
                "not-allowed-with-msg-and-alert": "No permitido con un mensaje de warning y alerta",
                "not-allowed-with-msg-and-disable":
                    "No permitido con un mensaje de warning y dehabilitar selecciones adicionales",
            },
            duplicatedRankPolicy: {
                "label": "Voto inválido - Política de rango duplicado",
                "allowed-warn-and-dialog":
                    "Mostrar advertencia y diálogo (el votante puede continuar)",
                "not-allowed-warn-and-dialog":
                    "Mostrar advertencia y diálogo (el votante no puede continuar)",
            },
            preferenceGapsPolicy: {
                "label": "Voto inválido - Política de rangos omitidos",
                "allowed-warn-and-dialog":
                    "Mostrar advertencia y diálogo (el votante puede continuar)",
                "not-allowed-warn-and-dialog":
                    "Mostrar advertencia y diálogo (el votante no puede continuar)",
            },
            paginationPolicy: {
                label: "Nombre de la página",
            },
            isAcclaimed: {
                label: "Resuelto por aclamación",
                helperText:
                    "Los votantes ven esta votación pero no pueden seleccionar nada, no se registra nada y todas las candidaturas se declaran ganadoras con cero votos. Configúrelo antes de publicar las papeletas: cambiarlo después invalida las papeletas ya emitidas.",
            },
            allowWriteins: {
                label: "Permitir candidaturas manuales",
            },
            maxVotes: {
                helperText:
                    "Número máximo de candidatos que un votante puede seleccionar (votación no preferencial).",
                helperTextPreferential:
                    "Posición de rango más alta disponible para los votantes (p.ej. '5' significa posiciones 1–5). Debe ser al menos igual al número de candidatos a ordenar (votación preferencial).",
            },
            error: {},
            createContestSuccess: "Pregunta creada",
            createContestError: "Error al crear la pregunta",
        },
        keysGeneration: {
            ...englishTranslation.translations.keysGeneration,
            downloadStep: {
                ...englishTranslation.translations.keysGeneration.downloadStep,
                title: "Descargar archivo de custodia",
                downloadButton: "Descargar archivo de custodia",
                confirmdDialog: {
                    ...englishTranslation.translations.keysGeneration.downloadStep.confirmdDialog,
                    title: "Guardar archivo de custodia",
                },
            },
            checkStep: {
                ...englishTranslation.translations.keysGeneration.checkStep,
                subtitle: "Carga tu archivo de custodia guardado o una prueba de custodia firmada:",
                required: "Se requiere confirmar la custodia",
                reconfirmButton: "Volver a confirmar la custodia (opcional)",
                reconfirmDescription:
                    "Tu custodia ya está confirmada. Esta confirmación adicional es opcional.",
            },
            serverCustody: {
                recoveryInstructions:
                    "Si ha importado este evento electoral o ha perdido el archivo guardado, descargue aquí el archivo de custodia actual. Guárdelo y luego cárguelo a continuación.",
                fileFormat: "Formato admitido: JSON",
                waiting: "Esperando a que tu servidor custodio cree una prueba de custodia…",
                unavailable:
                    "El servidor custodio no ha respondido. Comprueba que está activo y vuelve a cargar el archivo de custodia.",
                failed: "El servidor custodio no pudo crear la prueba: {{error}}",
                invalidReply:
                    "El servidor custodio devolvió una respuesta incompleta. Inténtalo de nuevo.",
                cancelled: "La solicitud se canceló. Vuelve a cargar el archivo para continuar.",
                downloadInstructions:
                    "Descarga y guarda este archivo de custodia. Carga el mismo archivo en el siguiente paso y en futuros recuentos. No es una copia de seguridad de tus claves privadas.",
                uploadInstructions:
                    "Carga el archivo de custodia descargado. Tu servidor custodio creará la prueba. También puedes cargar una prueba creada por un custodio sin conexión o de navegador.",
                tallyInstructions:
                    "Carga el archivo guardado durante la ceremonia de claves. Tu servidor custodio creará una nueva prueba para este recuento. También se aceptan pruebas de custodios sin conexión o de navegador.",
            },
            configureStep: {
                create: "Crear Ceremonia de Claves",
                name: "Nombre de la Ceremonia de Claves",
                allElections: "Todas las Elecciones",
                title: "Crear Ceremonia de Claves del Evento Electoral",
                subtitle:
                    "Cada autoridad genera y conserva sus claves localmente y confirma su custodia. Elija las autoridades y el número mínimo necesario para el recuento.",
                trusteeList: "Autoridades",
                threshold: "Umbral",
                errorMinTrustees_one:
                    "Seleccionaste sólo {{selected}} autoridad, pero debes seleccionar al menos {{threshold}}.",
                errorMinTrustees_many:
                    "Seleccionaste sólo {{selected}} autoridades, pero debes seleccionar al menos {{threshold}}.",
                errorMinTrustees_other:
                    "Seleccionaste sólo {{selected}} autoridades, pero debes seleccionar al menos {{threshold}}.",
                errorThreshold:
                    "Seleccionaste un umbral de {{selected}} pero debe estar entre {{min}} y {{max}}.",
                errorCreatingCeremony: "Error creando Ceremonia de Claves: {{error}}",
                createCeremonySuccess: "Ceremonia de Claves creada",
                confirmdDialog: {
                    ok: "Sí, Crear Ceremonia de Claves",
                    cancel: "Cancelar",
                    title: "¿Estás seguro de que quieres Crear una Ceremonia de Claves?",
                    automaticCeremonyTitle:
                        "¿Estás seguro de que quieres crear una ceremonia de llaves automática?",
                    description:
                        "Estás a punto de Crear una Ceremonia de Claves. Esta acción notificará a las Autoridades para participar en la creación y distribución de las Claves del Evento Electoral.",
                    automaticCeremonyDescription:
                        "Las ceremonias automáticas omiten las confirmaciones manuales de custodia. Los ejecutores de las autoridades deben generar y verificar las claves electorales.",
                },
                filterTrustees: "Filtrar Autoridades",
                errorPermisionLabels:
                    "No se puede crear la ceremonia de claves: falta una o más etiquetas de permisos.",
                automaticCeremonyToggle: "Ceremonia automática",
            },
            // Native custody instructions use the English fallback until translated.
            ceremonyStep: {
                ...englishTranslation.translations.keysGeneration.ceremonyStep,
                custodyConfirmed: "Custodia confirmada",
                custodyComplete:
                    "Tu parte ha finalizado. No necesitas realizar ninguna otra acción.",
                waitingForTrustees:
                    "Esperando a que las autoridades restantes completen la confirmación de custodia y cualquier aprobación requerida.",
                moreActions: "Más acciones",
                cancel: "Cancelar Ceremonia de Claves",
                progressHeader: "Progreso de Ceremonia de Claves",
                executionStatus: "Estado: {{status}}",
                confirmdDialog: {
                    ok: "Sí, Cancelar Creación de Ceremonia de Claves",
                    cancel: "Volver a la Ceremonia de Claves",
                    title: "¿Estás seguro de que quieres Cancelar la Ceremonia de Claves?",
                    description:
                        "Estás a punto de Cancelar la Ceremonia de Claves. Después de realizar esta acción, para tener una Ceremonia de Claves exitosa tendrás que Crear una nueva.",
                },
                header: {
                    ...englishTranslation.translations.keysGeneration.ceremonyStep.header,
                    trusteeName: "Nombre de Autoridad",
                    fragment: "Fragmento de Clave Generado",
                },
                logsHeader: {
                    title: "Logs",
                    date: "Fecha",
                    entry: "Entrada",
                },
                emptyLogs: "Sin logs aún.",
            },
        },
        miruExport: {
            create: {
                success: "Paquete de Transmisión Creado",
                error: "Error al crear el Paquete de Transmisión",
            },
            send: {
                success: "Paquete de Transmisión Enviado",
                error: "Error al enviar el Paquete de Transmisión",
            },
        },
        tally: {
            errorUploadingSignature: "Hubo un error al subir la firma",
            downloadTransmissionPackage: "Descargar paquete",
            resultsPublication: {
                sectionTitle: "Publicar en el sitio web de resultados",
                policyTitle: "Sitio web de resultados",
                policyAccess: "Acceso al sitio web de resultados",
                policyVisibility: "Visibilidad del sitio web de resultados",
                enabled: "Activado",
                disabled: "Desactivado",
                fullEvent: "Evento completo",
                areaBased: "Por área",
                publishStarted: "Publicación de resultados iniciada",
                publishError: "No se pudo iniciar la publicación de resultados",
                revoked: "Publicación de resultados revocada",
                revokeError: "No se pudo revocar la publicación de resultados",
                waitingForTally:
                    "Los resultados se pueden publicar cuando este recuento haya finalizado.",
                writePermissionRequired:
                    "Necesitas el permiso publish-results-write para publicar o revocar resultados.",
                readPermissionRequired:
                    "Necesitas el permiso publish-results-read para ver el historial de publicaciones.",
                disabledPolicy:
                    "La publicación en el sitio web de resultados está desactivada para este evento electoral. Actívala en los datos del evento electoral antes de publicar resultados.",
                loadingElectionContext:
                    "La publicación de resultados está cargando el contexto de la elección.",
                route: "Ruta",
                eventResults: "Resultados del evento",
                electionResults: "Resultados de la elección",
                election: "Elección",
                access: "Acceso",
                publicAccess: "Acceso público",
                authenticatedAccess: "Acceso autenticado",
                visibility: "Visibilidad",
                fullPublishedScope: "Ámbito publicado completo",
                personalVisibility: "Visibilidad personal",
                contests: "Preguntas",
                noTalliedContests: "No hay preguntas escrutadas disponibles.",
                publishSelectedContests: "Publicar las preguntas seleccionadas",
                selectedContestCount_one: "{{count}} pregunta seleccionada",
                selectedContestCount_many: "{{count}} preguntas seleccionadas",
                selectedContestCount_other: "{{count}} preguntas seleccionadas",
                history: "Historial de publicaciones",
                version: "Versión",
                status: "Estado",
                published: "Publicado",
                revokedAt: "Revocado",
                actions: "Acciones",
                open: "Abrir",
                revoke: "Revocar",
                noPublications: "Aún no hay publicaciones.",
                confirmTitle: "¿Iniciar la publicación en el sitio web de resultados?",
                confirmDescription:
                    "Esto creará una nueva publicación a partir del recuento actual. Los resultados que ven los votantes seguirán activos hasta que esta tarea de publicación termine correctamente.",
                close: "Cerrar",
            },
            transmissionPackage: {
                title: "Paquete de Transmisión para el Área '{{name}}' y Elección '{{eventName}}'",
                description:
                    "Te permite exportar un Paquete de Transmisión a los Servidores de Destino o descargarlo.",
                erFileSha256: "Archivo ER SHA-256:",
                actions: {
                    sign: {
                        title: "Regenerar",
                        dialog: {
                            title: "¿Desea firmar el paquete de transmisión?",
                            description:
                                "Por favor, confirme que desea regenerar el paquete de transmisión para el área `{{name}}`",
                            confirm: "Firmar paquete de transmisión",
                            cancel: "Cerrar",
                            input: {
                                placeholder: "Ingrese su contraseña",
                            },
                        },
                    },
                    send: {
                        title: "Enviar",
                        dialog: {
                            title: "¿Quieres enviar el Paquete de Transmisión?",
                            description:
                                "Por favor, confirma que deseas enviar el Paquete de Transmisión para el Área '{{name}}' a los Servidores de Destino.",
                            confirm: "Enviar Paquete de Transmisión",
                            cancel: "Cerrar",
                        },

                        disabled:
                            "Faltan las firmas requeridas o el paquete de transmisión ya se ha enviado a todos los destinos.",
                    },
                    regenerate: {
                        title: "Regenerar",
                        dialog: {
                            title: "¿Desea regenerar el paquete de transmisión?",
                            description:
                                "Por favor confirme que desea regenerar el paquete de transmisión para el área `{{name}}`",
                            confirm: "Regenerar paquete de transmisión",
                            cancel: "Cerrar",
                        },
                    },
                    download: {
                        title: "Descargar",
                        emlTitle: "Descargar EML {{date}}",
                        transmissionPackageTitle: "Descargar el Paquete de Transmisión {{date}}",
                        transmissionReportTitle: "Descargar informe de transmisión",
                        csvTitle: "Descargar CSV de resultados {{date}}",
                        ovcsPublicKeyTitle: "Descargar clave pública de la OVCS",
                        recordError:
                            "No se pudo registrar la descarga, así que no se ha iniciado. Inténtalo de nuevo.",
                        dialog: {
                            title: "¿Quieres descargar el Paquete de Transmisión?",
                            description:
                                "Por favor, confirma que deseas descargar el Paquete de Transmisión para el Área '{{name}}.'",
                            confirm: "Descargar Paquete de Transmisión",
                            cancel: "Cerrar",
                        },
                    },
                },
                destinationServers: {
                    title: "Servidores de Destino",
                    description:
                        "La tabla a continuación muestra el estado de envío de cada uno de los Servidores de Destino.",
                    status: "Enviado a {{signed}} de {{total}}",
                    manualReceipt: "Recepción manual",
                    handOver: "Descarga el paquete y entrégalo en mano",
                    sealedFor: "Sellado para su clave, SHA-256 {{key}}…",
                    table: {
                        serverName: "Nombre del Servidor",
                        sendStatus: "Estado de Envío",
                    },
                },
                signatures: {
                    title: "Firmas",
                    description:
                        "Los miembros pueden firmar el paquete de transmisión. La tabla muestra el estado de firma de cada miembro.",
                    table: {
                        trusteeName: "Miembro",
                        signed: "Ha Firmado",
                    },
                    status: "{{signed}} de {{total}}, se necesitan {{minimum}}",
                    statusNoMinimum: "{{signed}} de {{total}}",
                },
            },
            sendToTransmissionPackageServers:
                "Enviar paquete de transmisión para el área '{{name}}'",
            uploadTransmissionPackage: "Subir",
            uploadTransmissionPackageDesc:
                "Sube tu firma para firmar el paquete de Resultados Electorales. Esta operación es opcional.",
            exportElectionArea: "Enviar paquete de transmisión para el área '{{name}}'",
            generateReport: "Generar {{name}}",
            templateTitle: "Plantilla de Resultados",
            templateSubTitle: "Opcionalmente sobrescribir la plantilla de resultados.",
            keysCeremonyTitle: "Ceremonia de Llaves",
            keysCeremonySubTitle: "Selecciona la Ceremonia de Llaves para este recuento",
            ceremonyTitle: "Elecciones para el Recuento",
            initializationTitle: "Elecciones para el informe de inicialización",
            ceremonySubTitle: "Seleccione las elecciones para el recuento",
            tallyTitle: "Progreso del Recuento de Elecciones",
            logsTitle: "Logs",
            resultsTitle: "Resultados & Participación",
            generalInfoTitle: "Información General",
            sessionId: "ID de la sesión de recuento",
            selectedQuorum: "Quórum seleccionado",
            trusteeTallyTitle: "Trustee",
            trusteeTallySubTitle: "Estado de confirmación de custodia",
            ballotBoxes: {
                unavailable: "Sellos no disponibles",
                sealed: "{{sealed}} de {{total}} selladas",
                publishing: "Selladas, {{published}} de {{total}} actas de sellado publicadas",
                sealing: "Se sella a las {{time}}",
                notSealed: "Sin sellar",
                help: "Se puede hacer el recuento de una elección cuando todas sus urnas están selladas y sus actas de sellado están publicadas.",
                failed: "Sin sellar: incidencia",
                overdue: "Sellado con retraso",
                blocked: "{{name}}: {{reason}}",
                reason: {
                    "not-sealed":
                        "la votación no se ha cerrado, así que sus urnas aún no tienen sellos",
                    "sealing": "sus urnas se sellan cuando termina el periodo de gracia",
                    "overdue":
                        "sus urnas han superado su plazo y aún no están selladas (consulte su Panel de Control)",
                    "publishing":
                        "las actas de sellado de algunas de sus urnas aún se están publicando",
                    "failed":
                        "no se pudo sellar una urna, es una incidencia (consulte su Panel de Control)",
                    "unavailable": "no se pudieron leer los sellos de sus urnas",
                },
            },
            eligibility: {
                ballotBoxesUnavailable:
                    "No se pudieron leer los sellos de las urnas de una elección seleccionada, así que aún no se puede hacer su recuento. Recargue la página para intentarlo de nuevo.",
                selectElection: "Selecciona al menos una elección.",
                publishElection: "Publica cada elección seleccionada antes de crear su recuento.",
                tallyDisallowed: "El recuento está deshabilitado para una elección seleccionada.",
                endVoting:
                    "Finaliza la votación en cada elección seleccionada y detén sus canales activos antes de crear el recuento.",
                sealBallotBoxes:
                    "Se puede hacer el recuento de una elección cuando todas sus urnas están selladas y sus actas de sellado están publicadas.",
            },
            createTallySuccess: "Recuento creado",
            createTallyError: "Error creando recuento",
            startTallySuccess: "Recuento iniciado",
            startTallyError: "Error iniciando recuento",
            startTallyCeremonySuccess: "Iniciada la ceremonya del recuento",
            startTallyCeremonyError: "No se ha podido iniciar la ceremonia del recuento",
            cancelTallyCeremonySuccess: "Cancelada la ceremonya del recuento",
            cancelTallyCeremonyError: "No se ha podido cancelar la ceremonia del recuento",
            recountTallyCeremony: "Repetir el recuento",
            recountTallyCeremonyMessage:
                "Esto generará un nuevo evento de resultados para la sesión de recuento completada.",
            recountTallyCeremonyStarting: "Iniciando el recuento...",
            recountTallyCeremonySuccess: "Recuento iniciado",
            recountTallyCeremonyError: "No se ha podido iniciar el recuento",
            recountTallyCeremonyOk: "Repetir el recuento",
            trusteeTitle: "Proceso del trustee",
            trusteeSubTitle:
                "Carga tu archivo de custodia guardado o una prueba de custodia firmada:",
            invited: "Has sido invitado a participar en una ceremonia de recuento. Por favor, ",
            click: "haz clic en la acción de recuento",
            participate: "para participar.",
            breadcrumbSteps: {
                start: "Inicio",
                finish: "Final",
                tally: "Recuento",
                results: "Resultados",
                ceremony: "Ceremonia",
            },
            common: {
                title: "Recuento",
                subTitle: "Configuración del Recuento.",
                cancel: "Atras",
                next: "Siguiente",
                date: "Fecha de Recuento",
                global: "Global",
                noTrustees: "No hay trustees aún",
                imported: " autoridades confirmaron la custodia",
                needed: " trustees necesarios para el recuento",
                start: "Iniciar Recuento",
                ceremony: "Iniciar Ceremonia de Recuento",
                initialization: "Iniciar informe de inicialización",
                results: "Resultados",
                dialog: {
                    ok: "Ok",
                    okTally: "Iniciar recuento",
                    okCancel: "Cancelar recuento",
                    cancel: "Cerrar",
                    title: "¿Estás seguro de que quieres iniciar una ceremonia?",
                    tallyTitle: "¿Estás seguro de que quieres iniciar el recuento?",
                    cancelTitle: "¿Estás seguro de que quieres cancelar el recuento?",
                    message:
                        "Esta ceremonia solicita a las autoridades pruebas públicas de custodia. Mantenga sus ejecutores activos con el estado local del protocolo.",
                    cancelMessage:
                        "Estás a punto de cancelar la ceremonia de recuento. Esta acción no se puede deshacer.",
                    ceremony:
                        "Las autoridades necesarias han confirmado la custodia. ¿Iniciar el recuento con sus ejecutores activos?",
                    startAutomatedTallyMessage:
                        "Selecciona 'Start Tally' para ejecutar el proceso de recuento y mostrar los resultados, o 'Close' para cancelar.",
                },
            },
            table: {
                ballotBoxes: "Urnas",
                elections: "Elecciones",
                selected: "Seleccionadas",
                status: "Estado",
                progress: "Progreso",
                method: "Método de Recuento",
                elegible: "Votantes Elegibles",
                number: "Número de Votos",
                total: "Total",
                turnout: "%",
                candidates: "Resultados de Candidatos",
                options: "Optiones",
                global: "Resumen de participación",
                elegible_census: "Censo de votantes elegibles",
                cast_votes: "Número de Votos",
                cast_votes_percent: "Porcentajes de Votos",
                total_votes: "Total de votantes",
                total_votes_percent: "Participación",
                total_votes_counted: "Total de Votos Contados",
                total_auditable_votes: "Total de Votos Auditables",
                total_valid_votes: "Total de votos válidos",
                total_valid_votes_percent: "Porcentaje de votos válidos",
                total_invalid_votes: "Total de votos inválidos",
                total_invalid_votes_percent: "Porcentaje de votos inválidos",
                explicit_invalid_votes: "Votos explícitamente inválidos",
                explicit_invalid_votes_percent: "Porcentaje de votos explícitamente inválidos",
                implicit_invalid_votes: "Votos implícitamente inválidos",
                implicit_invalid_votes_percent: "Porcentaje de votos implícitamente inválidos",
                blank_votes: "Votos en blanco",
                explicit_blank_votes: "Votos en blanco explícitos",
                implicit_blank_votes: "Votos en blanco implícitos",
                blank_votes_percent: "Porcentaje de votos en blanco",
                number_of_votes: "Número de votos",
                winning_position: "Posición ganadora",
                weight: "Peso",
                preferential: {
                    candidate: "Candidato",
                    winner: "Ganador",
                    eliminated: "Eliminado",
                    round: "Ronda",
                },
                total_declined_to_vote: "Total de votos de renuncia",
                total_blank_ballots: "Total de Papeletas en Blanco",
                participation_by_channel: "Participación por canal",
                channel: "Canal",
                channel_online: "En línea",
                channel_kiosk: "Quiosco",
                channel_early_voting: "Votación anticipada",
                channel_telephone: "Teléfono",
                channel_paper: "Papel",
                channel_postal: "Postal",
                channel_in_person: "Presencial",
                acclamation_note:
                    "Elegido por aclamación. Esta votación se resolvió sin votación, por lo que no se registró ningún voto.",
            },
            pendingResolutions: {
                round: "Ronda {{round}}",
                tieResolutionRequired: "Resolución de empate necesaria",
                tieResolved: "Empate resuelto",
                globalArea: "Global",
                pendingResolutionsHeader: "Resoluciones pendientes",
                pendingResolutionStatus: "Resolución pendiente",
                resolvedStatus: "Resuelta",
                resolutionTitle: "Resolución",
                selectContest: "Seleccione un elemento a la izquierda para ver los detalles",
                selectCandidateToAdvance: "Seleccione el candidato a avanzar",
                undoResolution: "Deshacer resolución",
                applyResolutions: "Aplicar resoluciones y recalcular",
                submitSuccess: "Resoluciones enviadas. El recuento está reanudando...",
                submitError: "Error al enviar las resoluciones. Por favor, inténtelo de nuevo.",
                filter: "Filtrar",
                save: "Guardar",
                pendingApplyStatus: "Cálculo pendiente",
                filterElection: "Elección",
                filterContest: "Pregunta",
                filterArea: "Área",
                filterStatusLabel: "Estado",
                clearFilters: "Borrar filtros",
                candidateWithVotes: "{{name}} ({{votes}} votos)",
                candidateWithVotesAndPercent: "{{name}} ({{votes}} votos, {{percent}}%)",
                tieInfoTitle: "Recuento pausado por empate sin resolver (Ronda {{round}})",
                tieInfoBody:
                    "Candidatos empatados ({{votes}} votos, {{percent}}%): {{candidates}}. Se requiere desempate manual para continuar el recuento.",
                tallyResumedTitle: "Recuento reanudado tras aplicar la resolución",
                tallyResumedBody: "El empate fue resuelto el {{date}} por {{user}}",
            },
            chart: {
                votesForCandidates: "Votos por Candidatos",
                blankVotes: "Votos en Blanco",
                invalidVotes: "Votos Inválidos",
                totalVoters: "Total de Votantes",
                nonVoters: "No Votantes",
            },
            exportAllAreas:
                "Exportar los resultados de todas las áreas en formato {{format}} para '{{item}}'",
        },
        publish: {
            initialization: {
                countryInfo:
                    "Genere el informe para todo el puesto o para un país. La votación permanece bloqueada hasta completar toda la inicialización requerida por país y para el evento.",
                countriesError:
                    "No se pudieron cargar los países elegibles. Cierre y vuelva a intentarlo.",
                noCountries:
                    "Este puesto no tiene países elegibles con estilos de papeleta activos. Compruebe sus áreas y publicación antes de inicializar.",
                country: "País",
                entirePost: "Todo el puesto",
            },
            preview: {
                publicationAreas: "Selecciona el área para la vista previa",
                action: "Vista previa",
                copy: "Copiar enlace",
                copy_success: "Se copió correctamente el enlace de vista previa",
                copy_error: "No se pudo copiar el enlace de vista previa",
                success: "Éxito al abrir la vista previa",
            },
            header: {
                change: "Cambios a Publicar",
                viewChange: "Ver Publicación",
                history: "Historico de Cambios",
            },
            action: {
                generateInitializationReport: "Generar Informe de Inicialización",
                startVotingPeriod: "Comenzar votación",
                startKioskVoting: "Comenzar Votación en Quiosco",
                startOnlineVoting: "Comenzar Votación en Línea",
                startEarlyVoting: "Comenzar Votación Anticipada",
                startTelephoneVoting: "Comenzar Votación Telefónica",
                stopVotingPeriod: "Detener votación",
                stopOnlineVoting: "Detener Votación en Línea",
                stopEarlyVoting: "Detener Votación Anticipada",
                stopTelephoneVoting: "Detener Votación Telefónica",
                stopKioskVotingPeriod: "Cerrar Quiosco",
                pauseVotingPeriod: "Pausar votación",
                pauseKioskVoting: "Pausar Votación en Quiosco",
                pauseOnlineVoting: "Pausar Votación en Línea",
                pauseEarlyVoting: "Pausar Votación Anticipada",
                pauseTelephoneVoting: "Pausar Votación Telefónica",
                generate: "regenerar",
                publish: "Publicar Cambios",
                back: "Atrás",
            },
            empty: {
                header: "Ninguna Publicación aún.",
                action: "Generar Publicación",
            },
            forbidden: {
                header: "No es posible Publicar hasta que la Ceremonia de Llaves haya terminado.",
            },
            skippedElections: {
                dismiss: "Descartar",
                title: "Algunas elecciones siguen cerradas",
                ballotBoxSealPolicy:
                    "{{name}} sigue cerrada: su votación se ha cerrado y Sellar al cierre hace que el cierre sea definitivo.",
                other: "{{name}} se ha dejado como estaba ({{reason}}).",
            },
            dialog: {
                title: "Confirmar Acción",
                info: "Has hecho clic en una acción sensible, por lo que necesitamos que la confirmes para poder continuar.",
                initializationInfo:
                    "Estás a punto de generar el informe de inicialización. ¿Estás seguro de que deseas continuar?",
                startInfo:
                    "Está a punto de comenzar el período de votación. ¿Está seguro de que desea continuar?",
                stopInfo:
                    "Está a punto de detener el período de votación. ¿Está seguro de que desea continuar?",
                stopSeal:
                    "Está a punto de detener la votación en {{name}}. Después se sellan sus urnas: no se puede añadir, cambiar ni borrar ninguna papeleta, y la votación no puede volver a empezar. ¿Está seguro de que desea continuar?",
                stopSealEvent:
                    "Está a punto de detener la votación en todas las elecciones. Después se sellan sus urnas: no se puede añadir, cambiar ni borrar ninguna papeleta, y la votación no puede volver a empezar. ¿Está seguro de que desea continuar?",
                startSealNote:
                    "Con Sellar al cierre, la votación cerrada sigue cerrada: las elecciones cuya votación se ha cerrado no se abrirán.",
                channel: {
                    ONLINE: "En línea",
                    KIOSK: "Quiosco",
                    EARLY_VOTING: "Votación anticipada",
                    TELEPHONE: "Votación telefónica",
                },
                kioskStopInfo:
                    "Estás a punto de detener el período de votación en el quiosco. ¿Estás seguro de que deseas continuar?",
                pauseInfo:
                    "Está a punto de pausar el período de votación. ¿Está seguro de que desea continuar?",
                publishInfo:
                    "Estás a punto de generar una publicación. ¿Estás seguro de que deseas continuar?",
                ok: "Confirmar",
                ko: "Cancelar",
                error: "Error al cargar las papeletas publicadas",
                error_publish: "Error al publicar la papeleta",
                error_capacity: "Fallo al generar el estilo de papeleta: {{message}}",
                error_status: "Error al cambiar el estado de la publicación",
                error_preview: "Error al obtener la vista previa de la publicación",
                diff: "Renderizar todos los cambios podría hacer que la página no responda. ¿Estás seguro de que quieres continuar?",
                confirmation:
                    "La acción que estás a punto de realizar es sensible y requiere confirmación. Por favor, introduce tu contraseña para continuar con {{action}}.",
                stopSealNotYet:
                    "Está a punto de detener el período de votación. {{holding}} ¿Está seguro de que desea continuar?",
                sealHolding_one:
                    "Con Sellar al cierre, sus urnas se sellan cuando todos los canales habilitados están cerrados: el canal {{channels}} sigue habilitado y sin cerrar.",
                sealHolding_other:
                    "Con Sellar al cierre, sus urnas se sellan cuando todos los canales habilitados están cerrados: los canales {{channels}} siguen habilitados y sin cerrar.",
                sealNotEnabled:
                    "El canal {{channel}} no está cerrado ni habilitado en esta elección: deténgalo para sellar las urnas.",
                sealNotEnabledPost:
                    "En {{post}}, el canal {{channel}} no está cerrado ni habilitado: deténgalo en esa elección para sellar sus urnas.",
                sealNoChannel:
                    "Ningún canal está habilitado en esta elección y ninguno ha estado abierto, así que sus urnas no se sellan.",
                stopNeverOpened_one:
                    "El canal {{channels}} nunca se abrió: al detenerlo, ya no se abrirá.",
                stopNeverOpened_other:
                    "Los canales {{channels}} nunca se abrieron: al detenerlos, ya no se abrirán.",
                stopSealGrace_one:
                    "Está a punto de detener la votación en {{name}}. Sus urnas se sellan cuando termina el periodo de gracia, {{count}} minuto después: a partir de entonces no se puede añadir, cambiar ni borrar ninguna papeleta. La votación no puede volver a empezar. ¿Está seguro de que desea continuar?",
                stopSealGrace_other:
                    "Está a punto de detener la votación en {{name}}. Sus urnas se sellan cuando termina el periodo de gracia, {{count}} minutos después: a partir de entonces no se puede añadir, cambiar ni borrar ninguna papeleta. La votación no puede volver a empezar. ¿Está seguro de que desea continuar?",
                stopSealEventGrace_one:
                    "Está a punto de detener la votación en todas las elecciones. Sus urnas se sellan cuando termina el periodo de gracia de cada elección, hasta {{count}} minuto después: a partir de entonces no se puede añadir, cambiar ni borrar ninguna papeleta. La votación no puede volver a empezar. ¿Está seguro de que desea continuar?",
                stopSealEventGrace_other:
                    "Está a punto de detener la votación en todas las elecciones. Sus urnas se sellan cuando termina el periodo de gracia de cada elección, hasta {{count}} minutos después: a partir de entonces no se puede añadir, cambiar ni borrar ninguna papeleta. La votación no puede volver a empezar. ¿Está seguro de que desea continuar?",
                stopSealEventSome:
                    "Está a punto de detener la votación en todas las elecciones. {{sealed}} {{holding}} ¿Está seguro de que desea continuar?",
                sealedNowPart:
                    "Después se sellan las urnas de {{names}}: no se puede añadir, cambiar ni borrar ninguna papeleta, y la votación no puede volver a empezar en ellas.",
                sealedGracePart_one:
                    "Las urnas de {{names}} se sellan cuando termina su periodo de gracia, hasta {{count}} minuto después.",
                sealedGracePart_other:
                    "Las urnas de {{names}} se sellan cuando termina su periodo de gracia, hasta {{count}} minutos después.",
                holdingEventPart_one:
                    "{{names}} mantiene otro canal habilitado y sin cerrar: sus urnas se sellan cuando ese canal se cierre.",
                holdingEventPart_other:
                    "{{names}} mantienen otro canal habilitado y sin cerrar: sus urnas se sellan cuando esos canales se cierren.",
                noChannelEventPart_one:
                    "{{names}} no habilita ningún canal y ninguno ha estado abierto: sus urnas no se sellan.",
                noChannelEventPart_other:
                    "{{names}} no habilitan ningún canal y ninguno ha estado abierto: sus urnas no se sellan.",
                startSealNoteList:
                    "Con Sellar al cierre, la votación cerrada sigue cerrada: {{items}}.",
                startKeptChannels_one: "{{post}}: el canal {{channels}} sigue cerrado",
                startKeptChannels_other: "{{post}}: los canales {{channels}} siguen cerrados",
                startKeptSealed: "{{post}} sigue cerrado, ya que sus urnas están selladas",
                startNotEnabledList:
                    "Con Sellar al cierre, un inicio abre un canal solo en los Posts que lo habilitan: {{items}}.",
                startNotEnabledChannels_one:
                    "{{post}}: el canal {{channels}} no está habilitado allí y sigue No iniciado",
                startNotEnabledChannels_other:
                    "{{post}}: los canales {{channels}} no están habilitados allí y siguen No iniciados",
                stopNeverOpenedPosts_one:
                    "{{names}} nunca se abrió: al detenerlo, se cierra y se sellan sus urnas vacías.",
                stopNeverOpenedPosts_other:
                    "{{names}} nunca se abrieron: al detenerlos, se cierran y se sellan sus urnas vacías.",
            },
            label: {
                current: "Actual",
                previous: "Publicación Anterior",
                diff: "Cambios a publicar",
                publication: "Publicación",
            },
            notifications: {
                generated: "Papeleta generada",
                published: "Papeleta publicada",
                change_status: "Elección cambiada de estado",
            },
            sealRefusals: {
                startAgain:
                    "La votación no puede volver a empezar: con Sellar al cierre, la votación que se ha cerrado sigue cerrada y sus urnas están selladas.",
                startDisabled:
                    "Comenzar votación no está disponible: las urnas de esta elección están selladas o sellándose, y con Sellar al cierre la votación cerrada sigue cerrada.",
                closedIsFinal:
                    "La votación cerrada no puede cambiar: con Sellar al cierre, la votación cerrada sigue cerrada.",
            },
        },
        emailEditor: {
            subject: "Asunto del Email",
            tabs: {
                plaintext: "Cuerpo de Texto Plano",
                richtext: "Cuerpo de Texto Enriquecido",
            },
        },
        sendCommunication: {
            send: "Enviar",
            title: "Enviar Notificación",
            subtitle: "Enviar una notificación a usuarios/votantes.",
            sendButton: "Enviar Notificación",
            voters: "Audiencia",
            schedule: "Calendario",
            nowInput: "Enviar ahora",
            dateInput: "Fecha y hora de comienzo de envío",
            chooseDate: "Por favor escoge una fecha",
            languages: "Idiomas",
            smsMessage: "Mensaje SMS",
            errorSending: "Error enviando la notificación: {{error}}",
            successSending: "Notificación programada con éxito",
            method: "Método de Plantillas",
            type: "Tipo de Comunicación",
            alias: "Alias de la Plantilla",
            votersSelection: {
                ALL_USERS: "Todos",
                NOT_VOTED: "Los que no votaron",
                VOTED: "Los que ya votaron",
                SELECTED: "A {{total}} {{voters}} seleccionados",
            },
            path: {
                users: "usuarios",
                voters: "votantes",
            },
            methodTitle: "Método de Comunicación",
            communicationMethod: {
                EMAIL: "Email",
                SMS: "SMS",
                WHATSAPP: "WhatsApp",
                VIBER: "Viber",
                MESSENGER: "Facebook Messenger",
            },
            communicationType: {
                CREDENTIALS: "Credenciales",
                BALLOT_RECEIPT: "Comprobante de Votación",
            },
            email: {
                subject: "Asunto",
            },
        },
        tallysheet: {
            title: "Urnas",
            subtitle: "Urnas digitalizadas por canal",
            createTallySuccess: "Hoja de Recuento creada",
            createTallyError: "Error creando Hoja de Recuento",
            createTallyErrorSameKindExists:
                "La hoja de recuento ya existe para esta pregunta con el mismo canal y área",
            allFieldsRequired: "Todos los campos son obligatorios",
            header: {
                change: "Cambios pendientes de publicar",
                viewChange: "Ver publicación",
                history: "Historial de publicaciones",
            },
            action: {
                start: "Iniciar elección",
                stop: "Detener elección",
                pause: "Pausar",
                generate: "Regenerar",
                publish: "Publicar cambios",
                back: "Atrás",
            },
            inputError: {
                totalValidDoesNotMatch:
                    "Los votos de candidatos ({{candidateVotesSum}}) deben estar entre {{lowerBound}} y {{upperBound}} según las reglas de votación de esta pregunta ({{nonBlankValidVotes}} votos válidos no en blanco × hasta {{maxMarks}} marcas por papeleta)",
                censusTooSmall:
                    "El total de votos ({{totalVotes}}) no puede ser mayor que el censo ({{census}})",
                totalInvalidDoesNotMatch:
                    "El total de votos inválidos ({{totalInvalid}}) debe ser igual a los votos inválidos implícitos ({{implicitInvalid}}) más los votos inválidos explícitos ({{explicitInvalid}})",
                totalVotesDoesNotMatch:
                    "El total de votos ({{totalVotes}}) debe ser igual al total de votos válidos ({{totalValidVotes}}) más el total de votos inválidos ({{totalInvalid}})",
                unknownCountingAlgorithm:
                    "El algoritmo de recuento de esta pregunta ({{countingAlgorithm}}) no se reconoce, por lo que no se puede determinar el número permitido de votos de candidatos. Revise la configuración de la pregunta.",
                blankBallotsInconsistent:
                    "Las Papeletas en Blanco deben tener el mismo valor en todas las hojas de escrutinio de esta urna",
                blankBallotsOutOfBounds:
                    "El valor de Papeletas en Blanco está fuera del rango que implican los recuentos de votos en blanco por pregunta de esta urna",
            },
            label: {
                area: "Área",
                channel: "Canal",
                total_votes: "Votos Totales",
                total_valid_votes: "Votos Válidos Totales",
                total_invalid: "Votos Inválidos Totales",
                explicit_invalid: "Votos Explícitamente Inválidos",
                implicit_invalid: "Votos Implícitamente Inválidos",
                total_blank_votes: "Votos en Blanco Totales",
                blank_ballots: "Papeletas en Blanco",
                census: "Censo",
            },
            common: {
                tallyCeremony: {
                    manage: "Gestionar la Ceremonia de Cómputo",
                    view: "Ver la Ceremonia de Cómputo",
                    cancel: "Cancelar la Ceremonia de Cómputo",
                    addKey: "Agregar Clave de Cómputo",
                },
                edit: "Editar",
                confirm: "Confirmar",
                back: "Atras",
                next: "Siguiente",
                cancel: "Atras",
                data: "Datos",
                title: "Hoja de Recuento",
                subtitle: "Configuración de la Hoja de Recuento.",
                candidates: "Candidatos",
                save: "Guardar",
                approve: "Aprobar",
                disapprove: "Desaprobar",
                show: "Mostrar",
                add: "Añadir",
                versions: "Versiones",
                warningDisapprove: "¿Está seguro de desaprobar esta Hoja de Recuento?",
                warningApprove: "¿Está seguro de aprobar esta Hoja de Recuento?",
            },
            empty: {
                header: "No hay Hojas de Recuento.",
                action: "Generar Hoja de Recuento",
                add: "Añadir",
            },
            breadcrumbSteps: {
                start: "Inicio",
                edit: "Editar",
                confirm: "Confirmar",
                view: "Ver",
            },
            table: {
                area: "Área",
                contest: "Pregunta",
                approvedVersion: "Versión aprobada",
                latestVersion: "Última versión",
                labels: "Etiquetas",
                annotations: "Anotaciones",
            },
            versionsTable: {
                title: "Versiones de la urna",
                version: "Versión",
                createdBy: "Creado por",
                reviewedBy: "Revisado por",
                createdAt: "Creado el",
                reviewedAt: "Revisado el",
                sourceImport: "Importación de origen",
                importStatus: "Estado de importación",
                openImport: "Abrir importación",
                sourceFile: "Archivo de origen",
            },
            message: {
                reviewError: "Error revisando la Hoja de Recuento",
                reviewSuccess: "Hoja de Recuento revisada",
            },
        },
        application: {
            import: {
                title: "Importar Aplicaciones",
                subtitle: "Importar datos de aplicaciones",
                paragraph:
                    "Importa aplicaciones usando un archivo de hoja de cálculo en formato de valores separados por comas (CSV). Descarga un archivo CSV de ejemplo aquí.",
                messages: {
                    success: "Aplicaciones importadas con éxito",
                    error: "Error al importar las aplicaciones",
                },
            },
            export: {
                title: "Exportar Aplicaciones",
                subtitle: "Exportar datos de aplicaciones",
                button: "Exportar",
                paragraph:
                    "Exporta aplicaciones usando un archivo de hoja de cálculo en formato de valores separados por comas (CSV).",
                messages: {
                    success: "Aplicaciones exportadas con éxito",
                    error: "Error al exportar las aplicaciones",
                },
            },
        },
        template: {
            noPermissions: "No tienes permisos para acceder a las Plantillas.",
            title: "Plantillas",
            subtitle: "Listado de plantillas",
            chooseMethods: "Elige métodos",
            default: "Usar plantilla predeterminada",
            empty: {
                title: "No hay plantillas",
                subtitle: "¿Quieres crear una nueva?",
            },
            action: {
                createOne: "Crear Plantilla",
            },
            create: {
                title: "Crear una Plantilla",
                success: "Plantilla creada",
                error: "Error creando plantilla",
            },
            update: {
                success: "Plantilla actualizada",
                error: "Error actualizando plantilla",
            },
            edit: {
                title: "Editar una Plantilla",
            },
            form: {
                smsMessage: "Mensaje SMS",
                document: "Documento",
                pdfOptions: "Opciones de PDF",
                name: "Nombre de la Plantilla",
                alias: "Alias de la Plantilla",
                type: "Tipo",
                communicationMethod: "Método",
                reportOptions: "Opciones de Informe",
            },
            type: {
                CREDENTIALS: "Credenciales",
                INITIALIZATION_REPORT: "Informe de Inicialización",
                ELECTORAL_RESULTS: "Resultados Electorales",
                BALLOT_IMAGES: "Imágenes de Boletas",
                BALLOT_RECEIPT: "Recibo de Voto",
                ACTIVITY_LOGS: "Registros de Actividades",
                MANUAL_VERIFICATION: "Verificación Manual",
                PARTICIPATION_REPORT: "Informe de Participación",
            },
            method: {
                email: "Email",
                sms: "SMS",
                document: "Documento",
                whatsapp: "WhatsApp",
                viber: "Viber",
                messenger: "Facebook Messenger",
            },
            import: {
                title: "Importar Plantillas",
                subtitle: "Importar datos de plantillas",
                paragraph:
                    "Importa plantillas usando un archivo de hoja de cálculo en formato de valores separados por comas (CSV). Descarga un archivo CSV de ejemplo aquí.",
            },
        },
        materials: {
            audioInstructions: {
                screenLabel: "Instrucciones en audio para la pantalla",
                languageLabel: "Idioma de la grabación",
                none: "No son instrucciones en audio",
                helperText:
                    "Los votantes oyen este archivo cuando piden las instrucciones en esa pantalla.",
                screens: {
                    "election-chooser": "Lista de elecciones",
                    "start": "Inicio",
                    "ballot": "Papeleta",
                    "review": "Revisión",
                    "confirmation": "Confirmación",
                    "audit": "Auditoría",
                    "ballot-locator": "Localizador de papeletas",
                    "support-materials": "Materiales de apoyo",
                },
            },
            createMaterialSuccess: "Material de soporte creado",
            createMaterialError: "Error creando material de soporte",
            updateMaterialSuccess: "Material de soporte actualizado",
            updateMaterialError: "Error actualizando material de soporte",
            common: {
                title: "Materiales de Soporte",
                subtitle: "Entrar datos del material de soporte.",
            },
            error: {
                title: "El título es obligatorio",
                document: "El documento es obligatorio",
            },
            fields: {
                isHidden: "Oculto",
                publicUrl: "Enlace público",
            },
            empty: {
                header: "Aún no hay material de soporte",
                action: "Generar material de soporte",
            },
        },
        widget: {
            logs: "Registros",
        },
        settings: {
            countries: {
                title: "Bloqueo de Países",
                votingDescription:
                    "Elige a continuación los países de los cuales deseas bloquear las votaciones.",
                enrollmentDescription:
                    "Elige a continuación los países de los cuales deseas bloquear las preinscripciones.",
                error: {
                    errorSaving: "Error al guardar la lista de países",
                },
            },
            backupRestore: {
                title: "Copia de seguridad / Restaurar configuración del inquilino",
                backup: {
                    label: "Respaldo",
                    subtitle: "Copia de seguridad de las configuraciones del inquilino",
                },
                restore: {
                    label: "Restaurar",
                    subtitle: "Restaurar la configuración del inquilino",
                    title: "Importar configuraciones del inquilino",
                    paragraph:
                        "Importar configuraciones del inquilino, configuraciones de Keycloak, roles y datos de permisos usando una carpeta comprimida.",
                    tenantConfigOption: "Importar configuraciones del inquilino",
                    keycloakConfigOption: "Importar configuraciones de Keycloak",
                    RolesConfigOption: "Importar configuraciones de roles y permisos",
                },
            },
            previewScreen: {
                label: "Vistas Previas",
                noContent: "No se encontraron vistas previas",
                table: {
                    title: "Vistas previas externas",
                    description:
                        "Un registro de las vistas previas de estilos de papeleta generadas mediante peticiones externas",
                    requestedBy: "Solicitado por",
                    document: "Documento",
                    url: "URL",
                },
            },
            languages: {
                logsTimezone: "Zona horaria de la bitácora del inquilino",
                default: "Idioma predeterminado",
            },
        },
        approvalsScreen: {
            column: {
                status: "Estado",
                id: "ID de la solicitud",
                applicantId: "ID del solicitante",
                verificationType: "Verificación",
                createdAt: "Solicitada",
                verified_by: "Verificada por",
                voter: "Votante",
                what: "Qué ha pasado",
                post: "Puesto",
                when: "Cuándo",
            },
            status: {
                PENDING: "Pendiente de revisión",
                ACCEPTED: "Aprobada",
                REJECTED: "Rechazada",
            },
            verification: {
                AUTOMATIC: "Automática",
                MANUAL: "Manual",
            },
            time: {
                minutes_one: "{{count}} minuto",
                minutes_other: "{{count}} minutos",
                hours_one: "{{count}} hora",
                hours_other: "{{count}} horas",
                days_one: "{{count}} día",
                days_other: "{{count}} días",
            },
            summary: {
                join: "{{head}} y {{last}}",
                differs_one: "{{fields}} no coincide con el registro",
                differs_other: "{{fields}} no coinciden con el registro",
                typedByHand: "Datos escritos a mano, no leídos de un documento escaneado",
                needsFaceToFace: "Necesita una comprobación cara a cara",
                scanVerified: "Documento escaneado y verificado",
                noVoter: "No se encontró ningún votante en el registro",
                allMatch: "Todos los datos coinciden con el registro",
                needsReview: "A la espera de que una persona decida",
                approvedBy: "Aprobada por {{name}}",
                approvedAuto: "Aprobada automáticamente",
                rejectedBy: "Rechazada por {{name}}",
                rejectedAuto: "Rechazada automáticamente",
            },
            list: {
                title: "Aprobaciones",
                subtitle:
                    "Las inscripciones que las reglas no pueden decidir por sí solas esperan aquí a una persona.",
                search: "Buscar",
                review: "Revisar la inscripción",
                openRecord: "Abrir la inscripción",
                seeRule: "Ver la regla que decidió",
                unnamed: "Solicitante sin nombre",
                waiting: "Lleva {{time}} esperando",
                applied: "Solicitada el {{date}}",
                empty: {
                    title: "No hay nada aquí",
                    text: "Las inscripciones con este estado aparecerán aquí. Pruebe con otra búsqueda u otro estado.",
                },
                searchHelp:
                    "Un nombre, un correo electrónico o un número de identificación, o la referencia de soporte que se dio al votante, como Q5KW-8RFX.",
                referenceError: "No se pudo buscar la referencia de soporte.",
            },
            flow: {
                stepsLabel: "Pasos de la revisión",
                steps: {
                    identity: "Comprobar la identidad",
                    voter: "Buscar al votante",
                    decide: "Decidir",
                },
                continue: "Continuar",
                backToList: "Volver a Aprobaciones",
                identity: {
                    details: "Datos de la inscripción",
                    confirm:
                        "He comprobado el documento de identidad del votante en persona o por videollamada, y coincide con esta inscripción.",
                    checked: "Comprobación cara a cara confirmada",
                    notChecked: "Comprobación cara a cara aún sin confirmar",
                },
                voter: {
                    none: "Ninguno de estos es el votante",
                    noneHint:
                        "En ese caso la inscripción solo se puede rechazar, por no haber un votante coincidente.",
                    noneChosen: "Ninguno de estos es el votante",
                    notChosen: "Aún no se ha elegido un votante",
                },
                decide: {
                    approve: "Aprobar",
                    reject: "Rechazar",
                    approveText:
                        "Vincula esta inscripción con {{voter}} en el registro. Se avisa al votante por correo electrónico o SMS y podrá iniciar sesión para votar cuando se abra la votación.",
                    rejectText: "Se le dice al votante el motivo. Esto no se puede deshacer.",
                    chooseVoter: "Elija el votante coincidente en el paso 2 para aprobar.",
                    noVoter:
                        "No ha encontrado un votante coincidente, así que esta inscripción solo se puede rechazar.",
                    enrolled: "El votante elegido ya está inscrito.",
                    faceToFace: "Confirme la comprobación cara a cara en el paso 1 para aprobar.",
                },
            },
            review: {
                loadError: "No se pudo cargar la inscripción.",
                applied: "Solicitada el {{date}}",
                waiting: "Lleva {{time}} esperando",
                whyTitle: "Por qué hace falta una persona",
                decisionTitle: "Cómo se decidió",
                rule: "Regla {{rule}} de la versión {{version}} de la matriz",
                ruleLast: "Última regla de la versión {{version}} de la matriz",
                seeRule: "Ver la regla",
                why: {
                    typedByHand:
                        "El votante escribió sus datos a mano en lugar de escanear un documento de identidad. Estas inscripciones nunca se aprueban automáticamente: antes, un funcionario confirma quién es.",
                    differs_one:
                        "Un dato no coincide con el registro: {{details}}. Las reglas de aprobación piden que una persona revise esta inscripción.",
                    differs_other:
                        "{{count}} datos no coinciden con el registro: {{details}}. Las reglas de aprobación piden que una persona revise esta inscripción.",
                    differsFields_one:
                        "Un dato no coincide con el registro: {{fields}}. Las reglas de aprobación piden que una persona revise esta inscripción.",
                    differsFields_other:
                        "{{count}} datos no coinciden con el registro: {{fields}}. Las reglas de aprobación piden que una persona revise esta inscripción.",
                    difference:
                        "en {{field}}, la inscripción dice “{{enrollment}}” y el registro dice “{{registry}}”",
                    noVoter:
                        "Ningún votante del registro tiene estos datos. Las reglas de aprobación piden que una persona revise esta inscripción.",
                    severalVoters:
                        "Más de un votante del registro encaja con esta inscripción. Una persona elige el correcto.",
                    pending:
                        "Las reglas de aprobación piden que una persona revise esta inscripción.",
                    unknown: "Esta inscripción está a la espera de que una persona decida.",
                    approvedAuto:
                        "Las reglas de aprobación aprobaron esta inscripción automáticamente. Se superaron todas las comprobaciones que exigen.",
                    approvedBy: "{{name}} aprobó esta inscripción el {{date}}.",
                    rejectedAuto:
                        "Las reglas de aprobación rechazaron esta inscripción automáticamente: {{reason}}.",
                    rejectedBy: "{{name}} rechazó esta inscripción el {{date}}: {{reason}}.",
                },
                registryHelp:
                    "Hemos buscado votantes con los mismos datos: {{fields}}. Elija aquel al que pertenece esta inscripción.",
                registrySearching:
                    "Estos son los votantes del registro que coinciden con su búsqueda. Elija aquel al que pertenece esta inscripción.",
                registrySearch:
                    "¿No está en la lista? Busque en el registro por nombre o correo electrónico",
                registryLoading: "Buscando en el registro",
                registryError: "No se pudo buscar en el registro.",
                noCandidates:
                    "Ningún votante del registro coincide. Pruebe a buscar por nombre o correo electrónico.",
                candidates: "Votantes del registro",
                alreadyEnrolled: "Ya inscrito",
                bestMatch: "Mejor coincidencia",
                detailsMatch: "Coinciden {{count}} de {{total}} datos",
                compareTitle: "Comparación con {{name}} en el registro",
                col: {
                    detail: "Dato",
                    enrollment: "En la inscripción",
                    registry: "En el registro",
                    result: "Resultado",
                },
                same: "Igual",
                differs: "Distinto",
                compareNote:
                    "En los nombres no se tienen en cuenta mayúsculas, acentos ni guiones.",
                compareJoint:
                    "Para permisos de conducir y libretas de marino, el nombre y el segundo nombre se comparan juntos.",
                applicationId: "ID de la solicitud",
                copy: "Copiar",
                copied: "Copiado",
                approve: "Aprobar la inscripción",
                approveDialog: {
                    title: "¿Aprobar a {{name}}?",
                    body: "Esto vincula la inscripción con el votante del registro que aparece abajo. Se avisa al votante por correo electrónico o SMS y podrá iniciar sesión para votar cuando se abra la votación.",
                    checked: "Ha comprobado el documento de identidad del votante cara a cara.",
                    irreversible: "Esto no se puede deshacer.",
                    confirm: "Aprobar",
                },
                reject: "Rechazar la inscripción",
            },
            idCheck: {
                title: "Comprobación del documento",
                method: {
                    VERIFIED: "Documento escaneado y verificado",
                    MANUAL_ENTRY: "Escrito a mano",
                    UNKNOWN: "No indicado",
                },
                verified:
                    "El proceso de inscripción verificó el documento de identidad del votante",
                typedByHand: "El votante escribió sus datos a mano",
                unknown: "El proceso de inscripción no indicó cómo se comprobó la identidad",
                faceToFaceTitle: "Compruebe su identidad cara a cara antes de aprobar",
                faceToFaceText:
                    "Reúnase con el votante en persona o por videollamada y compare su documento de identidad con los datos de esta página.",
            },
            reject: {
                rejectReason: "Motivo del rechazo",
                message: "Mensaje para el votante",
                messageRequired: "Escriba un mensaje para el votante cuando el motivo sea Otro.",
                reasons: {
                    "undefined": "-",
                    "insufficient-information": "Datos faltantes",
                    "no-matching-voter": "Votante no coincidente",
                    "voter-already-approved": "Ya aprobado",
                    "other": "Otro",
                },
                hint: {
                    "insufficient-information": "Faltan datos o no se pueden leer.",
                    "no-matching-voter": "La persona no está en el registro de votantes.",
                    "voter-already-approved": "Este votante ya está inscrito.",
                    "other": "Escriba su propio mensaje.",
                },
                preview: {
                    "insufficient-information":
                        "No hemos podido inscribirle porque faltan algunos de sus datos o no se pueden leer. Vuelva a inscribirse con los datos completos.",
                    "no-matching-voter":
                        "No hemos encontrado en el registro un votante que coincida con sus datos. Revise sus datos y vuelva a inscribirse, o póngase en contacto con su oficina electoral.",
                    "voter-already-approved":
                        "Ya está inscrito. Podrá iniciar sesión para votar cuando se abra la votación.",
                },
                previewTitle: "El votante verá",
            },
            notifications: {
                approveError: "No se pudo aprobar la inscripción",
                approveSuccess: "Inscripción de {{name}} aprobada. Se ha avisado al votante.",
                rejectError: "No se pudo rechazar la inscripción",
                rejectSuccess: "Inscripción de {{name}} rechazada. Se ha avisado al votante.",
                VoterApprovedAlready: "Este votante ya está inscrito.",
            },
            export: {
                success: "La exportación de solicitudes se completó con éxito",
                error: "Error al exportar las solicitudes",
            },
            matrix: {
                button: "Matriz de aprobación",
                title: "Matriz de aprobación",
                back: "Aprobaciones",
                subtitle:
                    "Las reglas deciden qué pasa con cada inscripción. Decide la primera regla que se cumple.",
                versionChip: "Versión {{version}}",
                savedBy: "Guardada el {{date}} por {{user}}",
                builtIn: "Reglas integradas, en uso hasta que se guarde una versión",
                unsaved: "Cambios sin guardar",
                viewOnly: "Solo lectura",
                readOnlyTitle: "Puede ver las reglas, pero no cambiarlas",
                readOnlyText:
                    "Pida a un administrador que tenga el permiso approval-matrix-write que haga los cambios.",
                loadError: "No se pudo cargar la matriz de aprobación.",
                compared: "Qué comparamos",
                comparedHelp:
                    "Cada inscripción se compara con el votante encontrado en el registro. En los nombres no se tienen en cuenta mayúsculas, acentos ni guiones; para permisos de conducir y libretas de marino, el nombre y el segundo nombre se comparan juntos.",
                addCompared: "Comparar otro dato",
                rules: "Reglas",
                rulesHelp:
                    "Las reglas se comprueban desde arriba. Decide la primera que se cumple; si no se cumple ninguna, se aplica la última regla.",
                when: "Cuando",
                then: "Entonces",
                otherwise: "En otro caso",
                noneApply: "No se aplica ninguna de las reglas anteriores",
                andWord: "y",
                and: " y ",
                appliesToExample: "Se aplica a su ejemplo",
                cameFrom: "Decidió la inscripción desde la que ha llegado",
                voterIsTold: "Al votante se le dice: “{{reason}}”.",
                sentence: "Cuando {{when}}, {{outcome}}.",
                sentenceOtherwise: "Si no se aplica ninguna de las reglas anteriores, {{outcome}}.",
                sentenceEmpty: "Añada una condición para indicar cuándo se aplica esta regla.",
                addRule: "Añadir regla",
                discard: "Descartar los cambios",
                actions: {
                    edit: "Editar la regla {{number}}",
                    editOtherwise: "Editar la última regla",
                    moveUp: "Subir la regla {{number}}",
                    moveDown: "Bajar la regla {{number}}",
                    delete: "Eliminar la regla {{number}}",
                },
                saveBar: {
                    title: "Tiene cambios sin guardar",
                    fix_one: "Corrija 1 regla antes de guardar",
                    fix_other: "Corrija {{count}} reglas antes de guardar",
                    more: "+{{count}} más",
                },
                test: "Probar un ejemplo",
                testHelp:
                    "Describa una inscripción para ver qué regla la decide. Los cambios sin guardar también cuentan.",
                testDetails: "Datos comparados",
                applies: "Se aplica la regla {{number}}",
                otherwiseApplies: "Se aplica la última regla",
                testError: "No se pudo probar el ejemplo.",
                testInvalid: "Corrija estas reglas para probar un ejemplo:",
                ruleError: "Regla {{number}}: {{error}}",
                invariants: {
                    MANUAL_ENTRY_NOT_ACCEPTED:
                        "Una identidad escrita a mano nunca se aprueba automáticamente, así que esto se envía a una persona.",
                    ALREADY_ENROLLED_NOT_ACCEPTED:
                        "Un votante que ya está inscrito nunca se aprueba de nuevo.",
                    NO_VOTER_NOT_ACCEPTED: "No se aprueba a nadie sin un votante en el registro.",
                    OTHERWISE_NOT_ACCEPTED: "La última regla nunca aprueba.",
                },
                dialog: {
                    editTitle: "Editar la regla {{number}}",
                    newTitle: "Nueva regla",
                    otherwiseTitle: "Editar la última regla",
                    summary: "En resumen",
                    whenHelp: "Todas deben cumplirse. Omita una condición cuando no importe.",
                    otherwiseHelp: "Si no se aplica ninguna de las reglas anteriores",
                    addCondition: "Añadir condición",
                    remove: "Quitar “{{condition}}”",
                    identity: "Comprobación de identidad",
                    voterFound: "Votante en el registro",
                    alreadyEnrolled: "Ya inscrito",
                    validId: "Tipo de documento",
                    differing: "Datos que difieren",
                    decision: "Decisión",
                    reason: "Qué se le dice al votante",
                    voterSees: "El votante ve",
                    apply: "Aplicar",
                    close: "Cerrar",
                    yes: "Sí",
                    no: "No",
                    notReported: "No indicado",
                },
                identity: {
                    VERIFIED: "Verificada con documento escaneado",
                    MANUAL_ENTRY: "Escrita a mano",
                },
                differing: {
                    none: "Ninguno",
                    exactly_1: "Exactamente 1",
                    at_most_1: "Como máximo 1",
                    exactly_2: "Exactamente 2",
                    at_most_2: "Como máximo 2",
                    at_least_3: "3 o más",
                },
                fieldMatch: {
                    MATCHES: "Igual",
                    DIFFERS: "Distinto",
                },
                decisions: {
                    ACCEPTED: "Aprobar automáticamente",
                    PENDING: "Enviar a una persona",
                    REJECTED: "Rechazar",
                },
                outcomeShort: {
                    ACCEPTED: "aprobar automáticamente",
                    PENDING: "enviar a una persona",
                    REJECTED: "rechazar",
                },
                outcomeHelp: {
                    ACCEPTED: "El votante queda inscrito sin que nadie lo revise.",
                    PENDING:
                        "Decide un funcionario, y al votante se le dice que su inscripción está en revisión.",
                    REJECTED: "Al votante se le dice el motivo, y puede volver a inscribirse.",
                },
                outcomeSentence: {
                    ACCEPTED: "se aprueba la inscripción automáticamente",
                    PENDING: "se envía la inscripción a una persona",
                    REJECTED: "se rechaza la inscripción",
                },
                reasons: {
                    NO_VOTER: "Votante no coincidente",
                    ALREADY_APPROVED: "Ya aprobado",
                    INSUFFICIENT_INFORMATION: "Datos faltantes",
                    IDENTITY_NOT_VERIFIED: "Identidad no verificada",
                    OTHER: "Otro",
                },
                voterText: {
                    NO_VOTER:
                        "No hemos encontrado en el registro un votante que coincida con sus datos. Revise sus datos y vuelva a inscribirse, o póngase en contacto con su oficina electoral.",
                    ALREADY_APPROVED:
                        "Ya está inscrito. Podrá iniciar sesión para votar cuando se abra la votación.",
                    INSUFFICIENT_INFORMATION:
                        "No hemos podido inscribirle porque faltan algunos de sus datos o no se pueden leer. Vuelva a inscribirse con los datos completos.",
                    IDENTITY_NOT_VERIFIED:
                        "No hemos podido verificar su identidad automáticamente, así que un funcionario electoral revisará su inscripción.",
                    OTHER: "Un funcionario electoral escribe este mensaje cuando decide.",
                },
                conditions: {
                    any: "Aún no hay condiciones",
                    identity: {
                        VERIFIED: "Identidad verificada con documento escaneado",
                        MANUAL_ENTRY: "Identidad escrita a mano",
                    },
                    voterFound: {
                        true: "Votante encontrado en el registro",
                        false: "Ningún votante encontrado en el registro",
                    },
                    alreadyEnrolled: {
                        true: "Ya inscrito",
                        false: "Aún no inscrito",
                    },
                    validId: "Documento: {{id}}",
                    differing: {
                        none: "Todos los datos coinciden",
                        exactly_1: "Exactamente 1 dato difiere",
                        at_most_1: "Como máximo 1 dato difiere",
                        exactly_2: "Exactamente 2 datos difieren",
                        at_most_2: "Como máximo 2 datos difieren",
                        at_least_3: "3 o más datos difieren",
                    },
                    field: {
                        MATCHES: "{{field}} coincide",
                        DIFFERS: "{{field}} difiere",
                    },
                },
                errors: {
                    ACCEPTS_MANUAL_ENTRY:
                        "Las inscripciones cuya identidad se escribió a mano no se pueden aprobar automáticamente.",
                    ACCEPTS_ALREADY_ENROLLED:
                        "Un votante que ya está inscrito no se puede aprobar de nuevo.",
                    ACCEPTS_WITHOUT_VOTER:
                        "No se puede aprobar una inscripción sin un votante en el registro.",
                    OTHERWISE_ACCEPTS:
                        "La última regla puede enviar inscripciones a una persona o rechazarlas, pero no aprobarlas.",
                    MISSING_REASON: "Elija qué se le dice al votante.",
                    UNEXPECTED_REASON: "Una aprobación no tiene motivo.",
                    NO_COMPARED_FIELDS: "Elija al menos un dato para comparar con el registro.",
                    DUPLICATE_COMPARED_FIELD: "Hay un dato comparado repetido.",
                    UNKNOWN_FIELD: "Una regla usa un dato que no se compara.",
                    NO_CONDITIONS:
                        "Añada al menos una condición. Solo la última regla se aplica a todo lo demás.",
                },
                change: {
                    added: "Regla {{number}} añadida",
                    decision: "Regla {{number}}: {{from}} → {{to}}",
                    edited: "Regla {{number}} modificada",
                    removed: "Se eliminó una regla ({{text}})",
                    moved: "Se reordenaron las reglas",
                    otherwise: "La última regla ha cambiado",
                    compared: "Los datos comparados han cambiado",
                },
                save: {
                    button: "Guardar como versión {{version}}",
                    title: "¿Guardar como versión {{version}}?",
                    body: "A partir de ahora, las nuevas inscripciones se deciden con estas reglas. Las inscripciones ya decididas conservan su decisión.",
                    changes: "Qué ha cambiado",
                    log: "La nueva versión queda registrada en la bitácora electoral.",
                    confirm: "Guardar la versión {{version}}",
                    success: "Guardada como versión {{version}}",
                    error: "No se pudo guardar la matriz de aprobación",
                },
            },
        },
        monitoring: {
            configuredDashboards: "Paneles configurables",
            modeChangeError: "No se pudo cambiar el modo del panel. Inténtelo de nuevo.",
            title: "Monitorización",
            loading: "Cargando la monitorización",
            unavailableAlert:
                "No se han podido cargar los paneles de monitorización, así que se muestra el panel estándar.",
            noDashboards: "Este evento no tiene paneles de monitorización.",
            dashboardFailed: "No se ha podido cargar el panel de monitorización.",
            dashboardInvalid: "Este panel no se puede mostrar: {{problem}}",
            retry: "Reintentar",
            errors: {
                busy: "El servidor está ocupado. Se volverá a intentar en unos segundos.",
                forbiddenScope: "No puede ver esta región, Post o país. Elija otro.",
                snapshotPruned:
                    "La actualización mostrada ya no se conserva. El panel muestra ahora la última actualización: vuelva a exportar para usarla.",
                checksUnavailable:
                    "El servicio de gráficos no está disponible ahora. Inténtelo más tarde.",
                lockedDown: "El evento electoral está bloqueado, así que esto no se puede cambiar.",
                notFound: "Este panel o widget ya no está configurado. Recargue la página.",
                badRequest: "No se aceptó la solicitud. Recargue la página e inténtelo de nuevo.",
                conflict: "Otra persona guardó un cambio antes. Recargue e inténtelo de nuevo.",
                invalid: "Algunos valores de la exportación no se aceptan.",
                unknown: "Algo ha fallado. Inténtelo más tarde.",
            },
            header: {
                dashboard: "Panel",
                updated: "Actualizado {{time}} ({{timeZone}})",
                notUpdated: "Aún sin recuento",
                refresh: "cada {{seconds}} s",
                export: "Exportar",
                editDashboard: "Editar panel",
                preset: "Preajuste de paneles",
                reload: "Buscar cifras nuevas",
            },
            footer: {
                dataThrough: "Datos hasta {{time}} ({{timeZone}})",
            },
            selectors: {
                region: "Región",
                post: "Puesto",
                country: "País",
                allRegions: "Todas las regiones",
                allPosts: "Todos los puestos",
                allAuthorizedPosts: "Todos los puestos autorizados",
                allCountries: "Todos los países",
                authorizedOnly: "{{all}} (autorizados)",
            },
            widget: {
                menu: "Acciones de {{widget}}",
                configure: "Configurar widget",
                viewData: "Ver datos",
                export: "Exportar",
                duplicate: "Duplicar",
                loading: "Cargando {{widget}}",
                missing: "El panel nombra un widget que no existe: {{id}}",
                updating: "Actualizando {{widget}}",
                updatingNote: "Actualizando: el gráfico mostrado es el anterior.",
            },
            frame: {
                title: "Gráfico de {{widget}}",
            },
            sources: {
                voter_turnout: "Participación",
                test_voting: "Votación de prueba",
                enrollment_decisions: "Decisiones de inscripción",
                voting_credentials: "Credenciales de voto",
                poll_status: "Estado de la votación",
                final_testing_lockdown: "Pruebas finales y bloqueo",
                counting_transmission: "Recuento y transmisión",
                voting_enrollment_activity: "Actividad de voto e inscripción",
                access_security: "Acceso y seguridad",
                attack_detections: "Detección de ataques",
                helpdesk: "Soporte",
            },
            reasons: {
                TEST_ELECTION_DESIGNATION: "aún no se pueden marcar las elecciones de prueba",
                CREDENTIAL_ISSUED_EVENT: "aún no se registra la emisión de credenciales",
                FINAL_TESTING_LOCKDOWN_STATE:
                    "aún no se registran las pruebas finales y el bloqueo",
                ATTACK_DETECTION_FEED: "no hay ninguna fuente de detección de ataques conectada",
                HELPDESK_INTEGRATION: "no hay ningún sistema de soporte conectado",
            },
            notices: {
                UNREGISTERED_ATTEMPTS_AT_EVENT_SCOPE_ONLY:
                    "Los intentos con nombres de usuario no registrados no pertenecen a ningún puesto, por lo que solo se cuentan para todo el evento.",
                UNREGISTERED_ATTEMPTS_EXCLUDED:
                    "Estas cifras no incluyen los intentos con nombres de usuario no registrados; se cuentan para todo el evento.",
                CREDENTIALS_ISSUED_WHEN_PASSWORD_SET:
                    "Las credenciales cuentan como emitidas cuando se establece la contraseña del votante, hasta que la plataforma registre su emisión.",
                CONFIG_NEWER_THAN_SNAPSHOT:
                    "Dibujado con la configuración más reciente; las cifras se cuentan con ella en la próxima pasada.",
                CONFIG_AT_SNAPSHOT_UNAVAILABLE:
                    "Dibujado con la configuración actual: la configuración con la que se contaron las cifras ya no se conserva.",
            },
            unavailable: {
                notConnected: "No conectado · {{reason}}",
                notConnectedHelp: "No se muestra nada hasta que lo esté.",
                unknownReason: "fuente de datos no disponible",
                noSnapshot: "Aún sin recuento",
                noSnapshotHelp:
                    "El primer recuento no ha terminado. Este widget se actualiza solo.",
                scopePending: "Contando esta selección",
                scopePendingHelp:
                    "Esta selección se cuenta en la siguiente pasada, en aproximadamente un minuto.",
                settingsPending: "Contando con la nueva configuración…",
                settingsPendingHelp:
                    "Las cifras se vuelven a contar con la configuración guardada, en aproximadamente un minuto.",
                renderFailed: "No se ha podido dibujar el gráfico",
                renderFailedHelp: "En su lugar se muestran las cifras.",
                invalid: "Este widget no se puede mostrar",
                invalidHelp: "Su configuración tiene un problema.",
                requestFailed: "No se ha podido cargar este widget",
                requestFailedHelp: "Se vuelve a intentar cuando el panel se actualiza.",
            },
            dataTable: {
                title: "{{widget}} · datos",
                close: "Cerrar",
                empty: "Sin filas",
                rowsPerPage: "Filas por página:",
                shownRows: "{{from}}–{{to}} de {{total}}",
                firstPage: "Primera página",
                previousPage: "Página anterior",
                nextPage: "Página siguiente",
                lastPage: "Última página",
            },
            columns: {
                numerator: "Numerador",
                denominator: "Denominador",
                pct: "Porcentaje",
                pct_label: "Porcentaje mostrado",
                group: "Grupo",
                group_key: "Clave del grupo",
                post: "Puesto",
                post_id: "ID del puesto",
                region: "Región",
                country: "País",
                reason: "Motivo",
                category: "Categoría",
                state: "Estado",
                state_label: "Estado mostrado",
                bucket_start: "Inicio del periodo",
                bucket_label: "Periodo",
                bucket_utc: "Inicio del periodo (UTC)",
                measure: "Medida",
                label: "Etiqueta",
                value: "Valor",
            },
            measures: {
                registered: "Registrados",
                pre_enrolled: "Preinscritos",
                credentials_issued: "Credenciales emitidas",
                test_voted: "Votos de prueba",
                voted: "Votaron",
                voted_pre_enrolled: "Preinscritos que votaron",
                applications: "Solicitudes",
                pending: "Pendientes",
                approved: "Aprobadas",
                disapproved: "Rechazadas",
                posts: "Puestos",
                initialized: "Inicializados",
                opened: "Abiertos",
                paused: "En pausa",
                closed: "Cerrados",
                tested: "Probados",
                locked_down: "Bloqueados",
                tallied: "Escrutados",
                transmitted: "Transmitidos",
                transmission_failed: "Transmisión fallida",
                logins: "Inicios de sesión",
                login_failures: "Inicios de sesión fallidos",
                login_failures_valid_user: "Fallidos, usuario válido",
                login_failures_unregistered: "Fallidos, usuario no registrado",
                password_resets: "Restablecimientos de contraseña",
                password_reset_requests: "Solicitudes de restablecimiento de contraseña",
                detections: "Detecciones",
                issues: "Incidencias",
                pending_issues: "Incidencias pendientes",
            },
            export: {
                title: "Exportar datos de monitorización",
                format: "Formato",
                csv: "CSV",
                sql: "SQL",
                from: "Desde",
                to: "Hasta",
                timeZoneHelp:
                    "Las horas están en {{timeZone}}. Los totales, los estados y los grupos son los de la actualización mostrada; solo las filas de las series de actividad se limitan al intervalo, desde la hora de inicio hasta la hora de fin, sin incluirla.",
                cancel: "Cancelar",
                export: "Exportar",
                invalidRange: "El fin debe ser posterior al inicio.",
                problems: {
                    unknownSelector:
                        "{{widget}} ya no tiene la opción «{{selector}}». Recargue el panel y vuelva a exportar.",
                    unknownOption:
                        "El valor elegido para «{{selector}}» en {{widget}} ya no se ofrece. Elíjalo de nuevo y exporte.",
                },
            },
            editor: {
                scopeSelector: {
                    region: "Región",
                    post: "Puesto",
                    country: "País",
                },
                sources: {
                    voter_turnout: "Participación de votantes",
                    test_voting: "Votación de prueba",
                    enrollment_decisions: "Decisiones de inscripción",
                    voting_credentials: "Credenciales de votación",
                    poll_status: "Estado de la mesa",
                    final_testing_lockdown: "Pruebas finales y bloqueo",
                    counting_transmission: "Recuento y transmisión",
                    voting_enrollment_activity: "Actividad de votación e inscripción",
                    access_security: "Acceso y seguridad",
                    attack_detections: "Detecciones de ataques",
                    helpdesk: "Asistencia técnica",
                },
                templates: {
                    summary: "Resumen",
                    by_group: "Por grupo",
                    by_post: "Por puesto",
                    timeseries: "A lo largo del tiempo",
                    by_measure: "Por medida",
                },
                yaml: {
                    label: "YAML",
                    readOnly:
                        "El YAML tiene un error de sintaxis. Corríjalo en la pestaña YAML para volver a usar los formularios.",
                },
                diagnostics: {
                    title: "Comprobaciones",
                    none: "No se encontraron problemas",
                    localUnavailable:
                        "Las comprobaciones en el navegador no están disponibles; los problemas aparecen tras la vista previa o Validar.",
                    line: "Línea {{line}}",
                    engineCode: "dbt Charts {{code}}",
                    severity: {
                        ERROR: "Error",
                        WARNING: "Advertencia",
                    },
                    origin: {
                        SYNTAX: "Sintaxis YAML",
                        LOCAL: "Comprobación del navegador",
                        SERVER: "Comprobación del servidor",
                    },
                },
                footer: {
                    preview: "Vista previa",
                    valid: "Válido",
                    errors_one: "{{count}} error",
                    errors_other: "{{count}} errores",
                    noWarnings: "sin advertencias de gráficos",
                    warnings_one: "{{count}} advertencia de gráfico",
                    warnings_other: "{{count}} advertencias de gráficos",
                    rendering: "Generando…",
                    previewFailed: "falló la vista previa",
                    renderedIn: "generado en {{ms}} ms",
                    revision: "Revisión {{revision}}",
                    savedBy: "guardado {{date}} por {{user}}",
                    unknownUser: "un administrador",
                    notSaved: "aún sin guardar",
                    unsaved: "cambios sin guardar",
                    cancel: "Cancelar",
                    validate: "Validar",
                },
                preview: {
                    title: "Vista previa",
                    empty: "La vista previa aparece cuando el YAML es válido.",
                    failed: "No se pudo generar la vista previa: {{reason}}",
                    notConnectedTail: "No se muestra nada hasta que lo esté.",
                    queryResult: "Resultado de la consulta · primeras filas",
                    noRows: "La consulta no devolvió filas.",
                    state: {
                        RENDERED: "Generado",
                        NOT_CONNECTED: "No conectado",
                        NO_SNAPSHOT: "Aún sin datos",
                        SCOPE_PENDING: "Se está contando este ámbito",
                        RENDER_FAILED: "No se pudo dibujar el gráfico",
                        INVALID: "La configuración no es válida",
                    },
                },
                configureWidget: {
                    title: "Configurar widget",
                    tabs: {
                        dataQuery: "Datos y consulta",
                        selectors: "Selectores",
                        yaml: "YAML",
                        preview: "Vista previa",
                    },
                    save: "Guardar widget",
                    loading: "Cargando el widget…",
                    loadFailed: "No se pudo cargar el widget: {{reason}}",
                    saved: "Widget guardado como revisión {{revision}}",
                    refused: "No se guardó el widget: corrija los problemas indicados.",
                    validated: "El widget es válido.",
                    invalid: "El widget tiene problemas: consulte las comprobaciones.",
                    requestFailed: "La solicitud falló: {{reason}}",
                    discardTitle: "¿Descartar los cambios?",
                    discardBody: "Los cambios en este widget no se han guardado.",
                    discard: "Descartar",
                    keepEditing: "Seguir editando",
                },
                dataQuery: {
                    title: "Título",
                    source: "Fuente de datos",
                    template: "Consulta",
                    measures: "Medidas",
                    ratio: "Numerador / denominador",
                    numerator: "Numerador",
                    denominator: "Denominador",
                    groupBy: "Agrupar por",
                    sort: "Ordenar",
                    sortOrder: "Orden",
                    templateOrder: "El orden propio de la consulta",
                    sortBy: {
                        label: "Etiqueta",
                        value: "Valor",
                        ratio: "Proporción",
                    },
                    order: {
                        asc: "Ascendente",
                        desc: "Descendente",
                    },
                    limit: "Filas",
                    noLimit: "Todas",
                    none: "Ninguno",
                    fromSelector: "Desde el selector: {{name}}",
                    follow: "Seguir los selectores del panel",
                    followHelp: "Un selector sin marcar no limita este widget.",
                    manyQueries: "Este widget tiene varias consultas; edítelas en la pestaña YAML.",
                    notConnected: "No conectado · {{reason}}",
                    sourceHelp: {
                        DISTINCT_VOTERS:
                            "Cuenta votantes distintos; las reglas de recuento las fija la fuente de datos.",
                        DISTINCT_PRE_ENROLLED_VOTERS:
                            "Cuenta votantes preinscritos distintos; las reglas de recuento las fija la fuente de datos.",
                        LATEST_DECISION_PER_VOTER:
                            "Cuenta la última decisión por votante; las reglas de recuento las fija la fuente de datos.",
                        APPROVED_VOTERS:
                            "Cuenta votantes aprobados; las reglas de recuento las fija la fuente de datos.",
                        POSTS_IN_SCOPE:
                            "Cuenta los puestos del ámbito; las reglas de recuento las fija la fuente de datos.",
                        FIRST_EVENT_PER_VOTER:
                            "Cuenta el primer voto o aprobación válido de cada votante; las reglas de recuento las fija la fuente de datos.",
                        ATTEMPTS:
                            "Cuenta intentos, no personas; las reglas de recuento las fija la fuente de datos.",
                        DETECTIONS:
                            "Cuenta detecciones; las reglas de recuento las fija la fuente de datos.",
                        REPORTED_ISSUES:
                            "Cuenta incidencias notificadas; las reglas de recuento las fija la fuente de datos.",
                        default: "Las reglas de recuento las fija la fuente de datos.",
                    },
                },
                selectors: {
                    help: "Los selectores aparecen en el encabezado del widget. Sus valores alimentan la consulta; los selectores del panel (Región, Puesto, País) se aplican a todos los widgets.",
                    name: "Nombre",
                    label: "Etiqueta",
                    control: "Control",
                    controls: {
                        dropdown: "Lista desplegable",
                        toggle: "Interruptor",
                    },
                    default: "Predeterminado",
                    optionValue: "Valor",
                    optionLabel: "Etiqueta de la opción",
                    addOption: "Añadir opción",
                    addSelector: "Añadir selector",
                    removeOption: "Quitar la opción {{option}}",
                    removeSelector: "Quitar el selector {{name}}",
                    moveUp: "Subir {{name}}",
                    moveDown: "Bajar {{name}}",
                    dynamic: "Las opciones provienen de los datos ({{source}}).",
                    none: "Este widget no tiene selectores.",
                    newLabel: "Nuevo selector",
                    newOption: "Nueva opción",
                    shownWhen: "Se muestra cuando {{selector}} es {{values}}",
                },
                conflict: {
                    title: "Alguien guardó antes",
                    body: "{{user}} guardó la revisión {{revision}} ({{date}}) mientras usted editaba.",
                    bodyShort: "Se guardó la revisión {{revision}} mientras usted editaba.",
                    saved: "Revisión guardada",
                    mine: "Mis cambios",
                    reload: "Recargar",
                    copy: "Copiar mi YAML",
                    copied: "Su YAML está en el portapapeles.",
                    copyFailed:
                        "El portapapeles no está disponible; seleccione el YAML y cópielo a mano.",
                    keepEditing: "Seguir editando",
                    removed:
                        "El documento se eliminó mientras usted editaba. Siga editando para guardarlo de nuevo.",
                },
                dashboard: {
                    editing: "Editando el panel",
                    title: "Título",
                    selectors: "Selectores del panel",
                    theme: "Tema",
                    editTheme: "Editar tema",
                    addWidget: "Añadir widget",
                    cancel: "Cancelar",
                    save: "Guardar panel",
                    width: "Ancho",
                    widthValue: "{{n}} de 12",
                    moveUp: "Subir",
                    moveDown: "Bajar",
                    remove: "Quitar",
                    duplicate: "Duplicar",
                    configure: "Configurar widget",
                    empty: "Este panel aún no tiene widgets.",
                    saved: "Panel guardado como revisión {{revision}}",
                    refused: "No se guardó el panel: corrija los problemas indicados.",
                    requestFailed: "La solicitud falló: {{reason}}",
                    dragHandle: "Arrastre para reordenar {{title}}",
                    widgets: "Widgets",
                    duplicated: "Duplicado como {{id}}",
                    resetToPreset: "Restablecer al preajuste",
                    actions: "Acciones de {{title}}",
                    discardBody: "Tus cambios en este panel no se han guardado.",
                    duplicateInvalid: "La copia no se guardó: {{problem}}",
                    layoutMalformed:
                        "Algunos elementos del diseño no son un widget con un ancho. Corrígelos en la pestaña YAML para reordenar los widgets.",
                },
                catalog: {
                    title: "Añadir widget",
                    search: "Buscar widgets, fuentes de datos o requisitos",
                    add: "Añadir",
                    empty: "Ningún widget coincide.",
                    onDashboard: "En este panel",
                    close: "Cerrar",
                },
                theme: {
                    title: "Tema del panel",
                    subtitle: "estilo de dbt Charts aplicado a todos los widgets de este panel",
                    appliesTo_one: "se aplica a {{count}} widget",
                    appliesTo_other: "se aplica a {{count}} widgets",
                    apply: "Aplicar tema",
                    saved: "Tema guardado como revisión {{revision}}",
                    discardBody: "Los cambios en este tema no se han guardado.",
                },
                reset: {
                    title: "Restablecer al preajuste",
                    body: "Todos los paneles, widgets y temas de este evento se sustituyen por los del preajuste. La configuración actual permanece en el historial.",
                    preset: "Preajuste",
                    confirm: "Restablecer",
                    cancel: "Cancelar",
                    done: "El evento ahora usa {{title}}.",
                    failed: "El restablecimiento falló: {{reason}}",
                    noPresets: "No hay preajustes disponibles.",
                    loading: "Cargando preajustes…",
                },
                lockedDown:
                    "El evento está bloqueado; su configuración de monitorización no puede cambiar.",
                document: {
                    loadFailed: "No se pudo cargar el documento: {{reason}}",
                    refused: "No se guardó: corrige los problemas indicados.",
                    validated: "El documento es válido.",
                    invalid: "El documento tiene problemas: consulta las comprobaciones.",
                    requestFailed: "La solicitud falló: {{reason}}",
                    savedWithWarnings_one: "{{count}} advertencia: consulta las comprobaciones.",
                    savedWithWarnings_other: "{{count}} advertencias: consulta las comprobaciones.",
                },
                errors: {
                    checksUnavailable:
                        "El motor de gráficos no pudo comprobar el cambio, así que no se guardó. Inténtalo de nuevo en un momento.",
                    busy: "Se están guardando otros cambios de este evento. Inténtalo de nuevo en un momento.",
                    lockedDown:
                        "El evento está bloqueado; su configuración de monitorización no puede cambiar.",
                    forbiddenScope:
                        "No puedes ver cifras de la región, el puesto o el país elegidos.",
                    badRequest:
                        "El editor envió una solicitud que el servidor no pudo leer. Recargue la página e inténtelo de nuevo.",
                },
                duplicate: {
                    copyTitle: "{{title}} (copia)",
                    done: "Se añadió {{id}}, una copia del widget, al panel.",
                    failed: "No se pudo duplicar el widget: {{reason}}",
                    notPlaced:
                        "La copia {{id}} se guardó, pero el panel no la incluyó ({{reason}}). Añádala con Editar panel.",
                },
            },
        },
        monitoringDashboardScreen: {
            voters: {
                title: "Votantes",
                enrolledOverseasVoters: "Votantes Inscritos en el Extranjero",
                approvalStatus: "Estado de Aprobación: Votantes Aprobados/Desaprobados",
                manuallyApproval: "Votantes Aprobados/Desaprobados Manualmente",
                automaticallyApproval: "Votantes Aprobados/Desaprobados Automáticamente",
                authenticatedVoters: "Votantes Autenticados",
                invalidUserErrors: "Errores de Usuario Inválido:",
                invalidPasswordErrors: "Errores de Contraseña Inválida:",
            },
            polls: {
                title: "Encuestas",
                initializedSystems: "Publicaciones con Sistemas Inicializados",
                votingOpened: "Publicaciones con Votación Abierta",
                votingClosed: "Publicaciones con Votación Cerrada",
                votingStarted: "Publicaciones con Votación Iniciada",
                voterTurnout: "Participación de los Votantes",
            },
            tally: {
                title: "Conteo",
                activeVotesCounting: "Publicaciones con Conteo de Votos Activo",
                generatedERs: "Publicaciones con ERs Generados",
                transmittedResults: "Publicaciones con Resultados Transmitidos",
            },
            testing: {
                title: "Pruebas",
                testElectionVoterCount: "Conteo de Votantes en Elección de Prueba",
            },
        },
        certificateAuthorities: {
            title: "Certificados",
            subtitle:
                "Autoridades de certificación (CA) de confianza para este evento electoral. Los CA importados se usan para validar los certificados de los votantes.",
            importButton: "Importar certificados",
            type: {
                root: "Raíz",
                intermediate: "Intermedio",
            },
            expiry: {
                expired: "Vencido",
                expiringSoon: "Por vencer",
                valid: "Válido",
            },
            columns: {
                commonName: "Nombre común",
                type: "Tipo",
                issuerCn: "CN del emisor",
                notBefore: "Válido desde",
                notAfter: "Vence",
                fingerprint: "Huella SHA256",
            },
            importDialog: {
                title: "Importar autoridades de certificación",
                subtitle: "Importar uno o más certificados CA desde un archivo PEM",
                description:
                    "Seleccione un archivo PEM que contenga uno o más certificados. Se admiten paquetes — cada certificado se importa individualmente.",
                selectFile: "Seleccionar archivo PEM",
                fileLoaded: "Archivo cargado ({{bytes}} bytes)",
                importButton: "Importar",
            },
            notify: {
                importSuccess: "Se importaron {{inserted}} certificado(s).",
                importSkipped: "{{count}} certificado(s) omitido(s) (ya presentes).",
                importErrors: "Problemas en la importación: {{errors}}",
                importError: "Error al importar: {{error}}",
                deleteSuccess: "Certificado eliminado.",
                deleteError: "Error al eliminar el certificado.",
                exportSuccess: "Certificado(s) exportado(s) correctamente.",
                exportError: "Error al exportar los certificados.",
            },
            exportDialog: {
                title: "Exportar autoridades de certificación",
                description: "Está a punto de exportar {{amount}} certificado(s).",
                all: "todos",
            },
            deleteDialog: {
                description: "¿Está seguro de que desea eliminar {{count}} certificado(s)?",
            },
            emptyHeader:
                "No se han importado autoridades de certificación para este evento electoral.",
            fileReadError: "Error al leer el archivo.",
            viewDialog: {
                title: "Detalles de la autoridad de certificación",
                subject: "Asunto",
                issuer: "Emisor",
                serialNumber: "Número de serie",
                pemContent: "Contenido PEM",
            },
            confirmDelete: "Eliminar autoridad de certificación",
            confirmDeleteDescription:
                '¿Está seguro de que desea eliminar el certificado "{{name}}" (huella: {{fingerprint}})?',
        },
        signing: {
            terms: {
                post: "Puesto",
                posts: "Puestos",
            },
            tab: {
                title: "Firmas",
                intro: "Las acciones protegidas solo se ejecutan cuando suficientes personas autorizadas las firman con sus certificados digitales. Cada firma se comprueba con los emisores de confianza y queda registrada en el registro.",
                protectedActions: "Acciones protegidas",
                certificates: "Certificados",
                requests: "Solicitudes",
            },
            loadError:
                "No se pudo cargar la configuración de firmas. Recargue la página para intentarlo de nuevo.",
            errors: {
                automatedCeremonies:
                    "Este evento utiliza ceremonias de claves automáticas. Los custodios no realizan estos pasos, por lo que no se pueden exigir sus firmas. Para exigir las firmas de los custodios, utiliza ceremonias de claves manuales.",
                forbidden: "No tiene permiso para este cambio.",
                invalid: "El servidor rechazó estos valores. Revíselos e inténtelo de nuevo.",
                conflict:
                    "Otra persona lo cambió mientras tanto. Recargue la página e inténtelo de nuevo.",
                lockedDown:
                    "El evento electoral está bloqueado: las reglas de firma solo cambian mediante una nueva versión de configuración.",
                notFound: "Ya no existe. Recargue la página.",
            },
            readOnly: {
                chip: "Solo lectura",
                rules: "Solo lectura. Para cambiar las reglas de firma se necesita el permiso «Firmas: editar acciones protegidas».",
                whoCanSign:
                    "Roles con el permiso «Firmar: {{action}}» en Usuarios y Roles. Para cambiarlos se necesita permiso para editar roles.",
            },
            groups: {
                "voting": "Votación",
                "results-and-reports": "Resultados e informes",
                "enrollment": "Inscripción",
                "configuration-and-keys": "Configuración y claves",
            },
            actions: {
                "initialize-voting": {
                    label: "Inicializar la votación",
                    short: "Inicialización",
                    permissionName: "inicializar la votación",
                    object: "inicialización de la votación",
                    appliesTo: "Cada $t(signing.terms.post)",
                    description:
                        "Se inicia en Publicar. Inicializa el $t(signing.terms.post) y genera su Informe de Inicialización.",
                },
                "open-voting": {
                    label: "Abrir la votación",
                    short: "Apertura",
                    permissionName: "abrir la votación",
                    object: "apertura de la votación",
                    appliesTo: "Cada $t(signing.terms.post)",
                    description:
                        "Se inicia en Publicar con Comenzar votación. Abre la votación en el $t(signing.terms.post).",
                },
                "close-voting": {
                    label: "Cerrar la votación",
                    short: "Cierre",
                    permissionName: "cerrar la votación",
                    object: "cierre de la votación",
                    appliesTo: "Cada $t(signing.terms.post)",
                    description:
                        "Se inicia en Publicar con Detener votación. Cierra la votación en el $t(signing.terms.post); las firmas de cierre se conservan en su acta.",
                    descriptionSealed:
                        "Se inicia en Publicar con Detener votación. Cierra la votación en el $t(signing.terms.post). Cuando todos los canales están cerrados, sus urnas se sellan, tras el periodo de gracia si lo hay: no se puede añadir, cambiar ni borrar ninguna papeleta, y la votación no puede volver a empezar. Las firmas de cierre se conservan en su acta.",
                },
                "generate-election-returns": {
                    label: "Generar actas electorales",
                    short: "Actas electorales",
                    permissionName: "generar actas electorales",
                    object: "actas electorales",
                    appliesTo: "Cada $t(signing.terms.post) y país",
                    description:
                        "Lo inicia el recuento, una solicitud por $t(signing.terms.post) y país. Libera las actas electorales firmadas para su impresión y transmisión.",
                },
                "generate-reports": {
                    label: "Generar otros informes electorales",
                    short: "Informe",
                    permissionName: "generar otros informes electorales",
                    object: "informe",
                    appliesTo: "Cada $t(signing.terms.post)",
                    description:
                        "Lo inicia el recuento para el Informe de Inicialización y Informes para el informe de participación. Libera el informe firmado.",
                },
                "transmit-results": {
                    label: "Transmitir resultados",
                    short: "Transmisión",
                    permissionName: "transmitir resultados",
                    object: "paquete de resultados",
                    appliesTo: "Cada $t(signing.terms.post) y país",
                    description:
                        "Se inicia en Recuento, Transmisión. Genera el paquete de resultados firmado para sus destinos; las firmas completan su lista de firmas.",
                },
                "approve-voter": {
                    label: "Aprobar manualmente a un votante",
                    short: "Aprobación de votante",
                    permissionName: "aprobar manualmente a un votante",
                    object: "aprobación de votante",
                    appliesTo: "El $t(signing.terms.post) del votante",
                    description:
                        "Se inicia en Aprobaciones. Aprueba al votante y emite sus credenciales.",
                },
                "approve-configuration": {
                    label: "Aprobar una versión de configuración",
                    short: "Versión de configuración",
                    permissionName: "aprobar una versión de configuración",
                    object: "versión de configuración",
                    appliesTo: "El evento electoral",
                    description: "Se inicia en Publicar. Publica la versión de configuración.",
                },
                "key-ceremony": {
                    label: "Confirmar un fragmento de clave (ceremonia de claves)",
                    short: "Fragmento de clave",
                    permissionName: "confirmar un fragmento de clave",
                    object: "fragmento de clave",
                    appliesTo: "Cada autoridad",
                    description:
                        "Lo inicia cada autoridad en Claves. Registra la firma de la autoridad en la ceremonia y en el tablón de anuncios.",
                },
                "tally-key": {
                    label: "Aportar un fragmento de clave (recuento)",
                    short: "Aportación de fragmento de clave",
                    permissionName: "aportar un fragmento de clave",
                    object: "aportación de fragmento de clave",
                    appliesTo: "Cada autoridad",
                    description:
                        "Lo inicia cada autoridad en Recuento. Registra la aportación de la autoridad.",
                },
            },
            protectedActions: {
                intro: "Cada firma se realiza con el certificado digital del token de seguridad del firmante.",
                columns: {
                    action: "Acción",
                    appliesTo: "Se aplica a",
                    whoCanSign: "Quién puede firmar",
                    signaturesNeeded: "Firmas necesarias",
                    requestExpires: "Caducidad de la solicitud",
                    waiting: "En espera",
                },
                off: "Desactivada",
                eachTrustee: "Cada autoridad",
                footerVersion:
                    "Las reglas de firma forman parte de la versión de configuración {{version}} de este evento.",
                footerFirstVersion:
                    "Las reglas de firma pasarán a formar parte de la primera versión de configuración de este evento cuando se publique.",
                footerChanged: "Último cambio: {{date}}.",
                footerChangedBy: "Último cambio: {{date}}, por {{name}}.",
                lockedDown:
                    "El evento electoral está bloqueado: sus reglas de firma pertenecen a su versión de configuración, por lo que solo cambian mediante una nueva versión de configuración.",
                edit: "Editar {{action}}",
                view: "Ver {{action}}",
                waitingCount_one: "{{count}} solicitud en espera",
                waitingCount_other: "{{count}} solicitudes en espera",
                capacityError:
                    "No se pudo cargar quién puede firmar, así que el número de firmas no se puede comprobar con los $t(signing.terms.posts).",
            },
            expiry: {
                "30": "30 minutos",
                "60": "1 hora",
                "120": "2 horas",
                "1440": "24 horas",
                "none": "Sin límite",
                "other": "{{count}} minutos",
            },
            rule: {
                needsSignatures: "Requiere firmas",
                whoCanSign: "Quién puede firmar",
                whoCanSignHelp:
                    "Estos roles reciben el permiso «Firmar: {{action}}» en Usuarios y Roles, para todos los eventos electorales. Los firmantes también deben tener acceso al $t(signing.terms.post).",
                signaturesNeeded: "Firmas necesarias",
                signaturesNeededHelp:
                    "Cada firmante usa su certificado digital. Cada $t(signing.terms.post) tiene al menos {{n}} personas que pueden firmar.",
                signaturesNeededShortHelp: "Cada firmante usa su certificado digital.",
                requesterSigning: "La persona que la inicia también puede firmar",
                expiresAfter: "Una solicitud caduca después de",
                trusteesSign: "Las autoridades firman este paso",
                trusteesHelp:
                    "Cada autoridad firma su propio paso con su certificado digital. La ceremonia de claves determina cuántas autoridades participan.",
                footer: "Los cambios quedan registrados en el registro del evento electoral y pasan a formar parte de la siguiente versión de configuración.",
                cancel: "Cancelar",
                save: "Guardar",
                saved: "Se guardó la regla de firma.",
                savedShort_one:
                    "Se guardó la regla de firma. {{posts}} aún no puede alcanzar el número: añada un firmante allí.",
                savedShort_other:
                    "Se guardó la regla de firma. {{posts}} aún no pueden alcanzar el número: añada firmantes allí.",
                checkedOnSave: "El número se comprueba con los nuevos roles al guardar.",
                savedRequesterShort:
                    "Se guardó la regla de firma. Algunos $t(signing.terms.posts) no pueden alcanzar el número sin la persona que inicia una solicitud.",
                saveError:
                    "No se pudo guardar la regla de firma. Es posible que otra persona la haya cambiado mientras tanto; recargue e inténtelo de nuevo.",
            },
            validation: {
                atLeastOne: "Al menos 1.",
                tooMany:
                    "Ningún $t(signing.terms.post) tiene {{n}} personas que puedan firmar. El máximo es {{max}}.",
                tooManyEvent:
                    "Solo {{max}} personas pueden firmar esto. Elija {{max}} como máximo.",
                atMost: "Como máximo {{max}}.",
                shortPosts_one:
                    "{{posts}} solo tiene {{n}} personas que pueden firmar, así que no puede alcanzar {{required}} firmas. Añada un firmante allí o reduzca el número.",
                requesterShort_one:
                    "Sin la persona que la inicia, {{posts}} solo tiene {{n}} personas que pueden firmar, así que no puede alcanzar {{required}} firmas.",
                requesterShort_other:
                    "Sin la persona que la inicia, {{posts}} solo tienen {{n}} personas que pueden firmar, así que no pueden alcanzar {{required}} firmas.",
                shortPosts_other:
                    "{{posts}} solo tienen {{n}} personas que pueden firmar, así que no pueden alcanzar {{required}} firmas. Añada un firmante allí o reduzca el número.",
            },
            pendingRequests_one:
                "{{count}} solicitud está esperando firmas con la regla actual. Al guardar se cancela; la persona que la inició tendrá que empezar de nuevo.",
            pendingRequests_other:
                "{{count}} solicitudes están esperando firmas con la regla actual. Al guardar se cancelan; las personas que las iniciaron tendrán que empezar de nuevo.",
            certificates: {
                issuersIntro:
                    "Los certificados del personal deben encadenar con uno de estos. Son distintos de los certificados con los que inician sesión los votantes.",
                checkRevocation: "Comprobar las listas de revocación",
                crlUnavailable: {
                    "label": "Cuando no se puede descargar una lista",
                    "refuse": "No aceptar firmas",
                    "accept-unchecked": "Aceptar y marcar la firma como no comprobada",
                },
                registration: {
                    "label": "Registro de un certificado a nombre de una persona",
                    "on-first-use": "Cuando su titular firma con él por primera vez",
                    "security-officer-only":
                        "Solo cuando lo registra alguien que puede registrar certificados",
                },
                onePost: "Un certificado firma solo para un $t(signing.terms.post)",
                issuers: "Emisores de confianza",
                import: "Importar certificados de emisores",
                importHelp:
                    "Elija un archivo PEM o CER con el certificado del emisor. Un archivo PEM puede contener varios certificados.",
                chooseFile: "Elegir un archivo de certificado",
                fileError: "No se pudo leer el archivo.",
                imported:
                    "{{imported}} certificados de emisores importados; {{skipped}} ya eran de confianza.",
                importedWithErrors:
                    "{{imported}} certificados de emisores importados, {{skipped}} ya eran de confianza. Rechazados: {{errors}}",
                importError: "No se pudieron importar los certificados de emisores.",
                deleteIssuer: "Eliminar {{name}}",
                deleteIssuerConfirm:
                    "¿Eliminar {{name}} de los emisores de confianza? Los certificados que emitió ya no podrán firmar.",
                deleteError: "No se pudo eliminar el emisor.",
                noIssuers:
                    "Aún no hay emisores de confianza. El personal no puede firmar hasta que se importe uno.",
                root: "Raíz",
                intermediate: "Intermedio",
                columns: {
                    issuer: "Emisor",
                    type: "Tipo",
                    issuedBy: "Emitido por",
                    validUntil: "Válido hasta",
                    sha256: "SHA-256",
                    person: "Persona",
                    post: "$t(signing.terms.post)",
                    certificate: "Certificado",
                    registered: "Registrado",
                    status: "Estado",
                },
                checks: "Comprobaciones",
                checksSaved: "Se guardaron las comprobaciones de certificados.",
                checksError: "No se pudieron guardar las comprobaciones de certificados.",
                crlSchedule: "Se descargan de cada emisor cada hora.",
                crlUpdated: "{{url}}: actualizada {{time}}",
                crlFailed: "{{url}}: no se pudo descargar (último intento {{time}})",
                registeredTitle: "Certificados registrados",
                search: "Buscar personas, certificados o $t(signing.terms.posts)",
                status: "Estado",
                statusAll: "Todos",
                statuses: {
                    "active": "Activo",
                    "expires-soon": "Caduca pronto",
                    "expired": "Caducado",
                    "revoked": "Revocado",
                },
                revokedOn: "Revocado el {{date}}",
                allPosts: "Todos",
                noCertificates: "No hay certificados registrados.",
                registeredHow: {
                    "first-use": "En la primera firma",
                    "security-officer": "Registrado por un administrador",
                },
                register: "Registrar un certificado",
                registerSubmit: "Registrar",
                registerDone: "Se registró el certificado.",
                registerError: "No se pudo registrar el certificado.",
                person: "Persona",
                personSearchHelp:
                    "Escriba parte de un nombre de usuario para encontrar a la persona.",
                registeredBy: "Por {{name}}",
                registerRefused:
                    "Este certificado no se puede registrar: compruebe que lo emitió un emisor de confianza, que es válido hoy y que está destinado a firmar.",
                registeredToOther:
                    "Este certificado está registrado a nombre de {{name}}. Si esta cuenta también es de {{name}}, vincúlela como su segunda cuenta.",
                linkAccount: "Vincular como segunda cuenta de la misma persona",
                alreadyRegistered: "Este certificado ya está registrado a nombre de esta persona.",
                pem: "Certificado (PEM)",
                revoke: "Revocar",
                revokeOf: "Revocar el certificado de {{name}}",
                revokeTitle: "Revocar el certificado de {{name}}",
                revokeHelp:
                    "Un certificado revocado ya no puede firmar. Las firmas que ya realizó siguen siendo válidas.",
                revokeReason: "Motivo",
                revokeDone: "Se revocó el certificado.",
                revokeError: "No se pudo revocar el certificado.",
            },
            requests: {
                exportCsv: "Exportar CSV",
                exportError: "No se pudieron exportar las solicitudes.",
                exportFileName: "signing-requests.csv",
                status: "Estado",
                statusAll: "Todas",
                statusCount: "{{status}} · {{count}} de {{total}}",
                expires: "Caduca {{time}}",
                lastSignatureBy: "{{name}}, {{time}}",
                empty: "Aún no hay solicitudes de firma.",
                columns: {
                    request: "Solicitud",
                    status: "Estado",
                    started: "Iniciada",
                    by: "Por",
                    lastSignature: "Última firma",
                    code: "Código",
                },
            },
            reports: {
                postRequired:
                    "Selecciona un puesto para generar este informe cuando se requieren firmas.",
                generateNotice:
                    "{{post}}: el documento se genera ahora. Se podrá imprimir y transmitir cuando lo hayan firmado {{n}} personas.",
            },
            status: {
                waiting: "En espera",
                completed: "Firmada",
                executed: "Hecha",
                cancelled: "Cancelada",
                expired: "Caducada",
                failed: "Fallida",
            },
            cancelReasons: {
                "by-requester": "La persona que la inició la canceló",
                "by-operator": "Un operador la canceló",
                "rule-changed": "Cambió la regla de firma de la acción",
                "payload-changed": "Cambió lo que se firma",
                "superseded": "Una solicitud más reciente la sustituyó",
                "certificate-revoked": "Se revocó un certificado que la firmó",
            },
            panel: {
                rulePost:
                    "Requiere {{n}} firmas de los firmantes de {{post}}, cada una con su certificado digital.",
                ruleEvent:
                    "Requiere {{n}} firmas, cada una con el certificado digital del firmante.",
                signingCode: "Código de firma",
                signers: "Firmantes",
                sign: "Firmar",
                handover: "El siguiente miembro inicia sesión",
                cancel: "Cancelar solicitud",
                signedAt: "Firmado {{time}}",
                notSigned: "Sin firmar",
                certificate: "Certificado {{name}}",
                you: "(usted)",
                expiresAt: "Caduca a las {{time}}",
                progress: "{{count}} de {{total}}",
                openDocument: "Abrir el documento",
                configurationVersion: "Versión de configuración {{version}}",
                configurationChanges: "Cambios en esta versión",
            },
            dialog: {
                title: "Firmar {{object}}",
                steps: {
                    check: "Revisar",
                    certificate: "Certificado",
                    signed: "Firmado",
                },
                localNote:
                    "La firma se realiza en este navegador. Su archivo de certificado, su clave privada y su contraseña nunca se envían. Solo su firma y su certificado público llegan al servidor.",
                check: {
                    signingAs: "Está firmando como {{name}}",
                    titlePost: "{{title}}, {{post}}",
                    sameCode: "Todas las personas que firman ven el mismo código.",
                    confirmDocument: "He revisado lo que firmo: {{object}}",
                },
                certificate: {
                    intro: "Inserte su token de seguridad y elija su archivo de certificado.",
                    password: "Contraseña del certificado",
                    open: "Abrir certificado",
                    chooseAnother: "Elegir otro archivo",
                },
                checks: {
                    "passed": {
                        "trusted-issuer": "Emitido por un emisor de confianza ({{root}})",
                        "valid-now": "Válido hoy",
                        "signing-key-usage": "Destinado a firmar",
                        "not-revoked": "No revocado (listas actualizadas {{time}})",
                        "registered": "Registrado a su nombre el {{date}}",
                        "registered-to-other": "No registrado a nombre de otra persona",
                        "already-signed": "Aún no usado para esta solicitud",
                        "post-binding": "Registrado para este $t(signing.terms.post)",
                        "signature": "La firma cubre esta solicitud",
                    },
                    "failed": {
                        "trusted-issuer": "No emitido por un emisor de confianza",
                        "valid-now": "No válido hoy",
                        "signing-key-usage": "No destinado a firmar",
                        "not-revoked":
                            "Revocado, o no hay una lista de revocación vigente para comprobarlo",
                        "registered": "No registrado a su nombre",
                        "registered-to-other": "Registrado a nombre de {{name}}",
                        "already-signed": "Ya usado para esta solicitud",
                        "post-binding": "Registrado para otro $t(signing.terms.post)",
                        "signature": "La firma no cubre esta solicitud",
                    },
                    "first-use": "Primer uso: se registrará a su nombre",
                },
                problems: {
                    wrongPassword: "Contraseña incorrecta. Revísela e inténtelo de nuevo.",
                    notForYou:
                        "Este certificado no puede firmar por usted. Use el certificado de su propio token de seguridad.",
                    issuerNotAccepted:
                        "Use el certificado que {{organization}} registró para usted. No se aceptan certificados de otros emisores.",
                    cancelled:
                        "Esta solicitud se canceló: {{reason}}. Las firmas dadas para ella ya no cuentan. Iníciela de nuevo para firmar la versión actual.",
                },
                signed: {
                    title: "Firmado",
                    withCertificate: "con el certificado de {{name}}",
                    count: "{{n}} de {{total}} firmas.",
                    allIn: "Ya están las {{total}} firmas.",
                    next: "A continuación firman: {{names}}.",
                },
                handover:
                    "Se cerrará su sesión. El siguiente miembro inicia sesión en este equipo y vuelve a esta solicitud para firmar. La solicitud sigue abierta hasta las {{time}}.",
                sign: "Firmar",
                back: "Atrás",
                cancel: "Cancelar",
            },
            widget: {
                continue: "Continuar",
                done: "Hecho",
                close: "Cerrar",
                retry: "Reintentar",
                loading: "Cargando la solicitud…",
                loadError: "No se pudo cargar la solicitud.",
                chooseFile: "Elegir archivo de certificado",
                fileInput: "Archivo de certificado",
                fileSize: "{{size}} KB",
                showPassword: "Mostrar contraseña",
                hidePassword: "Ocultar contraseña",
                opening: "Abriendo el certificado…",
                checking: "Comprobando el certificado…",
                signing: "Firmando…",
                certificateCard: "Emitido por {{issuer}} · válido hasta {{date}} · {{algorithm}}",
                fingerprint: "SHA-256 {{fingerprint}}",
                algorithms: {
                    "rsa-pkcs1-sha256": "RSA",
                    "ecdsa-p256-sha256": "EC P-256",
                },
                document: "{{type}} · SHA-256 {{hash}}",
                documentPages: "{{type}} · {{pages}} páginas · SHA-256 {{hash}}",
                checksTitle: "Comprobaciones del certificado",
                untrustedIssuer:
                    "{{issuer}} no es un emisor de confianza para este evento electoral",
                registeredToSomeoneElse: "Registrado a nombre de otra persona",
                checkPassedNoDetail: {
                    "trusted-issuer": "Emitido por un emisor de confianza",
                    "not-revoked": "No revocado",
                },
                organization: "su organización",
                cantSign: "Este certificado no puede firmar esta solicitud.",
                checkError: "No se pudo comprobar el certificado. Inténtelo de nuevo.",
                fileErrors: {
                    UNREADABLE_FILE:
                        "Este archivo no es un archivo de certificado (.p12 o .pfx), o está dañado.",
                    UNSUPPORTED_ENCRYPTION:
                        "Este navegador no puede abrir el cifrado que usa este archivo.",
                    NO_PRIVATE_KEY:
                        "Este archivo no tiene clave privada. Elija el archivo de certificado de su token de seguridad.",
                    NO_CERTIFICATE: "Este archivo no tiene ningún certificado.",
                    UNSUPPORTED_KEY:
                        "El tipo de clave de este certificado no es compatible. Use un certificado RSA o EC P-256.",
                    KEY_CERTIFICATE_MISMATCH:
                        "El certificado de este archivo no coincide con su clave.",
                },
                openError: "No se pudo abrir el certificado. Inténtelo de nuevo.",
                signError: "No se pudo enviar la firma. Inténtelo de nuevo.",
                refused: "El servidor rechazó la firma.",
                stale: "El documento cambió mientras firmaba. Firme de nuevo.",
                mismatch:
                    "Lo que se firmaría no coincide con esta solicitud. Cierre el diálogo y vuelva a abrir la solicitud.",
                documentMismatch: "El documento no coincide con el que firma esta solicitud.",
                documentError: "No se pudo descargar el documento. Inténtelo de nuevo.",
                alreadySigned: "Ya ha firmado esta solicitud.",
                closed: {
                    changed:
                        "Esta solicitud cambió después de abrirla. Cierre esta ventana y revísela de nuevo antes de firmar.",
                    allSigned: "Esta solicitud ya tiene todas sus firmas.",
                },
                chooseCertificate: "Certificado con el que firmar",
                renderError:
                    "No se pudo mostrar la solicitud de firma. Ciérrela y vuelva a abrirla.",
                signedAt: "{{time}}",
                panel: {
                    completedAt: "Firmada a las {{time}}",
                    expired:
                        "Esta solicitud caducó. Las firmas dadas para ella ya no cuentan. Iníciela de nuevo para firmar.",
                    failed: "Están todas las firmas, pero la acción falló. El registro tiene los detalles.",
                    details: "Detalles",
                    close: "Cerrar el panel de la solicitud",
                },
                cancelDialog: {
                    title: "¿Cancelar esta solicitud?",
                    body: "Las firmas dadas para ella ya no cuentan. La persona que la inició tendrá que empezar de nuevo.",
                    reason: "Motivo (opcional)",
                    confirm: "Cancelar solicitud",
                    back: "Mantenerla",
                    error: "No se pudo cancelar la solicitud. Inténtelo de nuevo.",
                },
                handoverDialog: {
                    title: "El siguiente miembro inicia sesión",
                    noExpiry:
                        "Se cerrará su sesión. El siguiente miembro inicia sesión en este equipo y vuelve a esta solicitud para firmar.",
                    confirm: "Cerrar sesión",
                    back: "Mantener la sesión",
                    error: "No se pudo registrar el relevo. Inténtelo de nuevo.",
                },
            },
            details: {
                keys_ceremony_id: "Ceremonia",
                tally_session_id: "Sesión de recuento",
                trustee_id: "Autoridad",
                custody_proof_sha256: "SHA-256 del prueba de custodia",
                channel: "Canal",
                channels: "Canales",
                publication_id: "Publicación de papeletas",
                ballot_publication_id: "Publicación de papeletas",
                digest: "SHA-256 de la configuración",
                signing_rules: "Reglas de firma",
                scheduled_events: "Nuevos eventos programados",
                ballots_and_contests: "Papeletas y concursos",
                application_id: "Solicitud de inscripción",
                applicant_registry_id: "Cuenta del registro",
                decision: "Decisión",
                submitted_at: "Enviada",
                reason: "Por qué requiere a una persona",
                registry_record: "Registro del padrón",
                status: "Estado de la solicitud de inscripción",
                from: "Estado anterior",
            },
            closed: {
                pending: "Están todas las firmas. La votación se cierra en un momento.",
                title: "La votación se cerró a las {{time}}.",
                titleSealed: "La votación se cerró a las {{time}}. Papeletas selladas.",
                record: "Acta de sellado",
                ballots: "Papeletas en el sello",
                sealHash: "{{algorithm}} del sello",
                signedBy: "Firmado por",
                signatures: "Firmas de cierre en el acta de sellado",
                signaturesValue_one: "{{count}}, código de firma {{code}}",
                signaturesValue_other: "{{count}}, código de firma {{code}}",
                signers: "Firmado por los miembros",
            },
            values: {
                ballots_and_contests: {
                    "first-version": "Primera versión",
                    "no-changes": "Sin cambios",
                    "changed": "Con cambios",
                },
                signing_rules: {
                    "initialize-voting": "$t(signing.actions.initialize-voting.label)",
                    "open-voting": "$t(signing.actions.open-voting.label)",
                    "close-voting": "$t(signing.actions.close-voting.label)",
                    "generate-election-returns":
                        "$t(signing.actions.generate-election-returns.label)",
                    "generate-reports": "$t(signing.actions.generate-reports.label)",
                    "transmit-results": "$t(signing.actions.transmit-results.label)",
                    "approve-voter": "$t(signing.actions.approve-voter.label)",
                    "approve-configuration": "$t(signing.actions.approve-configuration.label)",
                    "key-ceremony": "$t(signing.actions.key-ceremony.label)",
                    "tally-key": "$t(signing.actions.tally-key.label)",
                },
                channels: {
                    ONLINE: "En línea",
                    KIOSK: "Quiosco",
                    EARLY_VOTING: "Votación anticipada",
                    TELEPHONE: "Teléfono",
                },
                statuses: {
                    NOT_STARTED: "No iniciada",
                    OPEN: "Abierta",
                    PAUSED: "En pausa",
                    CLOSED: "Cerrada",
                },
                channelStatus: "{{channel}}: {{status}}",
                ruleChange: "{{action}}: {{rule}}",
                ruleChangeFrom: "{{action}}: {{rule}} (antes {{was}})",
                ruleNeeds: "necesita {{n}}",
                ruleOff: "desactivada",
                decision: {
                    approve: "Aprobar",
                },
            },
            results: {
                signatures: "Firmas",
                needs: "Requiere {{n}}",
                off: "Desactivada",
                openRequest: "Abrir la solicitud de firma",
                downloadSigned: "Descargar PDF firmado",
                print: "Imprimir",
                transmit: "Transmitir resultados",
                sendTo: "Enviar a {{count}} destinos",
                awaiting: "{{item}}: esperando firmas",
                transmission: {
                    title: "Firmas",
                    description:
                        "Cada firmante firma los resultados del paquete con su certificado digital, en este navegador. El paquete se puede enviar cuando lo hayan firmado {{n}} personas.",
                    waiting:
                        "El paquete se puede enviar cuando su solicitud de firma tenga todas sus firmas.",
                    signed: "El paquete lleva todas sus firmas y se puede enviar.",
                    ended: "La solicitud de firma de este paquete terminó. Vuelva a crear el paquete para firmarlo.",
                },
            },
            waiting: {
                title: "Pendiente de mi firma",
                buttonCount_one: "Pendiente de mi firma: {{count}} solicitud por firmar",
                buttonCount_other: "Pendiente de mi firma: {{count}} solicitudes por firmar",
                intro: "Solicitudes que esperan las firmas de las acciones que puede firmar, en sus $t(signing.terms.posts).",
                close: "Cerrar la lista",
                empty: "No hay nada pendiente de su firma.",
                loadError: "No se pudieron cargar las solicitudes que esperan firmas.",
                signedByYou: "Firmada por usted",
            },
            notes: {
                afterApproval: "Tras la aprobación",
                afterApprovalValue: "Se emiten las credenciales del votante y se le envían",
                custodyProof: "Su prueba de custodia",
                custodyProofChecked: "Comprobado: es su prueba de custodia para esta ceremonia",
                recordedIn: "Se registra en",
                recordedInCeremony: "La ceremonia de claves y el tablón de anuncios",
                recordedInTally: "La sesión de recuento",
            },
            custodyProof: {
                signing:
                    "Firme su prueba de custodia en el panel de firma. Se registra una vez que lo haya firmado.",
                record: "Registrar mi prueba de custodia",
                failed: "No se pudo registrar su prueba de custodia firmado: {{error}}",
                dropAgain:
                    "Vuelva a soltar su archivo de prueba de custodia para registrar su prueba de custodia firmado. (signed-custody-proof.json)",
                redo: "Su prueba de custodia se aportó sin su firma, que esta elección ahora requiere. Apórtelo de nuevo y fírmelo.",
                notTaken:
                    "La ceremonia ya no acepta este prueba de custodia. Vuelva a soltar su archivo de prueba de custodia. (signed-custody-proof.json)",
            },
        },
        lifecycle: {
            signedClose: {
                title: "Plazo de cierre firmado",
                deadline: "{{election}}: {{time}} · autorizado por la configuración {{code}}.",
                explanation:
                    "Este plazo firmado sigue siendo vinculante aunque se cambie o elimine el calendario editable. El planificador cierra los canales autorizados que aún estén abiertos.",
                reached:
                    "Este plazo firmado ya ha vencido. Compruebe el estado actual de la votación y el registro de auditoría; todavía no se ha registrado su procesamiento.",
                processed: "Plazo de cierre firmado procesado a las {{time}}.",
                signedAt: "Plazo firmado: {{time}}.",
                channels: "Canales que siguen cubiertos por este plazo: {{channels}}.",
                result: "Consulte el estado de la votación y el registro de auditoría para conocer los cambios reales y el resultado completo.",
                unavailable:
                    "No se pudieron cargar los plazos de cierre firmados. Compruebe el calendario publicado y el registro de auditoría.",
            },
            picker: {
                noMatch:
                    "Ninguna zona horaria coincide. Escribe una ciudad, un país, una zona, una abreviatura o un desfase.",
            },
            input: {
                timezone: "Zona horaria",
                scheduledAt: "Programado para",
                meetingStart: "Inicio de la reunión",
                cronZone:
                    "La programación se ejecuta en la zona horaria principal del evento, {{zone}}.",
                unconfiguredZone:
                    "{{zone}} no es una de las zonas horarias configuradas del evento. Elige una de ellas.",
            },
            schedule: {
                allElections: "Todas las elecciones",
                outcome: "Resultado",
                noOffset: "Sin desfase horario: nunca se ejecuta",
                unpublished: "Aún no publicado",
                notPublished:
                    "Aún no hay nada publicado: los votantes ven la programación tras la primera publicación.",
                unpublishedChanges_one:
                    "{{count}} evento programado ha cambiado desde la última publicación. Los votantes lo verán cuando publiques.",
                unpublishedChanges_other:
                    "{{count}} eventos programados han cambiado desde la última publicación. Los votantes los verán cuando publiques.",
                offsetless_one:
                    "{{count}} hora programada no tiene desfase horario, así que nunca se ejecuta. Edítala para fijar su zona horaria.",
                offsetless_other:
                    "{{count}} horas programadas no tienen desfase horario, así que nunca se ejecutan. Edítalas para fijar su zona horaria.",
                outcomeChange:
                    "Al guardar cambia lo que hace esta transición programada: {{before}} → {{after}}.",
                outcomeNew: "Una vez guardada, esta transición programada: {{after}}.",
                outcomeElections: "{{count}} de {{total}} elecciones",
                exportError: "No se ha podido exportar la programación.",
                exportFileName: "schedule.csv",
                totals: {
                    refused_one:
                        "{{count}} fila programada se rechazará ({{transitions}} transiciones de elecciones).",
                    refused_other:
                        "{{count}} filas programadas se rechazarán ({{transitions}} transiciones de elecciones).",
                    runsUnsigned_one:
                        "{{count}} cierre programado se ejecutará sin firmas ({{transitions}} transiciones de elecciones).",
                    runsUnsigned_other:
                        "{{count}} cierres programados se ejecutarán sin firmas ({{transitions}} transiciones de elecciones).",
                    review: "Revisar",
                    showAll: "Mostrar todo",
                    showing: {
                        refused:
                            "Se muestran las {{count}} filas programadas que se rechazarán ({{transitions}} transiciones de elecciones).",
                        runsUnsigned:
                            "Se muestran los {{count}} cierres programados que se ejecutarán sin firmas ({{transitions}} transiciones de elecciones).",
                    },
                },
                recompute: {
                    title_one:
                        "Una actualización de la base de datos de zonas horarias mueve {{count}} hora programada futura. No cambia nada hasta que la apliques.",
                    title_other:
                        "Una actualización de la base de datos de zonas horarias mueve {{count}} horas programadas futuras. No cambia nada hasta que las apliques.",
                    change: "{{type}}: {{before}} → {{after}}",
                    apply: "Aplicar",
                    applied_one: "{{count}} hora programada actualizada.",
                    applied_other: "{{count}} horas programadas actualizadas.",
                    error: "No se han podido actualizar las horas programadas.",
                },
                outcomeChangeElections_one: "Guardar cambia el resultado en {{count}} elección:",
                outcomeChangeElections_other:
                    "Guardar cambia el resultado en {{count}} elecciones:",
            },
            authorizes: {
                reportPolicyOf: "{{election}}: {{value}}",
                initializationRetained:
                    "Un informe obligatorio en esta configuración firmada sigue siendo obligatorio si la configuración actual del Puesto cambia a no obligatorio.",
                title: "Qué autoriza esta aprobación",
                schedule: "Aperturas y cierres programados",
                noSchedule:
                    "No hay aperturas ni cierres programados: los firmantes abren y cierran la votación.",
                opens: "Se abre {{time}}",
                closes: "Se cierra {{time}}",
                settings: "Ajustes",
                unsignedClose: "Cierre programado sin firmas: {{value}}",
                initialization: "Inicialización: {{value}}",
                firstConfiguration:
                    "Es la primera configuración aprobada: no hay nada con qué comparar.",
                sameAsPrevious:
                    "Los ajustes son los mismos que en la configuración aprobada anterior.",
                rule: {
                    openNeeds_one: "Abrir requiere {{count}} firma",
                    openNeeds_other: "Abrir requiere {{count}} firmas",
                    openNoSignatures: "Abrir no requiere firmas",
                    closeNeeds_one: "Cerrar requiere {{count}} firma",
                    closeNeeds_other: "Cerrar requiere {{count}} firmas",
                    closeNoSignatures: "Cerrar no requiere firmas",
                    openSetting: "Apertura de la votación",
                    closeSetting: "Cierre de la votación",
                    signatures_one: "{{count}} firma",
                    signatures_other: "{{count}} firmas",
                    none: "sin firmas",
                },
                diff: {
                    tightens: "Endurece: {{setting}} {{before}} → {{after}}",
                    loosens: "Relaja: {{setting}} {{before}} → {{after}}",
                    mixed: "Cambia: {{setting}} {{before}} → {{after}} (más estricto en un aspecto y menos en otro)",
                },
                comparedWith:
                    "Comparado con la configuración aprobada anterior, aprobación {{code}}:",
                channels: "Canales de votación por elección",
                channelsOf: "{{election}}: {{channels}}",
                noChannels: "ninguno",
            },
            publish: {
                openedAuthorized:
                    "La votación se abrió según la programación ({{time}}), autorizada por la aprobación de configuración {{code}} (firmada por {{names}}).",
                closedAuthorized:
                    "La votación se cerró según la programación ({{time}}), autorizada por la aprobación de configuración {{code}} (firmada por {{names}}).",
                closedUnsigned:
                    "La votación se cerró según la programación ({{time}}). Sin firmas de cierre: la programación cerró la votación en su hora límite.",
                authorizedBy: "Autorizado por",
                cancelledRequest: "La solicitud {{code}} tenía {{n}} de {{k}} firmas y se canceló.",
                openedRefused: "La apertura programada de {{time}} se rechazó.",
                closedRefused: "El cierre programado de {{time}} se rechazó.",
                openedNoSignaturesNeeded:
                    "La votación se abrió según la programación ({{time}}); no hacían falta firmas.",
                closedNoSignaturesNeeded:
                    "La votación se cerró según la programación ({{time}}); no hacían falta firmas.",
                openedNothingToChange:
                    "A las {{time}} la apertura programada no tenía nada que abrir: sus canales ya estaban abiertos.",
                closedNothingToChange:
                    "A las {{time}} el cierre programado no tenía nada que cerrar: sus canales ya estaban cerrados.",
            },
            import: {
                title: "Importar programación",
                subtitle:
                    "Una fila por evento y elección, en hora local. Deja la zona horaria vacía para usar la zona horaria de la elección.",
                chooseFile: "Elige un archivo CSV",
                template: "Descargar plantilla",
                templateFileName: "schedule-template.csv",
                ready: "{{ok}} eventos listos para {{posts}} elecciones.",
                needsAttention_one:
                    "{{ok}} eventos listos para {{posts}} elecciones. {{count}} fila requiere atención; corrige el archivo y vuelve a subirlo.",
                needsAttention_other:
                    "{{ok}} eventos listos para {{posts}} elecciones. {{count}} filas requieren atención; corrige el archivo y vuelve a subirlo.",
                preview: "Filas que se importarán",
                row: "Fila",
                asWritten: "{{local}} · {{place}}",
                moreRows: "…y {{count}} filas más",
                imported: "Programación importada: {{created}} creados, {{updated}} actualizados.",
                uploadError: "No se ha podido comprobar el archivo. Vuelve a subirlo.",
                importError: "No se ha podido importar la programación.",
                error: {
                    unknownElection: "Ninguna elección tiene el alias {{election}}.",
                    unknownEventType: "{{type}} no es un tipo de evento programado.",
                    invalidTimeZone: "{{zone}} no es una zona horaria.",
                    invalidDateTime: "La fecha y la hora deben tener el formato YYYY-MM-DDTHH:MM.",
                    invalidVotingChannels:
                        "Los canales de votación son desconocidos o abren a la vez la votación en línea y la anticipada.",
                    dstGap: "{{dateTime}} no existe en {{city}} porque se adelantan los relojes. Escribe una hora que exista.",
                    duplicate: "Otra fila programa el mismo evento para esta elección.",
                    other: "Esta fila no se puede importar ({{code}}).",
                    ambiguousElection: "Más de una elección tiene el alias {{election}}.",
                },
            },
            settings: {
                accordion: "Idioma, fecha y hora",
                dateAndTime: "Fecha y hora",
                configured: "Zonas horarias configuradas",
                configuredHelp:
                    "{{count}} zonas horarias. Las elecciones eligen la suya de esta lista; escribe una ciudad o un país para añadir una.",
                moreZones: "+{{count}}",
                primary: "Zona horaria principal",
                primaryHelp:
                    "Se usa para las programaciones de todo el evento, los informes y las elecciones sin zona horaria propia.",
                primaryInUse:
                    "{{zone}} es la zona horaria principal. Elige antes otra zona horaria principal.",
                inUse: "{{zone}} la usan {{names}}. Cambia antes esas elecciones.",
                logs: "Horas en los registros y sus exportaciones",
                logsPrimary: "Zona horaria principal ({{abbr}})",
                logsElection: "La zona horaria de la elección de cada fila",
                logsHelp: "Las filas sin elección usan la zona horaria principal.",
                electionZone: "Zona horaria",
                electionPrimary: "Principal del evento: {{zone}}",
                electionZoneHelp:
                    "Las programaciones, las pantallas de votantes y los informes de esta elección usan esta zona horaria, también en todas sus áreas. Vacía usa la zona horaria principal del evento.",
                electionUnconfigured:
                    "El evento ya no configura esta zona horaria, así que la elección usa la zona horaria principal, {{zone}}. Elige una de las zonas horarias configuradas.",
                electionUnconfiguredSave:
                    "Elige una de las zonas horarias configuradas del evento.",
            },
            policies: {
                accordion: "Ciclo de la votación",
                intro: "Estos ajustes forman parte de la configuración del evento electoral: la aprobación de la configuración los firma, y las aperturas y cierres programados siguen los más estrictos entre los ajustes actuales y los publicados.",
                nothingPublished:
                    "Aún no hay nada publicado: hasta la primera publicación, las aperturas y cierres programados usan los valores por defecto (por elección, rechazar).",
                publishedValue: "Configuración publicada: {{value}}",
                changedSincePublished:
                    "Ha cambiado desde la configuración publicada: las aperturas y cierres programados siguen la más estricta de las dos hasta la próxima publicación aprobada.",
                scope: {
                    title: "Inicialización antes de abrir la votación",
                    post: {
                        label: "Por elección",
                        help: "Una elección se abre cuando está inicializada.",
                    },
                    event: {
                        label: "Todo el evento",
                        help: "Ninguna elección se abre hasta que todas estén inicializadas.",
                        warning:
                            "Una elección sin inicializar mantiene cerradas todas las elecciones, también en sus aperturas programadas.",
                    },
                    postAndCountry: {
                        label: "Por elección y país",
                        help: "Una elección se abre cuando todos sus países (áreas) están inicializados.",
                        warning:
                            "Una elección sigue cerrada, también en su apertura programada, hasta que todos sus países están inicializados; cada país se inicializa con su propio informe.",
                    },
                },
                close: {
                    title: "Cierre programado sin firmas",
                    help: "Cuando cerrar la votación requiere firmas y un cierre programado no está en la configuración firmada.",
                    refuse: {
                        label: "Rechazar",
                        help: "El cierre no se ejecuta; los firmantes de la elección cierran la votación con sus firmas.",
                    },
                    runAsSystem: {
                        label: "Ejecutar como sistema",
                        help: "La votación se cierra en la hora límite y queda registrada como cerrada por la programación sin firmas.",
                        warning:
                            "Los cierres programados fuera de la configuración firmada cierran la votación sin la firma de nadie. El registro y los documentos lo indican.",
                    },
                },
                onSave: {
                    outcomes_zero: "Ninguna transición programada cambia de resultado.",
                    outcomes_one:
                        "{{count}} transición programada cambia de resultado. Revísala en Eventos Programados.",
                    outcomes_other:
                        "{{count}} transiciones programadas cambian de resultado. Revísalas en Eventos Programados.",
                },
                saveError: "No se han podido guardar los ajustes del ciclo de la votación.",
                publishedPerTarget: "Configuración publicada, por destino: {{values}}",
                publishedCount_one: "{{value}} ({{count}} destino)",
                publishedCount_other: "{{value}} ({{count}} destinos)",
                savedWithoutPolicies:
                    "El evento electoral se guardó, pero los ajustes del ciclo de la votación no: {{reason}}. Vuelve a guardarlos.",
            },
        },
        scheduledOutcome: {
            chip: {
                waitingForInitialization: "Esperando la inicialización",
                runs: "Se ejecutará",
                runsUnsigned: "Se ejecutará sin firmas",
                refused: "Se rechazará",
            },
            note: {
                waitingForInitialization: "Esperando la inicialización",
                authorized: "Autorizado por la configuración {{code}}",
                noSignaturesNeeded: "No necesita firmas",
                closesUnsigned: "Se cierra sin firmas",
                refused: {
                    initialization: "La inicialización requerida está incompleta",
                    votingClose: "La votación no puede abrirse después del plazo de cierre",
                    needsSignatures: "Necesita las firmas de los firmantes",
                    covered: "No está en la configuración firmada",
                    unsignedClose: "Un cierre sin firmas se rechaza",
                    stricterCopy:
                        "Ha cambiado desde la configuración publicada, que sigue decidiendo",
                    defaults: "Aún no hay nada publicado: se aplican los valores por defecto",
                },
                refusedWithStep: "{{reason}}. {{next}}",
            },
            why: {
                button: "¿Por qué?",
                title: {
                    waitingForInitialization: "Por qué espera la inicialización",
                    runs: "Por qué se ejecutará",
                    runsUnsigned: "Por qué se ejecutará sin firmas",
                    refused: "Por qué se rechazará",
                },
                checks: "Comprobaciones",
                check: "Comprobación",
                current: "Ajustes actuales",
                published: "Configuración publicada",
                verdict: "Resultado",
                allows: "Permite",
                blocks: "Bloquea",
                deciding: "Comprobación decisiva",
                nextStep: "Siguiente paso:",
                signedBy: "Firmado por {{names}}",
            },
            question: {
                initialization: "¿Se ha completado la inicialización requerida?",
                votingClose: "¿Esta apertura respeta el plazo de cierre de la votación?",
                needsSignatures: "¿Esta acción necesita firmas?",
                covered: "¿Está esta programación exacta en la configuración firmada?",
                unsignedClose: "¿Qué pasa con un cierre sin firmas?",
                stricterCopy: "¿Difieren los ajustes actuales y los publicados? ¿Cuáles deciden?",
                defaults: "¿Hay algo publicado ya?",
            },
            check: {
                initialization: {
                    waiting:
                        "Deben completarse las inicializaciones exigidas por la configuración actual y la publicada.",
                },
                votingClose: {
                    passed: "La votación cierra a las {{closes_at}}; esta apertura no puede ejecutarse en ese momento ni después.",
                },
                needsSignatures: {
                    yes: "Sí, {{signatures}} firmas",
                    yes_one: "Sí, {{count}} firma",
                    yes_other: "Sí, {{count}} firmas",
                    no: "No",
                },
                covered: {
                    overriddenBySignedPostRow:
                        "La configuración firmada {{code}} utiliza la apertura propia de este puesto, {{scheduled_event_id}}. La apertura para todo el evento no se aplica.",
                    yes: "Sí: aprobación {{code}}, sin cambios",
                    changed: "No: ha cambiado desde la aprobación {{code}}",
                    changedBy:
                        "No: editado el {{edited_at}} por {{edited_by}}, después de la aprobación {{code}}",
                    notInApproval: "No: la aprobación {{code}} no lo incluye",
                    noApproval: "Aún no hay ninguna configuración aprobada",
                    channelsChanged:
                        "No: los canales de votación de la elección cambiaron desde la aprobación {{code}}",
                    alreadyFired:
                        "No: esta transición de la aprobación {{code}} ya se ejecutó el {{fired_at}}; volver a ejecutarla necesita firmas",
                    late: "No: han pasado más de 15 minutos desde {{scheduled_date}} (aprobación {{code}}); ejecutarla ahora necesita firmas",
                },
                unsignedClose: {
                    refuse: "Rechazar",
                    runAsSystem: "Ejecutar como sistema",
                },
                stricterCopy: {
                    same: "Ambos son iguales",
                    currentStricter: "Los ajustes actuales son más estrictos: se aplican ya",
                    currentLooser:
                        "Los ajustes actuales son menos estrictos: se aplican tras la próxima publicación aprobada",
                    combined: "Cada uno es más estricto en un valor: se aplican ambos",
                },
                defaults: {
                    published: "Publicado el {{published_at}}",
                    nothingPublished: "Nada publicado: se aplican los valores por defecto",
                    noSnapshot:
                        "Publicado el {{published_at}}, antes de que las publicaciones guardaran estos ajustes: se aplican los valores por defecto",
                },
            },
            nextStep: {
                initialize:
                    "Complete la inicialización requerida. El planificador volverá a intentarlo antes del cierre de la votación.",
                closed: "Esta apertura no se ejecutará después del cierre de la votación.",
                none: "No hace falta hacer nada.",
                publishAndApprove: "Publica y aprueba la configuración.",
                requireConfigurationApproval:
                    "Haz que Aprobar configuración requiera firmas y, después, publica y aprueba la configuración.",
                askSignersToOpen: "Pide a los firmantes de la elección que abran la votación.",
                askSignersToClose: "Pide a los firmantes de la elección que cierren la votación.",
            },
            applies: {
                tightens: "Se aplica ya a las acciones manuales y programadas.",
                loosens:
                    "Se aplica ya a las acciones manuales; a las aperturas y cierres programados, tras la próxima publicación aprobada.",
                tightensAndLoosens:
                    "Su parte más estricta se aplica ya a las acciones manuales y programadas; su parte menos estricta se aplica ya a las acciones manuales y, a las aperturas y cierres programados, tras la próxima publicación aprobada.",
            },
        },
        messagingEvent: {
            tab: "Mensajería",
            intro: "Los canales que los votantes de este evento pueden elegir para códigos y avisos, y la cuenta desde la que envía cada uno. Las cuentas se gestionan en Ajustes > Mensajería.",
            readOnly:
                "Puede ver estos ajustes. Para cambiarlos necesita el permiso messaging-config-write.",
            savingNote:
                "Al guardar también se actualizan los canales que las páginas de inscripción ofrecen en cada Post.",
            save: "Guardar",
            saved: "Ajustes de mensajería guardados.",
            saveRejected:
                "Los ajustes de mensajería no se guardaron. Corrija los problemas indicados.",
            saveError: "No se pudieron guardar los ajustes de mensajería.",
            accountLabel: "Cuenta de {{channel}}",
            notUsed: "Sin usar",
            missingAccount: "Cuenta no encontrada",
            noAccount: "Añada antes una cuenta en Ajustes > Mensajería",
            missing: "Falta: {{blockers}}",
            purposeSwitch: "{{channel}}: {{purpose}}",
            sections: {
                channels: "Canales",
                templates: "Plantillas aprobadas",
                posts: "Canales por Post",
                postsCount: "Canales por Post ({{count}} Posts)",
                delivery: "Estado de entrega",
            },
            column: {
                channel: "Canal",
                account: "Envía desde",
                purpose: "Finalidad",
                language: "Idioma",
                template: "Plantilla del proveedor",
                status: "Estado",
                post: "Post",
                key: "Tipo de mensaje o plantilla",
                providerLanguage: "Idioma del proveedor",
                voters: "Votantes que lo usan",
                sent24h: "Enviados, últimas 24 h",
                sent24hHelp:
                    "Cada mensaje cuyo último intento fue en las últimas 24 horas, en cualquier estado, incluidos los fallidos. Las columnas de estado cuentan todos los mensajes del evento.",
            },
            outOfWindow: {
                utility:
                    "Fuera de la ventana de 24 horas, {{channel}} envía los avisos con la plantilla de utilidad aprobada vinculada a los avisos en Plantillas aprobadas.",
            },
            messageTypes: {
                otp: "Código de verificación",
                otl: "Enlace de acceso",
                enrollmentAccepted: "Inscripción aceptada",
                enrollmentAcceptedKiosk: "Inscripción aceptada (quiosco)",
                enrollmentPending: "Inscripción pendiente de aprobación",
                enrollmentRejected: "Inscripción rechazada",
                passwordReset: "Restablecimiento de contraseña",
                temporaryPassword: "Contraseña temporal",
                contactChanged: "Método de contacto cambiado",
            },
            templates: {
                empty: "Elija una cuenta que envíe plantillas aprobadas, como WhatsApp, Viber o Messenger, para vincular aquí sus plantillas.",
                help: "Seleccione un tipo de mensaje del sistema o un alias de Plantillas. Déjelo vacío para usar la plantilla aprobada por defecto para este propósito. Idioma es el idioma del votante. Plantilla del proveedor es el nombre o ID de la plantilla en el proveedor. Idioma del proveedor es el código del proveedor para esa plantilla cuando difiere del idioma del votante: WhatsApp necesita el código exacto de la plantilla aprobada, como en_US.",
                order: "Para cada mensaje gana la fila más específica: la fila del mensaje en el idioma del votante, después la fila del mensaje en cualquier idioma, después la predeterminada del propósito en el idioma del votante y, por último, cualquier predeterminada del propósito.",
                noneRequired:
                    "{{channel}} solo envía plantillas aprobadas. Añada al menos una plantilla predeterminada para cada propósito en uso.",
                noneOptional:
                    "No hay plantillas vinculadas para {{channel}}. Solo hacen falta para enviar avisos fuera de la ventana de conversación.",
                row: "Plantilla {{position}} de {{channel}}",
                keyDefault: "Predeterminada para este propósito",
                add: "Añadir plantilla de {{channel}}",
                remove: "Quitar la plantilla {{position}} de {{channel}}",
                incomplete: "Indique el idioma y la plantilla del proveedor, o quite esta fila.",
                approval: {
                    APPROVED: "Aprobada",
                    NOT_APPROVED: "No aprobada",
                    NOT_CHECKED: "Aprobación no comprobada",
                },
            },
            posts: {
                noChannels:
                    "Active códigos o avisos en un canal para elegir los canales de cada Post.",
                help: "La inscripción muestra a los votantes de cada Post los canales marcados aquí.",
                restricted: "{{count}} Posts ofrecen menos de los {{total}} canales.",
                allChannels:
                    "Todos los Posts ofrecen los {{total}} canales; desmarque un canal en el Post donde no funcione.",
                search: "Buscar Posts",
                cell: "{{post}}: {{channel}}",
                showing: "Se muestran {{shown}} de {{total}} Posts. Busque para encontrar otros.",
            },
            delivery: {
                notConfigured: "Este evento no lo ofrece",
                empty: "No se usa ningún canal.",
                help: "Los recuentos son por mensaje, según el estado de su último intento. Aceptado significa que el proveedor aceptó el mensaje, no que el votante lo recibiera. Desconocido significa que la entrega aún no está confirmada; pasadas 24 horas se muestra como Sin resolver, nunca como fallido. Una cuenta sin informes de entrega muestra la entrega como no disponible.",
                loadError: "No se pudieron cargar los recuentos de mensajes.",
                noReceipts:
                    "Esta cuenta no recibe informes de entrega, así que no se puede mostrar la entrega.",
                showFailed: "Ver los mensajes fallidos de {{channel}} en Logs",
                noChannelHelp:
                    "Votantes sin ningún canal de contacto registrado. Sus avisos fallan y nunca se cuentan en ningún canal.",
                showFailedNoChannel:
                    "Ver en Logs los mensajes fallidos de votantes sin canal de contacto",
            },
            error: {
                UNSUPPORTED_VERSION:
                    "Esta configuración usa la versión {{version}}, que no se admite.",
                DUPLICATE_CHANNEL: "{{channel}} está configurado más de una vez.",
                UNKNOWN_ACCOUNT: "La cuenta de {{channel}} ya no existe. Elija otra cuenta.",
                ACCOUNT_OF_ANOTHER_TENANT: "La cuenta seleccionada pertenece a otro inquilino.",
                ACCOUNT_CHANNEL_MISMATCH:
                    "La cuenta seleccionada no envía mensajes de {{channel}}.",
                PURPOSE_NOT_READY:
                    "{{channel}} aún no puede enviar {{purpose}}. Falta: {{blockers}}.",
                TEMPLATE_NOT_APPROVED:
                    "La plantilla de {{channel}} para {{purpose}} en {{language}} no está aprobada por el proveedor.",
                OUT_OF_WINDOW_NOT_SUPPORTED:
                    "{{channel}} no puede enviar fuera de una ventana de conversación con esta cuenta: no tiene ventana de conversación o sus avisos ya necesitan una plantilla.",
                ELECTION_CHANNEL_NOT_ENABLED:
                    "{{election}} ofrece {{channel}}, que este evento no usa.",
                UNKNOWN_ELECTION: "{{election}} no es una elección de este evento.",
                UTILITY_TEMPLATE_MISSING:
                    "Los avisos de {{channel}} fuera de la ventana de 24 horas necesitan la plantilla de utilidad aprobada. Vincúlela a los avisos en Plantillas aprobadas.",
            },
            noReadPermission:
                "Para ver estos ajustes se necesita el permiso messaging-config-read.",
            chooseAccount: "Elija antes una cuenta.",
            needsSavingAgain: "Hay que volver a guardar la configuración.",
            needsSavingAgainHelp:
                "Se copió del almacenamiento anterior y no superó las comprobaciones que hace un guardado. Corrija los problemas que se muestran y vuelva a guardarla.",
        },
        messaging: {
            channel: {
                EMAIL: "Correo electrónico",
                SMS: "SMS",
                WHATSAPP: "WhatsApp",
                VIBER: "Viber",
                MESSENGER: "Facebook Messenger",
            },
            provider: {
                AWS_SES: "Amazon SES",
                SMTP: "Servidor SMTP",
                AWS_SNS: "Amazon SNS",
                WHATSAPP_CLOUD_API: "WhatsApp Cloud API (Meta)",
                MESSENGER_SEND_API: "Messenger Platform (Meta)",
                VIBER_INFOBIP: "Viber Business Messages (Infobip)",
                CONSOLE: "Consola (ya no es compatible)",
                HTTP_API: "API HTTP personalizada (ya no es compatible)",
            },
            purpose: {
                OTP: "Códigos",
                NOTICE: "Avisos",
            },
            state: {
                QUEUED: "En cola",
                ACCEPTED: "Aceptado",
                DELIVERED: "Entregado",
                FAILED: "Fallido",
                UNKNOWN: "Desconocido",
                UNRESOLVED: "Sin resolver",
            },
            stateHelp: {
                QUEUED: "Pendiente de entregar al proveedor.",
                ACCEPTED:
                    "El proveedor aceptó el mensaje. Esto no significa que el votante lo recibiera.",
                DELIVERED: "El proveedor informó de que el mensaje se entregó.",
                FAILED: "El proveedor confirmó que el mensaje no se entregó.",
                UNKNOWN: "La entrega aún no está confirmada.",
                UNRESOLVED:
                    "No se pudo confirmar la entrega en 24 horas. Esto no significa que haya fallado.",
            },
            blocker: {
                UNCHECKED: "Sin comprobar para su número, página o remitente actuales",
                NOT_CONNECTED: "Sin conexión",
                UNSUPPORTED_PURPOSE: "No lo admite este proveedor",
                NEEDS_PROVIDER_APPROVAL: "Necesita la aprobación del proveedor",
                NEEDS_PRODUCTION_ACCESS: "Necesita acceso de producción",
                NEEDS_APPROVED_TEMPLATE: "Necesita una plantilla aprobada",
                UNSUPPORTED_PROVIDER: "Proveedor no compatible: sustituya esta cuenta",
                CALLBACK_NOT_CONFIGURED: "Dirección de retorno no configurada",
                REPORT_CREDENTIALS_MISSING: "Credenciales de informes no generadas",
            },
            readiness: {
                connected: "Conectada",
                notConnected: "Sin conexión",
                readyOtp: "Lista para códigos",
                readyNotice: "Lista para avisos",
                notReady: "No está lista",
                lastCheck: "Comprobada {{date}}",
                neverChecked: "Aún no comprobada",
            },
            approval: {
                PENDING: "Pendiente de aprobación del proveedor",
                CONFIRMED: "Aprobación del proveedor confirmada",
            },
            credential: {
                ACCESS_TOKEN: "Token de acceso",
                APP_SECRET: "Secreto de la aplicación",
                VERIFY_TOKEN: "Token de verificación",
                API_KEY: "Clave de API",
                SMTP_PASSWORD: "Contraseña",
                AWS_ACCESS_KEY_ID: "ID de clave de acceso de AWS",
                AWS_SECRET_ACCESS_KEY: "Clave de acceso secreta de AWS",
                REPORT_USERNAME: "Usuario de informes",
                REPORT_PASSWORD: "Contraseña de informes",
            },
            deliveryUnavailable: "Entrega no disponible",
            templates: {
                noMethod: "Elige al menos un método para la plantilla.",
                parameters: "Parámetros de la plantilla",
                parametersHelp:
                    "Lo que rellena cada marcador de la plantilla aprobada, en orden, como user.first_name o vote_url. Para una plantilla con parámetros con nombre escriba @nombre=valor, como @first_name=user.first_name; cualquier otra entrada es posicional.",
                parameter: "Parámetro {{position}}",
                removeParameter: "Quitar el parámetro {{position}}",
                addParameter: "Añadir parámetro",
                noAccount:
                    "Aún no hay ninguna cuenta de {{channel}}. Añade una en Ajustes > Mensajería para ver qué idiomas están aprobados.",
                account: "Cuenta",
                approvalTitle: "Plantillas aprobadas",
                language: "Idioma",
                approvalFor: "Aprobada para {{purpose}}",
                approved: "Aprobada",
                notApproved: "No aprobada",
                approvalHelp:
                    "Las aprobaciones vienen del proveedor y se actualizan con la comprobación de conexión de la cuenta.",
                messengerIntro:
                    "En las 24 horas siguientes al último mensaje del votante, Messenger envía el texto siguiente.",
                messengerMessage: "Mensaje dentro de las 24 horas",
                messengerWindow:
                    "Un destinatario de Messenger guardado no es un permiso para enviar. Fuera de la ventana de 24 horas este aviso se envía con la plantilla de utilidad aprobada indicada abajo o vinculada en el evento electoral; sin ella no se envía, y el fallo se muestra con su motivo. Los mensajes de utilidad necesitan el permiso page_utility_messaging y una plantilla UTILITY aprobada en la página.",
                intro: {
                    WHATSAPP:
                        "WhatsApp solo envía plantillas que Meta aprobó para la cuenta de WhatsApp Business. El mensaje debe coincidir con la plantilla aprobada; elige qué rellena sus parámetros.",
                    VIBER: "Viber solo envía códigos y mensajes transaccionales con plantillas aprobadas por el socio de Viber. El mensaje debe coincidir con la plantilla aprobada; elige qué rellena sus parámetros.",
                },
                approvedWording: "Texto aprobado",
                approvedWordingHelp:
                    "Una copia de la plantilla aprobada, usada como vista previa. Cambiarla aquí no cambia lo que envía el proveedor.",
                providerTemplateTitle: "Plantilla del proveedor",
                providerTemplateHelp:
                    "Opcional. El nombre o ID de la plantilla aprobada en el proveedor. Si está vacío, se usa la plantilla del evento electoral vinculada al alias de esta plantilla, o la predeterminada del evento para el propósito.",
                providerTemplate: "Nombre o ID de la plantilla del proveedor",
                providerLanguage: "Código de idioma del proveedor",
                providerLanguageHelp: {
                    WHATSAPP:
                        "Usa el código de idioma exacto de la plantilla de WhatsApp aprobada, como en o en_US. Solo se necesita un código regional si la plantilla se aprobó con él.",
                    VIBER: "El código de idioma con el que el proveedor de Viber conoce la plantilla, cuando lo necesita.",
                    MESSENGER:
                        "El código de idioma de la plantilla de utilidad aprobada, como en_US.",
                },
                noReadPermission:
                    "Para ver las cuentas se necesita el permiso messaging-config-read.",
            },
            send: {
                eachVoter: "El canal de cada votante",
                eachVoterHelp:
                    "Cada votante recibe un mensaje en su propio canal. Si la entrega falla, se reintenta en ese canal y el fallo se muestra en la pestaña Mensajería del evento. La entrega sin confirmar se muestra como Desconocido.",
                channelColumn: "Canal",
                sendsFrom: "Se envía desde",
                noAccount: "Sin cuenta",
                missingContent: "Esta notificación no tiene contenido para {{channels}}.",
                approvedTemplateHelp:
                    "Las plantillas de {{channel}} están aprobadas por el proveedor, por lo que su texto, plantilla del proveedor e idioma no se pueden cambiar aquí. Elige otra plantilla guardada para usar otro contenido aprobado.",
                voters: "Votantes",
                environment: "El remitente predeterminado de la instalación",
                noNoticeChannels:
                    "La configuración de mensajería de este evento no activa los avisos en ningún canal, así que ningún votante recibiría esta notificación. Active los avisos de un canal en la pestaña Mensajería del evento.",
                noReadPermission:
                    "Para mostrar el canal de cada votante se necesita el permiso messaging-config-read. Cada votante recibe igualmente la notificación por su propio canal.",
            },
            audience: {
                notOfferedRow:
                    "Ningún Post de estos votantes ofrece este canal para avisos, así que ninguno de ellos lo recibe: {{reason}}.",
                willSend: "Recibirán la notificación: {{willSend}}",
                selectedTimedOut:
                    "Contar los votantes seleccionados tardó demasiado, así que no se muestran recuentos. Cada votante recibe igualmente la notificación por su propio canal.",
                notSent: "No se envía",
                deletedAccount: "Cuenta eliminada",
                notOffered: "{{notOffered}} de ellos no lo reciben: {{reason}}.",
                noAccountHelp:
                    "Ninguna cuenta envía por este canal, así que los avisos de estos votantes fallan: {{reason}}.",
                noChannelHelp:
                    "Estos votantes no tienen canal de contacto registrado. No reciben nada, y cada uno aparece como fallido: {{reason}}.",
                total: "Votantes de esta audiencia: {{total}}",
                loadError:
                    "No se pudo cargar el número de votantes. Cada votante recibe igualmente la notificación por su propio canal.",
            },
            voterCount: {
                countedAt: "Contado: {{time}}.",
                never: "Aún no se ha contado.",
                pending: "Hay un recuento en curso.",
                timedOut: "El último recuento tardó demasiado y se detuvo.",
                failed: "El último recuento falló.",
                previous: "Los recuentos que se muestran son del anterior.",
                recount: "Volver a contar",
                recountAsked: "Se pidió un recuento. Los recuentos se actualizan cuando termine.",
                recountAlreadyAsked: "Ya hay un recuento en curso.",
                recountError: "No se pudo pedir el recuento.",
                loadError: "No se pudieron cargar los recuentos de votantes.",
            },
            volumes: {
                title: "Volúmenes mensuales",
                intro: "Intentos entregados a un proveedor cada mes, por canal, cuenta de envío, país de destino y categoría de facturación, para conciliar las facturas de los proveedores. Cada intento cuenta en el mes en que se hizo, así que un reintento vuelve a contar. La categoría de facturación es la prevista al enviar, o la que informó el proveedor; si un intento es facturable es lo que informó el proveedor. No se muestran precios.",
                month: "Mes",
                monthHelp: "Los meses se cuentan en UTC.",
                loading: "Cargando los volúmenes mensuales",
                loadError: "No se pudieron cargar los volúmenes.",
                empty: "No se entregó ningún intento a un proveedor en {{month}}.",
                noPermission:
                    "Para ver los volúmenes se necesita el permiso messaging-account-read o messaging-config-read.",
                notReported: "Sin informar",
                deploymentOrDeleted: "Remitente de la instalación o una cuenta eliminada",
                category: {
                    AUTHENTICATION: "Autenticación",
                    UTILITY: "Utilidad",
                    SERVICE: "Servicio",
                    TRANSACTIONAL: "Transaccional",
                    EMAIL: "Correo electrónico",
                },
                billableYes: "Facturable",
                billableNo: "No facturable",
                total: "Todos los intentos",
                column: {
                    account: "Enviado desde",
                    country: "País de destino",
                    category: "Categoría de facturación",
                    billable: "Facturable",
                    messages: "Intentos",
                },
            },
            voter: {
                listChannel: "Canal de mensajería",
                title: "Mensajería",
                notSet: "Sin definir",
                channel: "Canal",
                destination: "Se envía a",
                messengerPage: "Página de Messenger",
                consent: "Consentimiento",
                lastFailure: "Último fallo",
                noFailure: "Ninguno",
                failureNeedsPermission: "Se muestra con el permiso messaging-config-read.",
                noChannel: "Sin contacto registrado",
                notConfirmed: "Entrega no confirmada",
                notConfirmedHelp:
                    "El último mensaje cuya entrega aún no está confirmada. Pasadas 24 horas se muestra como Sin resolver, lo que no significa que haya fallado.",
            },
            logs: {
                channel: "Canal",
                status: "Estado",
                statusAsLogged:
                    "El estado cuando se escribió la entrada; el estado actual necesita el permiso messaging-config-read.",
            },
            stats: {
                sent: {
                    WHATSAPP: "Mensajes de WhatsApp enviados",
                    VIBER: "Mensajes de Viber enviados",
                    MESSENGER: "Mensajes de Messenger enviados",
                },
            },
            reason: {
                CANDIDATE_REFUSED:
                    "Las nuevas credenciales no conectaron: se mantienen las anteriores",
                ACCOUNT_IN_USE:
                    "La usan los ajustes de mensajería de un evento electoral: quítela allí primero",
                NUMBER_CHANGED:
                    "Un votante cambió su número de WhatsApp: pídale que verifique de nuevo su contacto",
                IDENTITY_CHANGED:
                    "La identidad de envío de la cuenta cambió: se comprueba de nuevo",
                TEMPLATES_NOT_READ: "No se pudieron leer las plantillas del proveedor",
                PARTNER_LOGIN_REFUSED: "Infobip rechazó la clave de API de la cuenta",
                NO_PAGES_MESSAGING: "El token de la página no tiene el permiso pages_messaging",
                NOT_SUBSCRIBED: "La aplicación no está suscrita a la página",
                NUMBER_STATUS_NOT_READ: "No se pudo leer el estado del número",
                CALLBACK_NOT_CONFIGURED: "Dirección de retorno no configurada",
                UNSUPPORTED_PROVIDER: "Proveedor no compatible: sustituya esta cuenta",
                CREDENTIALS_FOLLOW_ENDPOINT:
                    "Vuelva a introducir las credenciales para la nueva dirección",
                NO_CONTACT_CHANNEL: "Sin canal de contacto",
                NOT_RETRYABLE_NO_TEMPLATE: "No se puede reintentar: no hay plantilla guardada",
                CHANNEL_NOT_OFFERED_FOR_NOTICES: "Canal no ofrecido para avisos",
                CHANNEL_NOT_OFFERED_FOR_CODES: "Canal no ofrecido para códigos",
                NO_CONTENT_FOR_CHANNEL: "Sin contenido para el canal",
                NO_ACCOUNT_FOR_CHANNEL: "Sin cuenta de envío para el canal",
                INVALID_DESTINATION: "Destino no válido para el canal",
                NOT_RETRYABLE_PROTECTED_FIELDS:
                    "No se puede reintentar: contiene campos protegidos",
                REPORTED_TOO_LATE: "Notificado demasiado tarde para reintentar",
                CHANNEL_CHANGED: "El canal del votante cambió: no se reintenta",
                WORKER_STOPPED_BEFORE_SENDING: "El proceso se detuvo antes de enviar",
                WORKER_STOPPED_AFTER_SENDING:
                    "El proceso se detuvo antes de registrar el resultado",
                CODE_EXPIRED_BEFORE_SENDING: "El código caducó antes de poder enviarse",
                CAPACITY_BUSY:
                    "No enviado: la cuenta estuvo en su límite de envío durante demasiado tiempo",
                PROVIDER_ANSWER_LATE:
                    "El proveedor no respondió a tiempo: es posible que el mensaje se haya enviado",
                UNRESOLVED: "No se pudo confirmar la entrega",
                SELECTED_WITHOUT_VOTERS: "Seleccione al menos un votante",
                WRONG_RECIPIENT_KIND: "El destino no es del tipo al que envía este canal",
                UNSUPPORTED_PURPOSE: "El proveedor no envía este tipo de mensaje",
                MISSING_APPROVED_TEMPLATE: "No hay plantilla aprobada para este mensaje",
                OUTSIDE_CONVERSATION_WINDOW: "Fuera de la ventana de conversación",
                DESTINATION_NOT_ALLOWED: "Esta cuenta no envía al país del destino",
                CODE_EXPIRED: "El código caducó",
                MISSING_CODE: "No hay código que enviar",
                ENDPOINT_NOT_HTTPS: "La dirección del proveedor debe empezar por https://",
                PROVIDER_CODE: "Código de error del proveedor {{code}}",
                CHECK_DROPPED: "La cuenta cambió mientras se comprobaba: compruébela de nuevo",
                ACCOUNT_NOT_READY: "No enviado: la cuenta no está lista ({{blocker}})",
                PROVIDER_UNREACHABLE: "No se pudo contactar con el proveedor",
                PROVIDER_STATUS: "El proveedor respondió con el estado {{status}}",
                MISSING_CREDENTIAL: "Falta la credencial: {{credential}}",
                PROVIDER_CHECK_FAILED: "La comprobación del proveedor falló: {{detail}}",
                NUMBER_STATUS: "El estado del número es {{status}}",
                NUMBER_NAME_STATUS: "El nombre visible del número es {{status}}",
                NUMBER_IN_SANDBOX: "El número está en modo sandbox",
                NUMBER_NOT_VERIFIED: "El código del número no está verificado",
                SMS_SANDBOX: "La cuenta está en el sandbox de SMS",
                SES_SANDBOX: "La cuenta está en el sandbox de SES",
                SES_SENDING_DISABLED: "El envío está desactivado para esta cuenta",
                SMTP_REFUSED: "El servidor SMTP no es accesible o rechazó las credenciales",
                TOKEN_INVALID: "El token de acceso no es válido o ha caducado",
                TOKEN_CANNOT_SEND: "El token de acceso no tiene el permiso para enviar",
                TOKEN_NOT_CHECKED: "No se pudo comprobar el token de acceso",
                APP_SECRET_REFUSED: "Meta rechazó el secreto de la aplicación",
                APP_SECRET_NOT_CHECKED: "No se pudo comprobar el secreto de la aplicación",
                RETRY_NOT_PREPARED:
                    "No se puede reintentar: no se pudo volver a preparar el mensaje",
                NOT_THE_VOTERS_CHANNEL: "No es el canal del votante",
                REPORT_CREDENTIALS_MISSING:
                    "Genere las credenciales de informes y cree la suscripción de Infobip",
                CALLBACK_NOT_HTTPS: "La dirección de retorno debe ser https",
                UNKNOWN_ACCOUNT:
                    "Esta cuenta ya no existe: puede que se haya eliminado en otra pestaña. Vuelva a cargar la lista.",
                INTERNAL_ERROR:
                    "El servidor no pudo completar la solicitud. Vuelva a intentarlo más tarde.",
                VERIFY_TOKEN_GENERATED:
                    "El token de verificación lo genera el sistema; no se introduce",
                CREDENTIAL_GENERATED: "{{credential}}: lo genera el sistema; no se introduce",
                CREDENTIAL_NOT_USED: "Las cuentas de {{provider}} no usan {{credential}}",
                CREDENTIAL_EMPTY: "{{credential}} no puede estar vacío",
                NO_REPORT_CREDENTIALS:
                    "Las cuentas de {{provider}} no tienen credenciales de informes",
                UNKNOWN_AUDIENCE: "Selección de destinatarios desconocida",
                INVALID_MONTH: "El mes debe escribirse como AAAA-MM",
                INVALID_SUPPORT_REFERENCE: "No es una referencia de soporte",
                MESSENGER_PAGE_CHANGED:
                    "No enviado: el votante está conectado a otra página de Messenger",
            },
        },
        messagingAccounts: {
            status: {
                title: "Estado en el proveedor",
                daily: "Las cuentas se comprueban una vez al día y cada vez que compruebe la conexión.",
                unchecked:
                    "El número, la página o el remitente cambiaron desde la última comprobación. Hasta que la cuenta se compruebe de nuevo, los códigos y avisos que pasen por ella fallan por no estar lista; el mensaje de prueba sí se envía.",
                notReported: "No informado",
                yes: "Sí",
                no: "No",
                token: "Token de acceso",
                tokenExpires: "Válido · caduca el {{date}}",
                tokenNoExpiry: "Válido · no caduca",
                tokenInvalid: "No válido",
                missingScopes: "Faltan permisos: {{scopes}}",
                numberStatus: "Estado del número",
                nameStatus: "Nombre visible: {{status}}",
                qualityRating: "Calificación de calidad",
                quality: {
                    GREEN: "Alta",
                    YELLOW: "Media",
                    RED: "Baja",
                    UNKNOWN: "Desconocida",
                },
                messagingLimit: "Límite de mensajes",
                pagesMessaging: "Permiso para enviar (pages_messaging)",
                subscribed: "Aplicación suscrita a la página",
                getStarted: "Botón Empezar",
                getStartedMissing:
                    "Sin el botón, los votantes que nunca escribieron a la página no pueden iniciar la conversación que les trae su código.",
                partnerLogin: "Acceso del socio",
                appSecret: "Secreto de la aplicación",
                notChecked: "No se pudo comprobar",
                numberQualityWarning:
                    "Un aviso de calidad de Meta: el número sigue enviando, pero una calidad baja puede reducir su límite de mensajes.",
                accepted: "Aceptado",
                refused: "Rechazado",
                callback: "Dirección de retorno",
                callbackReady: "Configurada",
                approvalFromCheck: "Informada por la comprobación del proveedor",
                approvalEntered:
                    "Introducida por un administrador: la comprobación del proveedor no la informó",
                approvalEnteredHelp:
                    "Solo se usa hasta que la comprobación del proveedor informe de la aprobación.",
                templates: {
                    title: "Plantillas en el proveedor",
                    none: "Sin plantillas",
                    name: "Plantilla",
                    language: "Idioma",
                    category: "Categoría",
                    status: "Estado",
                    entered:
                        "Las plantillas de Viber las introduce un administrador en Plantillas aprobadas: el socio no tiene API para la aprobación de plantillas.",
                },
                templateCategory: {
                    AUTHENTICATION: "Autenticación",
                    UTILITY: "Utilidad",
                    MARKETING: "Marketing",
                },
                templateStatus: {
                    APPROVED: "Aprobada",
                    PENDING: "En revisión",
                    REJECTED: "Rechazada",
                    PAUSED: "En pausa",
                    DISABLED: "Desactivada",
                },
                numberChanges: {
                    title: "Votantes que cambiaron su número de WhatsApp",
                    description:
                        "WhatsApp informó de estos cambios de número. Pida a estos votantes que verifiquen de nuevo su contacto; no se cambió nada automáticamente.",
                    item: "{{old}} cambió a {{new}} el {{date}}.",
                    voters: "Votantes: {{voters}}",
                    noVoters:
                        "Ningún votante recibió mensajes de esta cuenta en el número anterior.",
                },
            },
            tab: "MENSAJERÍA",
            description:
                "Cuentas que envían a los votantes sus códigos y avisos. Cada evento electoral elige la cuenta de cada canal; los eventos nuevos empiezan con la cuenta predeterminada.",
            list: {
                title: "Cuentas de envío",
                add: "Añadir cuenta",
                loading: "Cargando cuentas",
                loadError: "No se pudieron cargar las cuentas de envío.",
                empty: "Aún no hay cuentas de envío.",
            },
            column: {
                channel: "Canal",
                name: "Cuenta",
                sender: "Envía como",
                provider: "Proveedor",
                default: "Predeterminada",
                isDefault: "Cuenta predeterminada",
                lastCheck: "Última comprobación",
                actions: "Acciones",
            },
            action: {
                edit: "Editar",
                editNamed: "Editar {{name}}",
                view: "Ver",
                viewNamed: "Ver {{name}}",
                check: "Comprobar conexión",
                checkNamed: "Comprobar la conexión de {{name}}",
                test: "Enviar mensaje de prueba",
                testNamed: "Enviar un mensaje de prueba desde {{name}}",
                delete: "Eliminar",
                deleteNamed: "Eliminar {{name}}",
            },
            check: {
                done: "Se comprobó {{name}}. Su estado está actualizado.",
                error: "No se pudo comprobar {{name}}.",
            },
            delete: {
                refused: "La cuenta no se eliminó: {{reason}}",
                title: "Eliminar cuenta",
                body: "¿Eliminar {{name}}? Los eventos electorales que la usan dejarán de enviar por su canal.",
                success: "Cuenta eliminada",
                error: "No se pudo eliminar la cuenta.",
            },
            editor: {
                addTitle: "Añadir cuenta",
                editTitle: "Editar cuenta de {{channel}}",
                subtitle:
                    "Los votantes reciben códigos y avisos de esta cuenta en los canales que la usan.",
                channel: "Canal",
                provider: "Proveedor",
                save: "Guardar",
                cancel: "Cancelar",
                close: "Cerrar",
                channelHelp: "No se puede cambiar después de crear la cuenta.",
            },
            field: {
                name: "Nombre de la cuenta",
                from_address: "Dirección del remitente",
                from_name: "Nombre del remitente",
                region: "Región de AWS",
                notification_topic_arn: "Tema de notificaciones de entrega (ARN de SNS)",
                server_url: "Servidor y puerto",
                sender_id: "ID de remitente",
                origination_number: "Número de origen",
                business_account_id: "ID de la cuenta de WhatsApp Business",
                phone_number_id: "ID del número de teléfono",
                display_phone_number: "Número",
                display_name: "Nombre visible",
                api_version: "Versión de Graph API",
                page_id: "ID de la página de Facebook",
                page_name: "Nombre de la página",
                page_username: "Nombre de usuario de la página",
                base_url: "URL base de la API",
                sender: "Nombre del remitente",
                provider_approval: "Aprobación del proveedor",
                is_default: "Cuenta de {{channel}} predeterminada para nuevos eventos electorales",
                api_base_url: "URL base de la Graph API",
            },
            fieldHelp: {
                from_address:
                    "La dirección que ven los votantes. Su dominio debe estar verificado con el proveedor.",
                notification_topic_arn:
                    "El tema de SNS en el que SES publica los eventos de entrega y rebote. Se rechazan las notificaciones de cualquier otro tema.",
                sender_id: "Hasta 11 letras y dígitos. Algunos países exigen registrarlo.",
                origination_number:
                    "Se usa en lugar del ID de remitente donde un país exige un número.",
                phone_number_id: "El número desde el que se envían los mensajes.",
                display_name: "El nombre visible que Meta aprobó para el número.",
                page_username:
                    "Se usa en el enlace m.me que los votantes abren para obtener su código.",
                api_version: "Por ejemplo, v23.0.",
                base_url: "La URL base de la API de Infobip de la cuenta.",
                sender: "El remitente aprobado que ven los votantes.",
                provider_approval:
                    "Meta solo permite la mensajería de gobiernos en WhatsApp mediante un acuerdo aprobado. Elija Aprobación del proveedor confirmada cuando Meta lo haya aprobado para esta cuenta; hasta entonces no se pueden activar códigos ni avisos en ella.",
                api_base_url:
                    "Solo cuando la Graph API no es la de Meta, como el punto de acceso de un proveedor de soluciones. Vacío usa la de Meta.",
            },
            error: {
                REQUIRED: "Obligatorio",
                NOT_A_COUNT: "Introduce un número entero",
                AT_LEAST_ONE: "Introduce 1 o más",
                OTP_ABOVE_TOTAL: "No puede superar los mensajes por segundo",
                INVALID_CALLING_CODE:
                    "Introduce prefijos telefónicos de país de 1 a 3 dígitos, como 63",
                DUPLICATE_LANGUAGE: "Este idioma ya tiene una plantilla para este propósito",
                NOT_A_URL: "Indique una dirección que empiece por https://",
                UNSUPPORTED_PROVIDER: "Proveedor no compatible: sustituya esta cuenta",
                CREDENTIALS_FOLLOW_ENDPOINT:
                    "Vuelva a introducir las credenciales para la nueva dirección",
            },
            warning: {
                pageChangeCount:
                    "Votantes conectados a {{page}} que deberán volver a conectarse: {{count}}.",
                pageChangeCounting: "Contando los votantes conectados a la página…",
                pageChangeCountError:
                    "No se pudo leer el número de votantes conectados a la página.",
                recheck:
                    "Al guardar, la cuenta se comprueba de nuevo en segundo plano. Hasta que esa comprobación salga bien, los códigos y avisos que pasen por ella fallan por no estar lista; el mensaje de prueba sí se envía.",
                pageChange:
                    "Las conversaciones de Messenger pertenecen a una página. Tras cambiar la página, no se envían códigos ni avisos a los votantes conectados a {{page}} hasta que vuelvan a conectarse. El votante se vuelve a conectar en su siguiente inicio de sesión, después de introducir su contraseña: el inicio de sesión muestra el paso de conexión de la página nueva, el votante pulsa Empezar en Messenger y recibe allí el código. Un votante que no puede iniciar sesión con contraseña, por ejemplo porque la ha olvidado, necesita que el personal le cambie el canal.",
                numberChange:
                    "Los mensajes llegarán desde otro número. Sus plantillas deben estar aprobadas en esa cuenta de empresa antes de poder enviar códigos, y los votantes verán un chat nuevo.",
            },
            viber: {
                title: "Plantillas aprobadas",
                description:
                    "Introduce las plantillas que Viber aprobó a través del socio, por propósito e idioma. La API de plantillas del socio no está disponible, así que esta lista se mantiene a mano y la comprobación de conexión la lee.",
                purpose: "Propósito",
                language: "Idioma",
                templateId: "ID de plantilla del socio",
                add: "Añadir plantilla",
                remove: "Quitar plantilla",
            },
            limits: {
                title: "Límites de envío",
                messagesPerSecond: "Mensajes por segundo",
                messagesPerSecondHelp: "Vacío: los mensajes no se limitan.",
                otpReservedPerSecond: "Reservados para códigos por segundo",
                otpReservedHelp:
                    "Un mínimo dentro del único presupuesto de envío compartido de la cuenta: los avisos se detienen en él y los códigos van primero. Si no se envían códigos, un envío masivo puede usar todo el ritmo.",
                otpConcurrency: "Códigos enviados a la vez",
                otpConcurrencyHelp:
                    "Vacío: 8. Los códigos y los avisos de Keycloak (enlaces de restablecimiento, contraseñas temporales) ocupan cada uno una plaza mientras se envían. Se cuenta por proceso del servidor; los demás envíos esperan su turno.",
                allowedCallingCodes: "Destinos permitidos (prefijos telefónicos de país)",
                allowedCallingCodesHelp:
                    "Separados por comas, por ejemplo 63, 971. Vacío permite cualquier destino.",
            },
            credentials: {
                title: "Credenciales",
                description:
                    "Las credenciales son de solo escritura: después de guardar, solo se muestra la fecha en que se reemplazó cada una.",
                set: "Configurada · reemplazada {{date}}. Se guarda cifrada y nunca se muestra.",
                replace: "Reemplazar",
                replaceNamed: "Reemplazar {{name}}",
                reenter: "La dirección ha cambiado: vuelva a introducir esta credencial.",
            },
            credentialHelp: {
                AWS_SES: {
                    AWS_ACCESS_KEY_ID: "Opcional. Sin claves, se usa el rol propio del servicio.",
                    AWS_SECRET_ACCESS_KEY:
                        "Opcional. Configúrala junto con el ID de clave de acceso.",
                },
                AWS_SNS: {
                    AWS_ACCESS_KEY_ID: "Opcional. Sin claves, se usa el rol propio del servicio.",
                    AWS_SECRET_ACCESS_KEY:
                        "Opcional. Configúrala junto con el ID de clave de acceso.",
                },
                SMTP: {
                    SMTP_PASSWORD: "La contraseña del servidor SMTP.",
                },
                WHATSAPP_CLOUD_API: {
                    ACCESS_TOKEN:
                        "Un token de un usuario del sistema del portafolio empresarial del propietario, con whatsapp_business_messaging.",
                    APP_SECRET: "Comprueba que las llamadas al webhook proceden de Meta.",
                },
                MESSENGER_SEND_API: {
                    ACCESS_TOKEN: "Un token de acceso de página con pages_messaging.",
                    APP_SECRET: "Comprueba que las llamadas al webhook proceden de Meta.",
                },
                VIBER_INFOBIP: {
                    API_KEY: "La clave de API de Infobip.",
                },
            },
            webhook: {
                title: "Informes de entrega",
                description: {
                    WHATSAPP_CLOUD_API:
                        "Introduzca esta URL de retorno en la configuración de webhooks de Meta. Los informes de entrega llegan aquí y se comprueban con el secreto de la aplicación.",
                    MESSENGER_SEND_API:
                        "Introduzca esta URL de retorno en la configuración de webhooks de Meta. Los informes de entrega y el toque del votante en Empezar llegan aquí y se comprueban con el secreto de la aplicación.",
                    VIBER_INFOBIP:
                        "Infobip envía los informes de entrega de Viber a esta URL de retorno mediante una suscripción con autenticación Basic.",
                    AWS_SES:
                        "Suscriba esta URL de retorno (HTTPS) al tema de notificaciones de entrega. Los eventos de entrega y rebote llegan aquí y se comprueban con la firma de Amazon.",
                },
                afterSaving: "Se muestra después de guardar",
                tokenSet: "Configurado · reemplazado {{date}}",
                tokenMissing: "Aún no generado",
                tokenAfterSaving: "Se genera después de guardar",
                generate: "Generar token de verificación",
                tokenTitle: "Token de verificación",
                tokenOnce:
                    "Introduce ahora este token en la configuración de webhooks de Meta. Solo se muestra una vez.",
                copyToken: "Copiar token de verificación",
                tokenDone: "Hecho",
                tokenError: "No se pudo generar el token de verificación.",
                url: "URL de retorno",
                copyUrl: "Copiar la URL de retorno",
                needsWrite:
                    "Se muestra a los administradores que pueden gestionar cuentas de mensajería.",
                loading: "Cargando…",
                unavailable: "No disponible",
                loadError: "No se pudo cargar la URL de retorno.",
            },
            copy: {
                success: "Copiado",
                error: "No se pudo copiar",
            },
            save: {
                success: "Cuenta guardada",
                error: "No se pudo guardar la cuenta.",
                refused: "No se ha guardado la cuenta: {{reason}}",
            },
            test: {
                title: "Enviar un mensaje de prueba desde {{name}}",
                description:
                    "Envía un mensaje real para el propósito elegido a este destino. El resultado muestra lo que informó el proveedor.",
                purpose: "Propósito",
                destination: {
                    EMAIL_ADDRESS: "Correo electrónico",
                    PHONE_NUMBER: "Número de teléfono (E.164)",
                    PAGE_SCOPED_ID: "ID de ámbito de página",
                },
                language: "Idioma",
                send: "Enviar mensaje de prueba",
                reason: "Motivo: {{reason}}",
                error: "No se pudo enviar el mensaje de prueba.",
                template: "Plantilla aprobada",
                templateHelp:
                    "El nombre o ID de la plantilla que el proveedor aprobó para este propósito e idioma.",
                viberTemplate:
                    "Viber usa la plantilla que esta cuenta indica como aprobada para el propósito y el idioma elegidos.",
                languageHelp:
                    "Para un proveedor que envía plantillas aprobadas, indique el código de idioma del proveedor para la plantilla, como en_US.",
            },
            unsupported:
                "Proveedor no compatible: sustituya esta cuenta. Su proveedor ya no es compatible, así que no puede enviar. Añada una cuenta para este canal con un proveedor compatible, elíjala en los eventos electorales que usan esta y después elimine esta cuenta.",
            reports: {
                label: "Credenciales de informes",
                missing: "Aún no generadas",
                generate: "Generar credenciales de informes",
                title: "Credenciales de informes",
                once: "Introdúzcalas ahora en la suscripción de Infobip. Solo se muestran una vez; generar otras nuevas las sustituye.",
                copyUsername: "Copiar el usuario de informes",
                copyPassword: "Copiar la contraseña de informes",
                error: "No se pudieron generar las credenciales de informes.",
                step: {
                    subscription:
                        "En Infobip, cree una suscripción para los informes de entrega de Viber.",
                    url: "Ponga como URL la URL de retorno de esta cuenta.",
                    auth: "Elija la autenticación Basic e introduzca el usuario y la contraseña de informes.",
                },
                confirmTitle: "¿Reemplazar las credenciales de informes?",
                confirmBody:
                    "El usuario y la contraseña de informes actuales dejan de funcionar de inmediato. Hasta que la suscripción de Infobip tenga los nuevos, los informes de entrega de Viber se rechazan y los mensajes se quedan en Aceptado.",
                confirm: "Reemplazar",
            },
        },
        sentMessages: {
            showMore: "Mostrar más",
            showLess: "Mostrar menos",
            tab: "Mensajes enviados",
            accountsTab: "Cuentas",
            tabsLabel: "Secciones de mensajería",
            description:
                "Mensajes que recibió el proveedor de demostración de este entorno de desarrollo, tal como los mostrarían las aplicaciones de los votantes. Ninguno llegó a un votante real. Esta pestaña solo aparece donde las rutas del proveedor de demostración están habilitadas.",
            modeLabel: "Mostrar mensajes de",
            all: "Todos",
            refresh: "Actualizar",
            refreshing: "Actualizando…",
            refreshError:
                "No se pudieron actualizar los mensajes. La lista mostrada es la última que se leyó.",
            empty: "Aún no hay mensajes.",
            emptyChannel: "Aún no hay mensajes de {{channel}}.",
            column: {
                time: "Hora",
                channel: "Canal",
                from: "De",
                to: "Para",
                message: "Mensaje",
                report: "Informe de entrega",
            },
            report: {
                accepted: "Informe aceptado por el componente de webhooks (HTTP 200)",
                refused: "Informe rechazado: HTTP {{status}}",
                pending: "Informe aún no enviado",
                notSent: "Informe no enviado: {{reason}}",
                noAnswer: "El informe no obtuvo respuesta: {{reason}}",
                noReason: "sin motivo indicado",
                unavailable: "Este canal no tiene informes de entrega",
            },
            content: {
                demoCode:
                    "{{code}} es su código de verificación. Por su seguridad, no lo comparta con nadie.",
                demoNotice:
                    "Recordatorio de votación\n\nHola {{name}}, la votación de {{event}} ya está abierta.\n\nAcceda al portal de votación para revisar su papeleta y emitir su voto:\n{{url}}\n\nNo comparta su código de acceso con nadie. Si ya ha votado, no necesita hacer nada más.",
                demoLegacyNotice:
                    "Recordatorio de votación\n\nHola {{name}}, la votación ya está abierta. Acceda al portal de votación para emitir su voto:\n{{url}}\n\nNo comparta su código de acceso con nadie.",

                template: "Plantilla {{name}} ({{language}})",
                noText: "(sin texto)",
                code: "Código",
                copyCode: "Copiar código",
                copied: "Código copiado",
                copyFailed: "No se pudo copiar el código",
                truncated:
                    "Recortado: el proveedor de demostración solo conserva los primeros 64 KiB de cada campo.",
            },
            chat: {
                phone: "{{channel}} en un teléfono",
                chats: "Chats de {{channel}}",
                open: "Abrir el chat con {{recipient}} desde {{sender}}",
                back: "Volver a los chats",
                from: "De {{sender}}",
                messages: "Mensajes para {{recipient}}",
            },
            getStarted: {
                title: "Empezar",
                help: "Pegue el enlace de Messenger que muestra la página de inscripción y pulse Empezar, como haría el votante en Messenger. Después, el código llega al chat.",
                link: "Enlace de Messenger",
                submit: "Empezar",
                accepted: "El componente de webhooks respondió 200. El código llega al chat.",
                answered: "El componente de webhooks respondió {{status}}.",
                failed: "No se pudo empezar: {{reason}}",
            },
            email: {
                inbox: "Bandeja de entrada",
                reading: "Correo seleccionado",
                noSubject: "(sin asunto)",
                from: "De",
                to: "Para",
                view: "Mostrar el correo como",
                html: "HTML",
                text: "Texto sin formato",
                htmlFrame: "Contenido del correo",
                noContent: "Este correo no tiene contenido.",
                loading: "Cargando el correo…",
                loadError:
                    "No se pudo leer el HTML del correo: puede que el proveedor de demostración ya no lo conserve.",
            },
            reason: {
                SEVERAL_ACCOUNTS: "Varias cuentas usan este remitente",
                ACCOUNTS_UNREADABLE: "No se pudieron leer las cuentas",
                UNREACHABLE: "El componente de webhooks no respondió",
                LINK_UNREADABLE: "Pegue el enlace m.me de la página de inscripción",
                LINK_WITHOUT_REF: "El enlace no tiene ref: copie el enlace completo",
                NO_PAGE_ACCOUNT: "Ninguna cuenta de Messenger de esta organización usa esta página",
                SEVERAL_PAGE_ACCOUNTS:
                    "Varias cuentas de Messenger de esta organización usan esta página",
            },
        },
        electoralLogConsole: {
            title: "Registro Electoral",
            subtitle:
                "Explore el registro electoral y la urna de un evento electoral, o consulte su base de datos del registro electoral. Nada en esta página modifica datos.",
            notAllowed:
                "Necesita el permiso electoral-log-console-read para abrir el registro electoral.",
            tabs: {
                tables: "Tablas",
                query: "Consulta",
            },
            electionEvent: "Evento Electoral",
            tenant: "Inquilino",
            table: "Tabla",
            tables: {
                records: "Entradas",
                ballots: "Papeletas",
                voters: "Votantes",
                queue: "Cola del Secuenciador",
            },
            order: {
                "label": "Orden",
                "newest-first": "Más recientes primero",
                "oldest-first": "Más antiguos primero",
            },
            filters: {
                statement_kind: "Tipo",
                election_id: "Elección",
                area_id: "ID de Área",
                user_id: "ID de Usuario o Votante",
                ballot_id: "ID de Papeleta",
                status: "Estado",
                created_after: "Desde",
                created_before: "Hasta",
                any: "Cualquiera",
                apply: "Aplicar Filtros",
                clear: "Limpiar",
                invalid:
                    "Revise los filtros marcados: las fechas deben ser válidas y los ID de área de la urna deben ser UUID.",
            },
            statuses: {
                valid: "Válida",
                pending: "Pendiente",
                rejected: "Rechazada",
            },
            estimatedRows: "Unas {{rows}} filas en esta tabla antes de filtrar.",
            personalDataHidden:
                "Los nombres de usuario, direcciones IP y países aparecen ocultos: verlos requiere el permiso electoral-log-personal-data-read.",
            loadError: "No se pudo leer el registro electoral.",
            record: {
                title: "Entrada {{position}}",
                view: "Ver entrada",
                message: "Mensaje (decodificado)",
                copy: "Copiar JSON",
                close: "Cerrar",
                loadError: "No se pudo leer la entrada.",
            },
            query: {
                help: "Las consultas se ejecutan sobre la base de datos del registro electoral del evento electoral en una transacción de solo lectura. Sus tablas son electoral_log_messages, ballot_box_ballot, ballot_box_voter, ballot_box_pending y ballot_box_sequencer; trellis_logs describe sus registros Merkle.",
                scope: "Cada evento electoral tiene su propia base de datos, así que una consulta lee un solo evento.",
                chooseEvent: "Elija el evento electoral que consultar.",
                placeholder: "SELECT …",
                limits: "Solo lectura · hasta 1.000 filas · límite de 30 s · cada consulta queda anotada en los registros del servidor con su usuario",
                run: "Ejecutar Consulta",
                summary: "Filas: {{rows}} · {{ms}} ms",
                truncated: "Solo se muestran las primeras {{rows}} filas.",
                error: "No se pudo ejecutar la consulta.",
            },
        },
        taskQueues: {
            title: "Colas de Tareas",
            subtitle:
                "Las colas de tareas del entorno: qué está esperando, qué se ha procesado y cuánto ha tardado. Las listas de las colas nunca muestran los argumentos de las tareas.",
            notAllowed:
                "Necesita el permiso task-queues-read del tenant de superadministración para abrir las colas de tareas.",
            live: "En directo",
            updated: "Actualizado a las {{time}}",
            error: "No se han podido cargar las colas de tareas.",
            columns: {
                queue: "Cola",
                ready: "Listos",
                runningOrScheduled: "En curso o programados",
                oldest: "Más antiguo",
                processed: "Procesados (última hora)",
                lastHour: "Resultados (última hora)",
                sent: "Enviados (total)",
            },
            outcomes: {
                succeeded: "Correctos",
                failed: "Fallidos",
                expired: "Caducados",
                rejected: "Rechazados",
                discarded: "Descartados",
                unknown: "Desconocido",
            },
            graphs: {
                title: "Rendimiento",
                period: "Periodo",
                periods: {
                    hour: "Última hora",
                    sixHours: "Últimas 6 horas",
                    day: "Últimas 24 horas",
                    week: "Últimos 7 días",
                },
                outcomes: "Mensajes procesados por resultado",
                durations: "Tiempo medio de espera y de proceso",
                wait: "Espera",
                processing: "Proceso",
                seconds: "Segundos",
                empty: "No se ha procesado ningún mensaje de esta cola en este periodo.",
            },
            messages: {
                title: "Mensajes",
                states: {
                    queued: "En cola",
                    archived: "Archivados",
                },
                argumentsHidden:
                    "Los argumentos de las tareas nunca se muestran, porque pueden contener datos de votantes.",
                refresh: "Actualizar",
                newest: "Más recientes",
                newer: "Más nuevos",
                older: "Más antiguos",
                empty: "No hay mensajes",
                unreadable: "Mensaje ilegible",
                columns: {
                    id: "ID",
                    task: "Tarea",
                    outcome: "Resultado",
                    event: "Evento",
                    error: "Error",
                    enqueued: "Encolado",
                    reads: "Lecturas",
                    retries: "Reintentos",
                    archived: "Archivado",
                    visible: "Visible desde",
                    size: "Tamaño",
                    taskId: "ID de tarea",
                },
            },
            deadLetters: {
                help: "Estos eventos del registro electoral no se han podido procesar. Seleccione eventos para reenviarlos a la cola de eventos o para descartarlos; los eventos descartados se quedan en el archivo.",
                noWrite:
                    "Necesita el permiso task-queues-write para reenviar o descartar estos eventos.",
                replay: "Reenviar",
                discard: "Descartar",
                cancel: "Cancelar",
                confirmTitle: {
                    replay: "¿Reenviar eventos?",
                    discard: "¿Descartar eventos?",
                },
                confirmBody: {
                    replay_one:
                        "Se reenviará {{count}} evento a la cola de eventos del registro electoral. Si vuelve a fallar, regresará aquí.",
                    replay_many:
                        "Se reenviarán {{count}} eventos a la cola de eventos del registro electoral. Los eventos que vuelvan a fallar regresarán aquí.",
                    replay_other:
                        "Se reenviarán {{count}} eventos a la cola de eventos del registro electoral. Los eventos que vuelvan a fallar regresarán aquí.",
                    discard_one:
                        "Se moverá {{count}} evento al archivo y no se añadirá al registro electoral.",
                    discard_many:
                        "Se moverán {{count}} eventos al archivo y no se añadirán al registro electoral.",
                    discard_other:
                        "Se moverán {{count}} eventos al archivo y no se añadirán al registro electoral.",
                },
                queued: {
                    replay_one: "Reenvío de {{count}} evento encolado como tarea {{taskId}}.",
                    replay_many: "Reenvío de {{count}} eventos encolado como tarea {{taskId}}.",
                    replay_other: "Reenvío de {{count}} eventos encolado como tarea {{taskId}}.",
                    discard_one: "Descarte de {{count}} evento encolado como tarea {{taskId}}.",
                    discard_many: "Descarte de {{count}} eventos encolado como tarea {{taskId}}.",
                    discard_other: "Descarte de {{count}} eventos encolado como tarea {{taskId}}.",
                },
                failed: "No se ha podido encolar la operación.",
            },
            tabs: {
                queues: "Colas",
                query: "Consulta",
            },
            query: {
                help: "Las consultas se ejecutan sobre la base de datos de colas de tareas del entorno en una transacción de solo lectura, con su rol de lectura. Cada cola tiene una tabla de mensajes en espera, pgmq.q_<cola>, y otra de mensajes procesados, pgmq.a_<cola>.",
                arguments:
                    "Las consultas leen los mensajes tal como están guardados, con los argumentos de sus tareas, que pueden incluir datos de votantes.",
                placeholder: "SELECT …",
                limits: "Solo lectura · hasta 1.000 filas · límite de 30 s · cada consulta queda anotada en los registros del servidor con su usuario",
                run: "Ejecutar Consulta",
                summary: "Filas: {{rows}} · {{ms}} ms",
                truncated: "Solo se muestran las primeras {{rows}} filas.",
                error: "No se pudo ejecutar la consulta.",
            },
        },
    },
}

export default spanishTranslation
