---
id: admin_portal_tutorials_export-data
title: Exportar datos
description: "Exporte los votantes de un proceso electoral a un archivo CSV, o inclúyalos en un archivo del proceso electoral, con o sin los campos secretos del votante."
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

El portal de administración ofrece dos rutas para exportar votantes:

- **Votantes > Exportar** crea un CSV con los votantes del proceso electoral actual.
- **Evento Electoral > Exportar** crea un archivo del proceso electoral y, opcionalmente, puede incluir un CSV de votantes.

## Campos secretos del votante {#secret-voter-fields}

Los campos secretos del votante se omiten de las exportaciones por defecto. Así se evita que los valores de almacenamiento cifrados aparezcan en un CSV cuando el usuario no pidió acceso a sus valores en texto plano.

Para incluir campos secretos descifrados en un CSV de votantes independiente, seleccione **Incluir campos secretos descifrados del votante**. El usuario que inicia la exportación debe tener el permiso `voter-secret-attribute-read`. La tarea de exportación comprueba esta autorización antes de leer los campos.

Un archivo del proceso electoral incluye campos secretos descifrados del votante solo cuando se cumplen todas estas condiciones:

1. **Incluir Votantes** está seleccionado.
2. **Cifrar con Contraseña** está marcado, así que el archivo resultante está protegido con contraseña.
3. El usuario que inicia la exportación tiene el permiso `voter-secret-attribute-read`.

La casilla **Cifrar con Contraseña** debe seleccionarse de forma explícita. Elegir otra opción de exportación que requiera cifrado del archivo (como informes, solicitudes o datos del tablón) no activa la exportación de los secretos de los votantes.

Si el usuario no tiene ese permiso, la exportación del proceso electoral incluye los campos ordinarios del votante, pero omite todos los campos secretos. Los archivos que contienen secretos de votantes descifrados se marcan como sensibles, así que descargarlos también requiere el permiso de lectura de secretos.

La opción **Archivos S3** también puede incluir informes con secretos cuando **Cifrar con Contraseña** está seleccionado de forma explícita y el operador tiene el permiso de lectura de secretos, aunque **Incluir Votantes** esté desactivado. Sin ambas salvaguardas, estos documentos se excluyen. Los objetos sin metadatos de documento confirmados siempre se excluyen. Las exportaciones CSV de secretos siguen siendo descargas en texto plano con una advertencia explícita. Guárdelas y transfiéralas con seguridad. Importar estas columnas CSV todavía requiere el permiso de escritura de secretos, y cifra cada valor para el proceso electoral y el votante de destino.
