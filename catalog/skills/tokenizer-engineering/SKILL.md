# Ingeniería de tokenizadores

El tokenizador fija qué puede representar el modelo y cuánto cuesta cada texto. Cambiarlo después obliga a
reentrenar, así que se valida antes.

## Diseño

- Usa BPE a nivel de bytes (o unigrama con respaldo de bytes): cualquier texto debe poderse representar sin
  un token desconocido.
- Entrénalo con una muestra representativa de **todos** los idiomas, dominios y formatos (código, tablas,
  Markdown) en las proporciones deseadas. Un idioma poco representado se fragmenta en muchos tokens.
- Elige el tamaño del vocabulario con un motivo: más tokens acortan las secuencias pero encarecen la
  matriz de embeddings y la salida. Redondea a un múltiplo de 64 o 128 para el hardware.
- Reserva los tokens especiales desde el principio (inicio, fin, relleno, separadores de turnos, herramientas)
  y deja espacio para añadir otros sin cambiar los identificadores existentes.

## Plantilla de chat

- Define una sola plantilla de conversación, versionada, que marque roles y fin de turno con tokens
  especiales. Usa la misma en entrenamiento, evaluación y servicio.
- El texto del usuario nunca debe poder producir un token especial. Tokeniza el contenido con los especiales
  desactivados y añádelos tú.

## Pruebas obligatorias

1. **Ida y vuelta:** `decode(encode(texto)) == texto` para un corpus grande, incluidos emojis, escrituras
   no latinas, espacios y saltos de línea raros.
2. **Fertilidad:** tokens por palabra o por carácter en cada idioma y dominio; compara con un tokenizador de
   referencia y registra la diferencia.
3. **Estabilidad:** una misma cadena tokeniza igual sin importar su contexto (espacios iniciales, mayúsculas).
4. **Especiales:** los identificadores de los tokens especiales coinciden entre tokenizador, modelo y
   servidor; comprueba el fin de turno y el relleno de forma explícita.
5. **Longitudes:** ningún ejemplo supera la ventana de contexto tras aplicar la plantilla; si se trunca,
   que sea una decisión consciente y registrada.

## Operación

- Versiona el tokenizador junto al modelo (el mismo hash en ambos). Un modelo con un tokenizador distinto al
  de su entrenamiento falla en silencio, con respuestas degradadas.
- Mide la velocidad de tokenización cuando sea parte del camino crítico y usa la implementación en Rust o C++
  en lotes.
