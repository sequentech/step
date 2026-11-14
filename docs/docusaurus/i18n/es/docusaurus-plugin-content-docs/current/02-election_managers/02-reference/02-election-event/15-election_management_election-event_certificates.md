---
id: election_management_election_event_certificates
title: Certificados
description: "Los votantes pueden autenticarse con certificados de cliente emitidos por una autoridad de certificación (CA). Para que esto funcione, las CA correspondientes deben subirse al proceso electoral. Distintos procesos electorales pueden tener conjuntos de CA diferentes."
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
-->

Los votantes pueden autenticarse con certificados de cliente emitidos por una autoridad de certificación (CA). Para que esto funcione, las CA correspondientes deben subirse al proceso electoral. Distintos procesos electorales pueden tener conjuntos de CA diferentes configurados.

---

## Activar la autenticación con certificado {#enabling-certificate-authentication}

La autenticación con certificado está desactivada por defecto. Para activarla:

1. Vaya a la pestaña **Datos** del proceso electoral.
2. Abra la sección **Configuración Avanzada**.
3. Active la opción **Política de certificado digital del votante**.

Una vez activada, aparecerá el botón **CERTIFICADO DIGITAL** en la página de inicio de sesión del portal de votación. Así, los votantes podrán autenticarse con su certificado de cliente.

---

## Gestionar los certificados {#managing-certificates}

La pestaña **Certificados** le permite gestionar los certificados de las CA del proceso electoral.

Los certificados deben estar en **formato PEM**. Si importa un archivo PEM que contiene varios certificados, se importan todos a la vez.

Acciones disponibles:

- **Importar**: suba un archivo PEM con uno o más certificados de CA.
- **Exportar**: descargue todos los certificados como un archivo PEM, o seleccione uno o varios y haga clic en **Exportar**.
- **Eliminar**: elimine uno o varios certificados seleccionados.
- **Ver**: muestre los detalles del certificado en un formato legible.

---

## Configuración de Keycloak {#keycloak-configuration}

Subir certificados de CA al portal de administración es solo una parte de la configuración. Keycloak también debe configurarse por separado para clasificar y autenticar a los votantes según esas CA.

### Flujo de navegador de certificados {#certificate-browser-flow}

La autenticación la gestiona el **flujo de navegador de certificados** de Keycloak. Este flujo contiene dos subflujos de primer nivel:

- **Credentials subflow**: clasifica el certificado y resuelve la identidad del votante.
- **Deny subflow**: gestiona los casos de error (consulte [Casos de error](#error-cases) más abajo).

### Subflujos clasificadores {#classifier-subflows}

Dentro del **Credentials subflow**, el autenticador `x509-cert-classifier` se ejecuta primero y establece una nota de autenticación `cert-type` según el CN del certificado. Después, cada CA se gestiona con un **subflujo condicional** que comprueba `cert-type` y, si coincide, ejecuta el autenticador `x509-user-resolution` para buscar al votante.

Al crear el proceso electoral se crean por defecto dos subflujos de ejemplo. Puede editarlos, eliminarlos o usarlos como referencia:

- **Dev Sequent**: coincide con los certificados con `cert-type = "Sequent Dev CA"`.
- **Other CA**: coincide con los certificados con `cert-type = "Other CA"`.

Cada subflujo clasificador contiene:

1. Un paso **Condition - auth note** que comprueba que `cert-type` es igual al CN de la CA esperada (por ejemplo, `"Sequent Dev CA"`).
2. Un paso **X509/Validate Username Form** (`x509-user-resolution`) configurado para buscar votantes por SubjectDN con la expresión regular `(?:CN=)([^,]*)`.

### Añadir un subflujo para una nueva CA {#adding-a-new-ca-subflow}

Por cada nueva CA que importe en la pestaña **Certificados**, añada un subflujo condicional correspondiente en el **Credentials subflow**:

1. En la consola de administración de Keycloak, vaya al realm del proceso electoral → **Authentication** → **flujo de navegador de certificados** → **Credentials subflow**.
2. Añada un nuevo **subflujo condicional** (por ejemplo, con el nombre de la CA).
3. Dentro del nuevo subflujo, añada:
   - Un paso **Condition - auth note** con `cert-type` = el CN de la nueva CA (establezca `value-is-regex` en `false`).
   - Un paso **X509/Validate Username Form** configurado para buscar votantes por SubjectDN.
4. Actualice la condición del subflujo **No classiffier match**: edite la configuración del autenticador `NoClassifierMatch condition` y añada el CN de la nueva CA a la expresión regular negada. Por ejemplo, si la expresión existente es `(Sequent Dev CA)|(Other CA)`, amplíela a `(Sequent Dev CA)|(Other CA)|(New CA CN)`.

:::tip
La condición `NoClassifierMatch condition` usa una expresión regular negada. Cualquier certificado cuyo `cert-type` no coincida con la expresión activará el autenticador de denegación de acceso, lo que produce un error `accessDenied`. Mantenga esta expresión regular sincronizada con todos los subflujos clasificadores configurados.
:::

---

## Casos de error {#error-cases}

El **Deny subflow** gestiona tres escenarios de error. Los votantes verán el mensaje correspondiente en la página de inicio de sesión:

| Clave del error | Mensaje | Causa |
|---|---|---|
| `certNotProvided` | Error de autenticación: No se proporcionó ningún certificado | El votante no presentó ningún certificado de cliente. |
| `userNotFound` | Error de autenticación: Se ha detectado un certificado válido, pero no se encontró ningún votante o usuario coincidente | El certificado era válido y estaba clasificado, pero ningún registro de votante coincidió. |
| `accessDenied` | Error de autenticación: Certificado inválido, acceso denegado | Ningún subflujo clasificó el certificado (la CA emisora no está configurada o no es de confianza). |

Estos mensajes se definen en los archivos de traducción del tema de Keycloak en:

```
packages/keycloak-extensions/sequent-theme/src/main/resources/theme/sequent.voting-portal/login/messages/messages_<lang>.properties
```

Se pueden sobrescribir por realm en la consola de administración de Keycloak, en **Realm settings** → **Localization**.
