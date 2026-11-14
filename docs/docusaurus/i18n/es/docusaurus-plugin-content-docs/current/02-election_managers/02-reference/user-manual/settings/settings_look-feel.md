---
id: settings_look-feel
title: Personalización de apariencia
description: "La pestaña PERSONALIZACIÓN DE APARIENCIA está en Configuracion. Establece el logotipo y el estilo del portal de administración para su inquilino, y los enlaces del menú Ayuda. La usa el administrador del inquilino."
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

La pestaña **PERSONALIZACIÓN DE APARIENCIA** está en **Configuracion**. Establece el logotipo y el estilo del portal de administración para su inquilino, y los enlaces del menú **Ayuda**. La usa el administrador del inquilino.

:::note NOTA
Estos ajustes cambian el portal de administración. El logotipo del portal de votación se establece para cada proceso electoral, en su pestaña **Datos**.
:::

## Cómo abrirla {#how-to-open-it}

1. En el menú de la izquierda, haga clic en **Configuracion**.
2. Haga clic en la pestaña **PERSONALIZACIÓN DE APARIENCIA**.

## Permiso necesario {#required-permission}

Para abrir **Configuracion**, su cuenta debe tener los permisos **Editar Inquilino** y **Ver configuración**. El botón **Guardar** solo se muestra a los usuarios con el permiso **Editar Inquilino**.

## Campos {#fields}

| Campo | Significado | Predeterminado |
| --- | --- | --- |
| **Logo URL** | La dirección web de la imagen que se muestra en la parte superior del portal de administración. Déjelo vacío para mostrar el logotipo estándar. | Vacío |
| **CSS Personalizado** | Reglas de estilo (CSS) que el portal de administración añade a sus páginas. Úselo para cambiar colores o tipografías. | Vacío |
| **Enlaces de Ayuda** | La lista de enlaces del menú **Ayuda**, en formato JSON. Consulte [Enlaces de ayuda](#help-links). | Lista vacía |
| **Guardar** | Guarda los tres campos. Después de guardar, el botón permanece deshabilitado hasta que cambie un campo. | |

## Enlaces de ayuda {#help-links}

**Enlaces de Ayuda** es una lista JSON. Cada enlace tiene estas claves:

| Clave | Obligatoria | Significado |
| --- | --- | --- |
| `title` | Sí | El texto del elemento del menú. |
| `url` | Sí | La dirección web que se abre en una nueva pestaña del navegador. |
| `i18n` | No | Una traducción del título para cada idioma, por ejemplo `{"es": {"title": "Ayuda"}}`. |

Ejemplo:

```json
[
  {"title": "User guide", "url": "https://example.com/guide"},
  {
    "title": "Support",
    "url": "https://example.com/support",
    "i18n": {"es": {"title": "Soporte"}}
  }
]
```

Cuando la lista tiene uno o más enlaces, el elemento **Ayuda** se muestra en el menú de la izquierda. Cuando la lista está vacía, el elemento **Ayuda** no se muestra.

## Cambiar la personalización de apariencia {#change-the-look-and-feel}

1. Abra la pestaña **PERSONALIZACIÓN DE APARIENCIA**.
2. En **Logo URL**, escriba la dirección web de la imagen del logotipo.
3. En **CSS Personalizado**, escriba las reglas de estilo.
4. En **Enlaces de Ayuda**, escriba la lista JSON de enlaces.
5. Haga clic en **Guardar**.

**Resultado esperado:** el portal de administración muestra el nuevo logotipo y estilo. El menú **Ayuda** muestra los enlaces.

:::caution PRECAUCIÓN
Asegúrese de que el **CSS Personalizado** no oculte botones ni campos. Las reglas de estilo incorrectas pueden hacer que partes del portal de administración sean difíciles de usar.
:::

Para quitar el logotipo o el estilo personalizados, borre el texto del campo y haga clic en **Guardar**.

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| Aparece el mensaje "Formato de Enlaces de Ayuda inválido". | El texto de **Enlaces de Ayuda** no es una lista JSON válida. Corríjalo. La lista debe empezar por `[` y terminar por `]`. |
| El elemento **Ayuda** no aparece en el menú. | Asegúrese de que **Enlaces de Ayuda** contiene uno o más enlaces y, después, haga clic en **Guardar**. |
| El logotipo no aparece. | Asegúrese de que la dirección web de **Logo URL** abre la imagen en un navegador. |
| El botón **Guardar** no aparece. | Su cuenta no tiene el permiso **Editar Inquilino**. |

**Procedimiento relacionado:** [Configurar el inquilino](../../../03-procedures/01-tenant.md).
