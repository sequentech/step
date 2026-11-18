---
id: admin_portal_tutorials_define-areas
title: Definir áreas
description: "Las áreas le permiten organizar una elección en divisiones geográficas o lógicas concretas, como distritos o circunscripciones. Así puede asignar preguntas específicas a grupos relevantes de votantes."
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

import GoogleVideo from '@site/src/components/GoogleVideo';

<GoogleVideo id="1mt0vOfcXRwW0cviJMwzawJWe6hC9Mf0L" />

Las áreas le permiten organizar una elección en divisiones geográficas o lógicas concretas, como distritos o circunscripciones. Así puede asignar preguntas específicas a grupos relevantes de votantes.

## Acceder al menú de Áreas {#accessing-the-areas-menu}

Para gestionar las áreas, abra el proceso electoral y haga clic en la pestaña **Áreas**.

![Ubicación del menú de Áreas](../../../../../../docs/02-election_managers/01-tutorials/assets/areas_menu_navigation.png)

Desde esta pantalla puede ver las áreas existentes, las preguntas asignadas a cada una y realizar acciones de gestión, como editar o eliminar.

## Crear un área nueva {#creating-a-new-area}

Siga estos pasos para definir un área nueva dentro de su elección:

1. Haga clic en **Añadir**. Si la lista está vacía, haga clic en **Crear un Área**.
2. **Nombre:** Escriba un nombre único para el área (por ejemplo, "Distrito 4").
3. **Descripción:** Escriba una descripción opcional.
4. **Preguntas:** Seleccione una o más preguntas que deben estar disponibles para los votantes de esta área concreta. Escriba tres o más letras del nombre de la pregunta para encontrarla.

![Diálogo de configuración del área](../../../../../../docs/02-election_managers/01-tutorials/assets/area_config_details.png)

5. **Parent:** Si el área forma parte de una jerarquía mayor, puede seleccionar un área padre.
6. **Permitir Votación Temprana:** Actívelo si los votantes del área pueden votar de forma anticipada. El interruptor solo está disponible si el canal **Votación anticipada** está activado en **Canales de Voto Permitidos** del proceso electoral.
7. Haga clic en el botón de guardar.

## Gestionar e importar áreas {#managing-and-importing-areas}

La interfaz de Áreas ofrece varias herramientas para organizar las áreas de forma eficiente y gestionar los datos en lote.

* **Filtros:** Use el botón **Añadir filtro** para reducir la lista de áreas por **Nombre**, **Descripción**, **ID** o **Tipo**.
* **Importar:** Haga clic en **Importar** para subir áreas en lote desde un archivo CSV sin fila de encabezado. Cada fila tiene seis columnas: identificador, código de país, código, nombre del área, marca de eliminación y política de votación anticipada. La importación usa solo las filas con `0` en la marca de eliminación. Usa la primera columna como **Nombre** del área y la cuarta columna como **Descripción**. En la columna de política de votación anticipada, el valor `allow_early_voting` permite la votación anticipada. Cualquier otro valor no la permite.
* **Insertar o actualizar áreas:** Suba un archivo CSV con fila de encabezado para establecer el padre de las áreas existentes. La columna 1 es el nombre del área y la columna 2 es el nombre del área padre. La importación ignora una fila si ninguna área tiene el nombre de la columna 1.

:::caution
Una importación vincula cada área importada a **todas** las preguntas del proceso electoral. Después de la importación, abra cada área y elimine las preguntas que no son de esa área.
:::

:::info
**Verificación de integridad:** Al importar datos de áreas, escriba un **hash SHA-256** en **Verificación de Integridad (SHA 256)** para comprobar la autenticidad del archivo y asegurarse de que no se ha modificado. Si deja el campo vacío, el portal de administración le pregunta "Importar Sin Verificación de Integridad?". **Insertar o actualizar áreas** no comprueba el hash.
:::

![Diálogo para importar áreas](../../../../../../docs/02-election_managers/01-tutorials/assets/import_areas.png)

1. Arrastre y suelte su archivo CSV, o haga clic en **Examinar** para seleccionarlo.
2. Haga clic en **Importar** para finalizar el proceso.
