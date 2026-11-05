---
id: settings_countries
title: Países
description: "La pestaña Países está en Configuracion. Su título es Bloqueo de Países. Bloquea el acceso al portal de votación desde los países que seleccione. Lo usa el administrador del inquilino."
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

La pestaña **Países** está en **Configuracion**. Su título es **Bloqueo de Países**. Bloquea el acceso al portal de votación desde los países que seleccione. La usa el administrador del inquilino.

La pestaña tiene dos listas:

- Los países desde los que los votantes no pueden iniciar sesión para votar.
- Los países desde los que los votantes no pueden inscribirse (registrarse).

El bloqueo se aplica a la dirección de internet del votante. El sistema obtiene el país a partir de esa dirección.

:::note NOTA
El bloqueo solo funciona si su instalación usa el servicio de cortafuegos web que Sequent configura para el bloqueo de países. Consulte con el soporte de Sequent si no está seguro.
:::

## Cómo abrirla {#how-to-open-it}

1. En el menú de la izquierda, haga clic en **Configuracion**.
2. Haga clic en la pestaña **Países**.

## Permiso necesario {#required-permission}

- Para abrir **Configuracion**, su cuenta debe tener los permisos **Editar Inquilino** y **Ver configuración**.
- Para guardar las listas, su cuenta también debe tener el permiso **Editar las reglas de bloqueo por país en Cloudflare**.

## Campos {#fields}

| Campo | Significado | Predeterminado |
| --- | --- | --- |
| **Países** (debajo de "Elige a continuación los países de los cuales deseas bloquear las votaciones.") | Los países desde los que los votantes no pueden iniciar sesión en el portal de votación. | Vacío: ningún país está bloqueado. |
| **Países** (debajo de "Elige a continuación los países de los cuales deseas bloquear las preinscripciones.") | Los países desde los que los votantes no pueden inscribirse. | Vacío: ningún país está bloqueado. |
| **Guardar** | Guarda las dos listas y actualiza las reglas de bloqueo. | |

Cada campo **Países** acepta más de un país. Escriba una parte del nombre del país y, después, seleccione el país en la lista.

## Bloquear países {#block-countries}

1. Abra la pestaña **Países**.
2. En el primer campo **Países**, seleccione cada país que quiera bloquear para votar.
3. En el segundo campo **Países**, seleccione cada país que quiera bloquear para inscribirse.
4. Haga clic en **Guardar**.

**Resultado esperado:** los campos muestran los países seleccionados cuando vuelve a abrir la pestaña.

:::caution PRECAUCIÓN
Asegúrese de no bloquear un país donde vivan sus votantes. Los votantes bloqueados no pueden iniciar sesión ni inscribirse, y el portal de votación no les indica el motivo.
:::

## Quitar un bloqueo {#remove-a-block}

1. Abra la pestaña **Países**.
2. En el campo, haga clic en la **x** del país que quiera quitar.
3. Haga clic en **Guardar**.

Si una lista está vacía al guardar, el sistema elimina la regla de bloqueo de esa lista.

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| Las listas no cambian después de **Guardar**, o aparece un mensaje de error. | Asegúrese de que su cuenta tiene el permiso **Editar las reglas de bloqueo por país en Cloudflare**. Si lo tiene, contacte con el soporte de Sequent. |
| Los votantes de un país que no está en la lista no pueden iniciar sesión. | Contacte con el soporte de Sequent. Otras reglas de red pueden bloquear el acceso. |

**Procedimiento relacionado:** [Configurar el inquilino](../../../03-procedures/01-tenant.md).
