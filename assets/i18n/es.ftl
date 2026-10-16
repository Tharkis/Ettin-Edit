# Textos de la interfaz de Ettin Edit en español.
# Traducción automática (hecha por máquina): necesita la revisión de un
# hablante nativo.
#
# Each line is `id = text`. Translations copy the ids and change the text.
# Text in { $braces } is filled in by the app; keep those names as they are.
# Fluent syntax: https://projectfluent.org/fluent/guide/

# The language's own name, shown in Preferences.
language-name = Español

## Main window: menu bar, tabs, status bar, dialogs (src/app.rs)
theme-problem = El tema tiene un problema: { $error }
theme-save-failed = No se pudo guardar el tema en { $path }: { $error }
open-lossy = { $path } tiene bytes que no son { $encoding } válido. Se muestran como � y se reemplazarán si guardas. Si el texto se ve mal, elige su codificación en Codificación > Reabrir con.
open-failed = No se pudo abrir { $path }: { $error }
dialog-open = Abrir
dialog-save-as = Guardar como
save-failed = No se pudo guardar { $path }: { $error }
rename-bad-name = El nombre no puede contener / ni \.
rename-exists = "{ $name }" ya existe en esta carpeta.
rename-failed = No se pudo cambiar el nombre del archivo: { $error }
not-saved-yet = { $name } (aún sin guardar)
restore-themes-failed = No se pudieron restaurar los temas integrados: { $error }
reload-failed = No se pudo recargar { $name }: { $error }
reopen-lossy = Algunos bytes de { $name } no son { $encoding } válido. Se muestran como �.
command-palette-title = Paleta de comandos
command-palette-hint = Escribe parte del nombre de un comando
tab-list-title = Pestañas abiertas
tab-list-hint = Escribe parte del nombre de una pestaña
no-results = Sin resultados
selected-count = { $count } seleccionados
marked-count = { $count } marcados
dialog-open-workspace = Abrir carpeta como área de trabajo
distraction-free-notice = Modo sin distracciones. Pulsa F11 para salir.
wrapped-to-top = Se volvió al principio
wrapped-to-bottom = Se volvió al final
not-found = No encontrado
no-more-matches = No hay más coincidencias
replaced-count = { $count } reemplazados
search-start-failed = No se pudo iniciar la búsqueda: { $error }
find-in-files-title = "{ $text }" en { $folder }
invalid-regex = Expresión regular no válida: { $error }
about-ettin-edit = Acerca de Ettin Edit
menu-file = Archivo
menu-edit = Editar
menu-search = Buscar
menu-view = Ver
menu-encoding = Codificación
menu-theme = Tema
menu-language = Lenguaje
menu-settings = Configuración
menu-tools = Herramientas
menu-plugins = Complementos
menu-macro = Macro
menu-help = Ayuda
open-recent = Abrir reciente
no-recent-files = No hay archivos recientes
new = Nuevo
open-dots = Abrir…
clear-recent-files = Borrar archivos recientes
save = Guardar
save-as = Guardar como…
save-all = Guardar todo
reload-from-disk = Recargar desde el disco
rename-dots = Cambiar nombre…
move-to-trash = Mover a la papelera…
open-containing-folder = Abrir carpeta contenedora
open-terminal-here = Abrir terminal aquí
print-dots = Imprimir…
export-html = Exportar como HTML…
open-folder-as-workspace = Abrir carpeta como área de trabajo…
close-workspace = Cerrar área de trabajo
close = Cerrar
close-all = Cerrar todo
close-others = Cerrar las demás
exit = Salir
find-dots = Buscar…
replace-dots = Reemplazar…
find-next = Buscar siguiente
find-previous = Buscar anterior
find-selected-next = Buscar selección siguiente
find-selected-previous = Buscar selección anterior
find-in-files = Buscar en archivos…
find-all-in-document = Buscar todo en el documento actual
find-all-in-tabs = Buscar todo en las pestañas abiertas
select-all-matches = Seleccionar todas las coincidencias
mark-all = Marcar todo
clear-all-marks = Borrar todas las marcas
go-to-line = Ir a la línea…
menu-bookmark = Marcador
toggle-bookmark = Alternar marcador
next-bookmark = Marcador siguiente
previous-bookmark = Marcador anterior
clear-all-bookmarks = Borrar todos los marcadores
cut-bookmarked-lines = Cortar líneas marcadas
copy-bookmarked-lines = Copiar líneas marcadas
remove-bookmarked-lines = Eliminar líneas marcadas
remove-unbookmarked-lines = Eliminar líneas no marcadas
inverse-bookmarks = Invertir marcadores
word-wrap = Ajuste de línea
line-numbers = Números de línea
highlight-current-line = Resaltar línea actual
menu-show-symbol = Mostrar símbolo
show-whitespace = Mostrar espacios en blanco
show-end-of-line = Mostrar fin de línea
show-invisible = Mostrar caracteres invisibles
show-all-characters = Mostrar todos los caracteres
indent-guides = Guías de sangría
long-line-marker-column = Marcador de línea larga (columna { $column })
function-list = Lista de funciones
document-map = Mapa del documento
character-panel = Panel de caracteres
hex-view = Vista hexadecimal
folder-as-workspace = Carpeta como área de trabajo
distraction-free-mode = Modo sin distracciones
tab-list = Lista de pestañas…
command-palette = Paleta de comandos…
move-to-other-view = Mover a la otra vista
clone-to-other-view = Clonar en la otra vista
focus-other-view = Enfocar la otra vista
menu-folding = Plegado
fold-all = Plegar todo
unfold-all = Desplegar todo
fold-current-level = Plegar nivel actual
unfold-current-level = Desplegar nivel actual
fold-level = Plegar nivel { $level }
zoom-in = Acercar
zoom-out = Alejar
reset-zoom = Restablecer zoom
summary-dots = Resumen…
themes-dark = Oscuros
themes-light = Claros
themes-custom = Personalizados
edit-theme-colours = Editar colores del tema…
open-themes-folder = Abrir carpeta de temas
reload-themes = Recargar temas
restore-built-in-themes = Restaurar temas integrados
preferences-dots = Preferencias…
shortcut-mapper-dots = Asignación de atajos…
auto-close-brackets = Cerrar paréntesis y comillas automáticamente
smart-indentation = Sangría inteligente
indent-with-spaces = Sangrar con espacios
word-completion = Completar palabras
keyword-completion = Completar palabras clave y funciones
parameter-hints = Sugerencias de parámetros
single-window-long = Ventana única (abrir archivos en el Ettin Edit en ejecución)
remember-unsaved = Recordar cambios sin guardar al salir
auto-reload = Recargar automáticamente archivos cambiados en el disco
long-line-column = Columna de línea larga
user-manual = Manual de usuario
user-manual-web = Manual de usuario en la web
check-for-updates = Buscar actualizaciones…
check-updates-automatically = Buscar actualizaciones automáticamente
encoding-save-as = Guardar como
reopen-with = Reabrir con
only-saved-files = Solo para archivos guardados en el disco
stop-recording = Detener grabación
start-recording = Iniciar grabación
playback = Reproducir
run-macro-multiple = Ejecutar una macro varias veces…
save-current-macro = Guardar macro actual…
delete-saved-macro = Eliminar macro guardada
new-language = Nuevo lenguaje…
open-languages-folder = Abrir carpeta de lenguajes
open-completion-lists-folder = Abrir carpeta de listas de autocompletado
reload-languages = Recargar lenguajes
language-guide = Guía de lenguajes
all-open-tabs = Todas las pestañas abiertas
new-tab = Nueva pestaña
unpin-tab = Desanclar pestaña
pin-tab = Anclar pestaña
copy-full-path = Copiar ruta completa
copy-file-name = Copiar nombre de archivo
status-length-lines = longitud: { $length }   líneas: { $lines }
click-to-copy = Clic para copiar
recording-macro = Grabando macro
read-only-status = Solo lectura
column-mode-status = Modo columna
column-mode-status-hover = Arrastrar y Mayús+flechas seleccionan un rectángulo. Haga clic para desactivarlo.
click-to-allow-editing = Clic para permitir la edición
status-position = Lín { $line }, Col { $column }
status-selection = Sel { $count }
status-carets = { $count } cursores
go-to-line-hover = Ir a la línea (Ctrl+G)
save-changes-title = ¿Guardar cambios?
save-changes-question = "{ $name }" tiene cambios sin guardar. ¿Guardarlos antes de cerrar?
cancel = Cancelar
dont-save = No guardar
trash-failed = No se pudo mover { $name } a la papelera: { $error }
about-version = Versión { $version }
about-tagline = Un editor de texto rápido y moderno para Linux y Windows.
about-built-with = Escrito en Rust con egui y syntect.
error-title = Algo salió mal
ok = Aceptar

## Documents (src/document.rs)
eol-unix = Unix (LF)
eol-windows = Windows (CR LF)
eol-mac = Macintosh (CR)
cant-encode-char = "{ $char }" (U+{ $code }) no se puede guardar como { $encoding }. Elige una codificación que lo incluya, como UTF-8, en el menú Codificación.
untitled-name = nuevo { $number }

## Edit and Tools menus (src/app/menus.rs)
duplicate = Duplicar
delete-lines = Eliminar líneas
move-lines-up = Subir
move-lines-down = Bajar
join-lines = Unir líneas
split-lines = Dividir líneas en la columna de línea larga
insert-blank-above = Insertar línea en blanco encima
insert-blank-below = Insertar línea en blanco debajo
sort-lexical-asc = Ordenar lexicográficamente ascendente
sort-lexical-desc = Ordenar lexicográficamente descendente
sort-ignore-case-asc = Ordenar sin distinguir mayúsculas ascendente
sort-ignore-case-desc = Ordenar sin distinguir mayúsculas descendente
sort-integer-asc = Ordenar como enteros ascendente
sort-integer-desc = Ordenar como enteros descendente
sort-decimal-asc = Ordenar como decimales ascendente
sort-decimal-desc = Ordenar como decimales descendente
sort-length-asc = Ordenar por longitud ascendente
sort-length-desc = Ordenar por longitud descendente
reverse-lines = Invertir orden de líneas
remove-duplicate-lines = Eliminar líneas duplicadas
remove-consecutive-duplicates = Eliminar líneas duplicadas consecutivas
remove-empty-lines = Eliminar líneas vacías
remove-blank-lines = Eliminar líneas vacías (incluidas las en blanco)
undo = Deshacer
redo = Rehacer
cut = Cortar
copy = Copiar
paste = Pegar
select-all = Seleccionar todo
menu-line-operations = Operaciones de línea
menu-convert-case = Convertir a
case-upper = MAYÚSCULAS
case-lower = minúsculas
case-title = Tipo Título
case-sentence = Tipo oración
case-invert = iNVERTIR mAYÚSCULAS
menu-comment = Comentario
toggle-line-comment = Alternar comentario de línea
toggle-block-comment = Alternar comentario de bloque
menu-blank-operations = Operaciones con espacios
trim-trailing = Quitar espacios finales
trim-leading = Quitar espacios iniciales
trim-both = Quitar espacios iniciales y finales
eol-to-space = Fin de línea a espacio
tabs-to-spaces = Tabulaciones a espacios
leading-spaces-to-tabs = Espacios iniciales a tabulaciones
all-spaces-to-tabs = Todos los espacios a tabulaciones
menu-insert = Insertar
date-time-short = Fecha y hora (corta)
date-time-long = Fecha y hora (larga)
menu-multi-select = Selección múltiple
select-next-occurrence = Seleccionar siguiente aparición
select-all-occurrences = Seleccionar todas las apariciones
menu-auto-completion = Autocompletado
complete-word = Completar palabra
show-parameter-hint = Mostrar sugerencia de parámetros
column-editor = Editor de columnas…
read-only = Solo lectura
column-mode = Modo columna
mime-base64-encode = Codificar Base64
mime-base64-encode-wrapped = Codificar Base64 con saltos de línea
mime-base64-url-encode = Codificar Base64, seguro para URL
mime-base64-decode = Decodificar Base64
mime-url-encode-component = Codificar URL (parte de una URL)
mime-url-encode = Codificar URL (URL completa)
mime-url-decode = Decodificar URL
mime-qp-encode = Codificar Quoted-Printable
mime-qp-decode = Decodificar Quoted-Printable
mime-html-encode = Codificar HTML
mime-html-encode-all = Codificar HTML, incluidos acentos y símbolos
mime-html-decode = Decodificar HTML
mime-saml-decode = Decodificar SAML
json-format = Formatear
json-minify = Minimizar
json-validate = Validar JSON
json-sort-keys = Ordenar claves JSON
json-tree = Árbol JSON
xml-pretty-print = Formatear
xml-pretty-print-attributes = Formatear, atributos en líneas nuevas
xml-linearize = Linealizar
xml-check-syntax = Comprobar sintaxis
xml-check-on-save = Comprobar sintaxis al guardar
xml-validate = Validar
xml-validate-against = Validar con…
xml-xsl-transform = Transformación XSL…
xpath-panel = Panel XPath
copy-current-xpath = Copiar XPath actual
xml-escape = Escapar caracteres especiales
xml-unescape = Desescapar caracteres especiales
close-tags-automatically = Cerrar etiquetas automáticamente
csvquery-panel = Panel CsvQuery
run-query = Ejecutar consulta
copy-results = Copiar resultados
results-to-new-tab = Resultados a nueva pestaña
menu-compare = Comparar
menu-csvquery = CsvQuery
menu-mime-tools = Herramientas MIME
menu-json = JSON y JavaScript
menu-xml-tools = Herramientas XML
compare-two-views = Comparar las dos vistas
compare-previous-tab = Comparar con la pestaña anterior
compare-with = Comparar con
compare-to-saved = Comparar con lo último guardado
compare-to-clipboard = Comparar con el portapapeles
compare-to-git = Comparar con el último commit (Git)
compare-selections = Comparar selecciones
next-difference = Diferencia siguiente
previous-difference = Diferencia anterior
show-comparison = Mostrar comparación
export-as-patch = Exportar como parche
close-comparison = Cerrar comparación
compare-ignore-whitespace = Ignorar espacios y tabulaciones
compare-ignore-case = Ignorar mayúsculas
compare-ignore-empty-lines = Ignorar líneas vacías
compare-detect-moves = Detectar líneas movidas

## Command names in the command palette (src/app/commands.rs)
long-line-marker = Marcador de línea larga
next-tab = Pestaña siguiente
previous-tab = Pestaña anterior
single-window = Ventana única
start-or-stop-recording = Iniciar o detener grabación
manage-plugins = Administrar complementos…
plugin-log = Registro de complementos
open-plugins-folder = Abrir carpeta de complementos
reload-plugins = Recargar complementos
install-sample-plugins = Instalar complementos de ejemplo
plugin-guide = Guía de complementos

## Plugins menu and plugin notices (src/app/plugins.rs)
install-plugin-from-file = Instalar complemento desde archivo
plugin-zip-filter = Complemento (.zip)
install-plugin-from-folder = Instalar complemento desde carpeta
no-plugins-folder = No hay carpeta de configuración donde poner complementos.
plugin-removed = Se eliminó { $name }; su carpeta está en la papelera.
plugin-updated = Se actualizó { $name }.
plugin-updated-version = Se actualizó { $name } a { $version }.
plugin-installed = Se instaló { $name }.
plugin-installed-version = Se instaló { $name } { $version }.
plugins-running = { $count ->
    [one] { $count } complemento en ejecución
   *[other] { $count } complementos en ejecución
}
sample-install-failed = No se pudo instalar el complemento de ejemplo { $name } en { $folder }: { $error }
samples-already-installed = Los complementos de ejemplo ya están en la carpeta de complementos.
samples-installed = Se instaló { $names }. Búscalos en el menú Complementos.
no-plugin-commands = No hay comandos de complementos

## Text tools (src/app/text_tools.rs)
tool-error-at = { $tool }: línea { $line }, columna { $column }: { $message }
tool-error = { $tool }: { $message }
tool-read-only = el documento es de solo lectura.
tool-ok-selection = { $result } (selección)
json-handled-as-js = No es JSON estricto, así que se trató como JavaScript. Validar JSON muestra en qué difiere.
json-valid = JSON válido
xml-well-formed = El XML está bien formado
copied-text = Copiado: { $text }
xpath-no-element = el cursor no está dentro de un elemento, o una etiqueta de cierre no coincide con su etiqueta de apertura.
xpath-not-xml = este documento no es XML.
copy-xpath = Copiar XPath
saved-not-well-formed = Guardado, pero { $name } no está bien formado: línea { $line }, columna { $column }: { $message }
xpath-type-expression = Escribe una expresión XPath, como //item[@id='1'].
xpath-not-well-formed = El documento no es XML bien formado (línea { $line }, columna { $column }: { $message })
xpath-error-at = { $message } (en el carácter { $position })

## Compare (src/app/compare.rs)
compare-selection-title = { $name } (selección en la línea { $line })
compare-same = Los dos lados son iguales.
compare-need-second-tab = Abre una segunda pestaña para comparar, o elige una en Herramientas > Comparar > Comparar con.
compare-not-saved = Esta pestaña aún no se ha guardado.
compare-saved-title = { $name } (guardado)
compare-read-saved-failed = No se pudo leer el archivo guardado: { $error }
compare-commit-title = { $name } (último commit)
compare-need-selections = Selecciona dos fragmentos de texto: Ctrl+arrastrar para una segunda selección, o selecciona en cada vista.
clipboard = Portapapeles
compare-clipboard-empty = El portapapeles no tiene texto para comparar.
compare-closed-title = { $name } (cerrado)
compare-tab-title = Comparar: { $left } | { $right }

## Languages folder (src/app/languages.rs)
make-folder-failed = No se pudo crear { $path }: { $error }
write-failed = No se pudo escribir { $path }: { $error }

## Printing (src/app/printing.rs)
dialog-export-html = Exportar como HTML

## XML validation and XSL (src/app/xml_schema.rs)
dialog-validate-against = Validar con un esquema
filter-schema = Esquema XML o DTD
filter-all-files = Todos los archivos
the-doctype = el DOCTYPE
read-failed = No se pudo leer { $path }: { $error }
validate-against = Validar con { $schema }
valid-against = El documento es válido según { $schema }
xsl-transform = Transformación XSL
filter-xsl = Hoja de estilo XSL

## Preferences (src/prefs_ui.rs)
prefs-editing = Edición
prefs-display = Visualización
prefs-new-documents = Documentos nuevos
prefs-general = Archivos e inicio
preferences = Preferencias
theme-colours-dots = Colores del tema…
prefs-font = Fuente
built-in-font = Integrada (Hack)
editor-font-label = Fuente del editor:
use-built-in = Usar integrada
font-size-label = Tamaño:
font-size-hint = Ctrl+0 vuelve a este tamaño tras hacer zoom
font-find-label = Buscar:
font-name-hint = Nombre de fuente
show-proportional-fonts = Mostrar fuentes proporcionales
looking-for-fonts = Buscando fuentes instaladas…
no-fonts-match = Ninguna fuente coincide
proportional = proporcional
prefs-show = Mostrar
prefs-word-wrap = Ajuste de línea
prefs-line-numbers = Números de línea
prefs-highlight-line = Resaltar la línea actual
prefs-whitespace = Símbolos de espacios en blanco
prefs-eol = Símbolos de fin de línea
prefs-invisible = Caracteres invisibles (caracteres de control, espacios poco comunes, caracteres de ancho cero)
prefs-indent-guides = Guías de sangría
prefs-edge = Marcador de línea larga en la columna
prefs-indentation = Sangría
prefs-tab-width = Ancho de tabulación:
prefs-columns = columnas
prefs-indent-tabs = Sangrar con tabulaciones
prefs-indent-spaces = Sangrar con espacios
prefs-smart-indent = Sangría inteligente (mantener la sangría y sangrar tras abrir un paréntesis)
prefs-typing = Escritura
prefs-auto-close = Cerrar paréntesis y comillas automáticamente
prefs-close-tags = Cerrar etiquetas HTML y XML automáticamente
prefs-word-completion = Sugerir palabras del documento al escribir
prefs-language-completion = Sugerir palabras clave y funciones del lenguaje
prefs-parameter-hints = Mostrar los parámetros de una función al escribir una llamada
prefs-line-ending = Fin de línea
prefs-system-line-ending = El predeterminado del sistema ({ $ending })
prefs-startup = Inicio y salida
prefs-restore-session = Reabrir los archivos que estaban abiertos la última vez
prefs-remember-unsaved = Salir sin preguntar por los cambios sin guardar, y recuperarlos la próxima vez
prefs-single-instance = Abrir archivos en la ventana de Ettin Edit en ejecución
prefs-files = Archivos
prefs-auto-reload = Recargar automáticamente archivos cambiados en el disco si no tienen cambios sin guardar
prefs-xml-check = Comprobar que el XML esté bien formado cada vez que se guarda
prefs-printing = Impresión
prefs-print-line-numbers = Imprimir números de línea
prefs-print-colours = Imprimir colores de sintaxis (oscurecidos para leerse bien en papel)
prefs-updates = Actualizaciones
prefs-check-updates = Buscar una nueva versión de Ettin Edit una vez por semana
prefs-plugin-programs = Programas que ejecutan complementos
prefs-plugin-path-hint = Déjalo vacío para buscarlos en el PATH.
prefs-plugin-reload-hint = Los cambios se aplican al recargar los complementos.
prefs-interface-language = Idioma de la interfaz
prefs-system-language = Igual que el sistema ({ $language })
open-translations-folder = Abrir carpeta de traducciones
reload-translations = Recargar
prefs-translations-hint = Añade un idioma o cambia una traducción poniendo un archivo .ftl en la carpeta de traducciones. Consulta Traducciones en el manual de usuario.

## Translations (src/i18n.rs)
no-config-folder = No hay carpeta de configuración donde guardar traducciones.

## Find bar, results, Go to Line, Find in Files (src/search_ui.rs)
toggle-replace = Mostrar u ocultar reemplazar
find-hint = Buscar
match-case = Distinguir mayúsculas
whole-word = Palabra completa
regular-expression = Expresión regular
extended-search = Extendido: \n, \r, \t, \0, \xNN y \uNNNN significan esos caracteres
extended-search-short = Extendido (\n, \t, \xNN)
match-of-total = { $current } de { $total }
matches-found = { $total } encontrados
find-status = { $count }, { $message }
previous-match = Coincidencia anterior (Shift+F3)
next-match = Coincidencia siguiente (F3)
wrap-around = Continuar desde el principio
mark-all-bookmark = Marcar todo y añadir marcadores
close-esc = Cerrar (Esc)
replace-hint-regex = Reemplazar  ($1 o \1 para grupos)
replace-hint-extended = Reemplazar  (se admiten \n, \t, \xNN)
replace = Reemplazar
replace-hover = Reemplazar esta coincidencia y buscar la siguiente
replace-all = Reemplazar todo
in-selection = En la selección
search-stopped = detenida
searched-files = { $count ->
    [one] { $count } archivo buscado
   *[other] { $count } archivos buscados
}
results-hits = { $count ->
    [one] { $count }{ $plus } resultado
   *[other] { $count }{ $plus } resultados
}
results-in-files = { $count ->
    [one] { $found } en { $count } archivo
   *[other] { $found } en { $count } archivos
}
results-in-documents = { $count ->
    [one] { $found } en { $count } documento
   *[other] { $found } en { $count } documentos
}
stop = Detener
close-results = Cerrar resultados
collapse-all = Contraer todo
expand-all = Expandir todo
go-to-line-title = Ir a la línea
go-to-line-prompt = Número de línea (1–{ $lines }), o línea:columna
enter-line-number = Escribe un número de línea
go = Ir
find-in-files-heading = Buscar en archivos
find-what = Buscar
folder = Carpeta
browse = Examinar…
search-in-folder = Buscar en la carpeta
file-filter-hint = *.rs *.toml   (vacío para todos, !nombre para omitir)
file-filter-hover = Patrones de nombre de archivo separados por espacios o punto y coma. Empieza uno con ! para excluir esos archivos.
include-subfolders = Incluir subcarpetas
include-hidden = Incluir archivos y carpetas ocultos
find-all = Buscar todo
enter-search-text = Escribe algo que buscar.
folder-missing = Esa carpeta no existe.
results-problems = { $count ->
    [one] { $count }{ $plus } problema
   *[other] { $count }{ $plus } problemas
}

## Rename, Move to Trash, Summary, file changed on disk (src/file_ui.rs)
rename-file-title = Cambiar nombre del archivo
new-name = Nombre nuevo
rename = Cambiar nombre
move-to-trash-title = Mover a la papelera
move-to-trash-question = { $windows ->
    [yes] ¿Mover "{ $name }" a la Papelera de reciclaje y cerrar su pestaña? Puedes restaurarlo desde la Papelera de reciclaje.
   *[no] ¿Mover "{ $name }" a la papelera y cerrar su pestaña? Puedes restaurarlo desde la papelera.
}
summary-title = Resumen
summary-file = Archivo
summary-created = Creado
summary-modified = Modificado
summary-characters = Caracteres (sin fines de línea)
summary-words = Palabras
summary-lines = Líneas
summary-size = Tamaño al guardar
summary-bytes = { $count ->
    [one] { $bytes } byte
   *[other] { $bytes } bytes
}
summary-cant-encode = no se puede guardar en esta codificación
summary-selection = Selección
summary-selection-size = { $line_count ->
    [one] { $characters } caracteres en { $lines } línea
   *[other] { $characters } caracteres en { $lines } líneas
}
disk-changed = Otro programa cambió este archivo.
disk-changed-unsaved = Otro programa cambió este archivo y aquí tienes cambios sin guardar. Recargar los reemplaza (Deshacer los recupera).
disk-deleted = Otro programa eliminó o movió este archivo.
reload = Recargar
keep-my-version = Mantener mi versión
keep-open = Mantener abierto

## Folder panel (src/folder_ui.rs)
no-folder-open = No hay ninguna carpeta abierta.
close-folder-panel = Cerrar el panel de carpetas
refresh-folder = Volver a leer la carpeta
collapse-folders = Contraer todas las carpetas
show-hidden-files = Mostrar archivos ocultos
open = Abrir
find-in-files-here = Buscar en archivos aquí…
copy-path = Copiar ruta
folder-truncated = Solo se muestran las primeras { $count } entradas

## Function list (src/function_list_ui.rs)
show-caret-symbol = Mostrar dónde está el cursor
filter-hint = Filtrar
reading-document = Leyendo el documento…
nothing-found-in-document = No se encontró nada en este documento.
line-number = Línea { $line }

## Command palette (src/palette_ui.rs)
nothing-matches = Nada coincide

## Shortcut mapper (src/shortcut_ui.rs)
shortcut-reserved-clipboard = Ctrl+C, Ctrl+X y Ctrl+V se reservan para copiar, cortar y pegar
shortcut-mapper = Asignación de atajos
command-or-shortcut = Comando o atajo
only-assigned = Solo comandos con atajo
shortcut-in-use = { $shortcut } ya lo usa "{ $command }".
move-it-here = Moverlo aquí
select-command = Selecciona un comando para cambiar su atajo.
shortcut-fixed = "{ $command }" pertenece al cuadro de texto y no se puede cambiar.
press-shortcut = Pulsa el nuevo atajo… (Esc cancela)
change-shortcut = Cambiar…
set-shortcut = Asignar…
add-another-shortcut = Añadir otro…
remove = Quitar
reset-to-default = Restablecer predeterminado
reset-all-shortcuts = Restablecer todos los atajos
shortcut-rule = Los atajos necesitan Ctrl o Alt, salvo que usen una tecla de función (F1–F12).
command = Comando
shortcut = Atajo

## Theme colours (src/style_ui.rs)
style-background = Fondo
style-text = Texto
style-caret = Cursor
style-selection = Selección
style-current-line = Línea actual
style-line-numbers = Números de línea
style-indent-guides = Guías de sangría
style-whitespace = Símbolos de espacios en blanco
style-search-matches = Coincidencias de búsqueda
style-accent = Énfasis (pestañas, botones, plegado)
unset-colour = No definir este color
set-colour = Definir
choose-colour = Elige un color
theme-read-failed = No se pudo leer el archivo del tema: { $error }
theme-colours = Colores del tema
theme-label = Tema:
discard-changes = Descartar cambios
save-as-new-theme = Guardar como tema nuevo…
new-theme-name = Nombre del tema nuevo:
new-theme-name-hint = Nombre para el tema nuevo
theme-name-taken = Ya hay un tema con este nombre
theme-colours-hint = Los cambios se ven al instante. Guardar los conserva; cerrar esta ventana sin guardar los descarta.
style-editor = Editor
style-syntax = Sintaxis
rule-filter-hint = Nombre o ámbito de la regla, como string o keyword
rules-hint = Cada regla colorea el texto cuyo ámbito coincide. Separa varios ámbitos con comas.
rule-name = Nombre
rule-scope = Ámbito
rule-style = Estilo
no-name = (sin nombre)
delete-rule = Eliminar esta regla
add-rule = Añadir regla
new-rule = Regla nueva

## Notices, JSON tree, XPath panel (src/text_tools_ui.rs)
go-to = Ir a
dismiss = Descartar
json-too-large = Este documento es demasiado grande para mostrarlo como árbol.
close-json-tree = Cerrar el árbol JSON
json-filter-hint = Filtrar claves y valores
json-not-valid = JSON no válido
line-column-message = Línea { $line }, columna { $column }: { $message }
go-to-error = Ir al error
no-matches = Sin coincidencias
first-matches = Primeras { $count } coincidencias
copy-key = Copiar clave
copy-value = Copiar valor
expand-all-inside = Expandir todo el interior
close-xpath-panel = Cerrar el panel XPath
evaluate = Evaluar
recent = Recientes
recent-expressions = Expresiones recientes
result-label = Resultado:
xpath-nodes = { $count ->
    [one] { $count } nodo
   *[other] { $count } nodos
}
xpath-nodes-stale = { $count ->
    [one] { $count } nodo (el documento ha cambiado; evalúa de nuevo para actualizar)
   *[other] { $count } nodos (el documento ha cambiado; evalúa de nuevo para actualizar)
}

## Column editor and macros (src/tools_ui.rs)
column-editor-title = Editor de columnas
column-editor-hint = Inserta en cada cursor, o en la columna del cursor en cada línea por debajo.
hex-offset = Posición 0x{ $hex } ({ $dec })
hex-selected = { $count } bytes seleccionados
hex-size = { $count } bytes en total
hex-read-only = Esta pestaña es de solo lectura.
hex-cant-encode = "{ $char }" no se puede escribir en { $encoding }.
hex-paste-hint = Pegue dígitos hexadecimales, como 48 65 6C.
hex-invalid = Sin cambios: esos bytes no son texto { $encoding } válido.
hex-round-trip = Sin cambios: el archivo se guardaría de otra forma (revise los finales de línea).
char-search-hint = Buscar: é, U+00E9, 233 o un nombre
char-panel-hint = Haga doble clic en un carácter para insertarlo. Clic derecho para copiar.
char-insert-character = Insertar el carácter
char-insert-html = Insertar la entidad HTML
char-insert-hex = Insertar el código (U+…)
char-insert-decimal = Insertar el código decimal
char-col-char = Car.
char-col-dec = Dec
char-col-hex = Hex
char-col-html = HTML
char-col-name = Nombre
char-copy-character = Copiar carácter
char-copy-html = Copiar entidad HTML
char-copy-code = Copiar código (U+…)
text-to-insert = Texto a insertar
numbers-to-insert = Números a insertar
initial-number = Número inicial
increase-by = Incrementar en
repeat = Repetir
leading-zeros = Ceros a la izquierda
format-dec = Dec
format-hex = Hex
format-oct = Oct
format-bin = Bin
insert = Insertar
must-be-whole-number = { $field } debe ser un número entero
run-macro-title = Ejecutar una macro varias veces
run = Ejecutar
times = veces
run-to-end = Ejecutar hasta el final del archivo
record-macro-first = Graba primero una macro.
save-macro-title = Guardar macro actual

## Comparison view (src/compare_ui.rs)
no-differences = Sin diferencias
lines-changed = { $count } cambiadas
lines-added = { $count } añadidas
lines-removed = { $count } eliminadas
lines-moved = { $count } movidas
compare-lines-summary = Líneas: { $parts }
first = Primera
first-difference = Primera diferencia
previous-difference-hover = Diferencia anterior (Shift+F7)
next-difference-hover = Diferencia siguiente (F7)
last = Última
last-difference = Última diferencia
identical = Idénticos
difference-of = Diferencia { $current } de { $total }
differences-count = { $count ->
    [one] { $count } diferencia
   *[other] { $count } diferencias
}
compare-approximate = (aproximado: los archivos son muy diferentes)
close-comparison-hover = Cerrar la comparación
export-patch = Exportar parche
export-patch-hover = Abrir las diferencias como diff unificado en una nueva pestaña
swap-sides = Intercambiar lados
options = Opciones
line-count = { $count ->
    [one] { $count } línea
   *[other] { $count } líneas
}

## CSV column types and delimiters (src/tools/csv.rs)
column-type-text = Texto
column-type-integer = Entero
column-type-decimal = Decimal
delimiter-comma = Coma
delimiter-semicolon = Punto y coma
delimiter-tab = Tab
delimiter-pipe = Barra vertical
delimiter-space = Espacio

## CsvQuery panel (src/csv_query.rs)
query-engine-stopped = El motor de consultas se detuvo. Inténtalo de nuevo.
csv-shape = { $rows } filas × { $columns } columnas
csv-this-hover = La pestaña actual es la tabla `this`; las demás pestañas son tablas con el nombre de sus archivos.
csv-open-file = Abre un archivo CSV o TSV para consultarlo.
close-csvquery = Cerrar CsvQuery
tables = Tablas
click-to-insert-name = Clic para insertar un nombre
no-queries-yet = Aún no hay consultas
run-query-hover = Ejecutar la consulta (Ctrl+Enter)
results = Resultados
columns = Columnas
csv-tables-list = Tablas: this, { $names }
csv-rows-changed = { $count ->
    [one] { $count } fila cambiada en la copia de consulta (el archivo no cambia)
   *[other] { $count } filas cambiadas en la copia de consulta (el archivo no cambia)
}
csv-first-rows = Primeras { $rows } filas · { $ms } ms
csv-rows = { $count ->
    [one] { $rows } fila · { $ms } ms
   *[other] { $rows } filas · { $ms } ms
}
open-in-new-tab = Abrir en nueva pestaña
open-in-new-tab-hover = Escribir todas las filas del resultado en una nueva pestaña como CSV
copy-results-hover = Copiar los resultados, separados por tabulaciones para pegar en una hoja de cálculo
click-to-sort = Clic para ordenar
copy-row = Copiar fila
csv-run-for-columns = Ejecuta una consulta para ver las columnas de esta pestaña.
csv-column = Columna
csv-type = Tipo
csv-filled = Con valor
csv-empty = Vacías
csv-distinct = Distintos
csv-min = Mín
csv-max = Máx
csv-average = Media
csv-type-hover = Cómo se guarda y compara la columna. Auto elige el tipo más estrecho que admite todos los valores.
auto-with = Auto ({ $detected })
delimiter = Delimitador
auto-quotes = Comillas auto
quotes-char = Comillas { $char }
no-quotes = Sin comillas
quote-character = Carácter de comillas
header-row = Fila de encabezado
header-row-hover = La primera fila da nombre a las columnas
reset = Restablecer
reset-format-hover = Volver al formato detectado

## Update check (src/update.rs)
update-bad-json = La lista de versiones no tenía sentido: { $error }
update-no-version = La lista de versiones no tiene versión
update-not-found = No se encontró ninguna versión publicada (HTTP 404). La búsqueda de actualizaciones solo funciona mientras el repositorio de Ettin Edit sea público.
update-http-error = GitHub respondió con HTTP { $code }.
update-unreachable = No se pudo conectar con GitHub: { $error }
update-read-failed = No se pudo leer la respuesta de GitHub: { $error }
check-for-updates-title = Buscar actualizaciones
update-asking = Consultando a GitHub la última versión…
update-available = Ettin Edit { $version } está disponible.
update-current = Tienes la versión { $version }.
open-download-page = Abrir página de descarga
dont-check-automatically = No buscar automáticamente
update-latest = Tienes la última versión, { $version }.
update-newer-local = Tienes la versión { $version }; la más reciente publicada es { $latest }.
update-failed = No se pudieron buscar actualizaciones.

## User manual window (src/manual.rs)
back = Atrás
forward = Adelante
search-manual = Buscar en el manual
open-on-web = Abrir en la web
web-manual-failed = No se pudo abrir el manual web: { $error }
open-url-failed = No se pudo abrir { $url }: { $error }
copy-to-clipboard = Copiar al portapapeles

## Encoding groups (src/encoding.rs)
enc-western-european = Europa occidental
enc-central-european = Europa central
enc-cyrillic = Cirílico
enc-greek = Griego
enc-turkish = Turco
enc-baltic = Báltico
enc-nordic = Nórdico
enc-hebrew = Hebreo
enc-arabic = Árabe
enc-vietnamese = Vietnamita
enc-thai = Tailandés
enc-romanian = Rumano
enc-chinese = Chino
enc-japanese = Japonés
enc-korean = Coreano

## Files, folders, terminal and Git (src/system.rs)
explorer-failed = No se pudo abrir el Explorador: { $error }
no-parent-folder = { $path } no tiene carpeta
terminal-failed = No se pudo abrir una terminal: { $error }
no-terminal = No se encontró ninguna terminal. Define la variable de entorno TERMINAL con el comando de tu terminal.
git-missing = Git no está instalado (o no está en el PATH).
git-failed = No se pudo ejecutar git: { $error }
not-in-git = { $path } no está en un repositorio Git.
not-in-repository = { $path } no está en el repositorio de { $root }.
git-not-committed = Este archivo aún no está en el último commit.
git-no-commits = El repositorio aún no tiene commits.

## Themes (src/theme.rs)
themes-write-failed = No se pudieron escribir los temas integrados en { $path }: { $error }
theme-file-problem = El archivo de tema { $path } tiene un problema, así que se usa el tema original { $name }: { $error }
theme-skipped = Se omitió el tema { $path }: { $error }

## Fonts (src/fonts.rs)
font-unusable = { $path } no es una fuente que Ettin Edit pueda usar.
font-read-failed = No se pudo leer la fuente del editor { $path }: { $error }

## Editor (src/editor/view.rs)
completion-keyword = palabra clave
completion-function = función

## Manage Plugins and plugin log (src/plugins/ui.rs)
plugins-warning = Los complementos se ejecutan como programas aparte con tu acceso a archivos y a la red. Ejecuta solo complementos de confianza.
plugins-installed-tab = Instalados ({ $count })
plugins-available-count = Disponibles ({ $count })
plugins-available = Disponibles
install-from-folder = Instalar desde carpeta…
install-from-folder-hover = Copiar la carpeta de un complemento (una con plugin.json) a la carpeta de complementos
install-from-file = Instalar desde archivo…
install-from-file-hover = Instalar un complemento desde un archivo .zip
remove-plugin-title = ¿Quitar “{ $name }”?
remove-plugin-hint = Su carpeta se mueve a la papelera, así que puedes recuperarla desde allí.
no-plugins-yet = Aún no hay complementos.
no-plugins-hint = Instala los ejemplos para ver cómo funcionan, busca complementos en Disponibles o pon la carpeta de un complemento en la carpeta de complementos.
found-on-path = { $program } (encontrado en el PATH)
reload-plugins-hover = Detener todos los complementos, volver a buscarlos e iniciarlos
plugin-filter-hint = Nombre, autor o descripción
refresh = Actualizar
refresh-index-hover = Volver a leer el índice de complementos
reading-plugin-index = Leyendo el índice de complementos…
try-again = Reintentar
index-empty = El índice de complementos no incluye ninguno.
no-plugins-match = Ningún complemento coincide con el filtro.
plugin-index = Índice
plugin-index-hover = La dirección de la lista JSON de complementos. Pulsa Enter para leerla.
default = Predeterminado
default-index-hover = Usar el índice de complementos propio de Ettin Edit
needs-version = Requiere Ettin Edit { $version }
update-to = Actualizar a { $version }
installed-no-version = Instalado: sin versión
installed-version = Instalado: { $version }
installed = Instalado
install = Instalar
by-author = Por { $author }
home-page = Página de inicio
plugin-toggle-hover = Activar o desactivar el complemento
plugin-running = En ejecución
plugin-stopped = Detenido
plugin-off = Desactivado
plugin-needs-permission = Esperando permiso
plugin-not-running = No se está ejecutando
update-plugin-hover = Descargar la nueva versión del índice de complementos; la antigua se mueve a la papelera
restart = Reiniciar
allow-dots = Permitir…
start = Iniciar
open-folder = Abrir carpeta
remove-dots = Quitar…
remove-plugin-hover = Mover la carpeta del complemento a la papelera
trust-changes = No preguntar cuando cambie
trust-changes-hover = Para un complemento que estás escribiendo: ejecutarlo tras cada cambio sin volver a preguntar
clear = Borrar
copy-all = Copiar todo
plugin-log-hint = Mensajes y salida de print() y console.log() de los complementos
nothing-yet = Nada todavía.
allow-plugin-title = ¿Permitir que “{ $name }” se ejecute?
plugin-changed = Este complemento ha cambiado desde que lo permitiste.
allow-plugin-warning = Los complementos se ejecutan como programas aparte con tu cuenta de usuario. Este podría leer y cambiar tus documentos abiertos y tus archivos, y usar la red. Permite solo complementos de confianza.
author = Autor
runs = Ejecuta
allow = Permitir
dont-allow = No permitir
dont-allow-hover = Desactivar el complemento. Puedes activarlo en Administrar complementos.

## Installing plugins (src/plugins/store.rs)
index-bad-json = El índice de complementos no es JSON válido: { $error }
index-no-list = El índice de complementos no tiene una lista "plugins"
zip-unsafe-name = El zip tiene un nombre de archivo inseguro, así que no se instaló nada: { $name }
zip-no-manifest = El zip no tiene { $file }, ni en la raíz ni dentro de una única carpeta.
zip-unreadable = No es un archivo zip que Ettin Edit pueda leer: { $error }
zip-too-many-files = El zip tiene más de { $count } archivos.
zip-damaged = El zip está dañado: { $error }
zip-has-link = El zip tiene un enlace, y los complementos no pueden contenerlos: { $name }
zip-encrypted = El zip está protegido con contraseña.
zip-too-big = El zip ocupa más de 256 MB al descomprimirlo.
checksum-mismatch = La descarga no coincide con su suma de comprobación SHA-256 (se esperaba { $expected }, se obtuvo { $got }). Puede estar dañada o alterada; no se instaló nada.
already-in-plugins-folder = Esa carpeta ya está en la carpeta de complementos.
copy-failed = No se pudo copiar { $path }: { $error }
not-a-plugin = No es un complemento que Ettin Edit pueda ejecutar: { $error }
plugin-trash-old-failed = No se pudo mover la copia instalada de { $id } a la papelera: { $error }
plugin-move-failed = No se pudo colocar el complemento en su sitio: { $error }
index-unreachable = No se pudo acceder al índice de complementos ({ $error }). Comprueba la dirección y que sea pública:
    { $url }
installing = Instalando { $name }…

## Running plugins (src/plugins/mod.rs)
plugin-started = Iniciado con { $program } { $script }
runtime-missing = { $runtime } no está instalado (no se encontró "{ $program }"). Instálalo o indica su ruta en Complementos > Administrar complementos.
plugin-start-failed = No se pudo iniciar { $program }: { $error }
plugin-exited = Dejó de ejecutarse.
plugin-exit-code = Se detuvo con el código de salida { $code }. Consulta el registro de complementos.
plugin-died = Se detuvo. Consulta el registro de complementos.
plugin-stopped-notice = El complemento { $name } se detuvo. Consulta Complementos > Registro de complementos.

## Shortcut names (src/plugins/shortcut.rs)
unknown-modifier = Modificador desconocido "{ $modifier }" en "{ $shortcut }"
unknown-key = Tecla desconocida "{ $key }" en "{ $shortcut }"
shortcut-needs-modifier = "{ $shortcut }" necesita Ctrl o Alt, o interferiría con la escritura
shortcut-reserved = "{ $shortcut }" se reserva para cortar, copiar y pegar

## Completion lists (src/lang_api.rs)
api-file-skipped = Se omitió el archivo de API { $name }: { $error }

## Language files (src/highlight.rs)
language-file-skipped = Se omitió el archivo de lenguaje { $file }: { $error }

## Plugin panels (src/plugins/panel.rs)
close-panel = Cerrar panel

## Shortcut settings (src/actions.rs)
shortcut-settings-error = Configuración de atajos: { $error }

## Toolbar (src/ribbon*.rs, src/app/ribbon.rs)
apply = Aplicar
customise-toolbar = Personalizar barra de herramientas
customise-toolbar-dots = Personalizar barra de herramientas…
toolbar-show = Barra de herramientas
toolbar-collapse-menu = Contraer barra de herramientas
toolbar-collapse = Contraer la barra de herramientas
toolbar-expand = Mostrar los botones de la barra de herramientas
toolbar-hide = Ocultar la barra de herramientas
menu-bar-show = Barra de menús
menu-bar-hide = Ocultar la barra de menús
menu-bar-show-again = Mostrar la barra de menús
menu-bar-hidden-notice = La barra de menús está oculta. Pulse Alt para verla un momento, o pulse { $shortcut } para recuperarla.
menu-bar-hidden-notice-alt = La barra de menús está oculta. Pulse Alt para verla y elija Ver > Barra de menús para recuperarla.
toolbar-empty = Aún no hay botones. Haz clic derecho para personalizar la barra de herramientas.
toolbar-remove-button = Quitar de la barra de herramientas
toolbar-read-failed = No se pudo leer el diseño de la barra de herramientas { $path }: { $error }. Se usa la barra predeterminada.
toolbar-save-failed = No se pudo guardar el diseño de la barra de herramientas en { $path }: { $error }
toolbar-commands = Comandos
toolbar-filter-hint = Filtrar comandos
toolbar-layout = Barra de herramientas
toolbar-drag-hint = Arrastra comandos aquí, o arrastra pestañas, grupos y botones para moverlos.
toolbar-add = Añadir ›
toolbar-add-tip = Añadir el comando elegido después de la selección de la derecha
toolbar-remove = ‹ Quitar
toolbar-move-up = Subir
toolbar-move-down = Bajar
toolbar-new-tab = Nueva pestaña
toolbar-new-group = Nuevo grupo
toolbar-new-tab-name = Nueva pestaña
toolbar-new-group-name = Nuevo grupo
toolbar-no-tabs = No hay pestañas. Añade una con Nueva pestaña.
toolbar-tab-visible = Mostrar esta pestaña
toolbar-unavailable = Este comando no está disponible ahora (puede que su complemento no esté en ejecución). El botón volverá cuando lo esté.
toolbar-select-hint = Selecciona una pestaña, un grupo o un botón para cambiarlo.
toolbar-tab-name = Nombre de la pestaña:
toolbar-group-name = Nombre del grupo:
toolbar-label = Etiqueta:
toolbar-size = Tamaño:
toolbar-size-large = Grande
toolbar-size-small = Pequeño
toolbar-icon = Icono:
toolbar-choose-icon = Elegir un icono
toolbar-change-icon = Cambiar…
toolbar-default-icon = Usar el icono del comando
toolbar-icon-filter-hint = Filtrar iconos
toolbar-icon-credit = Iconos: Phosphor (licencia MIT)
toolbar-reset = Restablecer valores predeterminados
toolbar-tab-home = Inicio
toolbar-tab-edit = Editar
toolbar-tab-view = Ver
toolbar-tab-tools = Herramientas
toolbar-group-file = Archivo
toolbar-group-clipboard = Portapapeles
toolbar-group-history = Deshacer
toolbar-group-find = Buscar
toolbar-group-tabs = Pestañas
toolbar-group-lines = Líneas
toolbar-group-sort = Ordenar
toolbar-group-case = Mayúsculas
toolbar-group-comment = Comentario
toolbar-group-blanks = Espacios
toolbar-group-selection = Selección
toolbar-group-show = Mostrar
toolbar-group-zoom = Zoom
toolbar-group-folding = Plegado
toolbar-group-panels = Paneles
toolbar-group-window = Ventana
toolbar-group-compare = Comparar
toolbar-group-json = JSON
toolbar-group-xml = XML
toolbar-group-data = Datos
toolbar-group-macros = Macros
toolbar-group-customise = Personalizar
