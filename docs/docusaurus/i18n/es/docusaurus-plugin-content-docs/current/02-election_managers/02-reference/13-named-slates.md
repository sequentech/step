---
id: named_slates
title: Candidaturas con nombre
sidebar_position: 13
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

Una **candidatura** es un grupo con nombre de candidatos que se presentan juntos en las preguntas de una elección, por ejemplo un candidato a presidente y tres candidatos a fideicomisarios. Las candidaturas son configuración: nombran a candidatos que ya están en la papeleta, y nunca son un voto propio. Una elección sin candidaturas mantiene su papeleta habitual.

---

## Configuración {#configuration}

Las candidaturas se configuran en el portal de administración, en **Diseño de la Papeleta** de la elección y en una página para cada candidatura. Consulte [Configuración de candidaturas](14-slates-configuration.md). El portal de administración las guarda como un texto JSON en la anotación de la elección `sequent.slates`, que es también lo que contiene un proceso electoral exportado:

```json
{
  "version": 1,
  "mobile_candidate_lists": "collapsed",
  "slates": [
    {
      "id": "forward",
      "name": {"en": "Forward Together", "es": "Adelante Juntos"},
      "members": {
        "<president contest id>": ["<candidate id>"],
        "<trustees contest id>": ["<candidate id>", "<candidate id>", "<candidate id>"]
      }
    },
    {
      "id": "voices",
      "name": {"en": "Independent Voices"},
      "members": {
        "<trustees contest id>": ["<candidate id>", "<candidate id>"]
      }
    }
  ]
}
```

| Campo | Significado |
| --- | --- |
| `version` | Siempre `1`. |
| `mobile_candidate_lists` | `collapsed` (predeterminado) o `expanded`: si los móviles muestran los candidatos de cada candidatura antes de que el votante lo pida. |
| `slates` | Las candidaturas, en el orden en que las ven los votantes. |
| `slates[].id` | Un identificador de hasta 64 letras, dígitos, guiones o guiones bajos, que empieza por una letra o un dígito. Único en la elección. |
| `slates[].name` | El nombre visible por código de idioma. |
| `slates[].alias` | Opcional. Un nombre corto de hasta 120 caracteres, en una sola línea. |
| `slates[].description` | Opcional. Una descripción de hasta 1000 caracteres. |
| `slates[].external_id` | Opcional. Un identificador de otro sistema, de hasta 255 caracteres, en una sola línea. |
| `slates[].members` | Los candidatos de la candidatura, por pregunta. Ambos son los identificadores de preguntas y candidatos existentes de la elección. |

Una candidatura no necesita un candidato en cada pregunta, y un candidato pertenece a una sola candidatura como máximo. Los candidatos que no están en ninguna candidatura son independientes.

### Nombre visible {#display-name}

Cada candidatura tiene un nombre por idioma. Un nombre es una línea de texto sin formato de 1 a 120 caracteres, y puede incluir un lema. Dos candidaturas no pueden tener el mismo nombre en el mismo idioma.

Los votantes ven el nombre en su idioma. Si la candidatura no tiene nombre en ese idioma, ven el nombre en el idioma predeterminado de la elección, y si no, el primer nombre configurado. Dé a cada candidatura un nombre en el idioma predeterminado de la elección. El campo **Nombre** de una candidatura en el portal de administración es el nombre en ese idioma; los nombres en otros idiomas se mantienen como están.

### Validación {#validation}

La configuración se rechaza cuando:

- no es JSON válido, tiene más de 64 KB o tiene un `version` distinto de `1`;
- tiene un campo que no aparece en la lista de arriba, o un alias, una descripción o un ID externo demasiado largo o, en el caso del alias y del ID externo, de más de una línea;
- un identificador de candidatura no es válido o está repetido;
- una candidatura no tiene nombre, tiene un nombre vacío, demasiado largo o que no es texto sin formato, o un nombre que ya usa otra candidatura en ese idioma;
- una candidatura no tiene candidatos, o incluye una pregunta sin candidatos;
- una pregunta o un candidato no existe en la elección, o un candidato no está en la pregunta en la que aparece;
- un candidato aparece dos veces, o en dos candidaturas.

El portal de votación no muestra una papeleta cuyas candidaturas no son válidas. Muestra un error de configuración con el motivo, en lugar de dejar fuera una candidatura.

Las candidaturas forman parte de la papeleta publicada. Un cambio en las candidaturas llega a los votantes con una nueva publicación de la papeleta.

La elección añade reglas propias, como los límites de cada pregunta, y la configuración se vuelve a comprobar al guardarla, importarla, generarla y publicarla. Consulte [Configuración de candidaturas](14-slates-configuration.md).

---

## Portal de votación {#voting-portal}

Cuando la elección tiene candidaturas, la pantalla de votación tiene dos pestañas, **Elegir una lista** y **Candidaturas individuales**. Ambas trabajan sobre la misma papeleta. Las elecciones sin candidaturas muestran la papeleta habitual, sin pestañas.

Encima de las pestañas se muestra el número de candidatos seleccionados sobre el máximo que permite la papeleta, por ejemplo **3 / 7 candidatos seleccionados**. En los teléfonos no se muestra.

En cada pregunta en la que una candidatura tiene candidatos, cada candidato muestra el nombre de su candidatura debajo de su propio nombre, en ambas pestañas y en la pantalla de revisión. Los candidatos que no están en ninguna candidatura muestran **Independiente**. Las opciones de voto por escrito, en blanco e inválido no muestran ninguna etiqueta. La etiqueta siempre procede del candidato, de modo que todas las pantallas muestran el mismo nombre.

---

## Coberturas completas y parciales {#complete-and-partial-slates}

Una candidatura puede incluir candidatos en solo algunas preguntas, por ejemplo solo fideicomisarios, y menos candidatos que cargos tiene una pregunta. Nunca se configura si una candidatura es completa o parcial: la plataforma lo deduce de las preguntas en las que puede votar el votante.

- Una candidatura es **completa** cuando tiene un candidato para cada cargo (el número máximo de opciones de la pregunta) de cada pregunta de la papeleta del votante.
- En caso contrario es **parcial**. Una candidatura con un fideicomisario de tres es parcial, aunque tenga candidato en cada cargo.
- La cobertura se calcula para cada estilo de papeleta. Las preguntas que no están en la papeleta de un votante se ignoran, de modo que una misma candidatura puede ser completa para un área y parcial para otra.
- Las preguntas sin oposición no cuentan ni como cubiertas ni como pendientes.
- Los candidatos deshabilitados, los marcadores de voto por escrito y las opciones explícitas en blanco o inválidas no cuentan como miembros.
- Una candidatura sin ningún candidato que el votante pueda elegir no se muestra a ese votante.

---

## Elegir una lista {#choose-a-slate}

Cada candidatura es una tarjeta, en el orden configurado. De arriba abajo, una tarjeta muestra:

- Una etiqueta que indica lo que cubre la candidatura: **Candidatura completa**, **Solo _pregunta_** (por ejemplo **Solo Fideicomisarios**) cuando tiene candidatos en una sola pregunta, o **Candidatura parcial**.
- El nombre de la candidatura.
- Una frase: **Candidatos para todos los cargos.**, o **Candidatos para _cargos_. Los demás cargos quedan en blanco.**
- El número de candidatos y de cargos, por ejemplo **3 candidatos · 1 cargo**.
- Cuánto de la candidatura está seleccionado (véase más abajo).
- Sus candidatos, cargo por cargo. Un cargo para el que la candidatura no tiene candidato muestra **Sin candidato**.
- El botón **Seleccionar lista**. Consulte [Elegir una candidatura](13-choosing-a-slate.md).

Debajo de las tarjetas, una nota dice: *Una lista selecciona sus candidatos y deja en blanco los cargos en los que no tiene candidato. Puedes cambiar cualquier opción individual en cualquier momento.* Cuando hay algo seleccionado, una franja debajo de la nota enumera cada cargo con sus candidatos seleccionados, o **Sin selección**.

Las tarjetas muestran las preguntas de la papeleta del votante para las que al menos una candidatura tiene candidatos, en el orden de la papeleta. Los candidatos aparecen en el orden en que los muestra la papeleta, siguiendo el orden de candidatos de cada pregunta.

### Qué dice una candidatura sobre la papeleta {#what-a-slate-says-about-the-ballot}

Las tarjetas siempre reflejan la papeleta actual, sea cual sea la forma en que se eligieron los candidatos. Cada miembro seleccionado tiene una marca de verificación y un resaltado, y los lectores de pantalla anuncian que está seleccionado. Cada candidatura indica cuánto de ella está seleccionado, mirando la papeleta completa:

| Resumen | Significado |
| --- | --- |
| **Los N seleccionados** | La papeleta tiene exactamente los candidatos de la candidatura, y nada en los cargos para los que no tiene candidato. |
| **Mixta · n de N seleccionados** | Algunos de sus candidatos están seleccionados, junto con cualquier otra opción de la papeleta, incluida una marca de voto inválido. |
| **Seleccionada en parte · n de N** | Algunos de sus candidatos están seleccionados y nada más lo está. |
| Sin resumen | Ninguno de sus candidatos está seleccionado. |

N es el número de candidatos de la candidatura en la papeleta del votante. Una candidatura con solo candidatos a fideicomisario muestra **Los N seleccionados** solo mientras los demás cargos estén en blanco. Un votante que selecciona un fideicomisario de cada una de tres candidaturas ve cada candidatura como **Mixta** con su propio recuento.

Una tarjeta cuya candidatura está en **Los N seleccionados** tiene un borde oscuro y una marca de verificación en lugar del icono de personas, y ofrece **Editar selecciones** en lugar de **Seleccionar lista**. **Editar selecciones** abre **Candidaturas individuales**. Una tarjeta con solo algunos de sus candidatos seleccionados tiene un borde más claro.

Los resúmenes se calculan desde la papeleta cada vez que cambia. El portal de votación no recuerda qué candidatura se eligió, así que siguen siendo correctos después de ir a **Candidaturas individuales** o a la pantalla de revisión y volver.

### Pantallas anchas {#wider-screens}

En pantallas de 750 px de ancho o más, las tarjetas están una al lado de otra, hasta tres por fila; las demás candidaturas continúan en la fila siguiente. Todos los candidatos son visibles sin hacer clic. Cada cargo empieza a la misma altura en cada tarjeta de una fila, también cuando un nombre ocupa dos líneas, para que un votante pueda leer de una tarjeta a otra y comparar los candidatos de un mismo cargo. La página nunca se desplaza horizontalmente.

### Teléfonos {#phones}

En pantallas de menos de 750 px, las tarjetas se apilan en el orden configurado y solo muestran los cargos para los que su candidatura tiene candidatos. Los candidatos de cada candidatura están en una lista que el votante puede mostrar u ocultar con **Mostrar candidatos** y **Ocultar candidatos**. La configuración **Listas de candidatos en móvil** de la elección decide cómo empiezan las listas:

| Listas de candidatos en móvil | Las listas empiezan |
| --- | --- |
| **Contraídas** (predeterminado) | Ocultas |
| **Desplegadas** | Visibles |

Mostrar u ocultar una lista nunca cambia la papeleta, y la elección del votante para cada lista se mantiene al moverse entre las pestañas y la pantalla de revisión. El resumen de una candidatura permanece visible encima de una lista oculta.

---

## Candidaturas individuales {#individual-candidates}

En una papeleta con candidaturas, **Candidaturas individuales** muestra un cargo cada vez. Una línea en la parte superior indica lo que contiene la papeleta: el nombre de la candidatura que contiene exactamente, **Selecciones individuales** o **Ningún candidato seleccionado**, seguido del número de candidatos seleccionados.

- En pantallas anchas hay una lista de cargos a la izquierda, cada uno con su recuento, por ejemplo **1/1**, y una marca de verificación cuando está completo. En los teléfonos, la lista es un desplegable **Cargo**.
- Cada cargo muestra **Cargo n de N**, su nombre, **Selecciona hasta N candidatos. k seleccionados.**, y un botón **Borrar** que quita las opciones de ese cargo. **Borrar** está deshabilitado mientras el cargo no tiene ninguna opción.
- **Cargo anterior** y **Cargo siguiente** cambian de cargo; el último cargo muestra **Último cargo**.

Todos los cargos permanecen en la pantalla de votación, así que **Siguiente** en la parte inferior va a la pantalla de revisión desde cualquiera de las dos pestañas. Las papeletas sin candidaturas mantienen sus páginas habituales.

---

## Revisión de la papeleta {#reviewing-the-ballot}

Cuando la elección tiene candidaturas, la pantalla de revisión muestra cada pregunta como de costumbre y añade:

- Debajo de cada pregunta, el número de candidatos seleccionados sobre el máximo, por ejemplo **2 de 3 seleccionados**. Una pregunta sin selección también muestra **Ningún candidato seleccionado**.
- Debajo de cada pregunta, un botón de edición llamado **Editar _pregunta_**. Abre **Candidaturas individuales** en ese cargo.
- **Tus selecciones**, junto a las preguntas en pantallas anchas: el número de candidatos seleccionados sobre el máximo que permite la papeleta, el nombre de la candidatura que contiene exactamente, **Selecciones individuales** o **Ningún candidato seleccionado**, y la nota *Tu voto se registra para cada candidato seleccionado.* No se muestra en los teléfonos.

Volver con un botón de edición o con **Atrás** mantiene todas las selecciones. Las preguntas sin oposición no tienen recuento ni botón de edición, y una papeleta en blanco no muestra ni el resumen ni los recuentos.

La pantalla de revisión siempre muestra las selecciones de la papeleta cifrada que está a punto de emitirse. Si las selecciones cambian después de cifrar la papeleta, por ejemplo con los botones atrás y adelante del navegador, el portal de votación vuelve a la pantalla de votación con las selecciones actuales.

### Cargos sin completar {#unfilled-offices}

Cuando la **Política de Subvoto** de una pregunta es **Advertir y Confirmar en Revisión**, emitir una papeleta con menos opciones de las permitidas en esa pregunta abre **Hay selecciones sin completar**. Enumera cada cargo de ese tipo con, por ejemplo, **0 de 1 seleccionadas · Sin voto para este cargo**. **Revisar selecciones** cierra el diálogo sin emitir; **Continuar con estas selecciones** emite. Una papeleta totalmente en blanco recibe la misma pregunta. La respuesta no se guarda: la pregunta vuelve a aparecer en cada emisión.

---

## Tamaños de pantalla y teclado {#screen-sizes-and-keyboard}

Las tarjetas de candidatura, las candidaturas individuales, los diálogos y la pantalla de revisión se ajustan a la pantalla desde 320 px de ancho: los nombres largos de candidaturas y candidatos continúan en la línea siguiente y la página nunca se desplaza horizontalmente.

Las pestañas, los botones **Mostrar candidatos** y **Ocultar candidatos** y los botones de cada candidatura miden al menos 44 px de alto. Se puede llegar a todos con la tecla Tab y se usan con Intro o Espacio, y muestran cuál tiene el foco.

---

## CSS personalizado {#custom-css}

| Clase | Elemento |
| --- | --- |
| `slate-selection-count` | El número de candidatos seleccionados encima de las pestañas y en la pantalla de revisión |
| `slate-ballot-tabs-container` | Contenedor de las pestañas y sus paneles |
| `slate-ballot-tabs-bar`, `slate-ballot-tabs` | La barra de pestañas y la lista de pestañas |
| `slate-ballot-tab-slates`, `slate-ballot-tab-candidates` | Las dos pestañas |
| `slate-ballot-panel`, `slate-ballot-panel-slates`, `slate-ballot-panel-candidates` | Los paneles |
| `slate-chooser`, `slate-list` | Las candidaturas y su lista |
| `slate-card` | Una tarjeta de candidatura |
| `slate-card-none`, `slate-card-some`, `slate-card-exact` | Una tarjeta sin candidatos seleccionados, con algunos o con exactamente los suyos |
| `slate-card-header`, `slate-symbol`, `slate-mark` | La parte superior de una tarjeta, su fila de iconos y el icono redondo |
| `slate-coverage-chip` | **Candidatura completa**, **Solo _pregunta_** o **Candidatura parcial**. Su atributo `data-coverage` es `complete` o `partial`. |
| `slate-name` | El nombre de la candidatura |
| `slate-coverage-sentence` | **Candidatos para todos los cargos.** o los cargos que cubre la candidatura |
| `slate-count` | El número de candidatos y de cargos |
| `slate-summary`, `slate-selection-status` | El resumen de una candidatura |
| `slate-selection-status-all`, `slate-selection-status-mixed`, `slate-selection-status-partly` | El resumen, por estado |
| `slate-member-lists`, `slate-candidate-list` | Los cargos de una tarjeta |
| `slate-candidate-list-toggle` | **Mostrar candidatos** / **Ocultar candidatos** en los teléfonos |
| `slate-contest`, `slate-contest-name` | Un cargo de una tarjeta y su nombre |
| `slate-contest-uncovered` | Un cargo para el que la candidatura no tiene candidato |
| `slate-no-candidate` | El texto **Sin candidato** |
| `slate-members`, `slate-member`, `slate-member-name` | Los candidatos de un cargo |
| `slate-member-selected`, `slate-member-check` | Un miembro seleccionado y su marca de verificación |
| `slate-actions`, `slate-card-actions` | Las acciones de una tarjeta |
| `slate-edit-selections-button` | El botón **Editar selecciones** |
| `slate-note` | La nota debajo de las tarjetas |
| `slate-all-selections`, `slate-all-selections-office`, `slate-all-selections-name` | La franja de selecciones actuales |
| `slate-selection-label` | La línea de la parte superior de **Candidaturas individuales** |
| `slate-office-navigator` | El navegador de cargos |
| `slate-office-list`, `slate-office-button`, `slate-office-count`, `slate-office-full` | La lista de cargos, un cargo, su recuento y su marca de verificación |
| `slate-office-picker`, `slate-office-select` | El desplegable **Cargo** en los teléfonos |
| `slate-office-index` | **Cargo n de N** |
| `slate-office-navigation`, `slate-office-previous`, `slate-office-next`, `slate-office-last` | **Cargo anterior**, **Cargo siguiente** y **Último cargo** |
| `contest-selection-count` | **Selecciona hasta N candidatos. k seleccionados.** |
| `contest-clear-button`, `contest-clear-status` | El botón **Borrar** de un cargo y su anuncio |
| `review-slate-layout`, `review-slate-contests`, `review-slate-aside` | La pantalla de revisión con **Tus selecciones** junto a las preguntas |
| `review-selection-summary` | **Tus selecciones** |
| `review-selection-title`, `review-selection-total`, `review-selection-caption`, `review-selection-note` | Su encabezado, total, subtítulo y nota |
| `review-selection-label`, `review-selection-label-slate`, `review-selection-label-individual`, `review-selection-label-none` | Lo que contiene la papeleta, por tipo |
| `review-contest-footer` | El recuento y el botón de edición de una pregunta |
| `review-contest-unfilled` | Una pregunta con menos opciones de las permitidas |
| `review-contest-count`, `review-contest-blank`, `review-contest-edit` | El recuento, **Ningún candidato seleccionado** y el botón de edición |
| `unfilled-contests-dialog`, `unfilled-contest` | **Hay selecciones sin completar** y uno de sus cargos |
