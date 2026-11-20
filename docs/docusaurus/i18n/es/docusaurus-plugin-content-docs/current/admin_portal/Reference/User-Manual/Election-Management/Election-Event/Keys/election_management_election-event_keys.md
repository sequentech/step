---
id: election_management_election_event_keys
title: Claves
description: "La pestaña Claves contiene las ceremonias de claves del proceso electoral. Una ceremonia de claves crea la clave pública que cifra las papeletas y da a cada fideicomisario un fragmento de la clave privada."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

La pestaña **Claves** contiene las ceremonias de claves del proceso electoral. Una ceremonia de
claves crea la clave pública que cifra las papeletas y da a cada fideicomisario un fragmento de la
clave privada. El administrador electoral crea la ceremonia. Después, cada fideicomisario guarda y
comprueba su fragmento. Para el procedimiento completo, consulte
[Realizar la ceremonia de claves](../../../../../procedures/04-keys.md).

## Abrir la pestaña {#open-the-tab}

1. En el menú de la izquierda, haga clic en el proceso electoral.
2. Haga clic en la pestaña **Claves**.

La pestaña se muestra con el permiso **Ver Claves del Evento Electoral** y uno de **Administrar
Ceremonia de Claves** o **Ceremonia de Fideicomisario**. Para crear una ceremonia, su rol necesita
**Crear Ceremonia**. La pestaña está oculta cuando el proceso electoral está confinado.

## Lista de ceremonias {#list-of-ceremonies}

Cuando no hay ninguna ceremonia, la pestaña muestra "Ninguna Ceremonia de Claves aún." y **Crear
Ceremonia de Claves**. Cuando hay ceremonias, la barra de herramientas tiene **Añadir**.

| Columna | Contenido |
| --- | --- |
| **Iniciada en** | La fecha y la hora de la creación. |
| **Estado** | El estado de la ceremonia. La tabla de abajo da los valores. |
| **Etiquetas de Permisos** | Las etiquetas de permiso de la ceremonia. Consulte [Etiquetas de permiso](../../../../../Tutorials/admin_portal_tutorials_permission-labels.md). |
| **Trustees** (fideicomisarios) | Los fideicomisarios de la ceremonia. |
| **Acciones** | El icono de ver (administrador) o el icono de llave (fideicomisario). |

| Estado | Significado |
| --- | --- |
| `STARTED` | La ceremonia está creada. |
| `IN_PROGRESS` | Los fideicomisarios pueden descargar y comprobar su fragmento de clave. |
| `SUCCESS` | Todos los fideicomisarios han comprobado su fragmento de clave. |
| `CANCELLED` | La ceremonia está cancelada. |

## Crear una ceremonia (administrador) {#create-a-ceremony-administrator}

El asistente tiene tres pasos: **Configurar**, **Ceremonia** y **Finished** (finalizado).

| Campo | Uso |
| --- | --- |
| **Umbral** | El número mínimo de fideicomisarios para el recuento. El valor por defecto es 2. Debe ser 2 o más, y no más que el número de fideicomisarios. |
| **Autoridades** | Los fideicomisarios de la ceremonia. Use **Filtrar Autoridades** para buscar un fideicomisario. Seleccione al menos tantos fideicomisarios como el umbral. |
| **Elección** | Una elección, o vacío para **Todas las Elecciones**. Solo se muestran las elecciones sin ceremonia de claves. |
| **Crear Ceremonia de Claves** | Abre la pregunta "¿Estás seguro de que quieres Crear una Ceremonia de Claves?". Haga clic en **Sí, Crear Ceremonia de Claves**. |

La página **Progreso de Ceremonia de Claves** muestra **Estado: ...**, la tabla de fideicomisarios
y los **Logs** (registros). La tabla de fideicomisarios tiene las columnas **Nombre de Autoridad**,
**Fragmento de Clave Generado**, **Fragmento Privado de Clave Descargado** y **Fragmento Privado de
Clave Comprobado**.

:::danger ADVERTENCIA
La versión 9.0 no tiene ningún botón para cancelar o reiniciar una ceremonia de claves. Compruebe
el umbral, los fideicomisarios y la elección antes de confirmar.
:::

## Participar en una ceremonia (fideicomisario) {#take-part-in-a-ceremony-trustee}

Un fideicomisario ve el mensaje "Ha sido invitado a participar a una Ceremonia de Claves." El
enlace del mensaje no funciona en la versión 9.0. Haga clic en el icono de llave de la columna
**Acciones**.

El asistente del fideicomisario tiene cuatro pasos: **Comienzo**, **Descargar**, **Comprobar** y
**Finalizado**.

| Paso | Lo que hace el fideicomisario |
| --- | --- |
| **Comienzo** | Lee la página **Ceremonia de Claves de Autoridad** y hace clic en **Siguiente**. Si se muestra "Esperando a la Generación de Claves..", espera hasta que **Siguiente** esté disponible. |
| **Descargar** | Hace clic en **Descargar tu Clave Privada Encriptada**, copia el archivo en dos dispositivos de almacenamiento, selecciona **Primera copia de seguridad realizada** y **Segunda copia de seguridad realizada**, y hace clic en **Confirmar copias de seguridad y Continuar**. |
| **Comprobar** | Sube una copia de seguridad en la página **Verifica tus Copias de Seguridad de tu Clave Privada Encriptada**. Espera a "Copia de seguridad verificada correctamente." y hace clic en **Siguiente**. |
| **Finalizado** | Ve el progreso de la ceremonia. |

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| "Seleccionaste un umbral de ... pero debe estar entre ... y ...." | Escriba un umbral entre 2 y el número de fideicomisarios. |
| "Seleccionaste sólo ... autoridades, pero debe seleccionar al menos ...." | Seleccione más fideicomisarios o escriba un umbral menor. |
| "Error creando Ceremonia de Claves: ..." con "there's already an existing running ceremony ..." | La elección, o todas las elecciones, ya tienen una ceremonia de claves. Contacte con el soporte de Sequent. |
| "No se puede crear la ceremonia de claves: falta una o más etiquetas de permisos." | Su cuenta no tiene la etiqueta de permiso de la elección. Consulte [Etiquetas de permiso](../../../../../Tutorials/admin_portal_tutorials_permission-labels.md). |
| "Copa de Seguridad de la Clave Encriptada Privada inválida, por favor inténtalo de nuevo" | Suba el archivo desde el otro dispositivo de almacenamiento. |
| "La descarga de la clave privada ya no está disponible porque la ceremonia ha avanzado." | Use sus copias de seguridad. No puede descargar el archivo de nuevo. |
| "Tu clave privada ya se había descargado y verificado." | Ya ha hecho su parte. No es necesaria ninguna acción. |
| El paso de comprobación muestra `keysGeneration.checkStep.noFileSelected`. | No se seleccionó ningún archivo. Suba el archivo de nuevo. Este texto no está traducido en la versión 9.0. |

## Páginas relacionadas {#related-pages}

- [Realizar la ceremonia de claves](../../../../../procedures/04-keys.md)
- [Configuración: Fideicomisarios](../../../Settings/Trustees/admin_portal_reference_user-manual_settings_trustees.md)
- [Recuento](../Tally/election_management_election-event_tally.md)
