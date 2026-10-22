---
id: election_management_contest_data
title: Datos
description: "La pestaña Datos de una pregunta contiene la configuración de una pregunta: sus textos, su sistema de votación, el número de selecciones y las políticas de la papeleta."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

La pestaña **Datos** de una pregunta contiene la configuración de una pregunta: sus textos, su
sistema de votación, el número de selecciones y las políticas de la papeleta.

## Abrir la pestaña {#open-the-tab}

1. En el menú de la izquierda, abra el proceso electoral y la elección.
2. Haga clic en la pregunta.
3. Haga clic en la pestaña **Datos**.

Para guardar cambios, su rol necesita el permiso **Editar Concurso**.

Para crear una pregunta, haga clic en la flecha junto a la elección en el menú para mostrar sus
preguntas. Después haga clic en **Crear un Concurso**. El formulario tiene **Nombre** y
**Descripción**. Una pregunta nueva tiene estos valores: **Min votes** (mínimo de votos) 0,
**Max votes** (máximo de votos) 1, **Winning candidates num** (número de candidatos ganadores) 1,
**Sin Preferencia** y **Mayoría Plural**.

## Secciones {#sections}

| Sección | Campos | Uso |
| --- | --- | --- |
| **General** | Una pestaña para cada idioma con **Nombre**, **Alias** y **Descripción** | Los textos de la pregunta. |
| **Sistema de votación de papeletas** | **Voting type** (tipo de votación), **Counting algorithm** (algoritmo de recuento) | La versión 9.0 solo tiene **Sin Preferencia** y **Mayoría Plural**. |
| **Diseño de la papeleta** | Consulte la tabla de abajo | El número de selecciones y el aspecto de la pregunta. |
| **Políticas** | Consulte la tabla de abajo | Lo que hace el portal de votación con las selecciones del votante. |
| **Imagen** | Un archivo de imagen | La imagen de la pregunta. |
| **Configuración Avanzada** | Un archivo JSON | Ajustes avanzados de la pregunta. Cámbielo solo con la ayuda del soporte de Sequent. |

### Diseño de la papeleta {#ballot-design}

| Campo | Uso |
| --- | --- |
| **Is acclaimed** (proclamada sin votación) | Marca la pregunta como decidida sin votación. |
| **Permitir candidaturas manuales** | Permite candidatos que escribe el votante. |
| **Min votes** | El número mínimo de selecciones. Escriba `0` si el votante puede dejar la pregunta en blanco. |
| **Max votes** | El número máximo de selecciones. |
| **Presentation columns** (columnas) | El número de columnas de candidatos en la papeleta. |
| **Winning candidates num** | El número de candidatos ganadores. |
| **Presentation candidates order** (orden de los candidatos) | **Aleatorias**, **Personalizado** o **Alfabético**. Con **Personalizado**, use **Reordernar candidatos**. |
| **Presentation enable checkable lists** (listas seleccionables) | **Candidatos Y Listas**, **Sólo Candidatos**, **Sólo Listas** o **Deshabilitado**. Para preguntas con listas de candidatos. |
| **Presentation max selections per type** (máximo de selecciones por tipo) | El número máximo de selecciones para cada tipo de candidato. |

:::caution PRECAUCIÓN
El portal de administración no comprueba que **Min votes** sea menor que **Max votes**. Compruebe
los valores antes de guardar. Asegúrese de que **Max votes** no es mayor que el número de
candidatos.
:::

### Políticas {#policies}

| Campo | Valores |
| --- | --- |
| **Política de Subvoto** | **Permitido**, **Advertir en Revisión**, **Advertir**, **Advertir y Alertar** |
| **Política de Voto Inválido** | **Permitido**, **Advertencia**, **Advertir Inválidos Implícitos y Explícitos**, **No Permitido** |
| **Política de voto en blanco** | **Permitido**, **Advertir en Revisión**, **Advertir**, **No permitido** |
| **Política de voto excesivo** | **Permitido**, **Permitido con un mensaje de warning**, **Permitido con un mensaje de warning y alerta**, **No permitido con un mensaje de warning y alerta**, **No permitido con un mensaje de warning y dehabilitar selecciones adicionales** |
| **Forma del icono de la casilla de verificación de los candidatos** | **Casilla de verificación cuadrada**, **Casilla de verificación redonda** |
| **Nombre de la página** | El nombre de la página de la pregunta en el portal de votación. |

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| "Error creando pregunta" al crear una pregunta | Compruebe el **Nombre** y guarde de nuevo. |
| El botón **Guardar** no está. | Su rol no tiene el permiso **Editar Concurso**. |
| Los votantes pueden seleccionar demasiados candidatos. | Compruebe **Max votes** y **Política de voto excesivo**. Después publique de nuevo. |

## Páginas relacionadas {#related-pages}

- [Crear el proceso electoral](../../../../../procedures/02-event.md#4-create-the-contests)
- [Hojas de recuento](../Tally-Sheets/election_management_contest_tally-sheets.md)
- [Candidato: Datos](../../Candidate/Data/election_management_candidate_data.md)
- [Proceso electoral: Áreas](../../Election-Event/Areas/election_management_election-event_areas.md)
