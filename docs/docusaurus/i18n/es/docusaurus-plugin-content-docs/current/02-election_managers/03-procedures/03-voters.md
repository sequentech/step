---
title: Añadir los votantes
sidebar_position: 3
description: "Este procedimiento crea la lista de votantes del proceso electoral: prepara el archivo de votantes, lo importa, añade o cambia votantes individuales y comprueba la lista."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Añadir los votantes

Este procedimiento crea la lista de votantes del proceso electoral. Usted prepara el archivo de votantes, lo importa, añade o cambia votantes individuales y comprueba la lista. Lo hace el administrador electoral. Puede añadir votantes hasta que termine el período de votación.

## Antes de empezar {#before-you-start}

- El proceso electoral y sus áreas existen. Consulte [Crear el proceso electoral](02-event.md).
- Tiene la lista de votantes de su fuente oficial, como archivo CSV o como los datos de cada votante.
- Sabe a qué área pertenece cada votante.
- Sabe cómo inician sesión los votantes: con una contraseña, con un código de un solo uso u otro método de acceso. Consulte [Tutoriales](../01-tutorials/101-admin_portal_tutorials_multi-attribute-password-login.md) para los métodos de acceso.

:::caution PRECAUCIÓN
El archivo de votantes contiene datos personales. Guárdelo en un ordenador protegido. Borre las copias de trabajo después de la importación, según las normas de protección de datos.
:::

## 1. Preparar el archivo de votantes {#1-prepare-the-voter-file}

El portal de administración no proporciona un archivo de ejemplo. Cree el archivo con la tabla y el ejemplo siguientes.

1. Cree un archivo CSV con una fila para cada votante.
2. Ponga el encabezado en la primera fila. Use los nombres de columna de la tabla.
3. Use una coma como separador. Para un separador de tabulación, dé al archivo la extensión `.tsv`.
4. Guarde el archivo.

| Columna | Contenido |
| --- | --- |
| `username` | El nombre de usuario del votante. Debe ser único en el proceso electoral. |
| `email` | La dirección de correo electrónico. |
| `first_name`, `last_name` | El nombre del votante. |
| `area_name` | El nombre del área del votante. Debe ser igual que en la pestaña **Áreas**. |
| `enabled` | `TRUE` o `FALSE`. Un votante con `FALSE` no puede iniciar sesión. |
| `password` | Opcional. La contraseña del votante. |
| `sequent.read-only.mobile-number` | Opcional. El número de móvil, para los mensajes de texto. |
| `vote-weight` | Opcional. Un número entero positivo. Úselo solo si el proceso electoral usa votación ponderada. |
| `authorized-election-ids` | Opcional. Las elecciones en las que puede votar el votante. Escriba el **ID externo** de cada elección, o su ID si no tiene ID externo. Separe los valores con `\|`. Si lo deja vacío, el votante puede votar en las elecciones de su área. |

Las demás columnas se convierten en atributos del votante, por ejemplo una fecha de nacimiento para el inicio de sesión. Use el nombre del atributo como encabezado, no la etiqueta que muestra el portal de administración.

Ejemplo con dos votantes. El segundo votante solo puede votar en las elecciones con los ID externos `election-1` y `election-2`:

```csv
username,email,first_name,last_name,area_name,enabled,authorized-election-ids
voter001,voter001@example.com,Ana,Lopez,North,TRUE,
voter002,voter002@example.com,Ben,Smith,South,TRUE,election-1|election-2
```

:::caution PRECAUCIÓN
Un valor de `authorized-election-ids` que no coincide con ninguna elección se ignora sin error. Si ningún valor coincide, el votante no puede votar en ninguna elección. Compruebe los ID externos de las elecciones antes de importar el archivo.
:::

**Resultado esperado:** tiene un archivo CSV con una fila para cada votante y el encabezado correcto.

## 2. Importar los votantes {#2-import-the-voters}

1. En la pestaña **Votantes**, haga clic en **Importar**.
2. Escriba el hash del archivo en **Verificación de Integridad (SHA 256)**.
3. Suba el archivo CSV.
4. Haga clic en **Importar**.

**Resultado esperado:** aparece el mensaje "Importación de Votantes lanzada en segundo plano con éxito."

5. Haga clic en la pestaña **Tareas**. Espere hasta que la tarea de importación termine.
6. Si la tarea muestra un error, abra la tarea y lea el error. Corrija el archivo e impórtelo de nuevo.

**Resultado esperado:** los votantes aparecen en la pestaña **Votantes**.

:::note NOTA
Si algunas columnas son campos secretos del votante, su cuenta necesita el permiso para escribirlos. Consulte [Importar la lista de votantes](../01-tutorials/08-admin_portal_tutorials_import-voters.md).
:::

## 3. Añadir o cambiar un votante {#3-add-or-change-one-voter}

Para añadir un votante:

1. En la pestaña **Votantes**, haga clic en **Crear Votante**. Si la lista ya tiene votantes, haga clic en **Añadir**. Se abre el panel **Crear votante**.
2. Escriba los campos del votante.
3. Seleccione el **Área** del votante.
4. Asegúrese de que **Habilitado** está seleccionado.
5. Si el votante inicia sesión con una contraseña, escríbala en **Contraseña** y en **Repetir Contraseña**.
6. Haga clic en **Guardar**.

**Resultado esperado:** aparece el mensaje "Votante creado". El votante aparece en la lista.

Para cambiar un votante, haga clic en **Editar** en la fila del votante. Cambie los campos, haga clic en **Guardar**, lea los cambios y haga clic en **Confirmar cambios**.

:::note NOTA
Para cambiar un votante que ya ha votado, su cuenta necesita el permiso **Editar a los votantes que votaron**. Sin este permiso, el portal de administración no le deja cambiar ese votante.
:::

## 4. Comprobar la lista de votantes {#4-check-the-voter-list}

1. Haga clic en la pestaña **Panel de Control**. Compare el número de votantes con derecho a voto con el número de votantes de su fuente.
2. Haga clic en la pestaña **Votantes**. Filtre la lista por **Área**. Compare los votantes de cada área con su fuente.
3. Abra algunos votantes al azar. Asegúrese de que sus datos son correctos.

**Resultado esperado:** la lista de votantes coincide con su fuente.

## 5. Enviar a los votantes sus datos de acceso (si es necesario) {#5-send-the-voters-their-access-data-if-necessary}

Si los votantes necesitan un mensaje con sus datos de acceso antes del período de votación:

1. En la pestaña **Votantes**, haga clic en **Enviar**. Para enviar solo a algunos votantes, selecciónelos antes en la lista.
2. Seleccione los destinatarios: **Todos**, **Los que no votaron**, **Los que ya votaron** o los votantes seleccionados.
3. Seleccione el método de comunicación y la plantilla.
4. Envíe el mensaje.

Para ver todas las opciones, consulte [Comunicación con votantes](../01-tutorials/15-admin_portal_tutorials_voter-communication.md).

Para dar a un votante una carta impresa con nuevos datos de acceso, haga clic en **Carta de información para el votante** en la fila del votante. El portal de administración crea un PDF cifrado con una contraseña nueva.

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| La tarea de importación falla. | Abra la tarea en la pestaña **Tareas** para ver el error. Compruebe la fila de encabezado, los nombres de área y el separador, y vuelva a importar. |
| Un votante no tiene área. | El valor de `area_name` no coincide con ningún área. Corrija el nombre del área y vuelva a importar, o haga clic en **Editar** y seleccione el **Área**. |
| La importación rechaza una columna secreta. | Su cuenta necesita el permiso para escribir los campos secretos del votante. Pregunte al administrador del inquilino. |
| El número de votantes no es el que espera. | Busque nombres de usuario duplicados y filas vacías en el archivo. Compare los votantes de cada área con su fuente. |
| "Votante creado, pero no se pudo establecer su contraseña" | La contraseña no cumple la política de contraseñas del proceso electoral. Haga clic en **Cambiar contraseña** en la fila del votante y escriba una contraseña válida. |

## Más información {#more-information}

- [Crear votantes](../01-tutorials/07-admin_portal_tutorials_create-voters.md)
- [Importar la lista de votantes](../01-tutorials/08-admin_portal_tutorials_import-voters.md)
- [Configurar la política de contraseñas](../01-tutorials/05-admin_portal_tutorials_password-policy.md)
- [Comunicación con votantes](../01-tutorials/15-admin_portal_tutorials_voter-communication.md)
- [Proceso electoral: Votantes](../02-reference/02-election-event/05-election_management_election-event_voters.md)
- [Elección: Votantes](../02-reference/03-election/04-election_management_election_voters.md)
- [Proceso electoral: Aprobaciones](../02-reference/02-election-event/14-election_management_election-event_approvals.md)

**Siguiente:** [Realizar la ceremonia de claves](04-keys.md).
