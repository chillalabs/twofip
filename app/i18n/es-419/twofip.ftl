# 2fip — Español (Latinoamérica)
# Traducción completa. El español de España (i18n/es) solo redefine las
# claves que cambian; el resto se toma de este archivo.

## Barra de menú
menu-file = Archivo
menu-edit = Editar
menu-view = Ver
menu-help = Ayuda
help = Ayuda
new-tab = Nueva pestaña
new-folder = Nueva carpeta
close-tab = Cerrar pestaña
quit = Salir
cut = Cortar
copy = Copiar
paste = Pegar
select-all = Seleccionar todo
rename-ellipsis = Cambiar nombre…
delete = Eliminar
list-view = Vista de lista
grid-view = Vista de cuadrícula
favorites = Favoritos
settings = Configuración

## Menú contextual
open = Abrir
open-with-ellipsis = Abrir con…
view = Ver
edit = Editar
calculate-size = Calcular tamaño
compress-ellipsis = Comprimir…
show-details = Ver detalles

## Columnas de la lista
column-name = Nombre
column-ext = Ext
column-size = Tamaño
column-modified = Modificado

## Operaciones en curso (panel de progreso)
op-copying = Copiando
op-moving = Moviendo
op-deleting = Eliminando
op-compressing = Comprimiendo
op-copy-failed = Error al copiar
op-move-failed = Error al mover
op-delete-failed = Error al eliminar
op-compress-failed = Error al comprimir
op-progress = { $operation } { $percent }%
op-files-done = { $done } de { $total } archivos
op-bytes-done = { $done } de { $total }
op-preparing = Preparando…
op-queued = { $count } en espera
op-queue-remove = Quitar
close = Cerrar
cancel = Cancelar

## Paneles
path-not-found = No se encontró: { $path }
status-items = { $count ->
    [one] 1 elemento
   *[other] { $count } elementos
}
status-items-filtered = { $count ->
    [one] 1 elemento coincide con el filtro
   *[other] { $count } elementos coinciden con el filtro
}
status-selected = { $selected } de { $total } seleccionados ({ $size })
tooltip-back = Atrás (Alt+←)
tooltip-forward = Adelante (Alt+→)
tooltip-up = Subir una carpeta (Retroceso)
tooltip-drive = Unidad
tooltip-tree-off = Árbol de carpetas: oculto (clic para mostrarlo junto al panel izquierdo)
tooltip-tree-left = Árbol de carpetas: panel izquierdo (clic para el panel derecho)
tooltip-tree-right = Árbol de carpetas: panel derecho (clic para ambos paneles)
tooltip-tree-both = Árbol de carpetas: ambos paneles (clic para ocultarlo)
tooltip-eject = Expulsar (para desconectar con seguridad)
drive-system = Sistema
drive-home = Carpeta personal
drive-removable = USB
drive-not-mounted = sin montar
drive-mount-failed = No se pudo abrir la unidad: { $error }
drive-eject-failed = No se pudo expulsar la unidad: { $error }
tooltip-new-tab = Nueva pestaña (Ctrl+T)
tooltip-refresh = Actualizar ambos paneles (Ctrl+R)
tooltip-edit-path = Editar ruta (Ctrl+L)
tooltip-cancel-esc = Cancelar (Esc)
no-tab-open = No hay pestañas abiertas
filter-placeholder = Filtrar (p. ej., informe o *.txt)
path-placeholder = Escribe la ruta de una carpeta

## Configuración
settings-general = General
settings-show-hidden = Mostrar archivos ocultos
settings-show-hidden-description = Archivos y carpetas cuyo nombre empieza con un punto
settings-separate-ext = Mostrar la extensión en su propia columna
settings-separate-ext-description = Vista de lista: columnas Nombre y Ext separadas, como en Total Commander
settings-thumbnails = Mostrar miniaturas
settings-thumbnails-description = Vistas previas de imágenes, PDF, videos y fuentes
settings-language = Idioma
language-system = Predeterminado del sistema
settings-theme = Tema
settings-color-theme = Tema de colores
settings-color-theme-description = Solo afecta a 2fip; Sistema sigue la apariencia de COSMIC
color-theme-system = Sistema
color-theme-light = Claro
color-theme-dark = Oscuro
settings-icon-style = Estilo de íconos
settings-icon-style-description = El estilo colorido es igual al de la aplicación COSMIC Files
icon-style-colorful = Colorido
icon-style-monochrome = Monocromático
icon-style-vivid = Vívido
icon-style-classic = Estilo Windows
icon-style-soft = Estilo macOS
settings-font-size = Tamaño de los nombres de archivo
settings-font-size-description = Un texto más pequeño muestra más archivos en pantalla
settings-row-lines = Líneas entre filas
settings-row-lines-description = En la vista de lista, una línea fina entre archivos y carpetas, como en COSMIC Files
font-size-medium = Mediano ({ $px } px)
font-size-small = Pequeño ({ $px } px)
font-size-smaller = Más pequeño ({ $px } px)
font-size-very-small = Muy pequeño ({ $px } px)
font-size-tiny = Diminuto ({ $px } px)
settings-corners = Esquinas redondeadas
settings-corners-description = Cuán redondeados son la ventana, los botones y los paneles
corners-square = Rectas
corners-small = Poco redondeadas
corners-medium = Redondeadas
corners-large = Muy redondeadas (COSMIC)

## Favoritos
favorites-saved = Carpetas guardadas
favorites-empty = Aún no hay favoritos guardados.
favorites-move-up = Subir (Ctrl+↑)
favorites-move-down = Bajar (Ctrl+↓)
favorites-add-current = Agregar la carpeta actual
favorites-add = Agregar
favorites-duplicate-title = Favorito ya guardado
favorites-duplicate-path = Esta carpeta ya está guardada como "{ $name }".
favorites-duplicate-name = Ya hay un favorito llamado "{ $name }", que apunta a { $path }.

## Diálogo "El archivo ya existe"
conflict-title = El archivo ya existe
conflict-body = "{ $existing }" ya existe en el destino. ¿Quieres reemplazarlo por "{ $new }"?
skip-all = Omitir todos
replace-all = Reemplazar todos
replace = Reemplazar
skip = Omitir

## Diálogo Comprimir
compress = Comprimir
compress-one = Comprimir "{ $name }" en un archivo zip.
compress-many = Comprimir { $count } elementos en un archivo zip.
archive-name = Nombre del archivo comprimido
pack = Comprimir archivos
pack-one = Comprimir "{ $name }" en un archivo comprimido.
pack-many = Comprimir { $count } elementos en un archivo comprimido.
pack-path = Archivo comprimido (carpeta y nombre)
pack-format = Formato
pack-password = Contraseña (opcional, cifra el archivo)
pack-password-unavailable = Contraseña: solo para zip y 7z
pack-move = Mover al archivo comprimido (borrar los originales después)
pack-single-file-only = gz y xz comprimen un solo archivo: elige tgz, txz u otro formato para carpetas o varios archivos.
unpack = Extraer archivos
unpack-one = Extraer "{ $name }" en { $dest }.
unpack-many = Extraer { $count } archivos comprimidos en { $dest }.
unpack-keep-paths = Extraer con las rutas guardadas en el archivo
unpack-overwrite = Sobrescribir archivos existentes
unpack-password = Contraseña de archivos cifrados
tooltip-pack = Comprimir archivos (Alt+F5)
tooltip-unpack = Extraer archivos en el otro panel (Alt+F9)
extract-ellipsis = Extraer…
op-unpacking = Extrayendo
op-unpack-failed = Falló la extracción
default-archive-name = Comprimido

## Diálogo Nueva carpeta
new-folder-default-name = Nueva carpeta
folder-name = Nombre de la carpeta
create = Crear

## Diálogo Cambiar nombre
rename = Cambiar nombre
renaming = Cambiando el nombre de "{ $name }"
new-name = Nombre nuevo

## Diálogo Eliminar
delete-one = ¿Mover "{ $name }" a la papelera?
delete-many = ¿Mover { $count } elementos a la papelera?
delete-permanently = Eliminar definitivamente
delete-permanently-one = ¿Eliminar "{ $name }" definitivamente? No se puede deshacer.
delete-permanently-many = ¿Eliminar { $count } elementos definitivamente? No se puede deshacer.

## Barra de teclas de función (la app agrega el prefijo "F2", etc.)
fkey-rename = Renombrar
fkey-view = Ver
fkey-edit = Editar
fkey-copy = Copiar
fkey-move = Mover
fkey-mkdir = Crear carpeta
fkey-delete = Eliminar
fkey-terminal = Terminal
fkey-exit = Salir

## Diálogo Abrir con
open-with = Abrir con
open-with-remember = Usar siempre esta aplicación para archivos { $mime }
open-with-one = Elige una aplicación para abrir "{ $name }".
open-with-many = Elige una aplicación para abrir { $count } elementos.
loading-apps = Cargando aplicaciones…
no-apps = No se encontraron aplicaciones.
recommended-apps = Aplicaciones recomendadas
other-apps = Otras aplicaciones
default-app = Predeterminada

## Diálogo Detalles
details-items-title = { $count } elementos
details-calculating = Calculando…
details-error = Error
details-contents = { $files } archivos y { $folders } carpetas en su interior
details-size = { $size } ({ $bytes } bytes)
details-items = Elementos
details-location = Ubicación
details-total-size = Tamaño total
details-type = Tipo
details-name = Nombre
details-link-target = Destino del enlace
details-size-label = Tamaño
details-modified = Modificado
details-accessed = Último acceso
details-created = Creado
details-permissions = Permisos
details-owner = Propietario
details-group = Grupo
details-unknown = Desconocido
kind-folder = Carpeta
kind-file = Archivo
kind-symlink = Enlace simbólico

## Diálogo Buscar archivos
find = Buscar archivos
tooltip-find = Buscar archivos (Ctrl+F)
find-in = En { $path } y sus subcarpetas
find-placeholder = Nombre o patrón (p. ej. informe o *.pdf)
find-search = Buscar
find-stop = Detener
find-hint = Escribe parte de un nombre y presiona Enter.
find-searching = Buscando… { $count } encontrados
find-results = { $count ->
    [one] 1 resultado
   *[other] { $count } resultados
}
find-no-results = No se encontró nada.
find-truncated = Se muestran los primeros { $count } resultados; prueba un nombre más específico.
find-go-to = Ir al archivo
find-listbox = Ver en lista
search-results-tab = Búsqueda: { $pattern }

## Ventana Acerca de
about-2fip = Acerca de 2fip
about-description = Un administrador de archivos de dos paneles, pensado para el teclado e inspirado en Total Commander.
about-version = Versión:
about-repository = Repositorio:
about-license = Licencia:

## Diálogo Conectar a un servidor
connect-to-server = Conectar a un servidor…
disconnect = Desconectar
connect-title = Conectar a un servidor
connect = Conectar
connect-host = Servidor (p. ej. archivos.ejemplo.com)
connect-port = Puerto
connect-user = Usuario
connect-password = Contraseña
connect-hint-sftp = Primero se prueban tu agente SSH y las claves de ~/.ssh, así que la contraseña es opcional.
connect-hint-ftp = Deja el usuario vacío para entrar de forma anónima. FTP envía la contraseña sin cifrar: mejor usa FTPS o SFTP.
connect-hint-ftps = FTP cifrado con TLS. El servidor necesita un certificado válido.
connect-connecting = Conectando…
connect-error-host = Escribe el nombre o la dirección del servidor.
connect-error-port = El puerto debe ser un número.
connect-error-login = No se pudo iniciar sesión: revisa el usuario y la contraseña.
connect-unknown-host-title = Servidor desconocido
connect-unknown-host-body = 2fip nunca se conectó a { $host }. Comprueba que esta huella de la clave coincida con la del servidor y confía en ella para conectarte. Se guardará en ~/.ssh/known_hosts.
connect-trust = Confiar y conectar
connect-key-changed-title = La clave del servidor cambió
connect-key-changed-body = La clave de { $host } no coincide con la guardada en ~/.ssh/known_hosts. Alguien podría estar interceptando la conexión, así que 2fip no se conectará. Si reinstalaron el servidor, borra su línea antigua de ~/.ssh/known_hosts.

## Panel Conexiones
connections = Conexiones
tooltip-connections = Conexiones
folder-size = Tamaño de carpeta
move-to-trash = Mover a la papelera
open-path-in-new-tab = Abrir ruta en una pestaña nueva
folder-size-loading = Calculando tamaños de carpetas…
folder-size-scanning = { $count } archivos y carpetas, { $size } hasta ahora
folder-size-drive = Unidad: { $used } usados de { $total } · { $free } libres
folder-size-other = Sin contar: { $size } (archivos del sistema y carpetas que 2fip no puede leer, datos del sistema de archivos)
folder-size-unreadable = { $count } carpetas no se pudieron leer (sin permiso)
folder-size-empty = Esta carpeta está vacía
folder-size-parent-share = % de la carpeta superior
folder-size-root-share = % de la carpeta analizada
folder-size-graph = Uso
folder-size-more = … { $count } más
folder-size-selected = { $count } carpetas seleccionadas
connection-add = Agregar conexión
connection-edit = Editar conexión
connection-save-only = Guardar
connection-save = Guardar conexión
connection-open = Abrir
connection-reconnect = Reconectar
connection-status-busy = Conectando…
connection-status-connected = Conectado
connection-status-lost = Conexión perdida
connection-status-closed = Sin conectar
connections-saved = Guardadas
connections-open-unsaved = Abiertas, sin guardar
connections-empty = Todavía no hay conexiones guardadas.
connections-hint = Las contraseñas que elijas recordar se guardan cifradas en el llavero del sistema (GNOME Keyring, KWallet), nunca en los archivos de 2fip.
connect-name = Nombre (opcional)
connect-password-stored = Guardada en el llavero (escribe para cambiarla)
connect-save = Guardar en Conexiones
connect-remember-password = Recordar la contraseña en el llavero del sistema

# Visor F3 y Vista rápida (Ctrl+Q)
quick-view = Vista rápida
viewer-loading = Cargando…
viewer-unreadable = No se puede leer este archivo.
viewer-folder = Esto es una carpeta.
viewer-previous = Archivo anterior (P)
viewer-next = Archivo siguiente (N)
viewer-text = Texto
viewer-hex = Hex
viewer-image = Imagen
viewer-zoom-out = Alejar (−)
viewer-zoom-fit = Ajustar a la ventana (0)
viewer-zoom-in = Acercar (+)
viewer-wrap = Ajustar líneas
viewer-find = Buscar
viewer-find-previous = Buscar anterior (Mayús+F3)
viewer-find-next = Buscar siguiente (F3)
viewer-searching = Buscando…
viewer-not-found = No encontrado
viewer-open-externally = Abrir con la aplicación predeterminada
viewer-close = Cerrar (Esc)

# F4: editor choice, Shift+F4, editing files on servers
new-file = Nuevo archivo
new-file-default-name = Nuevo archivo.txt
file-name = Nombre del archivo
settings-programs = Programas
settings-editor = Editor (F4)
settings-editor-description = La aplicación en la que F4 abre los archivos. Los archivos de un servidor se vuelven a subir cada vez que guardas.
editor-system-default = Editor de texto del sistema
notice-uploaded = { $name } se guardó en el servidor.
notice-upload-failed = No se pudo guardar { $name } en el servidor: { $error }

# Favorites menu (Ctrl+D) and Configure dialog
favorites-remove-current = Quitar la carpeta actual
favorites-configure = Configurar…
favorites-configure-title = Configurar favoritos
favorites-add-item = Agregar la carpeta actual
favorites-delete-item = Eliminar
favorites-up = Subir
favorites-down = Bajar
favorites-title = Título
favorites-folder = Carpeta
favorites-use-current = Usar la carpeta del panel activo
ok = Aceptar

# Folder colors (right-click → Folder color)
folder-color-ellipsis = Color de carpeta…
folder-color-one = Color de “{ $name }”
folder-color-many = Color de { $count } carpetas
folder-color-default = Predeterminado
color-red = Rojo
color-orange = Naranja
color-yellow = Amarillo
color-green = Verde
color-blue = Azul
color-purple = Morado
color-pink = Rosa
color-grey = Gris

# Ctrl+C, Ctrl+V in the same folder: "readme - Copy.txt"
copy-suffix = Copia

# Toolbar: list / grid view
tooltip-list-view = Vista de lista (Ctrl+1)
tooltip-grid-view = Vista de cuadrícula (Ctrl+2)
