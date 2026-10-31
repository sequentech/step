---
id: admin_portal_tutorials_contests-candidates
title: Crear preguntas y candidatos
description: "La ruta principal está en Crear el proceso electoral. Esta página da los detalles: todas las opciones de una pregunta y de un candidato, sus valores predeterminados y su efecto en la papeleta y en el recuento."
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

La ruta principal está en [Crear el proceso electoral](../03-procedures/02-event.md#4-create-the-contests).
Esta página da los detalles: todas las opciones de una pregunta y de un candidato, sus valores predeterminados y su efecto en la papeleta y en el recuento.

## Antes de empezar {#before-you-start}

- La elección de la pregunta existe. Consulte [Crear el proceso electoral](../03-procedures/02-event.md#3-create-the-elections).
- Su cuenta tiene los permisos para crear y editar preguntas y candidatos. Sin el permiso para editar una pregunta, la pestaña **Datos** de la pregunta no tiene botón de guardar.
- Conoce el método de recuento, el número de selecciones y el número de ganadores de cada pregunta.

:::caution PRECAUCIÓN
Configure las opciones de la pregunta antes de publicar la papeleta. Un cambio después de la publicación se aplica solo después de una nueva publicación. Un cambio en **Resuelto por aclamación** después de que los votantes hayan emitido sus papeletas invalida las papeletas que ya se han emitido.
:::

## Crear una pregunta {#create-a-contest}

1. En el menú de la izquierda, haga clic en la flecha junto a la elección para mostrar sus preguntas.
2. Debajo de las preguntas, haga clic en **Crear una Pregunta**.
3. Escriba el **Nombre**, la **Descripción** y el **ID externo**.
4. Haga clic en el botón de guardar.

**Resultado esperado:** el portal de administración abre la pestaña **Datos** de la nueva pregunta.

Una pregunta nueva tiene estos valores. Cámbielos en la pestaña **Datos**.

| Opción | Valor de una pregunta nueva |
| --- | --- |
| Algoritmo de recuento | **Mayoría Plural** |
| Número mínimo de selecciones | `0` |
| Número máximo de selecciones | `1` |
| Número de candidatos ganadores | `1` |
| Orden de los candidatos | **Alfabético** |
| **Permitir candidaturas manuales** | Activado |
| **Resuelto por aclamación** | Desactivado |

La pestaña **Datos** de una pregunta tiene las secciones **General**, **Sistema de votación de papeletas**, **Diseño de la Papeleta**, **Imagen** y **Configuración Avanzada**.

## General {#general}

| Campo | Uso |
| --- | --- |
| **ID externo** | El identificador de la pregunta en sus otros sistemas. |
| **Nombre** | El nombre de la pregunta en la papeleta. Escríbalo en la pestaña de cada idioma. |
| **Alias** | Un nombre corto. Donde el portal de administración muestra el alias, por ejemplo en los resultados, usa el nombre si no hay alias. |
| **Descripción** | La descripción de la pregunta. Escríbala en la pestaña de cada idioma. |
| **Mensaje IVR** | El texto que lee la votación telefónica para la pregunta. |

Las pestañas de idioma son los idiomas de la elección. Si la elección no tiene configuración de idiomas, las pestañas son los idiomas del proceso electoral.

## Sistema de votación de papeletas {#ballot-voting-system}

El algoritmo de recuento define cómo selecciona el votante y cómo cuenta la plataforma. La versión 10.1 tiene dos algoritmos de recuento:

| Algoritmo de recuento | Cómo vota el votante | Cómo cuenta la plataforma |
| --- | --- | --- |
| **Mayoría Plural** | El votante selecciona uno o más candidatos. | Ganan los candidatos con más votos. |
| **Segunda Vuelta Instantánea** | El votante ordena a los candidatos por orden de preferencia. | En cada ronda, se elimina al candidato con menos votos y sus votos pasan a la siguiente preferencia. |

Para **Segunda Vuelta Instantánea**, la sección también muestra la **Política de desempate**:

| **Política de desempate** | Efecto |
| --- | --- |
| **Aleatorias** (predeterminado) | La plataforma rompe un empate al azar. |
| **Procedimiento externo** | El recuento se detiene en el empate. Un administrador introduce la decisión. Consulte [Resolver un empate](16-admin_portal_tutorials_tally-ceremony.md#resolve-a-tie). |

## Diseño de la Papeleta {#ballot-design}

### Número de selecciones y de ganadores {#number-of-selections-and-winners}

| Campo | Uso |
| --- | --- |
| **Resuelto por aclamación** | Los candidatos se eligen sin votación. Los votantes ven la pregunta, pero no pueden seleccionar ningún candidato. Todos los candidatos son ganadores con cero votos. Consulte [Preguntas por aclamación](../02-reference/11-acclaimed-contests.md). |
| **Permitir candidaturas manuales** | Permite candidatos escritos a mano por el votante. Los votantes solo pueden escribir un nombre si la pregunta también tiene un candidato con **Voto por Escrito** activado. |
| Número mínimo de selecciones | Las selecciones mínimas que debe hacer el votante. Escriba `0` si el votante puede dejar la pregunta en blanco. |
| Número máximo de selecciones | Para **Mayoría Plural**: el número máximo de candidatos que puede seleccionar un votante. Para **Segunda Vuelta Instantánea**: el rango más bajo que puede dar un votante, por ejemplo `5` para los rangos del 1 al 5. |
| Número de columnas | El número de columnas de candidatos en la papeleta. |
| Número de candidatos ganadores | El número de candidatos que ganan la pregunta. |

:::caution PRECAUCIÓN
El portal de administración no comprueba que el mínimo sea menor que el máximo. Compruebe los valores antes de guardar. Para **Mayoría Plural**, asegúrese de que el máximo no sea mayor que el número de candidatos.
:::

### Orden de los candidatos {#order-of-the-candidates}

| Valor | Efecto |
| --- | --- |
| **Alfabético** | La papeleta muestra los candidatos en orden alfabético. |
| **Aleatorias** | La papeleta muestra los candidatos en un orden aleatorio. |
| **Personalizado** | La papeleta muestra los candidatos en el orden que usted establece. Arrastre los candidatos en **Reordernar candidatos**. |

### Listas de candidatos {#lists-of-candidates}

Los candidatos de una pregunta pueden estar en listas. El **Tipo** de un candidato es el nombre de su lista. Para cada lista, la sección **Diseño de la Papeleta** muestra un campo **List Name** para cada idioma.

| Campo | Valores |
| --- | --- |
| Listas seleccionables | **Candidatos Y Listas** (predeterminado), **Sólo Candidatos**, **Sólo Listas** o **Deshabilitado**. Define lo que puede seleccionar un votante: los candidatos, la lista o ambos. |
| **Listas plegables** | **Deshabilitado** (predeterminado), **Habilitado (empieza expandido)** o **Habilitado (empieza contraído)**. |
| Máximo de selecciones por lista | El número máximo de candidatos que puede seleccionar un votante en una lista. Déjelo vacío si no hay límite. |

### Políticas {#policies}

Las políticas definen lo que hace el portal de votación cuando la selección del votante no es normal.

| Política | Valores | Predeterminado |
| --- | --- | --- |
| **Política de Subvoto**: menos selecciones que el máximo | **Permitido**, **Advertir en Revisión**, **Advertir**, **Advertir y Alertar** | **Permitido** |
| **Política de Voto Inválido**: una selección no válida | **Permitido**, **Advertir**, **Advertir Inválidos Implícitos y Explícitos**, **No Permitido**, **Permitido con Voto Inválido Exclusivo** | **Permitido** |
| **Política de voto en blanco**: ninguna selección | **Permitido**, **Advertir en Revisión**, **Advertir**, **No permitido** | **Permitido** |
| **Política de voto excesivo**: más selecciones que el máximo | **Permitido**, **Permitido con un mensaje de warning**, **Permitido con un mensaje de warning y alerta**, **No permitido con un mensaje de warning y alerta**, **No permitido con un mensaje de warning y dehabilitar selecciones adicionales** | **Permitido** |
| **Voto inválido - Política de rango duplicado**: dos candidatos con el mismo rango. Solo para **Segunda Vuelta Instantánea**. | **Mostrar advertencia y diálogo (el votante puede continuar)**, **Mostrar advertencia y diálogo (el votante no puede continuar)** | El votante puede continuar |
| **Voto inválido - Política de rangos omitidos**: falta un rango, por ejemplo 1, 2, 4. Solo para **Segunda Vuelta Instantánea**. | **Mostrar advertencia y diálogo (el votante puede continuar)**, **Mostrar advertencia y diálogo (el votante no puede continuar)** | El votante puede continuar |

Dos campos más cambian el aspecto de la pregunta:

- **Forma del icono de la casilla de verificación de los candidatos**: **Casilla de verificación cuadrada** (predeterminado) o **Casilla de verificación redonda**.
- **Nombre de la página**: las preguntas con el mismo nombre de página se muestran en la misma página de la papeleta.

Para los detalles de las políticas, consulte [Pregunta: Datos](../02-reference/04-contest/01-election_management_contest_data.md).

## Imagen y configuración avanzada {#image-and-advanced-configuration}

- **Imagen**: suba un archivo de imagen. El portal de administración muestra la imagen en esta sección.
- **Configuración Avanzada**: la configuración de la pregunta como archivo JSON. Úselo solo con la ayuda del soporte de Sequent.

## Crear un candidato {#create-a-candidate}

1. En el menú de la izquierda, haga clic en la flecha junto a la pregunta para mostrar sus candidatos.
2. Debajo de los candidatos, haga clic en **Crear un Candidato**.
3. Escriba el **Nombre**, la **Descripción** y el **ID externo**.
4. Haga clic en el botón de guardar.

**Resultado esperado:** el portal de administración abre el nuevo candidato. El candidato aparece debajo de la pregunta en el menú.

## Opciones del candidato {#candidate-options}

La pestaña **Datos** de un candidato tiene las secciones **General**, **Tipo** e **Imagen**.

### General {#general-1}

| Campo | Uso |
| --- | --- |
| **ID externo** | El identificador del candidato en sus otros sistemas. |
| **Nombre**, **Alias**, **Descripción** | Los textos del candidato en la papeleta. Escríbalos en la pestaña de cada idioma. |
| **Mensaje IVR** | El texto que lee la votación telefónica para el candidato. |
| **Deshabilitado** | La papeleta no muestra el candidato. |

### Tipo {#type}

| Campo | Uso |
| --- | --- |
| **Tipo** | El nombre de la lista del candidato. Los candidatos con el mismo tipo están en la misma lista. |
| **Subtype** | Un grupo dentro de la lista. |
| **Voto Inválido** | El candidato es una opción para emitir un voto inválido explícito. |
| **Voto en Blanco** | El candidato es una opción para emitir un voto en blanco explícito. |
| **Lista** | El candidato es el encabezado de su lista, que es la lista de su **Tipo**. |
| **Voto por Escrito** | El candidato es un campo donde el votante escribe un nombre. La pregunta debe tener **Permitir candidaturas manuales** activado. |
| **Posición del Voto Inválido** | Para una opción de voto inválido o en blanco: **Ninguno (Por defecto)**, **Arriba** o **Abajo** de la lista de candidatos. |

### Imagen {#image}

Suba un archivo de imagen, por ejemplo una foto o un logotipo. El portal de administración muestra la imagen en esta sección.

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| La pestaña **Datos** de la pregunta no tiene botón de guardar. | Su cuenta no tiene el permiso para editar preguntas. Pregunte al administrador del inquilino. |
| **Crear una Pregunta** o **Crear un Candidato** no están en el menú. | Su cuenta no tiene el permiso para crearlos. Pregunte al administrador del inquilino. |
| **Política de desempate** no aparece. | Seleccione **Segunda Vuelta Instantánea**. El campo solo es para este algoritmo de recuento. |
| Un candidato no aparece en la vista previa de la papeleta. | Asegúrese de que **Deshabilitado** esté desactivado. Después, haga una nueva publicación. |
| Los votantes no pueden escribir un nombre por escrito. | Active **Permitir candidaturas manuales** en la pregunta y **Voto por Escrito** en un candidato. |
| Los votantes no ven la pregunta. | Añada la pregunta al área de los votantes. Consulte [Crear las áreas](../03-procedures/02-event.md#6-create-the-areas). |

## Más información {#more-information}

- [Crear el proceso electoral](../03-procedures/02-event.md)
- [Pregunta: Datos](../02-reference/04-contest/01-election_management_contest_data.md)
- [Candidato: Datos](../02-reference/05-candidate/01-election_management_candidate_data.md)
- [Preguntas por aclamación](../02-reference/11-acclaimed-contests.md)
- [Papeletas en blanco](../02-reference/09-blank-ballots.md)
- [Ceremonia de recuento](16-admin_portal_tutorials_tally-ceremony.md)
