---
title: Crear el proceso electoral
sidebar_position: 2
description: "Este procedimiento crea el proceso electoral y su contenido: las elecciones, las preguntas, los candidatos y las áreas. El administrador electoral hace este procedimiento."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Crear el proceso electoral

Este procedimiento crea el proceso electoral y su contenido: las elecciones, las preguntas, los
candidatos y las áreas. El administrador electoral hace este procedimiento. Los votantes se
añaden en el procedimiento siguiente, [Añadir los votantes](03-voters.md).

## Antes de empezar {#before-you-start}

- El inquilino está listo. Consulte [Configurar el inquilino](01-tenant.md).
- Usted tiene el contenido de la papeleta: las elecciones, las preguntas y los candidatos, en
  todos los idiomas de la elección.
- Usted tiene la lista de áreas y las preguntas de cada área.
- Si importa un proceso electoral: usted tiene el archivo y su hash SHA-256.

El orden de los pasos es importante. Cree las áreas después de las preguntas. Añada los votantes
después de las áreas.

## 1. Crear el proceso electoral {#1-create-the-election-event}

1. En el menú de la izquierda, haga clic en el icono **+** junto a **Procesos Electorales**.
2. Haga clic en **Crear un Evento Electoral**.
3. Escriba el **Nombre** del proceso electoral.
4. Escriba la **Descripción**.
5. Haga clic en **Guardar**.

**Resultado esperado:** el panel de la tarea "Crear Evento Electoral" muestra **SUCCESS**.
Aparece el mensaje "Evento Electoral creado" y el portal de administración abre el nuevo proceso
electoral. El proceso electoral muestra estas pestañas:

| Pestaña | Uso |
| --- | --- |
| **Panel de Control** | Las cifras del proceso electoral. |
| **Monitoreo** | La actividad de votación. |
| **Datos** | La configuración del proceso electoral. |
| **Localización** | Los textos del proceso electoral en cada idioma. |
| **Votantes** | La lista de votantes. Consulte [Añadir los votantes](03-voters.md). |
| **Áreas** | Las áreas y sus preguntas. |
| **Claves** | La ceremonia de claves. Consulte [Realizar la ceremonia de claves](04-keys.md). |
| **Recuento** | La ceremonia de recuento y los resultados. Consulte [Realizar la ceremonia de recuento](06-tally.md). |
| **Publicar** | La publicación de la papeleta y el período de votación. Consulte [Publicar y gestionar el período de votación](05-publish.md). |
| **Tareas** | Las tareas en segundo plano, por ejemplo importaciones y exportaciones, con su estado. |
| **Logs** | El registro de las acciones en el proceso electoral. |
| **Eventos Programados** | El inicio y el fin automáticos del período de votación. |
| **Informe** | Los informes y su programación. |
| **Aprobaciones** | Las solicitudes de los votantes que se inscriben. |

### Importar un proceso electoral (alternativa) {#import-an-election-event-alternative}

Use este método para crear un proceso electoral a partir de un archivo, por ejemplo una copia de
un proceso electoral anterior.

1. En el menú de la izquierda, haga clic en el icono **+** junto a **Procesos Electorales**.
2. Haga clic en **Importar Evento Electoral**.
3. Escriba el hash del archivo en **Verificación de Integridad (SHA 256)**.
4. Suelte el archivo en el área de carga, o haga clic en **Cargar fichero**. El archivo puede ser
   un archivo `.json`, una exportación `.zip` o una exportación cifrada `.ezip`.
5. Si el archivo es un archivo `.ezip`, escriba la contraseña en **Contraseña de Descifrado**.
6. Haga clic en **Importar**.

**Resultado esperado:** aparece el mensaje "Election event imported Successfully" (proceso
electoral importado correctamente).

:::caution PRECAUCIÓN
En esta versión el portal de administración no envía al servidor el hash de un archivo de evento
electoral, por lo que la importación no lo comprueba. Antes de importar, obtenga el hash del archivo
y compárelo con el hash del nombre del archivo. Consulte
[Formatos de archivos de importación](../Reference/import-file-formats.md#election-event). Sin esta
comprobación, no puede saber si alguien ha cambiado el archivo.
:::

## 2. Configurar el proceso electoral {#2-configure-the-election-event}

1. Abra el proceso electoral.
2. Haga clic en la pestaña **Datos**.
3. En **General**, escriba el **Nombre**, el **Alias** y la **Descripción** para cada idioma.
4. En **Idiomas**, seleccione los idiomas del proceso electoral. Seleccione el idioma
   **Por defecto**.
5. En **Canales de Voto Permitidos**, seleccione **En línea**, **Kiosco** o los dos.
6. En **Diseño de la Papeleta**, defina el logotipo y las otras opciones del portal de votación.
7. En **Voting Portal Countdown Policy** (la sección de configuración avanzada), asegúrese de que
   **Estado de Confinamiento** es **No Confinado**.
8. Haga clic en **Guardar**.

**Resultado esperado:** el portal de administración guarda los datos. El menú muestra el nuevo
nombre.

:::note
**Confinado** oculta la mayoría de las pestañas del proceso electoral y detiene todas las
publicaciones. Úselo solo cuando deba congelar el proceso electoral.
:::

## 3. Crear las elecciones {#3-create-the-elections}

Haga estos pasos para cada elección.

1. En el menú de la izquierda, haga clic en la flecha junto al proceso electoral para mostrar sus
   elecciones.
2. Debajo de las elecciones, haga clic en **Crear una Elección**.
3. Escriba el **Nombre**.
4. Escriba el **ID externo**. Es el identificador de la elección en sus otros sistemas.
5. Escriba la **Descripción**.
6. Haga clic en **Guardar**.
7. Haga clic en la pestaña **Datos** de la nueva elección.
8. En **General**, escriba el nombre y la descripción para cada idioma.
9. En **Canales de Voto Permitidos**, seleccione los canales de votación de esta elección.
10. En **Configuración Avanzada**, defina **Inicializar Política de Reportes**. Seleccione
    **Requerido** si debe hacer un informe de inicialización antes de que se abra el período de
    votación.
11. En **Configuración Avanzada**, defina **Permitir Recuento**. La tabla da los valores.
12. Haga clic en **Guardar**.

| Valor de **Permitir Recuento** | Significado |
| --- | --- |
| **Permitido** | Puede iniciar el recuento en cualquier momento. |
| **No permitido** | No puede iniciar el recuento. |
| **Permitir cuando termine el periodo de votación** | Puede iniciar el recuento solo después de que se cierre el período de votación. |

**Resultado esperado:** la elección aparece debajo del proceso electoral en el menú. El menú
muestra el **ID externo** de la elección, no su nombre.

:::note
No puede cambiar el **ID externo** después de guardarlo.
:::

## 4. Crear las preguntas {#4-create-the-contests}

Haga estos pasos para cada pregunta.

1. En el menú de la izquierda, haga clic en la flecha junto a la elección para mostrar sus
   preguntas.
2. Debajo de las preguntas, haga clic en **Crear un Concurso**.
3. Escriba el **Nombre** y la **Descripción**.
4. Haga clic en **Guardar**.
5. Haga clic en la pestaña **Datos** de la nueva pregunta.
6. En **Sistema de votación de papeletas**, asegúrese de que **Voting type** (tipo de votación) es
   **Sin Preferencia** y **Counting algorithm** (algoritmo de recuento) es **Mayoría Plural**. La
   versión 9.0 solo tiene estos valores.
7. En **Diseño de la papeleta**, defina el número de votos:
   - **Min votes** (votos mínimos): el número mínimo de selecciones. Escriba `0` si el votante
     puede dejar la pregunta en blanco.
   - **Max votes** (votos máximos): el número máximo de selecciones.
   - **Winning candidates num** (número de candidatos ganadores): el número de candidatos
     ganadores.
8. En **Diseño de la papeleta**, defina **Política de Subvoto**, **Política de voto excesivo** y **Política de
   voto en blanco**. Indican al portal de votación qué hacer cuando un votante selecciona
   muy pocos candidatos, demasiados candidatos o ningún candidato.
9. Haga clic en **Guardar**.

**Resultado esperado:** la pregunta aparece debajo de la elección en el menú.

:::caution PRECAUCIÓN
El portal de administración no comprueba que el mínimo es menor que el máximo. Compruebe los
valores antes de guardar. Asegúrese de que el máximo no es mayor que el número de candidatos.
:::

## 5. Crear los candidatos {#5-create-the-candidates}

Haga estos pasos para cada candidato.

1. En el menú de la izquierda, haga clic en la flecha junto a la pregunta para mostrar sus
   candidatos.
2. Debajo de los candidatos, haga clic en **Crear un Candidato**.
3. Escriba el **Nombre**.
4. Haga clic en **Guardar**.
5. Haga clic en la pestaña **Datos** del nuevo candidato.
6. En **General**, escriba el nombre y la descripción para cada idioma.
7. En **Imagen**, añada una foto o un logotipo si es necesario.
8. Haga clic en **Guardar**.

**Resultado esperado:** el candidato aparece debajo de la pregunta en el menú.

## 6. Crear las áreas {#6-create-the-areas}

Un área indica qué preguntas ven sus votantes. Cada votante pertenece a un área.

1. Abra el proceso electoral.
2. Haga clic en la pestaña **Áreas**.
3. Haga clic en **Añadir**. Si la lista está vacía, haga clic en **Crear un Área**.
4. Escriba el **Nombre** y la **Descripción**. Los votantes se refieren al área por su **Nombre**.
5. En **Preguntas del Área**, escriba parte del nombre de una pregunta y después seleccione la
   pregunta. Haga esto para cada pregunta del área.
6. Si el área forma parte de un área mayor, seleccione esa área en **Parent** (área superior).
7. Haga clic en **Guardar**.

**Resultado esperado:** aparece el mensaje "Área creada". El área aparece en la lista.

### Importar áreas (alternativa) {#import-areas-alternative}

1. Cree un archivo CSV sin fila de encabezado. Cada fila tiene cinco columnas en este orden:
   nombre del área, código de país, código, descripción, indicador de borrado. El código de país
   y el código no se usan. Puede dejarlos vacíos. Ejemplo:

   ```csv
   North,,,North District,0
   South,,,South District,0
   ```

2. Escriba `0` en la columna del indicador de borrado de cada área que quiera importar. Las filas
   con otro valor no se importan.
3. En la pestaña **Áreas**, haga clic en **Importar**.
4. Escriba el hash SHA-256 del archivo en **Verificación de Integridad (SHA 256)**.
5. Suelte el archivo en el área de carga, o haga clic en **Cargar fichero**. Después haga clic
   en **Importar**.

**Resultado esperado:** aparece el mensaje "Áreas Importadas Exitosamente". La primera columna se
convierte en el **Nombre** del área. La cuarta columna se convierte en su **Descripción**.

:::caution PRECAUCIÓN
Una importación vincula cada área importada a **todas** las preguntas del proceso electoral.
Después de la importación, abra cada área y quite las preguntas que no son de esa área.
:::

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| El icono **+** junto a **Procesos Electorales** no está. | Su cuenta no tiene el permiso para crear procesos electorales. Pregunte al administrador del inquilino. |
| La importación muestra "Hashes don't match. Integrity check failure." (los hashes no coinciden). | El archivo no es el archivo que espera. Obtenga el archivo de nuevo desde su origen. No lo importe sin la verificación. |
| Un idioma no está disponible en **Idiomas**. | Active el idioma en **Configuracion** > **IDIOMAS**. Consulte [Configurar el inquilino](01-tenant.md). |
| Una pregunta no está en la lista de **Preguntas del Área**. | Asegúrese de que creó la pregunta. Escriba parte del nombre de la pregunta. |
| "Error creando pregunta" cuando crea una pregunta | Compruebe el **Nombre** y después guarde de nuevo. |

## Más información {#more-information}

- [Configurar su primera elección](../Tutorials/admin_portal_tutorials_setting-up-your-first-election.md)
- [Proceso electoral: Datos](../Reference/User-Manual/Election-Management/Election-Event/Data/election_management_election-event_data.md)
- [Proceso electoral: Localización](../Reference/User-Manual/Election-Management/Election-Event/Localization/election_management_election-event_localization.md)
- [Proceso electoral: Áreas](../Reference/User-Manual/Election-Management/Election-Event/Areas/election_management_election-event_areas.md)
- [Elección: Datos](../Reference/User-Manual/Election-Management/Election/Data/election_management_election_data.md)
- [Pregunta: Datos](../Reference/User-Manual/Election-Management/Contest/Data/election_management_contest_data.md)
- [Candidato: Datos](../Reference/User-Manual/Election-Management/Candidate/Data/election_management_candidate_data.md)
- [Etiquetas de permiso](../Tutorials/admin_portal_tutorials_permission-labels.md)

**Siguiente:** [Añadir los votantes](03-voters.md).
