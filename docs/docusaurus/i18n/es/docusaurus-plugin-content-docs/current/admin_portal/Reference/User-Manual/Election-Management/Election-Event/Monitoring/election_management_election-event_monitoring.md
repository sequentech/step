---
id: election_management_election_event_monitoring
title: Monitoreo
description: "La pestaña Monitoreo muestra el progreso del proceso electoral durante el período de votación y el recuento. Muestra recuentos y porcentajes de los votantes, las elecciones y el recuento."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

La pestaña **Monitoreo** muestra el progreso del proceso electoral durante el período de votación
y el recuento. Muestra recuentos y porcentajes de los votantes, las elecciones y el recuento. Los
números se actualizan automáticamente.

## Abrir la pestaña {#open-the-tab}

1. En el menú de la izquierda, haga clic en el proceso electoral.
2. Haga clic en la pestaña **Monitoreo**.

La pestaña solo se muestra si su rol tiene el permiso **Vista del Panel de Monitoreo de Eventos
Electorales**. Cada elemento de la pestaña también necesita su propio permiso **Leer Monitoreo
...**, por lo que su pestaña puede mostrar menos elementos que las tablas de abajo.

## Secciones {#sections}

En los elementos de las secciones **Encuestas** y **Conteo**, la palabra "Publicaciones" significa
las elecciones del proceso electoral.

### Votantes {#voters}

| Elemento | Contenido |
| --- | --- |
| **Votantes Inscritos en el Extranjero** | Los votantes que se inscribieron. |
| **Estado de Aprobación: Votantes Aprobados/Desaprobados** | Las solicitudes de inscripción que se aprobaron y se rechazaron. |
| **Votantes Aprobados/Desaprobados Manualmente** | Las solicitudes que un administrador aprobó o rechazó en la pestaña **Aprobaciones**. |
| **Votantes Aprobados/Desaprobados Automáticamente** | Las solicitudes que la plataforma aprobó o rechazó automáticamente. |
| **Votantes Autenticados** | Los votantes que iniciaron sesión, con los recuentos de **Errores de Usuario Inválido:** y **Errores de Contraseña Inválida:**. |

### Encuestas {#polls}

| Elemento | Contenido |
| --- | --- |
| **Publicaciones con Sistemas Inicializados** | Las elecciones con un informe de inicialización. |
| **Publicaciones con Votación Abierta** | Las elecciones en las que la votación en línea está abierta. |
| **Publicaciones con Votación Cerrada** | Las elecciones en las que la votación en línea y en quiosco están cerradas. |
| **Publicaciones con Votación Iniciada** | Las elecciones en las que la votación ha empezado. |
| **Participación de los Votantes** | Los votantes que votaron y su porcentaje sobre los votantes con derecho a voto. |

### Conteo {#tally}

| Elemento | Contenido |
| --- | --- |
| **Publicaciones con Conteo de Votos Activo** | Las elecciones con un recuento en curso o completado. |
| **Publicaciones con ERs Generados** | Las elecciones con un recuento completado. |
| **Publicaciones con Resultados Transmitidos** | Las elecciones con resultados transmitidos. |

### Pruebas {#testing}

| Elemento | Contenido |
| --- | --- |
| **Conteo de Votantes en Elección de Prueba** | Los votantes que votaron en una elección de prueba. |

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| La pestaña no está. | Su rol no tiene el permiso **Vista del Panel de Monitoreo de Eventos Electorales**. Pídalo al administrador del inquilino. |
| Una sección o un elemento no está. | Su rol no tiene el permiso **Leer Monitoreo ...** de ese elemento. |
| Los elementos de inscripción muestran `0`. | El proceso electoral no usa inscripción. Esto es correcto. |

## Páginas relacionadas {#related-pages}

- [Elección: Monitoreo](../../Election/Monitoring/election_management_election_monitoring.md)
- [Panel de Control](../Dashboard/election_management_election-event_dashboard.md)
- [Aprobaciones](../Approvals/election_management_election-event_approvals.md)
