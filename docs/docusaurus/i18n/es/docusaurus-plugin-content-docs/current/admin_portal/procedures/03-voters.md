---
title: Añadir los votantes
sidebar_position: 3
description: "Este procedimiento crea la lista de votantes del proceso electoral. Usted prepara el archivo de votantes, lo importa, añade o cambia votantes individuales y comprueba la lista. El administrador electoral hace este procedimiento."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Añadir los votantes

Este procedimiento crea la lista de votantes del proceso electoral. Usted prepara el archivo de
votantes, lo importa, añade o cambia votantes individuales y comprueba la lista. El administrador
electoral hace este procedimiento.

## Antes de empezar {#before-you-start}

- El proceso electoral, sus preguntas y sus áreas existen. Consulte
  [Crear el proceso electoral](02-event.md).
- Usted tiene la lista de votantes de su fuente oficial, como archivo CSV o como los datos de cada
  votante.
- Usted conoce el área de cada votante. Los nombres de las áreas deben ser los mismos que en la
  pestaña **Áreas**.
- Su cuenta tiene los permisos para crear e importar votantes. Si el botón **Añadir** o
  **Importar** no está en la pestaña **Votantes**, pregunte al administrador del inquilino.

:::caution PRECAUCIÓN
El archivo de votantes contiene datos personales. Guárdelo en un ordenador protegido. Borre las
copias de trabajo después de la importación, como exigen sus normas de protección de datos.
:::

## 1. Preparar el archivo de votantes {#1-prepare-the-voter-file}

La versión 9.0 no ofrece un archivo de ejemplo en el portal de administración. Cree el archivo a
partir de la tabla siguiente.

1. Cree un archivo CSV con una fila para cada votante.
2. Ponga el encabezado en la primera fila. Use los nombres de columna de la tabla.
3. Use una coma como separador. Para un separador de tabulación, dé al archivo la extensión
   `.tsv`.
4. Guarde el archivo.

| Columna | Contenido |
| --- | --- |
| `username` | El nombre de usuario del votante. Use un valor diferente para cada votante. |
| `email` | Opcional. La dirección de correo electrónico. |
| `first_name`, `last_name` | Opcional. El nombre del votante. |
| `area_name` | El nombre del área del votante. Debe ser el mismo que en la pestaña **Áreas**. |
| `enabled` | `TRUE` o `FALSE`. Un votante con `FALSE` no puede iniciar sesión. |
| `password` | Opcional. La contraseña del votante. |
| `sequent.read-only.mobile-number` | Opcional. El número de móvil, para los mensajes de texto. |
| `authorized-election-ids` | Opcional. Las elecciones en las que el votante puede votar. Escriba el **Alias** de cada elección, o su ID si no tiene alias. Separe los valores con `\|`. Si lo deja vacío, el votante puede votar en las elecciones de su área. |

Una columna con otro nombre se convierte en un atributo del votante. Un nombre de columna solo
puede contener letras, dígitos, `.`, `_` y `-`.

Ejemplo con dos votantes. El segundo votante solo puede votar en las elecciones con los alias
`election-1` y `election-2`:

```csv
username,email,first_name,last_name,area_name,enabled,authorized-election-ids
voter001,voter001@example.com,Ana,Lopez,North,TRUE,
voter002,voter002@example.com,Ben,Smith,South,TRUE,election-1|election-2
```

:::caution PRECAUCIÓN
Un valor de `authorized-election-ids` que no coincide con ninguna elección se ignora sin error. Si
ningún valor coincide, el votante no puede votar en ninguna elección. Compruebe los alias de las
elecciones antes de importar el archivo.
:::

**Resultado esperado:** usted tiene un archivo CSV con una fila para cada votante y el encabezado
correcto.

## 2. Importar los votantes {#2-import-the-voters}

1. Abra el proceso electoral.
2. Haga clic en la pestaña **Votantes**.
3. Haga clic en **Importar**. Se abre el panel **Importar votantes**.
4. Escriba el hash SHA-256 del archivo en **Verificación de Integridad (SHA 256)**.
5. Suelte el archivo en el área de carga, o haga clic en **Cargar fichero** y seleccione el
   archivo.
6. Espere el mensaje "Fichero subido al servidor - pero no importado aún".
7. Haga clic en **Importar**.

**Resultado esperado:** aparece el mensaje "Importación de Votantes lanzada en segundo plano con
éxito."

:::note
El área de carga muestra "Formatos soportados: txt". Este texto no es correcto para la
importación de votantes. Cargue el archivo CSV o TSV.
:::

8. Haga clic en la pestaña **Tareas**. Busque la tarea **Importar usuarios**.
9. Espere hasta que su estado sea `SUCCESS`.
10. Si el estado es `FAILED`, haga clic en el icono de ver de la tarea y lea los registros.
    Corrija el archivo e impórtelo de nuevo.

**Resultado esperado:** los votantes aparecen en la pestaña **Votantes**.

:::caution PRECAUCIÓN
Si no escribe el hash, el portal de administración pregunta "Importar Sin Verificación de
Integridad?". Haga clic en **Volver** y escriba el hash. Sin el hash, no puede saber si alguien ha
cambiado el archivo.
:::

## 3. Añadir o cambiar un votante {#3-add-or-change-one-voter}

Para añadir un votante:

1. En la pestaña **Votantes**, haga clic en **Añadir**. Si la lista está vacía, haga clic en
   **Crear votante**.
2. Escriba los campos del votante.
3. Asegúrese de que **Habilitado** está seleccionado.
4. En **Área**, escriba parte del nombre del área y después seleccione el área.
5. Si el votante inicia sesión con una contraseña, escríbala en **Contraseña:** y en **Repetir
   Contraseña:**.
6. Si el votante debe cambiar la contraseña en el primer inicio de sesión, deje **Temporal**
   activado.
7. Haga clic en **Guardar**.

**Resultado esperado:** aparece el mensaje "Votante creado". El votante aparece en la lista.

Para cambiar un votante:

1. En la fila del votante, abra el menú de acciones y haga clic en **Editar**.
2. Cambie los campos.
3. Haga clic en **Guardar**.

**Resultado esperado:** aparece el mensaje "Votante editado".

Para dar una nueva contraseña a un votante, haga clic en **Cambiar contraseña** en el menú de
acciones del votante.

## 4. Comprobar la lista de votantes {#4-check-the-voter-list}

1. Haga clic en la pestaña **Panel de Control**. Compare **Electores** con el número de votantes
   de su fuente.
2. Haga clic en la pestaña **Votantes**. Haga clic en **Añadir filtro** y después añada el filtro
   **Área**.
3. Compare los votantes de cada área con su fuente.
4. Busque votantes con `-` en la columna **Área**. Estos votantes no tienen área.
5. Abra algunos votantes al azar. Asegúrese de que sus datos son correctos.

**Resultado esperado:** la lista de votantes coincide con su fuente.

## 5. Enviar un mensaje a los votantes (si es necesario) {#5-send-the-voters-a-message-if-necessary}

Haga este paso solo si los votantes deben recibir un mensaje antes del período de votación, por
ejemplo sus datos de acceso. Su inquilino necesita una plantilla del tipo correcto. Consulte
[Plantillas](../Reference/User-Manual/Templates/admin_portal_reference_user-manual_templates.md).

1. En la pestaña **Votantes**, haga clic en **Enviar**. Para enviar solo a algunos votantes,
   selecciónelos en la lista y después haga clic en **Enviar** encima de la lista.
2. En **Audiencia**, seleccione **Todos**, **Los que no votaron**, **Los que ya votaron** o los
   votantes seleccionados.
3. En **Calendario**, deje **Enviar ahora** activado, o escriba la fecha y la hora de inicio.
4. En **Método de Comunicación**, seleccione el **Método de Plantillas**, el **Tipo de
   Comunicación** y el **Alias de la Plantilla**.
5. Compruebe el texto del mensaje.
6. Haga clic en **Enviar Notificación**.

**Resultado esperado:** aparece el mensaje "Notificación programada/enviada con éxito".

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| La tarea **Importar usuarios** muestra `FAILED`. | Abra la tarea en la pestaña **Tareas** y lea los registros. Compruebe la fila de encabezado, los nombres de columna y el separador, y después importe de nuevo. |
| Un votante muestra `-` en la columna **Área**. | El valor de `area_name` no coincidía con ningún área. En el menú de acciones del votante, haga clic en **Editar** y seleccione el **Área**. |
| "Importar Sin Verificación de Integridad?" | Haga clic en **Volver** y escriba el hash SHA-256 del archivo. |
| "Error Importando Votantes." | La importación no se ha iniciado. Cargue el archivo de nuevo. Si el error continúa, contacte con el soporte de Sequent. |
| "Las contraseñas deben de coincidir" | Escriba la misma contraseña en **Contraseña:** y en **Repetir Contraseña:**. |
| "Error creando votante" | Compruebe los campos obligatorios, por ejemplo el nombre de usuario, y el área. Un nombre de usuario solo se puede usar una vez. |
| El número de votantes no es el que espera. | Busque nombres de usuario duplicados y filas vacías en el archivo. Compare los votantes de cada área con su fuente. |

## Más información {#more-information}

- [Proceso electoral: Votantes](../Reference/User-Manual/Election-Management/Election-Event/Voters/election_management_election-event_voters.md)
- [Elección: Votantes](../Reference/User-Manual/Election-Management/Election/Voters/election_management_election_voters.md)
- [Proceso electoral: Áreas](../Reference/User-Manual/Election-Management/Election-Event/Areas/election_management_election-event_areas.md)
- [Proceso electoral: Aprobaciones](../Reference/User-Manual/Election-Management/Election-Event/Approvals/election_management_election-event_approvals.md)
- [Proceso electoral: Tareas](../Reference/User-Manual/Election-Management/Election-Event/Tasks/election_management_election-event_tasks.md)
- [Plantillas](../Reference/User-Manual/Templates/admin_portal_reference_user-manual_templates.md)

**Siguiente:** [Realizar la ceremonia de claves](04-keys.md).
