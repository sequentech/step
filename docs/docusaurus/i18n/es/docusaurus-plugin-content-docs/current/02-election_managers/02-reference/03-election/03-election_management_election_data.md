---
id: election_management_election_data
title: Datos
description: "Gestione los datos de la elección. Esta pestaña permite configurar los ajustes principales de una elección individual, con una estructura similar a la pestaña Datos del proceso electoral, pero limitada a esta elección concreta…"
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
-->

## 1. General {#1-general}

Gestione los datos de la elección. Esta pestaña permite configurar los ajustes principales de una elección individual. Tiene una estructura similar a la pestaña **Datos** del proceso electoral, pero se aplica solo a esta elección.

- **General**: defina los datos básicos y los nombres multilingües.
- **Fechas**: configure las fechas y las horas de inicio y de fin.
- **Idioma**: defina los idiomas admitidos y el idioma por defecto.
- **Canales de Voto Permitidos**: indique los métodos de votación aplicables.
- **Diseño de la Papeleta**: gestione la presentación y el orden de las preguntas y de las opciones de auditoría.
- **Comprobantes**: configure cómo reciben los votantes la confirmación o el comprobante.
- **Imagen**: suba una imagen asociada a esta elección.
- **Configuración Avanzada**: importe ajustes avanzados y defina los límites de voto, las confirmaciones y los permisos.

Más información sobre cada sección aparece a continuación.

---

## General {#general}

Configure los datos básicos y los nombres multilingües de esta elección. Es igual que la sección "General" de la pestaña **Datos** del proceso electoral, pero se aplica solo a la elección actual.

- **Pestañas de idiomas**: configure cómo aparece esta elección en los distintos idiomas del portal de votación.
- **Nombre**: escriba el nombre oficial de la elección.
- **Alias (opcional)**: especifique un alias interno para mostrarlo en los menús laterales.
- **Descripción (Opcional)**: escriba una descripción que aclare la finalidad o el alcance de esta elección.

---

## Fechas {#dates}

Defina las fechas de inicio y de fin de la elección.

- **Fecha y hora de inicio**: inicia el período de votación de esta elección.
- **Fecha y hora de finalización**: finaliza el período de votación de esta elección.
- Use los selectores de calendario y de fecha y hora para elegir las fechas y horas exactas.

---

## Idioma {#language}

Gestione las opciones de idioma de esta elección. Es similar a la del proceso electoral, pero se aplica a cada elección por separado.

- **Definir idiomas**: seleccione qué idiomas están habilitados para esta elección mediante los botones de opción.
- **Establecer idioma por defecto**: marque un idioma como predeterminado seleccionando "Por defecto" junto a él.
- Nota: los idiomas disponibles aquí están limitados por la configuración de idiomas del proceso electoral.

---

## Diseño de la Papeleta {#ballot-design}

Gestione la presentación de los elementos de la papeleta para esta elección.

- **Opciones de visualización del botón de auditoría**: active o desactive la aparición del botón "Auditar papeleta" en el portal de votación.
- **Orden de presentación de las preguntas**:  
  - **Aleatorias**: las preguntas aparecen en un orden aleatorio en cada sesión de votación.  
  - **Alfabético**: las preguntas aparecen ordenadas alfabéticamente por nombre.  
  - **Personalizado**: reordene las preguntas manualmente. Al seleccionar "Personalizado", aparece una interfaz de arrastrar y soltar, o similar, para ordenar las preguntas en la secuencia deseada.
- **Ajustes de diseño adicionales** (si aplica):  
  - Logotipo, CSS personalizado u otras opciones de visualización (según las capacidades del sistema).  
  - Los ajustes del formato de los comprobantes pueden aparecer en **Comprobantes**, si están separados.

---

## Canales de Voto Permitidos {#voting-channels-allowed}

Indique qué canales de votación (métodos) son aplicables a esta elección.

- Use los botones de opción o las casillas de verificación para habilitar o deshabilitar canales (por ejemplo, En línea, EARLY_VOTING, Quiosco o Papel).
- Solo los canales seleccionados aceptarán votos en esta elección.
- **EARLY_VOTING**: habilita un período de votación anticipada para los votantes asignados a áreas en las que la política de área "Votación anticipada" está activada. Esta opción solo aparece si el canal está permitido a nivel del proceso electoral.
  - Cuando el proceso electoral inicia el canal **EARLY_VOTING** (pestaña **Publicar**), solo los votantes de áreas elegibles pueden votar anticipadamente.
  - El canal **En línea** controla el ciclo de vida de la votación anticipada: iniciar o cerrar **En línea** cierra automáticamente la votación anticipada, y no se puede iniciar la votación anticipada una vez iniciada la votación en línea.
- **Quiosco**: votación presencial en quioscos. Su estado es independiente de **En línea** y de la votación anticipada.
- Si un canal que ya se había iniciado se deshabilita después manualmente en esta sección, no se realiza ninguna acción inmediata. Los botones de acción permanecen deshabilitados hasta que se vuelva a permitir el canal.

---

## Comprobantes {#receipts}

Configure cómo reciben los votantes la confirmación de su voto en esta elección.

- **Método de comprobante**: elija si los votantes reciben una confirmación en pantalla, un comprobante por correo electrónico, un comprobante por SMS o una combinación de estos (según las capacidades del sistema).
- **Soporte de auditoría**: active o desactive la capacidad de los votantes para verificar el cifrado (si es independiente del botón de auditoría de **Diseño de la Papeleta**).
- **Personalización**: si el sistema lo permite, configure las plantillas o el texto que aparece en los comprobantes.

---

## Imagen {#image}

Suba una imagen para mostrarla en el sistema para esta elección (por ejemplo, un logotipo o un banner).

- **Subir**: arrastre y suelte un archivo `.png` (u otro formato compatible), o selecciónelo.
- **Vista previa**: después de subir la imagen, previsualice cómo aparece en el portal o en la interfaz de administración.
- **Opcional**: si no necesita una imagen, puede dejar esta sección en blanco.

---

## Configuración Avanzada {#advanced-configuration}

Importe ajustes avanzados para esta elección y defina los límites de voto, las confirmaciones y las etiquetas de permiso.

- **Modal de Confirmación de Voto**:  
  - Active o desactive la visualización de un cuadro de diálogo de confirmación antes de finalizar un voto.  
  - Si está activado, configure el texto de confirmación o el mensaje de advertencia.

- **Número de votos permitidos**:  
  - Especifique cuántas veces puede volver a votar un votante (por ejemplo, permitir reenvíos hasta N veces).  
  - Si no se permiten revotos ilimitados, indique aquí el número máximo.

- **Etiqueta de permiso**:  
  - Asigne una etiqueta de permiso a esta elección. Esta etiqueta controla qué usuarios del portal de administración pueden acceder a ella o gestionarla.  
  - Ejemplo de flujo de trabajo:  
    1. Cree o seleccione una etiqueta de permiso (por ejemplo, "Gestor de la Elección A").  
    2. Asocie uno o más usuarios administradores a esta etiqueta.  
    3. Solo los usuarios con esta etiqueta de permiso ven o modifican esta elección en el portal de administración.

- **Subir configuración avanzada** (Opcional):  
  - Arrastre y suelte un archivo de configuración (por ejemplo, JSON u otro formato propio del sistema) para aplicar ajustes avanzados predefinidos.  
  - Puede incluir ajustes como políticas de cifrado de preguntas, validaciones personalizadas o integraciones.

- **Política de título de la pantalla de inicio**

  - Defina el título de la pantalla de inicio (portal de votación) como el título de la elección o como el título del proceso electoral. El valor por defecto es el título de la elección.

- **Política de informe consolidado**
  - Si se selecciona la opción **Generar**, se generarán en el recuento los resultados consolidados de la elección para todas las áreas.

- **Política de abstención de voto**
  - Seleccione **Habilitado** para ofrecer a los votantes la opción de abstención de voto en la pantalla de inicio, para toda la elección y en todas las preguntas.
  - Seleccione **Deshabilitado** (por defecto) para ocultar la opción.
  - Este ajuste solo aparece si el proceso electoral usa la política de cifrado **Varios concursos**.
  - Consulte [Política de abstención de voto](../07-decline-to-vote.md) para ver la configuración completa, el flujo de votación y el comportamiento de los resultados.

- **Política de papeletas en blanco**
  - Seleccione **Habilitado** para informar de las papeletas que quedan vacías en todas las preguntas como una cifra distinta, **Total de Papeletas en Blanco**. Esta cifra está separada de la abstención de voto y de los votos en blanco por pregunta.
  - Seleccione **Deshabilitado** (por defecto) para dejar esta cifra no disponible.
  - Este ajuste solo aparece si el proceso electoral usa la política de cifrado **Varios concursos**. No hay un botón para los votantes: el voto en blanco se infiere cuando el votante deja vacías todas las preguntas durante la votación normal.
  - Consulte [Papeletas en blanco](../09-blank-ballots.md) para ver la configuración completa, el flujo de votación y el comportamiento de los resultados.

- **Política del botón Atrás de la pantalla de votación**
  - Controla a dónde lleva el botón **Atrás** de la primera pantalla de votación al votante.
  - Seleccione **Ir a la pantalla de selección de elecciones** (por defecto) para mantener el comportamiento anterior.
  - Seleccione **Ir a la pantalla de inicio de la elección** para devolver al votante a la pantalla de inicio de la elección, donde la papeleta vuelve a su estado inicial. Se recomienda cuando la [abstención de voto](../07-decline-to-vote.md) está habilitada, para que la opción de abstención siga siendo accesible desde la pantalla de votación.

---

### Notas y buenas prácticas {#notes--best-practices}

- Muchos ajustes de esta pantalla dependen o heredan la configuración del proceso electoral (por ejemplo, los idiomas disponibles y los canales de voto globales). Compruebe siempre que los ajustes del proceso electoral y los de cada elección sean coherentes.
- Antes de activar opciones avanzadas (por ejemplo, límites de revoto o etiquetas de permiso), asegúrese de entender su impacto en la experiencia del votante y en la seguridad.
- Use nombres descriptivos y alias para identificar rápidamente las elecciones en un proceso electoral con varias elecciones.
- Pruebe los cambios en un entorno de pruebas cuando sea posible, sobre todo en las configuraciones avanzadas o en los cambios del diseño de la papeleta.
