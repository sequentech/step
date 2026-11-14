---
id: election_management_election_voters
title: Votantes
description: "En esta sección se explica cómo ver y asignar votantes a una elección concreta."
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
-->


# Asignación de votantes en una elección

En esta sección se explica cómo ver y asignar votantes a una elección concreta. Tenga en cuenta que los votantes se crean y gestionan a nivel del proceso electoral. Su asignación a una elección se hace de forma indirecta, mediante áreas y preguntas.

## Visión general {#overview}

- **Los votantes no se crean por elección**: los registros de votantes existen a nivel del proceso electoral.
- **La asignación es indirecta**: para incluir a un votante en una elección, asígnelo a un área vinculada con una o más preguntas de esa elección.
- **Requisitos previos**: debe existir al menos una pregunta definida en la elección antes de asignar votantes.

## Conceptos clave {#key-concepts}

1. **Votantes del proceso electoral**  
   - Esta pestaña muestra todos los votantes registrados en el proceso electoral.  
   - En esta fase, un votante puede no tener área ni asignación de pregunta.

2. **Área**  
   - Representa una división geográfica u organizativa dentro del proceso electoral.  
   - Las preguntas se vinculan a las áreas. Los votantes asignados a un área tienen acceso a las preguntas asociadas a ella (es decir, a las elecciones de ese proceso electoral).

3. **Pregunta**  
   - Una pregunta pertenece a una elección dentro del proceso electoral.  
   - Las preguntas deben existir antes de que los votantes puedan participar en ellas de forma efectiva.

4. **Flujo de asignación**  
   1. Cree o verifique la pregunta en el proceso electoral.  
   2. Cree un área (o verifique las áreas existentes) en el proceso electoral.  
   3. Vincule la pregunta al área.  
   4. Asigne el votante (desde **Votantes del proceso electoral**) al área.  
   5. Como resultado, el votante forma parte de las elecciones asociadas a esa área y pregunta.

## Pasos detallados: asignar un votante a una elección {#detailed-steps-assign-a-voter-to-an-election}

Siga estos pasos cuando haya definido las preguntas en el proceso electoral:

1. **Compruebe que la pregunta existe**  
   - En el proceso electoral, confirme que la pregunta deseada está creada en la elección correspondiente.

2. **Compruebe la asignación actual del votante**  
   - Vaya a la pestaña **Votantes** del proceso electoral.  
   - Observe que el registro del votante existe, pero no tiene área asignada y, por tanto, todavía no está incluido en ninguna elección concreta.

3. **Cree o seleccione un área**  
   - Vaya a la pestaña **Áreas** del proceso electoral.  
   - Si no existe un área adecuada, cree una nueva (por ejemplo, "Distrito A").  
   - Si ya hay áreas, elija la correspondiente.

4. **Vincule la pregunta al área**  
   - En la lista de áreas, seleccione la acción **Editar** del área elegida.  
   - En la pantalla de edición del área, asigne la pregunta o las preguntas adecuadas a esta área.  
     - Así se vinculan esas preguntas (y su elección) con el área.  
   - Guarde los cambios.

5. **Asigne el votante al área**  
   - Vuelva a la pestaña **Votantes** del proceso electoral.  
   - Localice el registro del votante que desea incluir.  
   - Seleccione **Editar** (o "Asignar") para ese votante.  
   - En la pantalla de edición del votante, establezca el campo **Área** en el área que acaba de configurar.  
   - Guarde los cambios.

6. **Resultado**  
   - El votante queda asignado al área vinculada con la pregunta y la elección.  
   - En el contexto de esta elección concreta: el votante aparecerá en la lista de votantes de la elección (a través de la asignación al área).

## Ejemplo paso a paso {#example-walkthrough}

1. **Votantes del proceso electoral**  
   - Abra el proceso electoral y vaya a **Votantes**.  
   - Observe que el votante "Alice Example" existe, pero no muestra ningún área.

2. **Cree un área**  
   - Vaya a **Áreas**. Haga clic en **Añadir área**.  
   - Nombre: "Distrito Centro". Guarde.

3. **Vincule la pregunta al área**  
   - En la lista de áreas, haga clic en **Editar** junto a "Distrito Centro".  
   - En "Preguntas asignadas", seleccione "Carrera a la alcaldía" (pregunta de ejemplo).  
   - Guarde.

4. **Asigne el votante**  
   - Vuelva a **Votantes**. Haga clic en **Editar** junto a "Alice Example".  
   - En el desplegable **Área**, elija "Distrito Centro".  
   - Guarde.

5. **Compruebe**  
   - Vaya a la elección concreta (por ejemplo, "Elección a alcalde") y vea sus votantes.  
   - "Alice Example" aparece ahora como votante elegible para esa elección.

## Notas y consejos {#notes--tips}

- Si un área está vinculada a varias preguntas o elecciones, asignar un votante a esa área lo inscribe en todas las preguntas vinculadas.  
- Para excluir a un votante de una pregunta concreta, no vincule esa pregunta al área, o asigne al votante a otra área.  
- Para gestionar varias preguntas por área, use la pantalla de edición del área para añadir o quitar preguntas según sea necesario.  
- Los cambios surten efecto de inmediato. Después de la asignación, el votante puede acceder al portal de votación de esa elección.  
- Si más tarde quita la pregunta del área o reasigna al votante a otra área, su acceso a la elección se ajusta en consecuencia.

---
