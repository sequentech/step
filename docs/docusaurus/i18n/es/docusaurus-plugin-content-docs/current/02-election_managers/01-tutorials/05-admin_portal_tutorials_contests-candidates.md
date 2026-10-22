---
id: admin_portal_tutorials_contests-candidates
title: Preguntas y candidatos
description: "La ruta principal está en Crear el proceso electoral. Esta página da los detalles: todas las opciones de una pregunta, los algoritmos de recuento, el número de selecciones, las políticas del portal de votación y los campos de un candidato…"
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

La ruta principal está en [Crear el proceso electoral](../03-procedures/02-event.md#4-create-the-contests).
Esta página da los detalles: todas las opciones de una pregunta, los algoritmos de recuento, el número de selecciones, las políticas del portal de votación y los campos de un candidato.

## Antes de empezar {#before-you-start}

- La elección de la pregunta existe. Consulte [Crear el proceso electoral](../03-procedures/02-event.md#3-create-the-elections).
- Para crear una pregunta, su cuenta necesita los permisos para crear preguntas y elecciones. Para crear un candidato, también necesita el permiso para crear candidatos.
- Configure la pregunta y sus candidatos antes de publicar la papeleta. Después de un cambio, haga una nueva publicación. Consulte [Publicar y gestionar el período de votación](../03-procedures/05-publish.md).

## Crear una pregunta {#create-a-contest}

1. En el menú de la izquierda, haga clic en **Crear un Concurso** debajo de la elección.
2. Escriba el **Nombre**, la **Descripción** y el **ID externo**.
3. Haga clic en el botón de guardar.

**Resultado esperado:** el portal de administración abre la nueva pregunta. La pregunta aparece debajo de la elección.

Una pregunta nueva tiene estos valores. Cámbielos en la pestaña **Datos** de la pregunta.

| Valor | Predeterminado |
| --- | --- |
| Algoritmo de recuento | **Mayoría Plural** |
| **Min votes** | `0` |
| **Max votes** | `1` |
| **Winning candidates num** | `1` |
| Orden de los candidatos | **Alfabético** |
| **Permitir candidaturas manuales** | Activado |

## La pestaña Datos de una pregunta {#the-data-tab-of-a-contest}

La pestaña **Datos** tiene estas secciones: **General**, **Sistema de votación de papeletas**, **Diseño de la Papeleta**, **Imagen** y **Configuración Avanzada**. Haga clic en el botón de guardar después de un cambio.

### General {#general}

- **ID externo**: el identificador de la pregunta en sus otros sistemas.
- Una pestaña para cada idioma del proceso electoral, con **Nombre**, **Alias**, **Descripción** y **Mensaje IVR**. El **Mensaje IVR** es el texto para la votación telefónica.

### Sistema de votación de papeletas {#ballot-voting-system}

**Algoritmo de recuento** tiene dos valores en la versión 10.0:

| Valor | Cómo vota el votante | Cómo cuenta la plataforma |
| --- | --- | --- |
| **Mayoría Plural** | El votante selecciona uno o más candidatos. | Ganan los candidatos con más votos. |
| **Segunda Vuelta Instantánea** | El votante ordena a los candidatos por preferencia. | En cada ronda, la plataforma elimina al candidato con menos votos y traslada esos votos a la siguiente preferencia. |

Para **Segunda Vuelta Instantánea**, seleccione también la **Política de desempate**:

- **Aleatorias**: la plataforma rompe un empate al azar. Es el valor predeterminado.
- **Procedimiento externo**: el recuento se detiene en un empate. Una persona introduce la decisión. Consulte [Resolver un empate](16-admin_portal_tutorials_tally-ceremony.md#resolve-a-tie).

### Diseño de la Papeleta: número de selecciones {#ballot-design-number-of-selections}

| Campo | Significado |
| --- | --- |
| **Resuelto por aclamación** | La pregunta no tiene votación. Los votantes la ven, pero no pueden seleccionar nada. Cada candidato es ganador con cero votos. Consulte [Preguntas por aclamación](../02-reference/11-acclaimed-contests.md). |
| **Permitir candidaturas manuales** | El votante puede escribir el nombre de un candidato. Use esta opción con un candidato que tenga **Voto por Escrito** activado. |
| **Min votes** | El número mínimo de selecciones. Escriba `0` si el votante puede dejar la pregunta en blanco. |
| **Max votes** | Para **Mayoría Plural**, el número máximo de candidatos que puede seleccionar un votante. Para **Segunda Vuelta Instantánea**, el rango más alto que puede dar un votante, por ejemplo `5` para los rangos del 1 al 5. |
| **Presentation columns** | El número de columnas de candidatos en el portal de votación. El mínimo es `1`. |
| **Winning candidates num** | El número de candidatos que ganan la pregunta. |

:::caution PRECAUCIÓN
Configure **Resuelto por aclamación** antes de publicar la papeleta. Si lo cambia después de que los votantes hayan emitido sus papeletas, las papeletas que ya se han emitido quedan no válidas.
:::

:::caution PRECAUCIÓN
El portal de administración no comprueba que **Min votes** sea menor que **Max votes**. Compruebe los valores antes de guardar. Para **Mayoría Plural**, asegúrese de que **Max votes** no sea mayor que el número de candidatos.
:::

### Diseño de la Papeleta: orden y listas {#ballot-design-order-and-lists}

| Campo | Valores |
| --- | --- |
| **Presentation candidates order** | **Alfabético** (predeterminado), **Aleatorias** o **Personalizado**. Con **Personalizado**, aparece la sección **Reordernar candidatos**. Arrastre los candidatos hasta el orden de la papeleta. |
| **Presentation enable checkable lists** | **Candidatos Y Listas** (predeterminado), **Sólo Candidatos**, **Sólo Listas** o **Deshabilitado**. Indica lo que puede seleccionar un votante cuando los candidatos están en listas. |
| **Collapsible Lists** | **Deshabilitado** (predeterminado), **Habilitado (empieza expandido)** o **Habilitado (empieza contraído)**. |
| **Presentation max selections per type** | Opcional. El número máximo de selecciones para cada tipo de candidato. |

Cuando los candidatos tienen un **Tipo**, la sección también muestra cada tipo con su **List Name** para cada idioma y un **Sort order**.

### Diseño de la Papeleta: políticas {#ballot-design-policies}

La parte **Políticas** del **Diseño de la Papeleta** indica al portal de votación qué debe hacer cuando una papeleta no está completa o no es válida. El valor predeterminado de cada política es **Permitido**.

| Política | Cuándo se aplica | Valores |
| --- | --- | --- |
| **Política de Subvoto** | El votante selecciona menos candidatos de los posibles. | **Permitido**, **Advertir en Revisión**, **Advertir**, **Advertir y Alertar** |
| **Política de Voto Inválido** | La papeleta no es válida. | **Permitido**, **Advertencia**, **Advertir Inválidos Implícitos y Explícitos**, **No Permitido**, **Permitido con Voto Inválido Exclusivo** |
| **Política de voto en blanco** | El votante no selecciona ningún candidato. | **Permitido**, **Advertir en Revisión**, **Advertir**, **No permitido** |
| **Política de voto excesivo** | El votante selecciona demasiados candidatos. | **Permitido**, **Permitido con un mensaje de warning**, **Permitido con un mensaje de warning y alerta**, **No permitido con un mensaje de warning y alerta**, **No permitido con un mensaje de warning y dehabilitar selecciones adicionales** |

Solo para **Segunda Vuelta Instantánea** aparecen dos políticas más. Cada una tiene los valores **Mostrar advertencia y diálogo (el votante puede continuar)** y **Mostrar advertencia y diálogo (el votante no puede continuar)**:

- **Voto inválido - Política de rango duplicado**: el votante asigna el mismo rango a dos candidatos.
- **Voto inválido - Política de rangos omitidos**: el votante omite un rango.

Hay dos campos más que cambian el aspecto de la pregunta:

- **Forma del icono de la casilla de verificación de los candidatos**: **Casilla de verificación cuadrada** (predeterminado) o **Casilla de verificación redonda**.
- **Nombre de la página**: el portal de votación muestra en la misma página las preguntas que tienen el mismo nombre de página.

### Imagen y configuración avanzada {#image-and-advanced-configuration}

- **Imagen**: suba un archivo de imagen para la pregunta.
- **Configuración Avanzada**: importe un archivo de configuración para la pregunta. Úselo solo con la ayuda del soporte de Sequent.

## Crear un candidato {#create-a-candidate}

1. En el menú de la izquierda, haga clic en **Crear un Candidato** debajo de la pregunta.
2. Escriba el **Nombre**, la **Descripción** y el **ID externo**.
3. Haga clic en el botón de guardar.

**Resultado esperado:** el portal de administración abre el nuevo candidato. El candidato aparece debajo de la pregunta. Para los pasos siguientes, consulte [Crear el proceso electoral](../03-procedures/02-event.md#5-create-the-candidates).

## La pestaña Datos de un candidato {#the-data-tab-of-a-candidate}

La pestaña **Datos** tiene estas secciones: **General**, **Tipo** e **Imagen**.

### General {#general-1}

- **ID externo**: el identificador del candidato en sus otros sistemas.
- Una pestaña para cada idioma, con **Nombre**, **Alias**, **Descripción** y **Mensaje IVR**.
- **Deshabilitado**: un interruptor que marca el candidato como deshabilitado. Aparece en cada pestaña de idioma, pero es un único valor para el candidato.

### Tipo {#type}

| Campo | Significado |
| --- | --- |
| **Tipo** | Texto libre. Los candidatos con el mismo tipo están en la misma lista de la pregunta. |
| **Subtype** | Texto libre. Un subgrupo dentro del tipo. |
| **Voto Inválido** | La opción es un voto inválido explícito. El votante la selecciona para que el voto sea inválido. |
| **Voto en Blanco** | La opción es un voto en blanco explícito. |
| **Lista** | La opción es el encabezado de la lista de su **Tipo**, no una persona de la lista. |
| **Voto por Escrito** | El votante escribe un nombre en esta opción. La pregunta debe tener **Permitir candidaturas manuales** activado. |
| **Posición del Voto Inválido** | **Ninguno (Por defecto)**, **Arriba** o **Abajo**. La posición de la opción en la papeleta. |

### Imagen {#image}

Suba una foto o un logotipo del candidato.

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| **Crear un Concurso** o **Crear un Candidato** no están en el menú. | Su cuenta no tiene el permiso para crear preguntas o candidatos. Pregunte al administrador del inquilino. |
| El botón de guardar no aparece en la pestaña **Datos**. | Su cuenta no puede editar el elemento. Pregunte al administrador del inquilino. |
| Un algoritmo de recuento que necesita no aparece en la lista. | La versión 10.0 solo tiene **Mayoría Plural** y **Segunda Vuelta Instantánea**. Póngase en contacto con el soporte de Sequent. |
| No aparece la **Política de desempate**. | Seleccione **Segunda Vuelta Instantánea** en **Algoritmo de recuento**. |
| No aparece **Reordernar candidatos**. | Seleccione **Personalizado** en **Presentation candidates order**. |
| Los votantes no ven la pregunta. | Añada la pregunta al área de los votantes. Después, haga una nueva publicación. Consulte [Crear el proceso electoral](../03-procedures/02-event.md#6-create-the-areas). |

## Más información {#more-information}

- [Pregunta: Datos](../02-reference/04-contest/01-election_management_contest_data.md)
- [Candidato: Datos](../02-reference/05-candidate/01-election_management_candidate_data.md)
- [Preguntas por aclamación](../02-reference/11-acclaimed-contests.md)
- [Papeletas en blanco](../02-reference/09-blank-ballots.md)
- [Ceremonia de recuento](16-admin_portal_tutorials_tally-ceremony.md)
