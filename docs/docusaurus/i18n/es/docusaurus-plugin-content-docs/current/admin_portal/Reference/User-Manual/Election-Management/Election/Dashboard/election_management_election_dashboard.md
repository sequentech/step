---
id: election_management_election_dashboard
title: Panel de Control
description: "La pestaña Panel de Control de una elección muestra los números principales de una elección. Los números se actualizan automáticamente."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

La pestaña **Panel de Control** de una elección muestra los números principales de una elección.
Los números se actualizan automáticamente.

## Abrir la pestaña {#open-the-tab}

1. En el menú de la izquierda, abra el proceso electoral.
2. Haga clic en la elección.
3. Haga clic en la pestaña **Panel de Control**.

La pestaña solo se muestra si su rol tiene el permiso **Ver Dashboard de la Elección**.

## Números y gráficos {#numbers-and-charts}

| Elemento | Contenido |
| --- | --- |
| **Electores** | El número de votantes que pueden votar en esta elección. |
| **Votantes** | El número de votantes que han emitido un voto en esta elección. |
| **Áreas** | El número de áreas. |
| **Emails Enviados** | El número de correos electrónicos que la plataforma envió para esta elección. |
| **SMS Enviados** | El número de mensajes de texto que la plataforma envió para esta elección. |
| **Votos por día** | Un gráfico de los votos emitidos en los últimos siete días. |
| **Votantes por canales** | Un gráfico de los votantes de cada canal. En la versión 9.0 solo el canal en línea tiene valores. |
| **IP Addresses** (direcciones IP) | Una tabla de las direcciones IP desde las que votaron los votantes en esta elección. Solo se muestra con el permiso **Ver Dirección IP de la Elección**. |

Use **Votantes** para comprobar el resultado del recuento: después del recuento, compárelo con
**Total de Votos Contados**. Consulte [Obtener los resultados](../../../../../procedures/07-results.md).

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| La pestaña no está. | Su rol no tiene el permiso **Ver Dashboard de la Elección**. Pídalo al administrador del inquilino. |
| "No tienes permiso para acceder a esta elección." | La elección tiene una etiqueta de permiso que su cuenta no tiene. Consulte [Etiquetas de permiso](../../../../../Tutorials/admin_portal_tutorials_permission-labels.md). |
| Un número muestra `-`. | La plataforma todavía no tiene datos para este número. |

## Páginas relacionadas {#related-pages}

- [Proceso electoral: Panel de Control](../../Election-Event/Dashboard/election_management_election-event_dashboard.md)
- [Elección: Monitoreo](../Monitoring/election_management_election_monitoring.md)
