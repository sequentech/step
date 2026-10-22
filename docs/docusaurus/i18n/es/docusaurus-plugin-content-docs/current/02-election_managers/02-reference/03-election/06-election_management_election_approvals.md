---
id: election_management_election_approvals
title: Aprobaciones
description: "La pestaña Aprobaciones de una elección muestra las solicitudes de inscripción de los votantes. Un administrador electoral revisa cada solicitud y, después, la aprueba o la rechaza."
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
-->

# Aprobaciones

La pestaña **Aprobaciones** de una elección muestra las solicitudes de inscripción de los votantes. Un administrador electoral revisa cada solicitud y, después, la aprueba o la rechaza. Esta pestaña funciona igual que la pestaña **Aprobaciones** del proceso electoral. Esta página solo indica las diferencias. Para las columnas, los filtros, los botones y los pasos, consulte [Aprobaciones del proceso electoral](../02-election-event/14-election_management_election-event_approvals.md).

## Abrir la pestaña {#open-the-tab}

1. En el menú de la izquierda, abra el proceso electoral.
2. Haga clic en la elección.
3. Haga clic en la pestaña **Aprobaciones**.

La pestaña solo aparece si su rol tiene permiso para ver la pestaña de aprobaciones de la elección. Para aprobar o rechazar una solicitud, su rol también debe tener permiso para cambiar las solicitudes.

## Diferencias con la pestaña del proceso electoral {#differences-from-the-election-event-tab}

| Elemento | Pestaña del proceso electoral | Pestaña de la elección |
| --- | --- | --- |
| Solicitudes en la lista | Todas las solicitudes del proceso electoral. | Si la elección tiene una **Etiqueta de permiso**, solo las solicitudes con la misma etiqueta de permiso. Si la elección no tiene etiqueta de permiso, todas las solicitudes del proceso electoral. |
| Lista de **Votantes** en la página de la solicitud | Los votantes del proceso electoral. | Los votantes de esta elección. |
| **Exportar** | Todas las solicitudes del proceso electoral. | Todas las solicitudes del proceso electoral, no solo las de la lista. |

La **Etiqueta de permiso** está en la sección **Configuración Avanzada** de la pestaña **Datos** de la elección.

## Acciones principales {#main-actions}

| Acción | Dónde |
| --- | --- |
| Aprobar una solicitud | [Aprobar una solicitud](../02-election-event/14-election_management_election-event_approvals.md#approve-an-application) |
| Rechazar una solicitud | [Rechazar una solicitud](../02-election-event/14-election_management_election-event_approvals.md#reject-an-application) |
| Exportar las solicitudes | [Exportar las solicitudes](../02-election-event/14-election_management_election-event_approvals.md#export-the-applications) |

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| No ve la pestaña **Aprobaciones**. | Pida a su administrador de inquilino el permiso para ver la pestaña de aprobaciones de la elección. |
| "No tienes permiso para acceder a esta elección." | La elección tiene una **Etiqueta de permiso** que su usuario no tiene. Pida ayuda a su administrador de inquilino. |
| Una solicitud que espera no aparece en la lista. | Cambie el filtro **Estado**. Asegúrese de que la solicitud tiene la etiqueta de permiso de la elección, o abra la pestaña **Aprobaciones** del proceso electoral. |

Para los mensajes de las acciones de aprobar, rechazar, importar y exportar, consulte [Si hay un problema](../02-election-event/14-election_management_election-event_approvals.md#if-there-is-a-problem).

## Páginas relacionadas {#related-pages}

- [Votantes de la elección](04-election_management_election_voters.md)
- [Crear el proceso electoral](../../03-procedures/02-event.md)
