---
id: election_management_candidate_data
title: Datos del candidato
description: "La pestaña Datos del candidato permite configurar y gestionar los detalles de un candidato concreto dentro de una pregunta."
---
<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->
La pestaña **Datos** del candidato permite configurar y gestionar los detalles de un candidato concreto dentro de una pregunta. Muchos ajustes son iguales a los de otras pestañas de datos (proceso electoral, elección, pregunta), adaptados al contexto del candidato. A continuación se describen las secciones principales y sus detalles.

## Resumen de las secciones {#sections-overview}
- **General**: Datos básicos y opciones de elegibilidad.
- **Tipo**: Atributos que afectan al comportamiento de la papeleta (inválido, en blanco, lista, escritura libre).
- **Imagen**: Suba una foto o un icono para el candidato.

---

## General {#general}
Configure los datos básicos y la elegibilidad de voto de este candidato.

- **Nombre**  
  - Escriba el nombre oficial del candidato.  
  - Admite nombres en varios idiomas si la pregunta o la elección es multilingüe.
- **Alias (opcional)**  
  - Alias solo para uso del sistema, para la visualización interna o la ordenación.
- **Descripción o biografía (opcional)**  
  - Breve descripción o notas sobre el candidato (si se admite).
- **Interruptor Habilitado / Deshabilitado**  
  - **Habilitado**: el candidato aparece en las papeletas; los votantes pueden seleccionarlo.  
  - **Deshabilitado**: el candidato queda oculto o en gris; los votantes no pueden seleccionarlo.  
  - Use esta opción para desactivar un candidato sin eliminar su registro.
- **Campos adicionales (si están disponibles)**  
  - Cualquier campo de metadatos personalizado que vean los administradores (por ejemplo, afiliación o cualificaciones).

---

## Tipo {#type}
Defina funciones o comportamientos especiales que afectan a cómo se cuentan los votos de este candidato.

- **Voto inválido**  
  - Marca esta entrada como una opción de voto inválido explícito (por ejemplo, "Voto nulo" o "Papeleta anulada").  
  - Cuando un votante la selecciona, cuenta como voto inválido explícito en los resúmenes del recuento.
- **Voto en blanco**  
  - Marca esta entrada como una opción de voto en blanco.  
  - Cuando se selecciona, cuenta como voto válido (es decir, una elección consciente de emitir una papeleta en blanco).
- **Categoría/Lista**  
  - Marca esta entrada como una lista o una agrupación de partido en lugar de como un candidato individual.  
  - El comportamiento puede variar en las preguntas basadas en listas (por ejemplo, el recuento de votos de lista o la asignación de escaños).  
  - Si está habilitada, asegúrese de que la configuración de la pregunta (Diseño de la papeleta) admite la votación por listas.
- **Candidato por escrito**  
  - Indica que esta entrada representa un candidato por escrito.  
  - Los votantes pueden enviar textos libres si la pregunta permite candidatos por escrito; el sistema puede agregar las entradas por escrito bajo este perfil o tratarlas una a una.  
  - Asegúrese de que la pregunta permite candidatos por escrito antes de habilitarlo.

> **Nota sobre varios tipos**  
> Solo debe aplicarse un tipo a cada entrada de candidato, salvo que el sistema admita funciones combinadas (por ejemplo, es poco probable que una lista también admita votos en blanco). Compruebe que los ajustes no entren en conflicto (por ejemplo, un candidato no puede ser a la vez "Voto inválido" y un candidato normal).

---

## Imagen {#image}
Suba o gestione una foto o un icono para el candidato, que se usa en las pantallas de administración y en las papeletas que ven los votantes (si se admite).

- **Subir**  
  - Arrastre y suelte un archivo PNG (u otro formato admitido).  
  - El sistema puede aplicar límites de tamaño o de proporción; siga las recomendaciones que se muestren.
- **Vista previa**  
  - Después de subir la imagen, compruebe que se muestra correctamente (sin deformaciones y con el recorte adecuado).
- **Uso**  
  - La imagen del candidato puede aparecer junto a su nombre en las páginas de papeletas, en los resúmenes de resultados o en las listas de administración.
- **Reemplazar o quitar**  
  - Para actualizar la imagen, suba un archivo nuevo.  
  - Para quitarla, use la acción de eliminar si está disponible; asegúrese de que el comportamiento alternativo (por ejemplo, un icono predeterminado) es aceptable.

---

## Flujo de trabajo y consejos {#workflow--tips}
- **Orden**  
  - El orden de las entradas de candidato suele seguir la configuración de Diseño de la papeleta de la pregunta (aleatorio, alfabético o personalizado). Después de añadir o editar candidatos, compruebe el orden si es necesario.
- **Deshabilitar frente a eliminar**  
  - Use el interruptor Habilitado/Deshabilitado para quitar temporalmente un candidato de las papeletas sin perder los datos históricos.  
  - Elimine un candidato solo si está seguro de que nunca debe aparecer; tenga en cuenta las posibles implicaciones de auditoría.
- **Coherencia del tipo**  
  - Asegúrese de que solo una entrada de candidato esté marcada como "Voto en blanco" o "Voto inválido", salvo que se admitan varias opciones inválidas.  
  - En las preguntas basadas en listas, confirme que los candidatos de lista están configurados correctamente y que los ajustes de la pregunta coinciden.
- **Soporte multilingüe**  
  - Si la elección es multilingüe, proporcione traducciones para los nombres de los candidatos y para cualquier descripción.  
  - Compruebe cómo aparecen las etiquetas de voto por escrito o de voto inválido en cada idioma.
- **Buenas prácticas para imágenes**  
  - Use imágenes o iconos claros y reconocibles.  
  - Mantenga los archivos de un tamaño razonable para evitar problemas de rendimiento.
- **Validación**  
  - Después de guardar los cambios, previsualice la papeleta (si la función de vista previa existe) para confirmar que el candidato aparece como se espera o que está oculto cuando está deshabilitado.
- **Consideraciones de auditoría**  
  - Cambiar el tipo de un candidato (por ejemplo, marcarlo como inválido) afecta a la interpretación de los resultados; aplique los cambios antes de publicar las papeletas.  
  - Documente cualquier cambio en las opciones de voto inválido o en blanco para que sea transparente.
