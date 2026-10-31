---
id: admin_portal_tutorials_create-election
title: Crear una elección
description: "Este tutorial le guía en la creación de un nuevo proceso electoral, en la configuración de elecciones concretas y en la adición de preguntas y candidatos dentro del portal de administración de Sequent."
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

import GoogleVideo from '@site/src/components/GoogleVideo';

<GoogleVideo id="1-LhcFt6-RG6X0wvkO5EvOJzgT2hL2jpK" />

Este tutorial le guía en la creación de un nuevo proceso electoral, en la configuración de elecciones concretas y en la adición de preguntas y candidatos dentro del portal de administración de Sequent.

## Paso 1: Iniciar la creación del proceso electoral {#step-1-initiate-election-event-creation}

Puede empezar desde el menú lateral del portal de administración de Sequent de estas formas:

![Crear un proceso electoral](../../../../../../docs/02-election_managers/01-tutorials/assets/elections_create_election_event.png)

* **Menú Procesos Electorales**: Haga clic en el icono **+** junto a **Procesos Electorales** y, después, haga clic en **Crear un Evento Electoral**.
* **Árbol de procesos electorales**: Haga clic en **Crear un Evento Electoral** al final del árbol de procesos electorales y, después, haga clic en **Crear un Evento Electoral** en el menú que se abre.
* **Icono de tres puntos**: Haga clic en el icono de tres puntos junto a un proceso electoral y, después, haga clic en **Crear un Evento Electoral**.

## Paso 2: Configurar los datos del proceso electoral {#step-2-configure-event-details}

Después de iniciar el proceso, el sistema le pedirá la configuración básica del proceso electoral.

![Dar nombre al proceso electoral](../../../../../../docs/02-election_managers/01-tutorials/assets/elections_name_desc.png)

1.  Escriba el **Nombre** del proceso electoral.
2.  Escriba una **Descripción**.
3.  Haga clic en el botón de guardar.

:::info
**Opciones posteriores a la creación**: Cuando se crea el proceso electoral, verá distintas opciones de menú a nivel de proceso electoral, por ejemplo **Panel de Control**, **Datos**, **Localización** y **Votantes**. Para ver la lista completa, consulte [Crear el proceso electoral](../03-procedures/02-event.md#1-create-the-election-event).
:::

## Paso 3: Establecer los datos y las preferencias del proceso electoral {#step-3-set-event-data-and-preferences}

En la pestaña **Datos**, puede gestionar las propiedades principales del proceso electoral.

![Pestaña Datos del proceso electoral](../../../../../../docs/02-election_managers/01-tutorials/assets/elections_data.png)

* **General**: Defina el nombre, el alias y la descripción en cada idioma, por ejemplo inglés y español.
* **Idiomas**: Configure los idiomas y el idioma predeterminado del proceso electoral.
* **Diseño de la Papeleta**: Personalice las opciones visuales de la papeleta digital.
* **Configuración Avanzada**: Gestione políticas específicas, como la **Voting Portal Countdown Policy** o la **Política de cifrado de concurso**.

## Paso 4: Crear una elección específica {#step-4-create-a-specific-election}

Un proceso electoral actúa como contenedor que puede tener una o más elecciones concretas.

![Crear una elección](../../../../../../docs/02-election_managers/01-tutorials/assets/elections_create_election.png)

1.  En el menú lateral, debajo de su nuevo proceso electoral, haga clic en **Crear una Elección**.
2.  Escriba el **Nombre**, el **ID externo** y la **Descripción** de la elección. **Nombre** e **ID externo** son obligatorios.
3.  Haga clic en el botón de guardar.
4.  (Opcional) Continúe personalizando su elección con las distintas configuraciones disponibles en la pestaña **Datos**.

:::tip
**Paneles a nivel de elección**: Cada elección tiene su propio panel para seguir métricas como los electores elegibles, los votantes reales y las tendencias de votación por día o por canal.
:::

## Paso 5: Añadir preguntas y candidatos {#step-5-add-contests-and-candidates}

Por último, defina la estructura de su papeleta añadiendo preguntas y sus candidatos respectivos.

![Crear una pregunta](../../../../../../docs/02-election_managers/01-tutorials/assets/elections_create_contest.png)

1.  **Crear una Pregunta**: Haga clic en **Crear una Pregunta** debajo de la elección. Escriba el **Nombre** (por ejemplo, "Pregunta 1") y haga clic en el botón de guardar.

![Crear un candidato](../../../../../../docs/02-election_managers/01-tutorials/assets/elections_create_candidate.png)

2.  **Añadir candidatos**: Debajo de la pregunta recién creada, haga clic en **Crear un Candidato**.
3.  Escriba el **Nombre** y los demás datos, y haga clic en el botón de guardar.
4.  (Opcional) Personalice su pregunta con las distintas configuraciones disponibles en la pestaña **Datos**. Consulte [Preguntas y candidatos](05-admin_portal_tutorials_contests-candidates.md).
