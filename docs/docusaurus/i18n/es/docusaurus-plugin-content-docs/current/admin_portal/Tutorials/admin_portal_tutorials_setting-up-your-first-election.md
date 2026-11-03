---
id: admin_portal_tutorials_setting-up-your-first-election
title: Configurar su primera elección
description: "Este tutorial realiza una pequeña elección de prueba desde el inicio hasta los resultados. Úselo para aprender el portal de administración y para comprobar su inquilino antes de una elección real."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

Este tutorial realiza una pequeña elección de prueba desde el inicio hasta los resultados.
Úselo para aprender el portal de administración y para comprobar su inquilino antes de una
elección real. Cada parte enlaza con el procedimiento que tiene los pasos completos, y le
indica qué comprobar en la prueba.

## La elección de prueba {#the-test-election}

| Elemento | Valor |
| --- | --- |
| Proceso electoral | "Test Election Event" |
| Elección | "Election 1", **ID externo** `election-1` |
| Pregunta | "Contest 1", **Min votes** (mínimo de votos) 0, **Max votes** (máximo de votos) 1, **Winning candidates num** (número de candidatos ganadores) 1 |
| Candidatos | "Candidate A", "Candidate B" |
| Área | "Area North", con "Contest 1" |
| Votantes | Dos votantes de prueba en "Area North", con contraseña |
| Fideicomisarios | Dos fideicomisarios, umbral 2 |

Use solo cuentas de prueba. No use los datos de votantes reales.

## 1. Comprobar el inquilino {#1-check-the-tenant}

Haga [Configurar el inquilino](../procedures/01-tenant.md) si no está hecho.

Compruebe que:

- **Configuración** > **IDIOMAS** tiene los idiomas de la prueba.
- **Configuración** > **AUTORIDADES** tiene dos o más fideicomisarios.
- Existen dos cuentas de fideicomisario, cada una con su propio **Actuar como fideicomisario**.

## 2. Crear el proceso electoral {#2-create-the-election-event}

Haga [Crear el proceso electoral](../procedures/02-event.md) con los valores de la tabla.

Compruebe que:

- El menú muestra el árbol: "Test Election Event" > "Election 1" > "Contest 1" > "Candidate A"
  y "Candidate B".
- En la pestaña **Áreas**, "Area North" tiene "Contest 1" en **Preguntas del Área**.
- En la pestaña **Datos** de "Election 1", **Permitir Recuento** es **Permitir cuando termine el
  periodo de votación**. Use este valor en la prueba, para ver cómo el recuento espera al fin del
  período de votación.

## 3. Añadir los votantes {#3-add-the-voters}

Haga [Añadir los votantes](../procedures/03-voters.md). Para dos votantes, use **Añadir** en la
pestaña **Votantes** en lugar de una importación.

Compruebe que:

- **Electores** en la pestaña **Panel de Control** es 2.
- La columna **Área** de los dos votantes muestra "Area North".

## 4. Realizar la ceremonia de claves {#4-run-the-key-ceremony}

Haga [Realizar la ceremonia de claves](../procedures/04-keys.md) con umbral 2 y los dos
fideicomisarios. Deje **Election** (elección) vacío, para que la ceremonia sea para **Todas las
Elecciones**.

Compruebe que:

- Cada fideicomisario tiene dos copias del archivo de clave.
- El estado de la ceremonia es `SUCCESS`.
- La pestaña **Panel de Control** marca la etapa **Claves**.

## 5. Publicar y votar {#5-publish-and-vote}

Haga [Publicar y gestionar el período de votación](../procedures/05-publish.md):

1. Genere la publicación, vea la vista previa de la papeleta de "Area North" y publíquela.
2. Haga clic en **Comenzar votación**.
3. Abra **URL de inicio de sesión de votantes** desde la pestaña **Panel de Control**. Inicie
   sesión como cada votante de prueba y vote.
4. Haga clic en **Detener votación**.

Compruebe que:

- **Votantes** en la pestaña **Panel de Control** de "Election 1" es 2.
- La columna **Voted** (votó) de la pestaña **Votantes** muestra los dos votantes.

## 6. Realizar el recuento y obtener los resultados {#6-run-the-tally-and-get-the-results}

Haga [Realizar la ceremonia de recuento](../procedures/06-tally.md) y después
[Obtener los resultados](../procedures/07-results.md).

Compruebe que:

- **Total de Votos Contados** es 2.
- Los votos de "Candidate A" y "Candidate B" son los votos que emitió.
- Puede descargar los resultados en **PDF**.

## 7. Limpiar {#7-clean-up}

Cuando la prueba termine, archive el proceso electoral: haga clic en los tres puntos junto a él
en el menú y después haga clic en **Archivar este Evento Electoral**. El proceso electoral pasa
a **Archivados** en el menú.

:::note
Un proceso electoral de prueba usa las mismas reglas de ceremonia de claves que uno real. Cree un
proceso electoral nuevo para la elección real. No reutilice el proceso electoral de prueba.
:::

## Páginas relacionadas {#related-pages}

- [Antes de empezar](../00-before-you-start.md)
- [Gestión electoral](../election_management.md)
- [Navegación básica](../Reference/basic_navigation.md)
