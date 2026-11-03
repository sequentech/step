---
id: election_management_election_event_reports
title: Informes
description: "La pestaña Informe crea documentos sobre el proceso electoral, por ejemplo los resultados electorales, un informe estadístico o el registro de actividad."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

La pestaña **Informe** crea documentos sobre el proceso electoral, por ejemplo los resultados
electorales, un informe estadístico o el registro de actividad. Cada fila de la lista es una
configuración de informe: un tipo, una elección, una plantilla y una política. El documento se
genera desde la fila. El encabezado de la pestaña dice "Generar informes para los eventos
electorales".

## Abrir la pestaña {#open-the-tab}

1. En el menú de la izquierda, haga clic en el proceso electoral.
2. Haga clic en la pestaña **Informe**.

La pestaña se muestra con el permiso **Ver Informes del Evento Electoral**. Sigue visible cuando
el proceso electoral está confinado. Las acciones necesitan estos permisos:

| Acción | Permiso |
| --- | --- |
| Crear | **Crear Informe** |
| Editar | **Editar Informes** |
| Borrar | **Eliminar Informe** |
| Generar | **Generar Informe** |
| Vista previa | **Vista Previa del Informe** |
| Seleccionar las columnas | **Columnas de los Informes del Evento Electoral** |

## Lista {#list}

Cuando la lista está vacía, la pestaña muestra "No hay informes todavía." y **Crear informe**.
Cuando hay informes, la barra de herramientas tiene **Añadir** y **Añadir filtro**.

| Columna | Contenido |
| --- | --- |
| **Tipo de informe** | El tipo del informe. |
| **Plantilla** | El alias de la plantilla, si la hay. |
| **Elección** | La elección del informe, si la hay. |
| Cifrado | Un icono que muestra si el archivo del informe está cifrado. |
| **Acciones** | **Editar**, **Eliminar**, **Generar** y **Vista previa**. |

## Formulario del informe {#report-form}

| Campo | Uso |
| --- | --- |
| **Tipo** | El tipo del informe, por ejemplo **Resultados Electorales**, **Informe Estadístico**, **Registros de Actividad**, **Registros de Auditoría**, **Recibo de Voto** o **Initialization Report** (informe de inicialización). |
| **Elección** | La elección del informe. Algunos tipos requieren una elección. Otros tipos no la permiten. |
| **Plantilla** | La plantilla del documento. Si no selecciona ninguna, la plataforma usa su plantilla por defecto para el tipo. Algunos tipos requieren una plantilla. |
| **Etiqueta de permiso** | La etiqueta de permiso del informe. |
| **Repetible** | Crea el informe de nuevo según una programación. Solo se muestra para los tipos que lo permiten. |
| **Expresión Cron** | La programación de un informe repetible. |
| **Destinatarios de correo electrónico** | Las direcciones de correo electrónico que reciben el informe repetible. Escriba una dirección y pulse Intro. |
| **Política de cifrado** | **Sin cifrar** o **Contraseña configurada**. |
| **Guardar** | Guarda la configuración del informe. |

Cuando selecciona **Contraseña configurada**, una ventana pide la contraseña. Escríbala en
**Contraseña:** y en **Repetir Contraseña:**. Después haga clic en **Guardar contraseña**. No puede
cambiar la política del informe después de guardar una contraseña.

## Acciones {#actions}

| Acción | Resultado |
| --- | --- |
| **Generar** | Crea el documento a partir de los datos reales. La tarea "Generar Reporte" crea el archivo. Descárguelo con **Descargar Archivo**. |
| **Vista previa** | Crea un ejemplo del documento con datos de muestra. Úsela para comprobar una plantilla. |
| **Editar** | Abre el formulario del informe. |
| **Eliminar** | Borra la configuración del informe después de la pregunta "¿Estás seguro de que quieres eliminar este informe?". |

Para un informe cifrado, la pestaña muestra "Descifrar archivo" con las opciones de comando `-in`,
`-out` y `-pass`.

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| "Error al crear el informe" | Compruebe los campos obligatorios y guarde de nuevo. |
| "Las contraseñas no coinciden. Asegúrate de que ambos campos contengan la misma contraseña." | Escriba la misma contraseña en los dos campos. |
| "Error al configurar el cifrado del informe" | Establezca la contraseña de nuevo. |
| El documento no usa su plantilla. | Compruebe que la fila del informe tiene la plantilla. Consulte [Asociación de valores a plantillas](../../../../../Tutorials/Reports-and-Templates/value_to_template_association.md). |

## Páginas relacionadas {#related-pages}

- [Informes y plantillas](../../../../../Tutorials/Reports-and-Templates/reports_and_templates.md)
- [Plantillas](../../../Templates/admin_portal_reference_user-manual_templates.md)
- [Obtener los resultados](../../../../../procedures/07-results.md#3-make-the-reports)
- [Tareas](../Tasks/election_management_election-event_tasks.md)
