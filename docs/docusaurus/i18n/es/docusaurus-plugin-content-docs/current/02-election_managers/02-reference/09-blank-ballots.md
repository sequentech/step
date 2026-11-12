---
id: blank_ballots
title: Papeletas en blanco
sidebar_position: 9
description: "La política de papeletas en blanco permite a un votante emitir deliberadamente una papeleta con todas las preguntas vacías, y registra ese recuento como una cifra distinta y verificable (Total de Papeletas en Blanco), separada de…"
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

La política de **papeletas en blanco** permite a un votante emitir deliberadamente una papeleta con todas las preguntas vacías, y registra ese recuento como una cifra distinta y verificable, **Total de Papeletas en Blanco**, separada de los votos en blanco por pregunta y de la [abstención de voto](./07-decline-to-vote.md). Es un resultado **a nivel de elección**: una papeleta está en blanco en todas las preguntas o no está en blanco en ninguna. El portal de votación nunca ofrece el voto en blanco como opción de candidato; se deduce del contenido de la papeleta.

Esta función está disponible **solo cuando el proceso electoral usa la política de cifrado Varios concursos**. No se aplica al cifrado de Concurso único.

> **Nota sobre la Segunda Vuelta Instantánea (IRV):** como ocurre con la abstención de voto, IRV no es compatible actualmente con el cifrado Varios concursos, por lo que las papeletas en blanco no se aplican a las elecciones que usan IRV hoy. Consulte [Algoritmo de Segunda Vuelta Instantánea](../../../../../../docs/07-developers/07-velvet/05-instant-runoff.md).

---

## Requisitos previos {#prerequisites}

Antes de poder usar las papeletas en blanco, deben configurarse los dos ajustes siguientes:

1. **Política de cifrado de concurso del proceso electoral:** configúrela en **Varios concursos** en la pestaña **Datos** del proceso electoral, dentro de **Configuración Avanzada**. Consulte [Datos del proceso electoral](./02-election-event/03-election_management_election-event_data.md).
2. **Política de papeletas en blanco de la elección:** configúrela en **Habilitado** en la pestaña **Datos** de la elección, dentro de **Configuración Avanzada**. Consulte [Datos de la elección](./03-election/03-election_management_election_data.md).

Si el proceso electoral usa el cifrado **Concurso único**, el ajuste de la política de papeletas en blanco queda oculto en el portal de administración y el resultado nunca se informa para esa elección.

---

## Portal de administración {#admin-portal}

### Activar la política {#enabling-the-policy}

1. Abra el **proceso electoral** y confirme que **Política de cifrado de concurso** está en **Varios concursos**.
2. Abra la **elección** de destino y vaya a la pestaña **Datos**.
3. Despliegue **Configuración Avanzada**.
4. Configure **Política de papeletas en blanco** en **Habilitado** (el valor predeterminado es **Deshabilitado**).
5. Guarde la elección.

La política se configura por elección, no por pregunta. Una vez que la votación ha comenzado, trate la política como fija para esa elección: cambiarla a mitad de una elección en curso mezcla papeletas emitidas con reglas distintas.

### Resultados del recuento {#tally-results}

Después de que termine un recuento, las papeletas en blanco aparecen en los resultados a nivel de elección:

- En la pestaña **Recuento**, la tabla **Resultados y participación** de la sesión de recuento incluye una columna **Total de Papeletas en Blanco** cuando al menos una elección incluida tiene habilitadas las papeletas en blanco con cifrado Varios concursos.
- La tabla **Elecciones** de resultados (resumen del recuento) muestra la misma columna, con los datos del campo `blank_ballots` de la fila de resultados de cada elección.

### Resultados en papel (hojas de recuento) {#paper-results-tally-sheets}

Para los resultados que llegan en papel:

- La canalización de **importación CSV** acepta una cifra de papeletas en blanco por urna (se espera el mismo valor en cada hoja de escrutinio de una urna, ya que el hecho de estar en blanco es una propiedad de la papeleta completa). La importación comprueba que la cifra es coherente en las hojas de una urna y que está dentro de los límites que implican las cifras propias de cada pregunta de esa urna, y rellena automáticamente el valor cuando esos límites lo fijan a un único número.
- La **introducción manual de hojas de recuento** (el formulario de una sola hoja del portal de administración) también acepta la cifra, pero, a diferencia de la importación CSV, no ve las demás hojas de pregunta de la urna mientras edita una sola hoja, así que no puede comprobar cruzado ni rellenar el valor como lo hace la importación CSV. Introduzca a mano la misma cifra de forma coherente en cada hoja de pregunta de una urna.
- Una urna que nunca recibió una cifra enviada se informa como **no disponible**, no como cero, y cualquier total que incluya esa urna también se informa como no disponible.

---

## Portal de votación {#voting-portal}

A diferencia de la abstención de voto, **no hay ningún botón explícito de "emitir papeleta en blanco"**. Una papeleta queda en blanco únicamente como consecuencia de que el votante deje todas las preguntas sin selección durante el flujo normal de votación.

### Emitir una papeleta en blanco {#casting-a-blank-ballot}

1. El votante avanza por la papeleta de forma normal (**Iniciar Votación**, no Abstención de voto) y deja cada pregunta sin selección de candidato.
2. Cuando el votante hace clic en **Siguiente** para pasar a la revisión, un cuadro de diálogo de confirmación explica que la papeleta se emitirá en blanco y pide al votante que confirme antes de continuar.
3. En la **pantalla de revisión**, cada pregunta muestra la etiqueta **Papeleta en blanco** en lugar de las selecciones de candidatos.
4. Si la emisión de la papeleta está configurada para mostrar un cuadro de diálogo de confirmación antes de la emisión final, el texto de ese cuadro también es específico de la papeleta en blanco.
5. Después de emitir la papeleta, la pantalla de confirmación o de recibo muestra un texto específico de papeleta en blanco junto al ID de papeleta habitual, explicando que la papeleta se emitió en blanco como una elección válida y deliberada.

Si el votante selecciona al menos un candidato en cualquier pregunta, o marca explícitamente cualquier pregunta como inválida, la papeleta no está en blanco: se aplican el texto normal de revisión y de recibo, aunque otras preguntas hayan quedado vacías.

### Verificador de papeletas {#ballot-verifier}

El verificador de papeletas independiente muestra la misma etiqueta **Papeleta en blanco** (distinta de la etiqueta genérica de pregunta vacía que se usa para un subvoto normal) siempre que decodifica una papeleta cuyo indicador de papeleta en blanco está activado y la política está habilitada para esa elección.

---

## Resultados {#results}

### Nivel de elección {#election-level}

A nivel de elección, una papeleta en blanco:

- **Cuenta para la participación total de votantes** (incluida en **Total de votantes** / votos emitidos para la elección): una papeleta en blanco es una papeleta válida emitida.
- Se informa por separado como **Total de Papeletas en Blanco**, con su propio número y porcentaje. El porcentaje se calcula sobre los votos emitidos, no sobre el censo, ya que una papeleta en blanco es una papeleta válida emitida y no una no participación.
- Se determina primero por área (frente a las preguntas que recibieron realmente los votantes de esa área) y después se suma a los totales de la elección y del proceso electoral.

### Nivel de pregunta {#contest-level}

A nivel de pregunta, una papeleta en blanco no introduce una nueva categoría: cada pregunta de una papeleta en blanco es simplemente una selección vacía ordinaria para esa pregunta, y se cuenta como voto en blanco en las cifras de votos en blanco propias de esa pregunta (**Votos en blanco**, y su desglose en **Votos en blanco explícitos** y **Votos en blanco implícitos**), igual que cualquier otra selección vacía. Papeletas en blanco no cambia la forma de calcular los votos en blanco por pregunta; añade una cifra de papeleta completa por encima.

### Sitio web de resultados {#results-website}

El sitio web público de resultados muestra una columna **Total de Papeletas en Blanco** en su tabla de resumen de la elección siempre que los datos publicados la incluyan. La columna se **oculta por completo**, en lugar de mostrar cero, cuando la cifra no está disponible; por ejemplo, cuando la política está deshabilitada en todas las elecciones publicadas o cuando no se pudo establecer la cifra del recuento.

### Exclusión mutua con Abstención de voto {#mutual-exclusivity-with-decline-to-vote}

Una papeleta no puede estar a la vez en blanco y en abstención. Ambas son mutuamente excluyentes en el momento de la emisión, y una papeleta cuyo estado registrado no coincide con su contenido real se trata como no válida.

---

## Resumen {#summary}

| Aspecto | Comportamiento |
|--------|----------|
| **Política de cifrado** | Solo Varios concursos |
| **Algoritmo de recuento** | No compatible con IRV (IRV requiere la codificación de Concurso único) |
| **Ubicación de la configuración** | Elección **Datos** → **Configuración Avanzada** → **Política de papeletas en blanco** |
| **Punto de entrada del votante** | Ninguno: se deduce de dejar todas las preguntas vacías durante la votación normal; se confirma con un cuadro de diálogo antes de emitir |
| **Alcance** | Toda la papeleta (todas las preguntas a la vez) |
| **Resultados a nivel de elección** | Cuenta como participación; se muestra como **Total de Papeletas en Blanco** (número y porcentaje de votos emitidos) |
| **Resultados a nivel de pregunta** | Se cuenta dentro de las cifras de votos en blanco existentes de esa pregunta, no como categoría aparte |
| **Resultados en papel** | Cifra por urna; la importación CSV valida y rellena entre las hojas de una urna, la introducción manual no |
| **Datos no disponibles** | Se informan como no disponibles (ocultos o "-"), nunca se convierten en cero |
| **Abstención de voto** | Mutuamente excluyente con una papeleta de abstención |
