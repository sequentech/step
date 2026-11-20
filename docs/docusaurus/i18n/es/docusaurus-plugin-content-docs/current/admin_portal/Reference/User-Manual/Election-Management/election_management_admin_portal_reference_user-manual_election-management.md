---
id: election_management_admin_portal_reference_user_manual_election_management
title: Gestión electoral
description: "Un proceso electoral tiene cuatro niveles: el proceso electoral, sus elecciones, sus preguntas y los candidatos. En el menú de la izquierda se muestran como un árbol bajo Procesos Electorales. Haga clic en un elemento para abrirlo."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

Un proceso electoral tiene cuatro niveles: el proceso electoral, sus elecciones, sus preguntas y
los candidatos. En el menú de la izquierda se muestran como un árbol bajo **Procesos
Electorales**. Haga clic en un elemento para abrirlo. Cada elemento tiene pestañas. Esta página
lista la página de referencia de cada pestaña.

## Proceso electoral {#election-event}

| Pestaña | Página de referencia |
| --- | --- |
| **Panel de Control** | [Panel de Control](Election-Event/Dashboard/election_management_election-event_dashboard.md) |
| **Monitoreo** | [Monitoreo](Election-Event/Monitoring/election_management_election-event_monitoring.md) |
| **Datos** | [Datos](Election-Event/Data/election_management_election-event_data.md) |
| **Localización** | [Localización](Election-Event/Localization/election_management_election-event_localization.md) |
| **Votantes** | [Votantes](Election-Event/Voters/election_management_election-event_voters.md) |
| **Áreas** | [Áreas](Election-Event/Areas/election_management_election-event_areas.md) |
| **Claves** | [Claves](Election-Event/Keys/election_management_election-event_keys.md) |
| **Recuento** | [Recuento](Election-Event/Tally/election_management_election-event_tally.md) |
| **Publicar** | [Publicar](Election-Event/Publish/election_management_election-event_publish.md) |
| **Tareas** | [Tareas](Election-Event/Tasks/election_management_election-event_tasks.md) |
| **Logs** | [Registros](Election-Event/Logs/election_management_election-event_logs.md) |
| **Eventos Programados** | [Eventos programados](Election-Event/Scheduled-Events/election_management_election-event_scheduled-events.md) |
| **Informe** | [Informes](Election-Event/Reports/election_management_election-event_reports.md) |
| **Aprobaciones** | [Aprobaciones](Election-Event/Approvals/election_management_election-event_approvals.md) |

Cuando **Estado de Confinamiento** es **Confinado**, estas pestañas están ocultas: **Datos**,
**Localización**, **Áreas**, **Claves**, **Recuento**, **Publicar**, **Tareas**, **Eventos
Programados** y **Aprobaciones**.

## Elección {#election}

| Pestaña | Página de referencia |
| --- | --- |
| **Panel de Control** | [Panel de Control](Election/Dashboard/election_management_election_dashboard.md) |
| **Monitoreo** | [Monitoreo](Election/Monitoring/election_management_election_monitoring.md) |
| **Datos** | [Datos](Election/Data/election_management_election_data.md) |
| **Votantes** | [Votantes](Election/Voters/election_management_election_voters.md) |
| **Publicar** | [Publicar](Election/Publish/election_management_election_publish.md) |
| **Aprobaciones** | [Aprobaciones](Election/Approvals/election_management_election_approvals.md) |

## Pregunta {#contest}

| Pestaña | Página de referencia |
| --- | --- |
| **Datos** | [Datos](Contest/Data/election_management_contest_data.md) |
| **Tally Sheets** | [Hojas de recuento](Contest/Tally-Sheets/election_management_contest_tally-sheets.md) |

## Candidato {#candidate}

| Pestaña | Página de referencia |
| --- | --- |
| **Datos** | [Datos](Candidate/Data/election_management_candidate_data.md) |

## Acciones en el menú {#actions-in-the-menu}

Haga clic en los tres puntos junto a un elemento del menú para ver sus acciones:

| Elemento | Acciones |
| --- | --- |
| Proceso electoral | **Crear una Elección**, **Importar un Evento Electoral**, **Archivar este Evento Electoral** o **Desarchivar este Evento Electoral**, **Eliminar este Evento Electoral** |
| Elección | **Crear un Concurso**, **Eliminar esta Elección** |
| Pregunta | **Crear un Candidato**, **Eliminar este Concurso** |
| Candidato | **Eliminar este Candidato** |

Cada acción solo se muestra si su rol tiene su permiso. El portal de administración pide una
confirmación antes de archivar o eliminar.
