---
id: election_management_election_event_publish
title: Publicar
description: "La pestaña Publicar pone la papeleta a disposición en el portal de votación y controla el período de votación de todo el proceso electoral. Cada publicación es una versión de la papeleta."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

La pestaña **Publicar** pone la papeleta a disposición en el portal de votación y controla el
período de votación de todo el proceso electoral. Cada publicación es una versión de la papeleta.
Los votantes ven la última versión publicada. Para el procedimiento completo, consulte
[Publicar y gestionar el período de votación](../../../../../procedures/05-publish.md).

## Abrir la pestaña {#open-the-tab}

1. En el menú de la izquierda, haga clic en el proceso electoral.
2. Haga clic en la pestaña **Publicar**.

La pestaña se muestra con el permiso **Ver Publicación del Evento Electoral**. Está oculta cuando
el proceso electoral está confinado.

## Barra de herramientas {#toolbar}

| Botón | Uso | Permiso |
| --- | --- | --- |
| **Comenzar votación** | Abre el período de votación en los canales de votación del proceso electoral. | **Editar Estado de Elección** e **Iniciar Votación** |
| **Pausar votación** | Detiene la votación durante un tiempo. | **Editar Estado de Elección** y **Pausar Votación** |
| **Detener votación** | Cierra el canal **En línea**. | **Editar Estado de Elección** y **Detener Votación** |
| **Cerrar Quiosco** | Cierra el canal **Kiosco**. Solo se muestra si el canal **Kiosco** está activado. | **Editar Estado de Elección** |
| **Publicar Cambios** | Crea una nueva publicación y abre **Cambios a Publicar**. | **Editar Publicación** y **Publicar Cambios** |

Cuando no hay ninguna publicación, la pestaña muestra "Ninguna Publicación aún." y **Generar
Publicación**.

### Cuándo están disponibles los botones {#when-the-buttons-are-available}

| Botón | No está disponible cuando |
| --- | --- |
| **Comenzar votación** | El período de votación está abierto o cerrado. El informe de inicialización es obligatorio y todavía no existe. |
| **Pausar votación** | El período de votación no está abierto. |
| **Detener votación** | El período de votación no está abierto ni en pausa. Un evento programado todavía no permite el fin. |
| **Cerrar Quiosco** | La votación en quiosco no está abierta ni en pausa. |
| **Publicar Cambios** | El período de votación está cerrado. |

:::danger ADVERTENCIA
En la versión 9.0 no puede abrir de nuevo el período de votación después de **Detener votación**.
Tampoco puede publicar cambios después.
:::

## Historial de publicaciones {#publish-history}

La lista **Historico de Cambios** muestra las publicaciones. Cada fila tiene un icono de ver y un
icono de vista previa. El icono de ver abre **Ver Publicación**, que compara la **Publicación
Anterior** con la **Publicación**.

## Cambios a publicar {#changes-to-be-published}

Esta página se abre después de generar una publicación.

| Elemento | Uso |
| --- | --- |
| **Actual** y **Cambios a publicar** | Las diferencias entre la papeleta publicada y la nueva. |
| **regenerar** | Crea la publicación de nuevo a partir de los datos actuales. |
| **Exportar** | Exporta la publicación. La tarea "Exportar Publicación de Boleta" crea el archivo. |
| **Atrás** | Vuelve a la lista. |
| **Vista previa** | Abre la ventana de vista previa. Seleccione el área en **Selecciona el área para la vista previa** y haga clic en **Vista previa** o en **Copiar enlace**. |
| **Publicar Cambios** | Publica la papeleta nueva. Se muestra el mensaje "Papeleta publicada". |

## Acciones sensibles {#sensitive-actions}

La generación, la publicación y los cambios del estado de la votación piden confirmación. Se
muestra la ventana **Confirmar Acción**, por ejemplo con "Está a punto de comenzar el período de
votación. ¿Está seguro de que desea continuar?". Si su sesión necesita una nueva comprobación, el
portal de administración le pide su contraseña. Consulte
[Antes de empezar](../../../../../00-before-you-start.md#sensitive-actions).

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| "Election event is locked down" | Ponga **Estado de Confinamiento** en **No Confinado** en la pestaña **Datos**. |
| "Insufficient privileges" | Confirme su identidad con su contraseña y repita la acción. |
| "Ballot publication not generated yet, can't publish." | Genere primero una publicación. |
| "Error al cargar las papeletas publicadas" | La generación falló. Inténtelo de nuevo. Si el error continúa, contacte con el soporte de Sequent. |
| "Error al publicar la papeleta" | La publicación falló. Inténtelo de nuevo. |
| "Error al cambiar el estado de la publicación" | El estado de la votación no cambió. Compruebe la pestaña **Eventos Programados** y el informe de inicialización. |
| "Renderizar todos los cambios podría hacer que la página no responda. ¿Estás seguro de que quieres continuar?" | La lista de cambios es larga. Confirme para ver todos los cambios. |

## Páginas relacionadas {#related-pages}

- [Publicar y gestionar el período de votación](../../../../../procedures/05-publish.md)
- [Elección: Publicar](../../Election/Publish/election_management_election_publish.md)
- [Eventos programados](../Scheduled-Events/election_management_election-event_scheduled-events.md)
- [Recuento](../Tally/election_management_election-event_tally.md)
