---
id: admin_portal_reference_user_manual_settings_look___feel
title: Personalización de apariencia
description: "La pestaña PERSONILIZACIÓN DE APARIENCIA establece el logotipo, el estilo y los enlaces de ayuda del inquilino."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

La pestaña **PERSONILIZACIÓN DE APARIENCIA** establece el logotipo, el estilo y los enlaces de
ayuda del inquilino.

## Abrir la pestaña {#open-the-tab}

1. En el menú de la izquierda, haga clic en **Configuración**.
2. Haga clic en la pestaña **PERSONILIZACIÓN DE APARIENCIA**.

Para guardar cambios, su rol necesita el permiso **Editar Inquilino**.

## Campos {#fields}

| Campo | Uso |
| --- | --- |
| **Logo URL** | La dirección de la imagen del logotipo del inquilino. |
| **CSS Personalizado** | Reglas de estilo que cambian el aspecto de los portales. |
| **Enlaces de Ayuda** | Los enlaces del menú **Ayuda**, en formato JSON. Si la lista está vacía, el menú **Ayuda** no se muestra. |
| **Guardar** | Guarda los cambios. |

**Enlaces de Ayuda** es una lista JSON. Cada enlace tiene un `title` y una `url`:

```json
[
  {"title": "User guide", "url": "https://example.com/guide"}
]
```

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| "Formato de Enlaces de Ayuda inválido" | El texto no es una lista JSON. Compruebe los corchetes, las comillas y las comas. |
| El menú **Ayuda** no se muestra. | **Enlaces de Ayuda** está vacío. Añada uno o más enlaces y guarde. |

## Páginas relacionadas {#related-pages}

- [Configuración](../admin_portal_reference_user-manual_settings.md)
- [Navegación básica](../../../basic_navigation.md)
