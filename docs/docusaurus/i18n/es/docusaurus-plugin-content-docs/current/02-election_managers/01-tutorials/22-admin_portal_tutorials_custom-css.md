---
id: admin_portal_tutorials_custom_css
title: Personalizar el portal de votación con CSS
description: "Cambie el aspecto del portal de votación con el campo CSS personalizado del proceso electoral. Busque la clase de un elemento, escriba una regla, publique y compruebe el resultado."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

Use el campo **CSS personalizado** para cambiar el aspecto del portal de votación: colores, espaciado, alineación o elementos ocultos. Las reglas se aplican a todas las elecciones del proceso electoral. Este tutorial explica cómo buscar la clase de un elemento, escribir una regla y comprobar el resultado.

Use CSS solo para el aspecto. No oculte un texto que el votante debe leer para emitir un voto válido.

## Antes de empezar {#before-you-start}

- Tenga un rol que pueda editar el proceso electoral y publicarlo.
- Conozca lo básico de CSS: un selector y una lista de propiedades.

## 1. Busque el campo CSS personalizado {#1-find-the-custom-css-field}

1. Abra el proceso electoral.
2. Haga clic en la pestaña **Datos**.
3. Abra la sección **Diseño de la Papeleta**.
4. Busque el campo **CSS personalizado**.

El campo está solo en el proceso electoral. Una elección no tiene su propio campo **CSS personalizado**.

:::note
El campo **CSS personalizado** en **Configuración** → **PERSONILIZACIÓN DE APARIENCIA** cambia el portal de administración. No cambia el portal de votación.
:::

## 2. Busque la clase de un elemento {#2-find-the-class-of-an-element}

1. Abra la vista previa de la papeleta. Consulte [Previsualizar la papeleta](../03-procedures/05-publish.md#2-preview-the-ballot).
2. Haga clic con el botón derecho en el elemento que quiere cambiar.
3. Haga clic en **Inspeccionar** en el menú del navegador.
4. Lea el atributo `class` del elemento.

Use las clases estables. Las páginas [Ganchos CSS del portal de votación](../02-reference/09-voting-portal-custom-css.md) y [Estilo de los errores y advertencias de la papeleta con CSS personalizado](../02-reference/08-ballot-errors-custom-css.md) las enumeran.

:::caution
No use una clase que parezca generada, como `css-1abc2d`. Cambia cuando cambia el software. Una regla que la usa puede dejar de funcionar después de una actualización.
:::

## 3. Escriba la regla y publíquela {#3-write-the-rule-and-publish-it}

1. Escriba la regla en el campo **CSS personalizado**.
2. Haga clic en **Guardar**.
3. Haga una publicación. Consulte [Hacer una publicación](../03-procedures/05-publish.md#1-make-a-publication).
4. Abra la vista previa y compruebe el resultado.
5. Publique la papeleta. Consulte [Publicar la papeleta](../03-procedures/05-publish.md#3-publish-the-ballot).

El portal de votación lee el CSS de la papeleta publicada. Los votantes solo ven un cambio después de que lo publique.

**Resultado esperado:** la vista previa muestra el nuevo aspecto.

## Ejemplos {#examples}

Oculte el botón de ayuda de la pantalla de votación:

```css
.voting-screen .screen-help-button {
  display: none;
}
```

Centre el título de cada pregunta:

```css
.contest-title {
  text-align: center;
}
```

Cambie el color de fondo del pie de página:

```css
.footer-class {
  background-color: #f0f0f0;
}
```

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| La regla no tiene efecto. | Asegúrese de que hizo clic en **Guardar** y de que publicó los cambios. Abra la vista previa de nuevo. |
| La regla no tiene efecto después de publicar. | Compruebe la clase en el inspector. Haga el selector más específico, por ejemplo `.voting-screen .contest-title`. |
| La regla no tiene efecto en un cuadro de diálogo o en un menú. | Los cuadros de diálogo y los menús están fuera de la pantalla. El CSS personalizado no llega a ellos. Consulte [Ganchos CSS del portal de votación](../02-reference/09-voting-portal-custom-css.md). |
| El diseño se rompe. | Quite la regla del campo **CSS personalizado**. Haga clic en **Guardar**. Publique los cambios de nuevo. |

## Páginas relacionadas {#related-pages}

- [Ganchos CSS del portal de votación](../02-reference/09-voting-portal-custom-css.md)
- [Estilo de los errores y advertencias de la papeleta con CSS personalizado](../02-reference/08-ballot-errors-custom-css.md)
- [Publicar y gestionar el período de votación](../03-procedures/05-publish.md)
