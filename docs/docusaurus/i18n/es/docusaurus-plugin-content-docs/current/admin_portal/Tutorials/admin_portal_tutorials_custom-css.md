---
id: admin_portal_tutorials_custom_css
title: Personalizar el portal de votación con CSS
description: "Cambie el aspecto del portal de votación con el campo CSS personalizado del proceso electoral. Busque la clase de un elemento, escriba una regla, publique y compruebe el resultado."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

Use el campo **CSS personalizado** para cambiar el aspecto del portal de votación: colores,
espaciado, alineación o elementos ocultos. Las reglas se aplican a todas las elecciones del proceso
electoral. Este tutorial muestra cómo buscar la clase de un elemento, escribir una regla y comprobar
el resultado.

Use CSS solo para el aspecto. No oculte un texto que el votante debe leer para emitir un voto
válido.

## Antes de empezar {#before-you-start}

- Tenga un rol que pueda editar el proceso electoral y publicarlo.
- Conozca los conceptos básicos de CSS: un selector y una lista de propiedades.

## 1. Buscar el campo CSS personalizado {#1-find-the-custom-css-field}

1. Abra el proceso electoral.
2. Haga clic en la pestaña **Datos**.
3. Abra la sección **Diseño de la Papeleta**.
4. Busque el campo **CSS personalizado**.

El campo solo está en el proceso electoral. Una elección no tiene su propio campo **CSS
personalizado**.

:::note
El campo **CSS Personalizado** de **Configuracion** → **PERSONILIZACIÓN DE APARIENCIA** cambia el portal de administración. No cambia el portal de votación.
:::

## 2. Buscar la clase de un elemento {#2-find-the-class-of-an-element}

1. Abra la vista previa de la papeleta. Consulte [Previsualizar la papeleta](../procedures/05-publish.md#2-preview-the-ballot).
2. Haga clic con el botón derecho en el elemento que quiere cambiar.
3. Haga clic en **Inspeccionar** en el menú del navegador.
4. Lea el atributo `class` del elemento.

Use las clases estables de la tabla de abajo. Existen en esta versión.

| Clase | Elemento |
| --- | --- |
| `.election-selection-screen` | La pantalla de la lista de papeletas. |
| `.start-screen` | La pantalla de instrucciones. |
| `.voting-screen` | La pantalla de voto. |
| `.review-screen` | La pantalla de revisión. |
| `.confirmation-screen` | La pantalla de confirmación. |
| `.error-screen` | La pantalla de error. |
| `.screen` | Todas las pantallas anteriores. |
| `.header-class` | La cabecera. |
| `.footer-class` | El pie de página. |
| `.stepper-box` | Los pasos de progreso de la pantalla de voto. |
| `.contest-title` | El título de una pregunta. |
| `.candidates-container` | La lista de candidatos de una pregunta. |
| `.candidate-item` | Un candidato. |
| `.candidate-title` | El nombre de un candidato. |
| `.election-item` | Una elección de la lista de papeletas. |
| `.election-title` | El título de una elección de la lista de papeletas. |
| `.click-to-vote-button` | El botón que abre una elección. |
| `.start-voting-button` | El botón de la pantalla de instrucciones. |
| `.next-button` | El botón **Siguiente** de la pantalla de voto. |
| `.cast-ballot-button` | El botón que emite la papeleta. |
| `.finish-button` | El botón de la pantalla de confirmación. |

:::caution
No use una clase que parezca generada, como `css-1abc2d`. Cambia cuando cambia el software. Una
regla que la usa puede dejar de funcionar después de una actualización.
:::

## 3. Escribir la regla y publicarla {#3-write-the-rule-and-publish-it}

1. Escriba la regla en el campo **CSS personalizado**.
2. Haga clic en **Guardar**.
3. Cree una publicación. Consulte [Crear una publicación](../procedures/05-publish.md#1-make-a-publication).
4. Abra la vista previa y compruebe el resultado.
5. Publique la papeleta. Consulte [Publicar la papeleta](../procedures/05-publish.md#3-publish-the-ballot).

El portal de votación lee el CSS de la papeleta publicada. Los votantes solo ven un cambio después
de que usted lo publique.

**Resultado esperado:** la vista previa muestra el aspecto nuevo.

## Ejemplos {#examples}

Ocultar los pasos de progreso en la pantalla de voto:

```css
.voting-screen .stepper-box {
  display: none;
}
```

Centrar el título de cada pregunta:

```css
.contest-title {
  text-align: center;
}
```

Cambiar el color de fondo del pie de página:

```css
.footer-class {
  background-color: #f0f0f0;
}
```

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| La regla no tiene efecto. | Asegúrese de que hizo clic en **Guardar** y publicó los cambios. Abra la vista previa otra vez. |
| La regla no tiene efecto después de publicar. | Compruebe la clase en el inspector. Haga el selector más específico, por ejemplo `.voting-screen .contest-title`. |
| La regla no tiene efecto en un cuadro de diálogo o en un menú. | Los cuadros de diálogo y los menús están fuera de la pantalla. El CSS personalizado no llega a ellos. |
| El diseño se rompe. | Quite la regla del campo **CSS personalizado**. Haga clic en **Guardar**. Publique los cambios otra vez. |

## Páginas relacionadas {#related-pages}

- [Publicar y gestionar el período de votación](../procedures/05-publish.md)
