---
id: admin_portal_tutorials_multi_idp_attribute_linking
title: Vincular varias identidades de IdP a un único usuario mediante un atributo personalizado
description: "De forma predeterminada, la vinculación de identidades de Keycloak asocia un usuario de un proveedor de identidad (IdP) externo con un usuario de Keycloak de forma uno a uno, normalmente por coincidencia de correo electrónico o de nombre de usuario."
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## Descripción general {#overview}

De forma predeterminada, la vinculación de identidades de Keycloak asocia un usuario de un proveedor de identidad (IdP) externo con un usuario de Keycloak de forma uno a uno, normalmente por coincidencia de `email` o de `username`. Este tutorial explica cómo configurar el autenticador **Custom Attribute IdP Identity Linking** para que varias identidades de IdP (por ejemplo, distintos correos electrónicos o identificadores de sujeto del mismo IdP o de IdP diferentes) se puedan asignar a un único usuario de Keycloak.

El autenticador lee una notificación configurable de la identidad de IdP entrante y busca un usuario de Keycloak cuyo atributo personalizado de **múltiples valores** contenga ese valor. Cuando encuentra exactamente una coincidencia, la nueva identidad de IdP se vincula automáticamente al usuario existente.

---

## Requisitos previos {#prerequisites}

- Acceso a la consola de administración de Keycloak.
- La extensión `sequent.idp-linking-authenticator.jar` desplegada en el directorio `providers/` de Keycloak (incluida de forma predeterminada en la imagen Docker de Keycloak de Sequent).
- Un proveedor de identidad externo configurado en el realm de destino.

---

## Paso 1 – Cree el atributo de usuario personalizado {#step-1--create-the-custom-user-attribute}

El autenticador busca usuarios por un atributo del perfil de usuario de Keycloak. Debe crear este atributo antes de configurar el flujo.

1. En la consola de administración de Keycloak, seleccione el realm que quiere configurar.
2. Vaya a **Realm settings** → **User profile** → **Create attribute**.
3. Establezca **Name** en `linked_idp_identities` (o en cualquier otro nombre que vaya a usar en la configuración del autenticador).
4. Active el atributo **multi-valued** (no lo restrinja a un único valor).
5. Opcionalmente, restrinja los permisos de lectura y escritura para que solo los administradores puedan gestionarlo.
6. Haga clic en **Save**.

> **Consejo:** Después de crear el atributo, puede rellenarlo para los usuarios existentes desde la consola de administración (**Users** → seleccione el usuario → pestaña **Attributes**) o mediante la API REST de administración.

---

## Paso 2 – Cree el primer flujo de inicio de sesión del broker {#step-2--create-the-first-broker-login-flow}

En este paso creará directamente el flujo `sequent first broker login multivalue`.

1. Vaya a **Authentication** → **Flows**.
2. Haga clic en crear flujo (**create flow**).
3. Asigne al nuevo flujo un nombre descriptivo, como `sequent first broker login multivalue`.

---

## Paso 3 – Añada el autenticador personalizado al flujo {#step-3--add-the-custom-authenticator-to-the-flow}

1. Abra el flujo duplicado.
2. Haga clic en **Add step** dentro del subflujo adecuado.
3. Busque **Custom Attribute IdP Identity Linking** y añádalo.
4. Establezca el requisito en **REQUIRED** si quiere que el flujo falle cuando no se encuentre ninguna coincidencia, o en **ALTERNATIVE** si quiere que el autenticador llame a `attempted()` cuando no se encuentre ningún usuario coincidente, lo que permite ejecutar el siguiente paso (por ejemplo, para crear un usuario si no existe).
5. Haga clic en **⚙ Config** (icono de engranaje) junto al nuevo paso para configurarlo:

   | Parámetro | Descripción | Predeterminado |
   |---|---|---|
   | **IdP Claim** | Nombre de la notificación o del atributo que se leerá de la identidad de IdP entrante. Use nombres conocidos (`email`, `username`, `id`/`sub`, `firstname`, `lastname`) o un atributo personalizado asignado (por ejemplo, `SAFE_ID`). | `email` |
   | **User Attribute** | Atributo de usuario de Keycloak (de múltiples valores) en el que se buscará el valor de la notificación. | `linked_idp_identities` |

6. Haga clic en **Save**.

---

## Paso 4 – Asocie el nuevo flujo al proveedor de identidad {#step-4--bind-the-new-flow-to-the-identity-provider}

1. Vaya a **Identity Providers** y seleccione el IdP que quiere configurar.
2. En el desplegable **First Login Flow** (o **First Broker Login Flow**), seleccione el flujo duplicado que creó en el Paso 2.
3. Haga clic en **Save**.

---

## Paso 5 – Asigne la notificación personalizada del token del IdP {#step-5--map-the-custom-claim-from-the-idp-token}

Si la notificación que quiere usar (por ejemplo, `SAFE_ID`) no es un campo estándar de OIDC o de SAML, añada un mapeador de atributos en el IdP:

1. En la configuración del IdP, abra la pestaña **Mappers**.
2. Haga clic en **Add mapper**.
3. Establezca **Mapper type** en **Attribute Importer** (para OIDC) o en **SAML Attribute** (para SAML).
4. Asigne el nombre de la notificación del IdP al nombre del atributo de Keycloak que coincida con lo que estableció en **IdP Claim** (por ejemplo, `SAFE_ID`).
5. Haga clic en **Save**.

---

## Resumen del comportamiento {#behavior-summary}

| Escenario | Acción del autenticador |
|---|---|
| Falta la configuración | Pasa al siguiente paso (`attempted`). |
| La notificación del IdP está vacía o no existe | Pasa al siguiente paso (`attempted`). |
| No se encuentra ningún usuario con el valor del atributo | Pasa al siguiente paso (`attempted`). |
| Se encuentra exactamente un usuario | Vincula la identidad del IdP a ese usuario y tiene éxito. |
| Se encuentra más de un usuario | Hace fallar el flujo con `IDENTITY_PROVIDER_ERROR` para evitar una vinculación ambigua. |
