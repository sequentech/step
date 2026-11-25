---
id: election_management_election_event_reports
title: Informes
description: "La pestaña Informes permite configurar, generar y programar informes con plantillas predefinidas (consulte la sección Plantillas para más detalles)."
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->


La pestaña **Informes** permite configurar, generar y programar informes con plantillas predefinidas (consulte la sección Plantillas para más detalles). Algunos informes (por ejemplo, el Informe de Inicialización, la verificación manual o el Recibo de Voto) se generan automáticamente mediante acciones del sistema. Otros informes (por ejemplo, el Informe Estadístico o el resumen de la lista de votantes) se pueden programar o generar manualmente cuando sea necesario. Esta pestaña ofrece control total sobre la configuración y la programación de informes, lo que aporta flexibilidad y automatización cuando se necesita.

### Añadir una configuración de informe {#adding-a-report-configuration}

1. Seleccione **Añadir**.
2. Seleccione un **Tipo de informe**.
   - Esto determina el tipo de datos que se generarán.
   - Ejemplo: el **Informe Estadístico** genera datos estadísticos sobre los resultados del recuento.
3. Seleccione una **Elección** de la que se recopilarán datos.
   - Si lo deja en blanco, la configuración se aplica a TODAS las elecciones.
4. Seleccione la **Plantilla** correspondiente (opcional).
   - Si la deja en blanco, se usará una plantilla predeterminada diseñada por Sequent.
   - Las plantillas controlan cómo se muestran los datos del informe (código HTML). Las plantillas se asocian con los tipos de informe en la pestaña **Plantillas**.
5. Añada la **Etiqueta de permiso** correspondiente al informe.
   - Esto determina qué usuarios pueden ver, generar y previsualizar el informe.
6. Seleccione **Guardar**.

### Informes repetibles {#repeatable-reports}

Habilitar la opción **Repetible** permite generar y enviar informes repetidamente según una programación (expresión Cron).

1. Habilite **Repetible**.
2. Defina su **Expresión Cron**.
   - Consulte "Configurar una expresión Cron" más abajo para obtener orientación.
3. Establezca los **Destinatarios de correo electrónico** de este informe.
4. Seleccione **Guardar**.

#### Configurar una expresión Cron {#setting-up-a-cron-expression}

Una expresión cron es una secuencia de cinco campos que indica cuándo ejecutar una tarea:
- **Minuto** (0–59)
- **Hora** (0–23)
- **Día del mes** (1–31)
- **Mes** (1–12)
- **Día de la semana** (0–6, donde 0 = domingo, 6 = sábado)

Ejemplos:
- **Informe horario**: `0 * * * *`  
  Se ejecuta cada hora, en el minuto 0 (por ejemplo, 1:00, 2:00).
- **Informe diario**: `15 9 * * *`  
  Se ejecuta todos los días a las 9:15.
- **Informe mensual**: `30 10 1 * *`  
  Se ejecuta el primer día de cada mes, a las 10:30.
- **Informe por día de la semana**: `0 14 * * 5`  
  Se ejecuta todos los viernes a las 14:00.

### Informes cifrados {#encrypted-reports}

Para proteger la información sensible, los informes se pueden cifrar con una contraseña elegida por el usuario. El estado de cifrado y la contraseña se conservan durante la exportación y la importación, por lo que no es necesario volver a cifrarlos.

1. Haga clic en **Crear Informe** o en **Añadir**.
2. Elija el **Tipo de informe**.
3. Elija la **Elección** (opcional).
4. Elija la **Plantilla** (opcional).
5. Establezca la **Política de cifrado** en "Contraseña configurada".
6. Escriba una contraseña en el campo **Contraseña**.
7. Repita la contraseña en el campo **Repetir Contraseña**.
   - Esta contraseña será necesaria para descifrar todos los informes generados con esta configuración.
8. Haga clic en **Guardar contraseña** (o en **Guardar**, según la interfaz).

> **Nota:** Los informes cifrados conservan su cifrado al exportarse y volver a importarse. Se debe usar la misma contraseña para descifrarlos, sea cual sea el contexto.

---

Con estas instrucciones, los usuarios pueden configurar informes puntuales o programados, y elegir cifrar los resultados sensibles. Ajuste la terminología o los ejemplos según las etiquetas de su interfaz y su flujo de trabajo.
