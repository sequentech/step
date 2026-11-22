---
id: admin_portal_tutorials_election-logs
title: Registros electorales
description: "Los procedimientos Publicar y gestionar el período de votación, Realizar la ceremonia de recuento y Obtener los resultados describen la ruta principal de una elección."
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

Los procedimientos [Publicar y gestionar el período de votación](../03-procedures/05-publish.md), [Realizar la ceremonia de recuento](../03-procedures/06-tally.md) y [Obtener los resultados](../03-procedures/07-results.md) describen la ruta principal de una elección. Cada acción importante de esa ruta crea un registro en el registro electoral. Esta página le indica cómo leer, filtrar, exportar y auditar el registro electoral.

## Qué es el registro electoral {#what-the-electoral-log-is}

Cada proceso electoral tiene su propio registro electoral. La plataforma añade un registro por cada acción importante y firma cada registro. No puede cambiar ni eliminar un registro desde el portal de administración. El registro electoral es la prueba de lo que ocurrió en el proceso electoral y de cuándo ocurrió.

El registro electoral guarda, por ejemplo:

- La publicación de la papeleta, y las publicaciones que fallaron.
- Cada apertura, pausa y cierre de un canal de votación, para una elección o para el proceso electoral.
- La ceremonia de claves, y la subida de cada fragmento de clave en una ceremonia de recuento.
- El inicio y el final de cada recuento.
- Cada pausa de un recuento por un empate, cada resolución de empate y cada reinicio.
- Cada voto emitido, con su canal, y cada error cuando un votante emite un voto.
- La publicación y la revocación de los resultados en el sitio de resultados.
- Los mensajes enviados a los votantes, los eventos de las cuentas de los votantes y los cambios en las autoridades de certificación.

## Abrir la pestaña Logs {#open-the-logs-tab}

1. Abra el proceso electoral.
2. Haga clic en la pestaña **Logs**.

La pestaña muestra primero los registros más recientes. Para ver la pestaña, su cuenta necesita **Ver Registros del Evento Electoral**. Para leer los registros, necesita **Leer Registros**. Sin este permiso, la pestaña muestra "No tienes permiso para acceder a los registros."

## Las columnas {#the-columns}

| Columna | Contenido |
| --- | --- |
| **ID** | El número del registro en el registro electoral. |
| **ID de usuario** | El ID del usuario que realizó la acción. Está oculta por defecto. |
| **Nombre de Usuario** | El nombre del usuario que realizó la acción, o "-" para una acción del sistema. |
| **Creado** | La fecha y la hora en que la plataforma guardó el registro. |
| **Marca de tiempo de declaración** | La fecha y la hora de la acción. |
| **Tipo de declaración** | El tipo de registro, por ejemplo `ElectionPublish`, `ElectionVotingPeriodOpen`, `TallyOpen` o `CastVote`. |
| **Tipo de declaración** (segunda columna) | `USER` para una acción de una persona, `SYSTEM` para una acción de la plataforma. |
| **Tipo de registro** | `INFO` o `ERROR`. |
| **Descripción** | Un texto corto que indica lo que ocurrió. Haga clic para ver todo el texto. |
| **Mensaje** | El contenido completo del registro. |

Para mostrar u ocultar columnas, use el botón de columnas. Para este botón, su cuenta necesita **Columnas de los Registros del Evento Electoral**.

## Filtrar los registros {#filter-the-records}

Haga clic en el botón de filtro y seleccione un filtro:

- **ID de usuario** o **Nombre de Usuario**: los registros de un usuario.
- **Creado** o **Marca de tiempo de declaración**: los registros de una fecha y hora.
- **Tipo de declaración**: los registros de un tipo, por ejemplo `ElectionVotingPeriodClose`.

## Exportar el registro {#export-the-log}

Su cuenta necesita **Exportar Registros**.

1. En la pestaña **Logs**, haga clic en el botón de exportar.
2. Haga clic en **Exportar en CSV** o **Exportar en PDF**.
3. Lea el mensaje "Por favor, confirma que deseas ejecutar esta acción; puede tardar un tiempo en completarse."
4. Haga clic en **Exportar**.

**Resultado esperado:** aparece la tarea **Exportar Informe de Registros de Actividad** en el panel de tareas. Cuando la tarea termina, descargue el archivo.

La exportación contiene el registro de actividad del proceso electoral. Para una exportación periódica, use un informe del tipo **Registros de Actividad** en la pestaña **Informe**. Puede marcarlo como **Repetible** con una **Expresión Cron** y **Destinatarios de correo electrónico**. Consulte [Informes y plantillas](18-reports_and_templates.md).

## Auditar el registro {#audit-the-log}

La auditoría comprueba cada registro con el registro Merkle del proceso electoral y verifica los puntos de control publicados. Un registro Merkle es una cadena de hashes: un cambio en un registro rompe la cadena. Su cuenta necesita **Auditar Registro Electoral**.

1. En la pestaña **Logs**, haga clic en **Auditar**.
2. Lea el mensaje en **Auditar el registro electoral**.
3. Haga clic en **Auditar**.

**Resultado esperado:** aparece la tarea **Auditar registro electoral** en el panel de tareas. El resultado de la auditoría se guarda en la tarea. También puede verlo en la pestaña **Tareas** del proceso electoral.

## Los registros de un votante {#the-logs-of-one-voter}

1. Abra el proceso electoral.
2. Haga clic en la pestaña **Votantes**.
3. En la fila del votante, abra el menú de acciones.
4. Haga clic en **Registros del usuario**.

El cuadro de diálogo muestra solo los registros del registro electoral de ese votante. Su cuenta necesita **Ver Registros de Votantes del Evento Electoral**.

## La página del registro electoral {#the-electoral-log-page}

El menú principal tiene el elemento **Registro Electoral**. Esta página permite a un auditor explorar el registro electoral y la urna de un proceso electoral, o consultar la base de datos del registro electoral. La página no cambia ningún dato. Su cuenta necesita **Explorar Registro Electoral**. Los nombres de usuario, las direcciones IP y los países aparecen ocultos sin **Leer Datos Personales del Registro Electoral**.

## Por qué los registros importan en una auditoría {#why-the-logs-matter-for-an-audit}

- Use el registro para demostrar cuándo abrió y cerró cada canal, y quién lo hizo.
- Compare los registros del recuento con los registros de la ceremonia de recuento.
- Ejecute la auditoría antes de publicar los resultados y guarde el resultado de la tarea.
- Exporte el registro al final de la elección y guárdelo con los demás registros. Consulte [Obtener los resultados](../03-procedures/07-results.md#5-keep-the-records).

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| No se muestra la pestaña **Logs**. | Pida el permiso **Ver Registros del Evento Electoral**. |
| "No tienes permiso para acceder a los registros." | Pida el permiso **Leer Registros**. |
| No se muestra el botón de exportar o **Auditar**. | Pida el permiso **Exportar Registros** o **Auditar Registro Electoral**. |
| La tarea de exportación o de auditoría falló. | Mire la pestaña **Tareas**. Vuelva a intentarlo y, después, contacte con el soporte de Sequent. |
| La auditoría encuentra un error. | No publique los resultados. Contacte de inmediato con el soporte de Sequent y dé el nombre del proceso electoral. |
| Un registro no está en la lista. | Quite los filtros. La lista muestra primero los registros más recientes. |
