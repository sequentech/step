---
id: ballot_errors_custom_css
title: Dar estilo a los errores y advertencias de la papeleta con CSS personalizado
sidebar_position: 8
description: "Mientras un votante completa su papeleta, el portal de votación valida las selecciones y muestra cuadros de error y de advertencia, por ejemplo cuando el votante selecciona menos opciones que el mínimo, supera el…"
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

Mientras un votante completa su papeleta, el portal de votación valida las selecciones y muestra cuadros de error y de advertencia, por ejemplo cuando el votante selecciona menos opciones que el mínimo, supera el máximo o deja un orden de preferencia con huecos. Cada uno de estos cuadros tiene ganchos CSS estables, de modo que cada mensaje puede estilarse (o ocultarse) individualmente con el ajuste **CSS Personalizado**, sin afectar al resto de mensajes.

El CSS personalizado se configura en el portal de administración en **Datos → Diseño de la papeleta → CSS Personalizado** del proceso electoral. Las elecciones no tienen un campo de CSS personalizado propio.

---

## Ganchos CSS {#css-hooks}

Cada cuadro de error o de advertencia que muestra el portal de votación expone:

- **Una clase CSS específica del mensaje**, derivada del identificador del mensaje anteponiendo `warn--` y sustituyendo cada carácter que no sea una letra, un dígito, un guion bajo o un guion por un guion. Por ejemplo, el mensaje `errors.implicit.underVote` tiene la clase `warn--errors-implicit-underVote`.
- **`data-warn-id`**: el identificador del mensaje sin transformar, por ejemplo `data-warn-id="errors.implicit.underVote"`.
- **`data-warn-type`**: la categoría de la validación, una de `Implicit`, `Explicit` o `EncodingError` (consulte [Tipos de mensaje](#message-types) más abajo).

Los mensajes se muestran con dos gravedades:

- Los **errores** se muestran en cuadros de **advertencia** (amarillos de forma predeterminada). Indican que la selección no es válida o que no se pudo decodificar.
- Las **alertas** se muestran en cuadros de **información** (azules de forma predeterminada). Informan al votante de algo relevante, como un subvoto, que no invalida necesariamente la papeleta.

Los errores también aparecen en la pantalla de revisión, junto a cada pregunta, antes de emitir la papeleta, con los mismos ganchos CSS.

## Tipos de mensaje {#message-types}

El atributo `data-warn-type` clasifica cada mensaje:

- **`Implicit`**: la combinación de selecciones hace que el voto sea inválido o relevante (subvoto, sobrevoto, voto en blanco, posición de preferencia duplicada, …). Si cada condición produce un error, una alerta o nada, depende de la política de la pregunta correspondiente (política de voto inválido, política de sobrevoto o subvoto, etc.).
- **`Explicit`**: relacionado con el votante que marca explícitamente la papeleta como inválida, cuando la configuración de la pregunta lo permite o lo prohíbe.
- **`EncodingError`**: la papeleta no se pudo codificar o decodificar correctamente (por ejemplo, un texto de candidato por escrito demasiado largo).

## Referencia de mensajes {#message-reference}

| Identificador del mensaje | Clase CSS | Tipo | Gravedad |
|---|---|---|---|
| `errors.implicit.selectedMin` | `warn--errors-implicit-selectedMin` | Implicit | Error. Se seleccionan menos opciones que el mínimo. |
| `errors.implicit.selectedMax` | `warn--errors-implicit-selectedMax` | Implicit | Error (sobrevoto); según la política de sobrevoto, también se muestra como alerta. |
| `errors.implicit.underVote` | `warn--errors-implicit-underVote` | Implicit | Alerta. Se seleccionan menos opciones que el máximo (se muestra cuando la política de subvoto no es `ALLOWED`). |
| `errors.implicit.overVoteDisabled` | `warn--errors-implicit-overVoteDisabled` | Implicit | Alerta. Se alcanzó el número máximo de selecciones y las demás opciones quedan deshabilitadas. |
| `errors.implicit.blankVote` | `warn--errors-implicit-blankVote` | Implicit | Error cuando la política de voto en blanco es `NOT_ALLOWED`; si no, alerta. |
| `errors.implicit.maxSelectionsPerType` | `warn--errors-implicit-maxSelectionsPerType` | Implicit | Error. Se seleccionan más opciones para una lista de candidatos que el máximo de esa lista. |
| `errors.implicit.duplicatedPosition` | `warn--errors-implicit-duplicatedPosition` | Implicit | Error. Se seleccionó la misma posición de preferencia para dos o más candidatos. |
| `errors.implicit.preferenceOrderWithGaps` | `warn--errors-implicit-preferenceOrderWithGaps` | Implicit | Error. El orden de preferencia tiene uno o más huecos. |
| `errors.explicit.notAllowed` | `warn--errors-explicit-notAllowed` | Explicit | Error. La papeleta se marcó explícitamente como inválida, pero la pregunta no lo permite. |
| `errors.explicit.alert` | `warn--errors-explicit-alert` | Explicit | Alerta. La selección marcada se considerará un voto inválido. |
| `errors.encoding.writeInCharsExceeded` | `warn--errors-encoding-writeInCharsExceeded` | EncodingError | Error. El texto del candidato por escrito supera el número máximo de caracteres. |
| `errors.encoding.notEnoughChoices` | `warn--errors-encoding-notEnoughChoices` | EncodingError | Error. No hay suficientes opciones para decodificar la papeleta. |
| `errors.encoding.writeInChoiceOutOfRange` | `warn--errors-encoding-writeInChoiceOutOfRange` | EncodingError | Error. Una opción de candidato por escrito está fuera de rango. |
| `errors.encoding.writeInNotEndInZero` | `warn--errors-encoding-writeInNotEndInZero` | EncodingError | Error. La codificación de un candidato por escrito no termina en 0. |
| `errors.encoding.bytesToUtf8Conversion` | `warn--errors-encoding-bytesToUtf8Conversion` | EncodingError | Error. No se pudo convertir un candidato por escrito de bytes a una cadena UTF-8. |
| `errors.encoding.ballotTooLarge` | `warn--errors-encoding-ballotTooLarge` | EncodingError | Error. La papeleta codificada es más grande de lo esperado. |
| `errors.encoding.invalidMaxVotes` | `warn--errors-encoding-invalidMaxVotes` | EncodingError | Error. La configuración del máximo de votos de la pregunta no es válida. |
| `errors.encoding.invalidMinVotes` | `warn--errors-encoding-invalidMinVotes` | EncodingError | Error. La configuración del mínimo de votos de la pregunta no es válida. |

## Ejemplos {#examples}

Resalte la alerta de subvoto con un borde rojo y texto en negrita:

```css
.warn--errors-implicit-underVote {
    border: 2px solid #d32f2f;
    font-weight: bold;
}
```

Oculte por completo la alerta de "máximo alcanzado":

```css
.warn--errors-implicit-overVoteDisabled {
    display: none;
}
```

Dé estilo a una categoría completa de mensajes con el atributo `data-warn-type`:

```css
[data-warn-type="EncodingError"] {
    background-color: #fdecea;
}
```

Seleccione un mensaje por su identificador sin transformar en lugar de por la clase derivada:

```css
[data-warn-id="errors.implicit.selectedMax"] {
    text-decoration: underline;
}
```

El ajuste CSS Personalizado existe solo en el proceso electoral, por lo que los estilos se aplican a todas las elecciones del proceso.
