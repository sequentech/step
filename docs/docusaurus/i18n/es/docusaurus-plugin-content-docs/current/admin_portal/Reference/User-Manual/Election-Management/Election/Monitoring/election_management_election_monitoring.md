---
id: election_management_election_monitoring
title: Monitoreo
description: "La pestaña Monitoreo de una elección muestra el progreso de una elección durante el período de votación. Los números se actualizan automáticamente."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

La pestaña **Monitoreo** de una elección muestra el progreso de una elección durante el período de
votación. Los números se actualizan automáticamente.

## Abrir la pestaña {#open-the-tab}

1. En el menú de la izquierda, abra el proceso electoral.
2. Haga clic en la elección.
3. Haga clic en la pestaña **Monitoreo**.

La pestaña solo se muestra si su rol tiene el permiso **Vista del Panel de Monitoreo de
Elecciones**. Cada elemento también necesita su propio permiso **Leer Monitoreo ...**.

## Secciones {#sections}

### Votantes {#voters}

| Elemento | Contenido |
| --- | --- |
| **Votantes Inscritos en el Extranjero** | Los votantes de esta elección que se inscribieron. |
| **Estado de Aprobación: Votantes Aprobados/Desaprobados** | Las solicitudes de inscripción que se aprobaron y se rechazaron. |
| **Votantes Aprobados/Desaprobados Manualmente** | Las solicitudes que un administrador aprobó o rechazó. |
| **Votantes Aprobados/Desaprobados Automáticamente** | Las solicitudes que la plataforma aprobó o rechazó automáticamente. |
| **Votantes Autenticados** | Los votantes que iniciaron sesión, con los recuentos de **Errores de Usuario Inválido:** y **Errores de Contraseña Inválida:**. |

### Encuestas {#polls}

| Elemento | Contenido |
| --- | --- |
| **Participación de los Votantes** | Los votantes que votaron en esta elección y su porcentaje sobre los votantes con derecho a voto. |

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| La pestaña no está. | Su rol no tiene el permiso **Vista del Panel de Monitoreo de Elecciones**. Pídalo al administrador del inquilino. |
| Una sección o un elemento no está. | Su rol no tiene el permiso **Leer Monitoreo ...** de ese elemento. |

## Páginas relacionadas {#related-pages}

- [Proceso electoral: Monitoreo](../../Election-Event/Monitoring/election_management_election-event_monitoring.md)
- [Elección: Panel de Control](../Dashboard/election_management_election_dashboard.md)
