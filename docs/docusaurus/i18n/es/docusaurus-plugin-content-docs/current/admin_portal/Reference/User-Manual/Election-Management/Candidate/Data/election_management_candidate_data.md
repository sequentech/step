---
id: election_management_candidate_data
title: Datos
description: "La pestaña Datos de un candidato contiene los textos, el tipo y la imagen de un candidato. Un candidato es una opción que un votante puede seleccionar en una pregunta."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

La pestaña **Datos** de un candidato contiene los textos, el tipo y la imagen de un candidato. Un
candidato es una opción que un votante puede seleccionar en una pregunta.

## Abrir la pestaña {#open-the-tab}

1. En el menú de la izquierda, abra el proceso electoral, la elección y la pregunta.
2. Haga clic en el candidato.
3. Haga clic en la pestaña **Datos**.

Para guardar cambios, su rol necesita el permiso **Editar Candidatos**.

Para crear un candidato, haga clic en la flecha junto a la pregunta en el menú para mostrar sus
candidatos. Después haga clic en **Crear un Candidato**. Escriba el **Nombre** y haga clic en
**Guardar**.

## Secciones {#sections}

| Sección | Campos | Uso |
| --- | --- | --- |
| **General** | Una pestaña para cada idioma con **Nombre**, **Alias** y **Descripción**, y **Deshabilitado** | Los textos del candidato. **Deshabilitado** marca el candidato como deshabilitado. |
| **Tipo** | **Tipo**, **Subtype** (subtipo), **Voto Inválido**, **Voto en Blanco**, **Lista**, **Voto por Escrito**, **Posición del Voto Inválido** | La clase de opción. Use estos campos para listas, opciones de voto por escrito y opciones que significan un voto inválido o en blanco. |
| **Imagen** | Un archivo de imagen | La foto o el logotipo del candidato. |

**Posición del Voto Inválido** tiene **Ninguno (Por defecto)**, **Arriba** y **Abajo**. Indica
dónde se muestra la opción de voto inválido en la papeleta.

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| "Error creating candidato" | Compruebe el **Nombre** y guarde de nuevo. |
| Un cambio no se muestra en el portal de votación. | Haga y publique una nueva publicación. Consulte [Publicar y gestionar el período de votación](../../../../../procedures/05-publish.md). |
| El botón **Guardar** no está. | Su rol no tiene el permiso **Editar Candidatos**. |

## Páginas relacionadas {#related-pages}

- [Crear el proceso electoral](../../../../../procedures/02-event.md#5-create-the-candidates)
- [Pregunta: Datos](../../Contest/Data/election_management_contest_data.md)
