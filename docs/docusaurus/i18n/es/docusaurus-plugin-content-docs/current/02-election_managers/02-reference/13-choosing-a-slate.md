---
id: choosing_a_slate
title: Elegir una candidatura
sidebar_position: 13
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

Cuando una elección tiene candidaturas, cada tarjeta de candidatura de la pestaña **Elegir una lista** tiene un botón **Seleccionar lista**. Seleccionar una candidatura es un atajo: selecciona los candidatos de la candidatura en cada pregunta, exactamente como si el votante los hubiera seleccionado uno por uno. No es un voto adicional. Nada de la candidatura se cifra, se emite ni se cuenta, y una papeleta rellenada al seleccionar una candidatura es idéntica a otra rellenada a mano con los mismos candidatos.

Las elecciones sin candidaturas no muestran el botón y funcionan como antes.

## Qué hace la selección de una candidatura {#what-selecting-a-slate-does}

Al seleccionar una candidatura se seleccionan todos sus candidatos, y solo ellos, en toda la papeleta:

- En cada pregunta en la que la candidatura tiene candidatos, esos candidatos pasan a ser la selección del votante.
- Cada otra pregunta de la papeleta queda sin ninguna opción. Por ejemplo, una candidatura con solo candidatos a fideicomisario selecciona sus fideicomisarios y deja en blanco Presidente, Vicepresidente y los demás cargos.
- El resultado nunca supera el número máximo de opciones de una pregunta, porque los candidatos de la candidatura sustituyen la selección actual en lugar de añadirse a ella. Una pregunta con menos opciones que su máximo es un subvoto, que está permitido. La pantalla de revisión pide al votante que lo confirme cuando la **Política de Subvoto** de la pregunta es **Advertir y Confirmar en Revisión**.
- Las preguntas resueltas por aclamación no cambian.
- La candidatura se aplica a toda la papeleta de una vez o no se aplica.

Después de seleccionar, aparece un aviso encima de las tarjetas: **_Candidatura_ seleccionada. Aún puedes cambiar cualquier candidato.** Si la candidatura deja cargos en blanco, el aviso los nombra: **_Candidatura_ seleccionada. No tiene candidato para _cargos_, así que esos cargos quedan en blanco. Aún puedes cambiar cualquier candidato.** El aviso permanece mientras la papeleta contenga exactamente esa candidatura. La tarjeta muestra entonces **Los N seleccionados** y ofrece **Editar selecciones**. El votante puede seguir cambiando cualquier opción de forma individual.

## Ver una candidatura {#viewing-a-slate}

Cada tarjeta de candidatura también tiene un botón **Ver lista**. Abre un cuadro de diálogo con el nombre de la candidatura, que indica qué cubre y cuántos candidatos tiene, por ejemplo **Candidatura completa · 7 candidatos**, y enumera sus candidatos cargo por cargo: solo los cargos para los que la candidatura tiene candidatos, en el orden de la papeleta, con los candidatos en el orden en que los muestra la papeleta.

- **Seleccionar esta lista** selecciona la candidatura exactamente como lo hace **Seleccionar lista**, incluida la confirmación siguiente cuando se quitarían opciones.
- Cuando la papeleta ya contiene exactamente esa candidatura, el botón es **Editar selecciones**, igual que en la tarjeta.
- **Volver**, el botón de cerrar y Escape cierran el cuadro de diálogo sin cambiar nada, y el foco vuelve a **Ver lista**.

## Sustituir las selecciones existentes {#replacing-existing-choices}

Si seleccionar la candidatura quitaría alguna opción que el votante ya había elegido, en una pregunta en la que la candidatura tiene candidatos o en una en la que no tiene ninguno, el portal de votación pregunta primero con **¿Reemplazar estas selecciones?**:

- Indica **Al seleccionar _Candidatura_ cambiarán los siguientes cargos.**
- Para cada cargo que pierde una opción, muestra **Selecciones actuales**, una flecha y los candidatos de la candidatura para ese cargo, o **Sin candidato** si el cargo quedará en blanco.
- Para una candidatura que no tiene candidato para algunos cargos, una nota añade **_Candidatura_ no tiene candidato para algunos cargos. Esos cargos quedarán en blanco.**
- **Reemplazar selecciones** aplica la candidatura. Cuando solo cambia un cargo, el botón lo nombra, por ejemplo **Reemplazar presidente**.
- **Mantener mis selecciones** cierra el cuadro de diálogo y deja la papeleta sin cambios.

Si seleccionar la candidatura no quita nada, porque la papeleta está vacía o solo contiene candidatos que la candidatura conserva, no se pregunta.

## Borrar las opciones {#clearing-choices}

En la pestaña **Candidaturas individuales**, cada cargo de una papeleta con candidaturas tiene un botón **Borrar** que quita las opciones de ese cargo. Está deshabilitado mientras el cargo no tiene ninguna opción, y los lectores de pantalla anuncian **Selecciones borradas para _cargo_.**

El botón **Limpiar selecciones** de la parte inferior de la pantalla de votación quita todas las opciones de la papeleta, en ambas pestañas. Antes pregunta con **¿Borrar todas tus opciones?**, que muestra el antes y el después de cada cargo que pierde una opción. **Borrar opciones** borra la papeleta y **Mantener mis opciones** la deja sin cambios. El botón está deshabilitado mientras no hay nada seleccionado. Las papeletas sin candidaturas mantienen el comportamiento habitual de **Limpiar selecciones**.

## Límites de selección {#selection-limits}

Una candidatura nunca cambia cuántos candidatos acepta una pregunta. El límite de cada pregunta es su **Máximo de votos**, y se aplica del mismo modo a los candidatos seleccionados uno por uno y a los seleccionados mediante una candidatura.

| Configuración de la pregunta | Seleccionar un candidato | Seleccionar una candidatura |
| --- | --- | --- |
| Máximo de votos 1 y `candidates_selection_policy` con el valor `radio` | Seleccionar otro candidato sustituye al actual. | El candidato de la candidatura sustituye al actual, previa confirmación del votante. |
| **Política de voto excesivo** con el valor **No permitido con un mensaje de warning y dehabilitar selecciones adicionales** | Al llegar al máximo, los candidatos restantes se deshabilitan y un mensaje indica al votante que primero debe deseleccionar uno. Una selección por encima del máximo se rechaza y se mantienen las opciones anteriores. | Los candidatos de la candidatura sustituyen a los actuales en esa pregunta. Después, los candidatos restantes se deshabilitan del mismo modo. |
| Cualquier otra **Política de voto excesivo** | El mensaje, la alerta y el botón **Siguiente** bloqueado de esa política, como en una elección sin candidaturas. | Los candidatos de la candidatura sustituyen a los actuales en esa pregunta. |

En una elección con cargos de un único puesto y una junta de tres fideicomisarios, la primera fila es la configuración habitual de cada cargo y la segunda la configuración habitual de la pregunta de fideicomisarios, con Máximo de votos 3. Así, un votante puede combinar hasta tres fideicomisarios de cualquier candidatura y de candidatos independientes, y nunca se acepta un cuarto.

Deseleccionar un candidato siempre es posible.

No se puede seleccionar una candidatura con más candidatos en una pregunta de los que la pregunta permite. Su botón está deshabilitado y una línea junto a él indica la pregunta, el número de candidatos de la candidatura en ella y el máximo. No se cambia ninguna pregunta, y el votante puede seguir seleccionando candidatos uno por uno.

## CSS personalizado {#custom-css}

| Clase | Elemento |
| --- | --- |
| `slate-apply` | Contenedor del botón **Seleccionar lista** |
| `slate-apply-button` | El botón **Seleccionar lista** |
| `slate-apply-unavailable` | La línea que indica por qué no se puede seleccionar una candidatura |
| `slate-view-button` | El botón **Ver lista** |
| `slate-details-dialog` | El cuadro de diálogo que muestra los candidatos de una candidatura |
| `slate-details-summary` | Lo que cubre la candidatura y cuántos candidatos tiene |
| `slate-details-offices`, `slate-details-office`, `slate-details-office-name`, `slate-details-candidate` | Los cargos de la candidatura, un cargo, su nombre y cada candidato |
| `slate-applied-notice` | El aviso tras seleccionar una candidatura |
| `slate-replace-dialog` | **¿Reemplazar estas selecciones?** |
| `slate-clear-dialog` | **¿Borrar todas tus opciones?** |
| `slate-clear-status` | La línea que confirma que la papeleta se ha borrado |
| `slate-changes`, `slate-replace-content` | El cuerpo de cualquiera de los dos cuadros de diálogo y su primera frase |
| `slate-replace-contest`, `slate-replace-contest-name` | Un cargo que cambia y su nombre |
| `slate-change-columns`, `slate-change-label`, `slate-change-candidate` | Las columnas de antes y después, sus encabezados y cada candidato |
| `slate-replace-removed`, `slate-replace-added` | Las selecciones actuales y lo que las sustituye |
| `slate-replace-none` | Un cargo que quedará con **Sin candidato** |
| `slate-replace-note` | La nota sobre los cargos dejados en blanco |
