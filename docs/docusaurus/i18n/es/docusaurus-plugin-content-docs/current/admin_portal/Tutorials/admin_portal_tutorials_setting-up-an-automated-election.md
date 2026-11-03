---
id: admin_portal_tutorials_setting-up-an-automated-election
title: Configurar una elección automatizada
description: "La versión 9.0 no tiene ceremonias automáticas. La ceremonia de claves y la ceremonia de recuento siempre necesitan a los fideicomisarios: cada fideicomisario descarga y comprueba un fragmento de clave, y lo vuelve a subir para el recuento."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

La versión 9.0 no tiene ceremonias automáticas. La ceremonia de claves y la ceremonia de
recuento siempre necesitan a los fideicomisarios: cada fideicomisario descarga y comprueba un
fragmento de clave, y lo vuelve a subir para el recuento. No hay ninguna opción para realizar
las ceremonias sin los fideicomisarios.

Lo que la versión 9.0 puede hacer automáticamente es el período de votación y algunos permisos,
con eventos programados. Úselos para abrir y cerrar el período de votación a horas fijadas, sin
un administrador delante de la pantalla.

## Qué puede programar {#what-you-can-schedule}

| Tipo de evento programado | Acción en la fecha y la hora |
| --- | --- |
| **Inicio del Período de Votación** | Abre el período de votación. |
| **Fin del Período de Votación** | Cierra el período de votación. |
| **Allow Voting Period End** (permitir el fin del período de votación) | Permite el fin del período de votación de una elección. |
| **Allow Initialization Report** (permitir el informe de inicialización) | Permite el informe de inicialización de una elección. |
| **Permitir Recuento** | Establece **Permitir Recuento** de una elección en **Permitido**. |
| **Inicio del Período de Inscripción** y **Fin del Período de Inscripción** | Abren y cierran la inscripción de los votantes. |
| **Inicio del Período de Bloqueo de Datos Censales** y **Fin del Período de Bloqueo de Datos Censales** | Confinan el proceso electoral y terminan el confinamiento. |

## Programar el período de votación {#schedule-the-voting-period}

Antes de empezar, la ceremonia de claves está completa y la papeleta está publicada. Si una
elección necesita un informe de inicialización, créelo antes de la hora de inicio.

1. Abra el proceso electoral.
2. Haga clic en la pestaña **Eventos Programados**.
3. Cree un evento **Inicio del Período de Votación** con la fecha y la hora de inicio.
4. Cree un evento **Fin del Período de Votación** con la fecha y la hora de fin.
5. Compruebe la zona horaria en la etiqueta del campo de fecha.

Para los pasos, consulte
[Publicar y gestionar el período de votación](../procedures/05-publish.md#7-schedule-the-voting-period-alternative).

**Resultado esperado:** el período de votación se abre y se cierra a las horas fijadas. La
pestaña **Panel de Control** muestra las etapas **Iniciado** y después **Finalizado**.

:::caution PRECAUCIÓN
En la versión 9.0 no puede volver a abrir el período de votación después de cerrarlo.
Compruebe la fecha y la hora de fin antes de guardar el evento.
:::

## Páginas relacionadas {#related-pages}

- [Eventos programados](../Reference/User-Manual/Election-Management/Election-Event/Scheduled-Events/election_management_election-event_scheduled-events.md)
- [Realizar la ceremonia de claves](../procedures/04-keys.md)
- [Realizar la ceremonia de recuento](../procedures/06-tally.md)
