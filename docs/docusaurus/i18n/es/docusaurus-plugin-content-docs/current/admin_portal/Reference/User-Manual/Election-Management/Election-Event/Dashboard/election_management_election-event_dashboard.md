---
id: election_management_election_event_dashboard
title: Panel de Control
description: "La pestaña Panel de Control muestra la etapa del proceso electoral y sus números principales. Úsela para seguir el proceso electoral desde su creación hasta los resultados. Los números se actualizan automáticamente."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

La pestaña **Panel de Control** muestra la etapa del proceso electoral y sus números principales.
Úsela para seguir el proceso electoral desde su creación hasta los resultados. Los números se
actualizan automáticamente.

## Abrir la pestaña {#open-the-tab}

1. En el menú de la izquierda, haga clic en el proceso electoral.
2. Haga clic en la pestaña **Panel de Control**.

La pestaña solo se muestra si su rol tiene el permiso **Vista del Panel de Administración**.

## Etapas {#stages}

La parte superior de la pestaña muestra las etapas del proceso electoral. El portal de
administración marca la última etapa completada.

| Etapa | La etapa está completada cuando |
| --- | --- |
| **Creado** | El proceso electoral existe. |
| **Claves** | Una ceremonia de claves tiene el estado `SUCCESS`. |
| **Publicar** | La papeleta está publicada. |
| **Iniciado** | El período de votación está abierto o en pausa. |
| **Finalizado** | El período de votación está cerrado. |
| **Resultados** | Un recuento del tipo **Resultados Electorales** está completado. Un informe de inicialización no cuenta. |

## Números y gráficos {#numbers-and-charts}

| Elemento | Contenido |
| --- | --- |
| **Electores** | El número de votantes del proceso electoral. |
| **Elecciones** | El número de elecciones. |
| **Áreas** | El número de áreas. |
| **Emails Enviados** | El número de correos electrónicos que la plataforma envió para el proceso electoral. |
| **SMS Enviados** | El número de mensajes de texto que la plataforma envió. |
| **Votos por día** | Un gráfico de los votos emitidos en los últimos siete días. |
| **Votantes por canales** | Un gráfico de los votantes de cada canal. En la versión 9.0 solo el canal en línea tiene valores. |
| **IP Addresses** (direcciones IP) | Una tabla de las direcciones IP desde las que votaron los votantes: **IP**, **País**, **Número de votos**, **Nombre de la elección**. Solo se muestra con el permiso **Ver Dirección IP**. |

## Enlaces al portal de votación {#links-to-the-voting-portal}

La parte inferior de la pestaña tiene estos enlaces:

| Enlace | Uso |
| --- | --- |
| **URL de inicio de sesión de votantes** | La página de inicio de sesión del portal de votación para este proceso electoral. |
| **URL de inscripción de votantes** | La página de inscripción, para procesos electorales en los que los votantes se inscriben. |
| **Kiosk URL de inscripción de votantes** | La página de inscripción para quioscos. Solo se muestra si el canal de votación **Kiosco** está activado para el proceso electoral. |

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| La pestaña no está. | Su rol no tiene el permiso **Vista del Panel de Administración**. Pídalo al administrador del inquilino. |
| Un número muestra `-`. | La plataforma todavía no tiene datos para este número. |
| **Electores** no es el número que espera. | Compruebe la lista de votantes. Consulte [Añadir los votantes](../../../../../procedures/03-voters.md). |
| La etapa **Claves** no está marcada después de la ceremonia de claves. | La ceremonia de claves no está completada. Consulte [Realizar la ceremonia de claves](../../../../../procedures/04-keys.md). |

## Páginas relacionadas {#related-pages}

- [Elección: Panel de Control](../../Election/Dashboard/election_management_election_dashboard.md)
- [Monitoreo](../Monitoring/election_management_election-event_monitoring.md)
