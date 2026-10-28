---
id: election_management_election_event_tasks
title: Tareas
description: "La pestaña Tareas muestra las tareas en segundo plano del proceso electoral, por ejemplo importaciones, exportaciones e informes. Úsela para ver si una tarea está completada, para leer sus registros y para descargar su archivo."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

La pestaña **Tareas** muestra las tareas en segundo plano del proceso electoral, por ejemplo
importaciones, exportaciones e informes. Úsela para ver si una tarea está completada, para leer sus
registros y para descargar su archivo.

## Abrir la pestaña {#open-the-tab}

1. En el menú de la izquierda, haga clic en el proceso electoral.
2. Haga clic en la pestaña **Tareas**.

La pestaña se muestra con el permiso **Ver Tareas del Evento Electoral** y **Leer la Ejecución de
Tareas**. Está oculta cuando el proceso electoral está confinado.

Cuando inicia una acción larga, también se muestra un panel de tarea en la parte inferior de la
pantalla. Muestra "Tarea: ..." con el nombre de la tarea, sus registros, **Ver Tarea** y, cuando
hay un archivo, **Descargar Archivo**.

## Barra de herramientas {#toolbar}

| Botón | Uso | Permiso |
| --- | --- | --- |
| **Columnas** | Selecciona las columnas. | **Columnas de las Tareas del Evento Electoral** |
| **Añadir filtro** | Filtra por **Índice**, **Tipo** o **Estado**. | **Filtros de las Tareas del Evento Electoral** |
| **Exportar** | Exporta la lista de tareas. | **Exportar Tareas** |

## Columnas {#columns}

| Columna | Contenido |
| --- | --- |
| **Nombre de la tarea** | El nombre de la tarea, por ejemplo **Importar usuarios**. |
| **Hora de inicio** | La fecha y la hora del inicio. |
| **Estado** | `STARTED`, `IN_PROGRESS`, `SUCCESS`, `FAILED` o `CANCELLED`. |
| **Acciones** | El icono de ver. Abre los detalles de la tarea. |

## Detalles de la tarea {#task-details}

| Elemento | Contenido |
| --- | --- |
| **Información de la tarea** | El **Tipo**, el **Ejecutor**, la **Hora de inicio** y la **Hora de finalización**. |
| Registros | Los mensajes de la tarea. Léalos cuando el estado sea `FAILED`. |
| **Atrás** | Vuelve a la lista. Necesita el permiso **Volver a las Tareas del Evento Electoral**. |
| **Descargar Archivo** | Descarga el archivo de la tarea. Solo está disponible cuando el estado es `SUCCESS`. |

## Tipos de tarea {#task-types}

| Tarea | Iniciada por |
| --- | --- |
| **Crear Evento Electoral** | La creación de un proceso electoral. |
| **Importar evento electoral** | **Importar un Evento Electoral** en el menú. |
| **Exportar evento electoral** | **Exportar** en la pestaña **Datos**. |
| **Importar usuarios** | **Importar** en la pestaña **Votantes**. |
| **Exportar votantes** | **Exportar** en la pestaña **Votantes**. |
| **Importar candidatos** | **Importar Candidatos** en la pestaña **Datos**. |
| **Exportar Publicación de Boleta** | **Exportar** en la página **Cambios a Publicar**. |
| **Generar Reporte** | **Generar** en la pestaña **Informe**. |
| **Exportar Informe de Registros de Actividad** | **Exportar** en la pestaña **Logs**. |
| **Exportar Solicitudes** | **Exportar** en la pestaña **Aprobaciones**. |
| **Renderizar el documento PDF** | La opción PDF de un documento de resultados HTML. |

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| Una tarea muestra `FAILED`. | Abra la tarea y lea los registros. Corrija la causa y vuelva a iniciar la acción. |
| Una tarea se queda en `STARTED` o `IN_PROGRESS` durante mucho tiempo. | Las importaciones y exportaciones grandes pueden tardar. Si nada cambia después de mucho tiempo, contacte con el soporte de Sequent. |
| **Descargar Archivo** no está disponible. | La tarea no está completada o no crea ningún archivo. |
| "Error al exportar la Ejecución de Tareas" | Intente la exportación de nuevo. |

## Páginas relacionadas {#related-pages}

- [Votantes](../Voters/election_management_election-event_voters.md)
- [Datos](../Data/election_management_election-event_data.md)
- [Informes](../Reports/election_management_election-event_reports.md)
- [Registros](../Logs/election_management_election-event_logs.md)
