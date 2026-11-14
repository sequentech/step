---
title: Publicar y gestionar el período de votación
sidebar_position: 5
description: "Este procedimiento pone la papeleta a disposición en el portal de votación y después abre, pausa y cierra el período de votación. El administrador electoral hace este procedimiento."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Publicar y gestionar el período de votación

Este procedimiento pone la papeleta a disposición en el portal de votación y después abre, pausa
y cierra el período de votación. El administrador electoral hace este procedimiento.

## Antes de empezar {#before-you-start}

- El contenido del proceso electoral está completo y es correcto. Consulte [Crear el proceso electoral](02-event.md).
- La lista de votantes está completa. Consulte [Añadir los votantes](03-voters.md).
- El estado de la ceremonia de claves es `SUCCESS`. Consulte [Realizar la ceremonia de claves](04-keys.md).
- Su cuenta tiene los permisos para publicar y para cambiar el estado de la votación.
- Usted conoce su contraseña. La publicación y el estado de la votación son acciones sensibles.
  Consulte [Antes de empezar](../00-before-you-start.md#sensitive-actions).

:::note NOTA
Puede publicar antes de que el estado de la ceremonia de claves sea `SUCCESS`. Entonces el portal de
votación funciona en modo demo: los revisores pueden comprobar cómo se ve el proceso electoral antes
de hacerlo definitivo, pero los votos no son reales. Después de la ceremonia de claves, cree y
publique una nueva publicación. Inicie el periodo de votación solo cuando el estado de la ceremonia
de claves sea `SUCCESS` y haya publicado de nuevo.
:::

## 1. Crear una publicación {#1-make-a-publication}

1. Abra el proceso electoral.
2. Haga clic en la pestaña **Publicar**.
3. Haga clic en **Generar Publicación**. Si el proceso electoral ya tiene una publicación, haga
   clic en **Publicar Cambios**.
4. Si se abre una ventana **Confirmar Acción**, lea el mensaje y haga clic en **Confirmar**.
5. Si el portal de administración le pide su contraseña, escríbala.

**Resultado esperado:** aparece el mensaje "Papeleta generada". La página **Cambios a Publicar**
compara la publicación **Actual** con los **Cambios a publicar**.

6. Lea los cambios. Asegúrese de que son los cambios que espera.

## 2. Previsualizar la papeleta {#2-preview-the-ballot}

1. En la página **Cambios a Publicar**, haga clic en **Vista previa**.
2. En **Selecciona el área para la vista previa**, seleccione un área.
3. Haga clic en **Vista previa**. Se abre la papeleta del área. Para enviar la vista previa a otra
   persona, haga clic en **Copiar enlace**.
4. Compruebe la papeleta del área: las elecciones, las preguntas, los candidatos, los textos en
   cada idioma y el número de selecciones.
5. Haga los pasos 2 a 4 para cada área.

Si encuentra un error, corríjalo en el proceso electoral. Después empiece de nuevo en el paso 1 de
[Crear una publicación](#1-make-a-publication).

## 3. Publicar la papeleta {#3-publish-the-ballot}

1. En la página **Cambios a Publicar**, haga clic en **Publicar Cambios**. El portal de
   administración publica inmediatamente, sin ventana de confirmación.

**Resultado esperado:** aparece el mensaje "Papeleta publicada". La lista **Historico de Cambios**
muestra la nueva publicación.

:::note
Para cambiar la papeleta después de publicarla, cambie el proceso electoral y haga este
procedimiento de nuevo desde el paso 1. Los votantes ven la última publicación.
:::

## 4. Crear el informe de inicialización (si es necesario) {#4-make-the-initialization-report-if-necessary}

Haga este paso solo si **Inicializar Política de Reportes** de una elección es **Requerido**. El
informe de inicialización muestra que la urna está vacía antes de que se abra el período de
votación. **Comenzar votación** no está disponible hasta que el informe existe.

1. Abra el proceso electoral.
2. Haga clic en la pestaña **Recuento**.
3. Haga clic en **Crear Informe de Inicialización**.
4. Seleccione las elecciones en **Elecciones para el informe de inicialización**.
5. Haga clic en **Iniciar informe de inicialización**.
6. Continúe como en una ceremonia de recuento. Consulte [Realizar la ceremonia de recuento](06-tally.md).

**Resultado esperado:** la lista **Recuento** muestra una fila con el **Tipo de Conteo**
**Resultados de Inicialización**.

## 5. Abrir el período de votación {#5-open-the-voting-period}

Haga este paso a la hora de inicio del período de votación. Para abrir el período de votación
automáticamente, use [eventos programados](#7-schedule-the-voting-period-alternative).

1. Abra el proceso electoral.
2. Haga clic en la pestaña **Publicar**.
3. Haga clic en **Comenzar votación**.
4. En la ventana **Confirmar Acción**, lea el mensaje y haga clic en **Confirmar**.
5. Si el portal de administración le pide su contraseña, escríbala. El portal de administración
   inicia entonces el período de votación.

**Resultado esperado:** aparece el mensaje "Votación cambiada de estado". Los votantes pueden
votar en el portal de votación por los canales de votación del proceso electoral.

6. Inicie sesión en el portal de votación con una cuenta de votante de prueba, si tiene una, y
   asegúrese de que la papeleta aparece.

## 6. Pausar o cerrar el período de votación {#6-pause-or-close-the-voting-period}

Para detener la votación durante un tiempo corto:

1. En la pestaña **Publicar**, haga clic en **Pausar votación**.
2. Confirme la acción.
3. Para continuar, haga clic en **Comenzar votación** y confirme la acción.

Para cerrar el período de votación al final:

:::danger ADVERTENCIA
En la versión 9.0 no puede abrir de nuevo el período de votación desde el portal de administración
después de cerrarlo. Cierre el período de votación solo en su final oficial.
:::

1. En la pestaña **Publicar**, haga clic en **Detener votación**. Esto cierra el canal **En
   línea**.
2. Confirme la acción.
3. Si el proceso electoral usa quioscos, haga clic en **Cerrar Quiosco** y confirme la acción.

**Resultado esperado:** aparece el mensaje "Votación cambiada de estado". Los votantes no pueden
votar.

:::note
Después de hacer clic en **Detener votación**, **Publicar Cambios** no está disponible. Haga todos
los cambios de la papeleta antes del final del período de votación.
:::

## 7. Programar el período de votación (alternativa) {#7-schedule-the-voting-period-alternative}

Use eventos programados para abrir y cerrar el período de votación a una hora fijada.

1. Abra el proceso electoral.
2. Haga clic en la pestaña **Eventos Programados**.
3. Haga clic en **Añadir**. Si la lista está vacía, haga clic en **Crear Evento Programado**.
4. En **Tipo**, seleccione **Inicio del Período de Votación**.
5. En **Elección**, seleccione una elección. Déjelo vacío para todas las elecciones.
6. Escriba la fecha y la hora. El portal de administración usa la zona horaria de su ordenador. La
   etiqueta del campo muestra la zona horaria.
7. Haga clic en **Guardar**.
8. Haga de nuevo los pasos 3 a 7 con el tipo **Fin del Período de Votación**.

**Resultado esperado:** la pestaña **Eventos Programados** muestra los dos eventos.

:::caution PRECAUCIÓN
Compruebe la zona horaria en la etiqueta del campo de fecha. Si su ordenador no está en la zona
horaria de la elección, convierta las horas.
:::

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| "Election event is locked down" (el proceso electoral está confinado) | Defina **Estado de Confinamiento** como **No Confinado** en la pestaña **Datos**. Consulte [Crear el proceso electoral](02-event.md). |
| "Ballot publication not generated yet, can't publish." (la publicación todavía no se ha generado) | Cree primero una publicación. Consulte el [paso 1](#1-make-a-publication). |
| "Insufficient privileges" (privilegios insuficientes) | Confirme de nuevo su identidad con su contraseña y después repita la acción. |
| **Comenzar votación** no está disponible. | Cree el informe de inicialización, si la elección lo necesita. Si el período de votación está cerrado, contacte con el soporte de Sequent. |
| **Detener votación** no está disponible. | Un evento programado todavía no permite el final del período de votación. Mire la pestaña **Eventos Programados**. |
| Los votantes no ven una pregunta. | Asegúrese de que la pregunta está en el área del votante. Después cree y publique una nueva publicación. |
| **Publicar Cambios** no está disponible. | El período de votación está cerrado. La versión 9.0 no permite una nueva publicación después de **Detener votación**. |
| "Error al cambiar el estado de la publicación" | El estado de la votación no ha cambiado. Compruebe la pestaña **Eventos Programados** y el informe de inicialización, y después inténtelo de nuevo. |

## Más información {#more-information}

- [Proceso electoral: Publicar](../Reference/User-Manual/Election-Management/Election-Event/Publish/election_management_election-event_publish.md)
- [Elección: Publicar](../Reference/User-Manual/Election-Management/Election/Publish/election_management_election_publish.md)
- [Proceso electoral: Eventos programados](../Reference/User-Manual/Election-Management/Election-Event/Scheduled-Events/election_management_election-event_scheduled-events.md)
- [Proceso electoral: Panel de control](../Reference/User-Manual/Election-Management/Election-Event/Dashboard/election_management_election-event_dashboard.md)
- [Proceso electoral: Monitoreo](../Reference/User-Manual/Election-Management/Election-Event/Monitoring/election_management_election-event_monitoring.md)
- [Configurar una elección automatizada](../Tutorials/admin_portal_tutorials_setting-up-an-automated-election.md)

**Siguiente:** [Realizar la ceremonia de recuento](06-tally.md).
