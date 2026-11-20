---
id: election_management_election_event_logs
title: Registros
description: "La pestaña Logs muestra el registro de las acciones del proceso electoral, por ejemplo las ceremonias, las publicaciones y los cambios del estado de la votación. Úsela para comprobar qué pasó y cuándo."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

La pestaña **Logs** (registros) muestra el registro de las acciones del proceso electoral, por
ejemplo las ceremonias, las publicaciones y los cambios del estado de la votación. Úsela para
comprobar qué pasó y cuándo. Puede exportar el registro para sus archivos.

## Abrir la pestaña {#open-the-tab}

1. En el menú de la izquierda, haga clic en el proceso electoral.
2. Haga clic en la pestaña **Logs**.

La pestaña se muestra con el permiso **Ver Registros del Evento Electoral**. Para leer las
entradas, su rol necesita **Leer Registros**. Sin él, la pestaña muestra "No tienes permiso para
acceder a las bitácoras.". La pestaña sigue visible cuando el proceso electoral está confinado.

## Barra de herramientas {#toolbar}

| Botón | Uso | Permiso |
| --- | --- | --- |
| **Columnas** | Selecciona las columnas. | **Columnas de los Registros del Evento Electoral** |
| **Añadir filtro** | Filtra por **ID de usuario**, **Nombre de Usuario**, **Creado**, **Marca de tiempo de declaración** o **Tipo de declaración**. | **Filtros de los Registros del Evento Electoral** |
| **Exportar** | Abre un menú con **Exportar en CSV** y **Exportar en PDF**. | **Exportar Registros** |

## Columnas {#columns}

| Columna | Contenido |
| --- | --- |
| **ID** | El número de la entrada. |
| **ID de usuario** y **Nombre de Usuario** | El usuario que hizo la acción. |
| **Creado** | La fecha y la hora de la entrada. |
| **Marca de tiempo de declaración** | La fecha y la hora de la acción. |
| **Tipo de declaración** | La clase de acción. |
| **Tipo de registro** | El tipo de entrada. |
| **Descripción** | Una descripción de la acción. |
| **Mensaje** | El contenido completo de la entrada. |

## Exportar el registro {#export-the-log}

1. Haga clic en **Exportar**.
2. Haga clic en **Exportar en CSV** o **Exportar en PDF**.
3. Lea el mensaje "Por favor, confirma que deseas ejecutar esta acción; puede tardar un tiempo en
   completarse."
4. Haga clic en **Exportar**.
5. Cuando la tarea "Exportar Informe de Registros de Actividad" esté completada, haga clic en
   **Descargar Archivo**.

**Resultado esperado:** tiene el archivo del registro.

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| "No tienes permiso para acceder a las bitácoras." | Su rol no tiene el permiso **Leer Registros**. Pídalo al administrador del inquilino. |
| La exportación tarda mucho. | Un registro grande tarda tiempo. Espere a que termine la tarea. |

## Páginas relacionadas {#related-pages}

- [Tareas](../Tasks/election_management_election-event_tasks.md)
- [Votantes](../Voters/election_management_election-event_voters.md) (la acción **Registros del usuario** muestra las entradas de un votante)
- [Informes](../Reports/election_management_election-event_reports.md)
