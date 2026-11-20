---
id: election_management_election_publish
title: Publicar
description: "La pestaña Publicar de una elección publica la papeleta y controla el período de votación de una sola elección. Úsela cuando las elecciones de un proceso electoral se abren o se cierran en momentos distintos."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

La pestaña **Publicar** de una elección publica la papeleta y controla el período de votación de
una sola elección. Úsela cuando las elecciones de un proceso electoral se abren o se cierran en
momentos distintos. Para actuar sobre todas las elecciones a la vez, use la pestaña **Publicar**
del proceso electoral.

## Abrir la pestaña {#open-the-tab}

1. En el menú de la izquierda, abra el proceso electoral.
2. Haga clic en la elección.
3. Haga clic en la pestaña **Publicar**.

La pestaña se muestra con el permiso **Ver Publicación de la Elección**.

## Elementos de la pantalla {#screen-elements}

La pestaña tiene los mismos botones, lista y páginas que la pestaña **Publicar** del proceso
electoral. Consulte
[Proceso electoral: Publicar](../../Election-Event/Publish/election_management_election-event_publish.md).

Se aplican estas diferencias:

| Elemento | Diferencia |
| --- | --- |
| **Publicar Cambios** | Crea una publicación para esta elección. |
| **Comenzar votación**, **Pausar votación**, **Detener votación**, **Cerrar Quiosco** | Cambian el estado de la votación solo de esta elección. |
| **Comenzar votación** | No está disponible si esta elección requiere un informe de inicialización y el informe todavía no existe. |

:::danger ADVERTENCIA
En la versión 9.0 no puede abrir de nuevo el período de votación de la elección después de
**Detener votación**.
:::

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| **Comenzar votación** no está disponible. | Compruebe **Inicializar Política de Reportes** en la pestaña **Datos** de la elección. Si es **Requerido**, haga el informe de inicialización en la pestaña **Recuento** del proceso electoral. |
| **Detener votación** no está disponible. | Un evento programado todavía no permite el fin. Compruebe la pestaña **Eventos Programados** del proceso electoral. |
| "No tienes permiso para acceder a esta elección." | La elección tiene una etiqueta de permiso que su cuenta no tiene. Consulte [Etiquetas de permiso](../../../../../Tutorials/admin_portal_tutorials_permission-labels.md). |

## Páginas relacionadas {#related-pages}

- [Publicar y gestionar el período de votación](../../../../../procedures/05-publish.md)
- [Proceso electoral: Publicar](../../Election-Event/Publish/election_management_election-event_publish.md)
- [Elección: Datos](../Data/election_management_election_data.md)
