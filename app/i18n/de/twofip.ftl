# 2fip — Deutsch (vollständige Übersetzung)

## Menüleiste
menu-file = Datei
menu-edit = Bearbeiten
menu-view = Ansicht
menu-help = Hilfe
help = Hilfe
new-tab = Neuer Tab
new-folder = Neuer Ordner
close-tab = Tab schließen
quit = Beenden
cut = Ausschneiden
copy = Kopieren
paste = Einfügen
select-all = Alles auswählen
rename-ellipsis = Umbenennen…
delete = Löschen
list-view = Listenansicht
grid-view = Rasteransicht
favorites = Favoriten
settings = Einstellungen

## Kontextmenü
open = Öffnen
open-with-ellipsis = Öffnen mit…
view = Anzeigen
edit = Bearbeiten
calculate-size = Größe berechnen
compress-ellipsis = Komprimieren…
show-details = Details anzeigen

## Spalten der Liste
column-name = Name
column-ext = Erw.
column-size = Größe
column-modified = Geändert

## Laufende Vorgänge (Fortschrittsanzeige)
op-copying = Kopieren
op-moving = Verschieben
op-deleting = Löschen
op-compressing = Komprimieren
op-copy-failed = Kopieren fehlgeschlagen
op-move-failed = Verschieben fehlgeschlagen
op-delete-failed = Löschen fehlgeschlagen
op-compress-failed = Komprimieren fehlgeschlagen
op-progress = { $operation } { $percent } %
op-files-done = { $done } von { $total } Dateien
op-bytes-done = { $done } von { $total }
op-preparing = Wird vorbereitet…
op-queued = { $count } wartend
op-queue-remove = Entfernen
close = Schließen
cancel = Abbrechen

## Bereiche
path-not-found = Nicht gefunden: { $path }
status-items = { $count ->
    [one] 1 Element
   *[other] { $count } Elemente
}
status-items-filtered = { $count ->
    [one] 1 Element entspricht dem Filter
   *[other] { $count } Elemente entsprechen dem Filter
}
status-selected = { $selected } von { $total } ausgewählt ({ $size })
tooltip-back = Zurück (Alt+←)
tooltip-forward = Vorwärts (Alt+→)
tooltip-up = Übergeordneter Ordner (Rücktaste)
tooltip-tree-off = Ordnerbaum: aus (Klick zeigt ihn neben dem linken Bereich)
tooltip-tree-left = Ordnerbaum: linker Bereich (Klick für den rechten Bereich)
tooltip-tree-right = Ordnerbaum: rechter Bereich (Klick für beide Bereiche)
tooltip-tree-both = Ordnerbaum: beide Bereiche (Klick blendet ihn aus)
tooltip-eject = Auswerfen (sicher entfernen)
drive-system = System
drive-home = Persönlicher Ordner
drive-removable = USB
drive-not-mounted = nicht eingehängt
drive-mount-failed = Laufwerk konnte nicht geöffnet werden: { $error }
drive-eject-failed = Laufwerk konnte nicht ausgeworfen werden: { $error }
tooltip-new-tab = Neuer Tab (Ctrl+T)
tooltip-refresh = Beide Bereiche aktualisieren (Ctrl+R)
tooltip-edit-path = Pfad bearbeiten (Ctrl+L)
tooltip-cancel-esc = Abbrechen (Esc)
no-tab-open = Kein Tab geöffnet
filter-placeholder = Filter (z. B. bericht oder *.txt)
path-placeholder = Ordnerpfad eingeben

## Einstellungen
settings-general = Allgemein
settings-show-hidden = Versteckte Dateien anzeigen
settings-show-hidden-description = Dateien und Ordner, deren Name mit einem Punkt beginnt
settings-separate-ext = Dateierweiterung in eigener Spalte anzeigen
settings-separate-ext-description = Listenansicht: getrennte Spalten „Name“ und „Erw.“, wie im Total Commander
settings-thumbnails = Vorschaubilder anzeigen
settings-thumbnails-description = Vorschau von Bildern, PDFs, Videos und Schriften
settings-language = Sprache
language-system = Systemstandard
settings-theme = Design
settings-color-theme = Farbschema
settings-color-theme-description = Betrifft nur 2fip; „System“ folgt dem Erscheinungsbild von COSMIC
color-theme-system = System
color-theme-light = Hell
color-theme-dark = Dunkel
settings-icon-style = Symbolstil
settings-icon-style-description = „Farbig“ entspricht der App COSMIC Files
icon-style-colorful = Farbig
icon-style-monochrome = Einfarbig
icon-style-vivid = Lebhaft
icon-style-classic = Windows-Stil
icon-style-soft = macOS-Stil
settings-font-size = Schriftgröße der Dateinamen
settings-font-size-description = Kleinere Schrift zeigt mehr Dateien auf dem Bildschirm
settings-row-lines = Linien zwischen Zeilen
settings-row-lines-description = In der Listenansicht eine dünne Linie zwischen Dateien und Ordnern, wie in COSMIC Files
font-size-medium = Mittel ({ $px } px)
font-size-small = Klein ({ $px } px)
font-size-smaller = Kleiner ({ $px } px)
font-size-very-small = Sehr klein ({ $px } px)
font-size-tiny = Winzig ({ $px } px)
settings-corners = Abgerundete Ecken
settings-corners-description = Wie rund Fenster, Schaltflächen und Bereiche sind
corners-square = Eckig
corners-small = Leicht abgerundet
corners-medium = Abgerundet
corners-large = Stark abgerundet (COSMIC)

## Favoriten
favorites-saved = Gespeicherte Ordner
favorites-empty = Noch keine Favoriten gespeichert.
favorites-move-up = Nach oben (Ctrl+↑)
favorites-move-down = Nach unten (Ctrl+↓)
favorites-add-current = Aktuellen Ordner hinzufügen
favorites-add = Hinzufügen
favorites-duplicate-title = Favorit bereits gespeichert
favorites-duplicate-path = Dieser Ordner ist bereits als „{ $name }“ gespeichert.
favorites-duplicate-name = Es gibt bereits einen Favoriten namens „{ $name }“, der auf { $path } zeigt.

## Dialog „Datei existiert bereits“
conflict-title = Datei existiert bereits
conflict-body = „{ $existing }“ ist im Ziel bereits vorhanden. Durch „{ $new }“ ersetzen?
skip-all = Alle überspringen
replace-all = Alle ersetzen
replace = Ersetzen
skip = Überspringen

## Dialog Komprimieren
compress = Komprimieren
compress-one = „{ $name }“ in ein ZIP-Archiv komprimieren.
compress-many = { $count } Elemente in ein ZIP-Archiv komprimieren.
archive-name = Archivname
pack = Dateien packen
pack-one = „{ $name }“ in ein Archiv packen.
pack-many = { $count } Elemente in ein Archiv packen.
pack-path = Archiv (Ordner und Name)
pack-format = Format
pack-password = Passwort (optional, verschlüsselt das Archiv)
pack-password-unavailable = Passwort: nur für zip und 7z
pack-move = In Archiv verschieben (Originale danach löschen)
pack-single-file-only = gz und xz packen eine einzelne Datei: Wählen Sie tgz, txz oder ein anderes Format für Ordner oder mehrere Dateien.
unpack = Dateien entpacken
unpack-one = „{ $name }“ nach { $dest } entpacken.
unpack-many = { $count } Archive nach { $dest } entpacken.
unpack-keep-paths = Gespeicherte Pfadnamen mit entpacken
unpack-overwrite = Vorhandene Dateien überschreiben
unpack-password = Passwort für verschlüsselte Dateien
tooltip-pack = Dateien packen (Alt+F5)
tooltip-unpack = Dateien in den anderen Bereich entpacken (Alt+F9)
extract-ellipsis = Entpacken…
op-unpacking = Entpacken
op-unpack-failed = Entpacken fehlgeschlagen
default-archive-name = Archiv

## Dialog Neuer Ordner
new-folder-default-name = Neuer Ordner
folder-name = Ordnername
create = Erstellen

## Dialog Umbenennen
rename = Umbenennen
renaming = „{ $name }“ umbenennen
new-name = Neuer Name

## Dialog Löschen
delete-one = „{ $name }“ in den Papierkorb verschieben?
delete-many = { $count } Elemente in den Papierkorb verschieben?
delete-permanently = Endgültig löschen
delete-permanently-one = „{ $name }“ endgültig löschen? Das kann nicht rückgängig gemacht werden.
delete-permanently-many = { $count } Elemente endgültig löschen? Das kann nicht rückgängig gemacht werden.

## Funktionstastenleiste (das Präfix „F2“ usw. fügt die App hinzu)
fkey-rename = Umbenennen
fkey-view = Anzeigen
fkey-edit = Bearbeiten
fkey-copy = Kopieren
fkey-move = Verschieben
fkey-mkdir = Neuer Ordner
fkey-delete = Löschen
fkey-terminal = Terminal
fkey-exit = Beenden

## Dialog Öffnen mit
open-with = Öffnen mit
open-with-remember = Diese App immer für { $mime }-Dateien verwenden
open-with-one = Anwendung zum Öffnen von „{ $name }“ wählen.
open-with-many = Anwendung zum Öffnen von { $count } Elementen wählen.
loading-apps = Anwendungen werden geladen…
no-apps = Keine Anwendungen gefunden.
recommended-apps = Empfohlene Anwendungen
other-apps = Andere Anwendungen
default-app = Standard

## Dialog Details
details-items-title = { $count } Elemente
details-calculating = Wird berechnet…
details-error = Fehler
details-contents = { $files } Dateien, { $folders } Ordner darin
details-size = { $size } ({ $bytes } Bytes)
details-items = Elemente
details-location = Ort
details-total-size = Gesamtgröße
details-type = Typ
details-name = Name
details-link-target = Linkziel
details-size-label = Größe
details-modified = Geändert
details-accessed = Letzter Zugriff
details-created = Erstellt
details-permissions = Berechtigungen
details-owner = Besitzer
details-group = Gruppe
details-unknown = Unbekannt
kind-folder = Ordner
kind-file = Datei
kind-symlink = Symbolischer Link

## Dialog „Dateien suchen“
find = Dateien suchen
tooltip-find = Dateien suchen (Ctrl+F)
find-in = In { $path } und seinen Unterordnern
find-placeholder = Name oder Muster (z. B. bericht oder *.pdf)
find-search = Suchen
find-stop = Anhalten
find-hint = Geben Sie einen Teil des Namens ein und drücken Sie Enter.
find-searching = Suche läuft… { $count } gefunden
find-results = { $count ->
    [one] 1 Ergebnis
   *[other] { $count } Ergebnisse
}
find-no-results = Nichts gefunden.
find-truncated = Die ersten { $count } Ergebnisse werden angezeigt; versuchen Sie einen genaueren Namen.
find-go-to = Zur Datei
find-listbox = In Liste anzeigen
search-results-tab = Suche: { $pattern }

## Fenster „Über“
about-2fip = Über 2fip
about-description = Ein tastaturgesteuerter Dateimanager mit zwei Bereichen, inspiriert von Total Commander.
about-version = Version:
about-repository = Repository:
about-license = Lizenz:

## Dialog „Mit Server verbinden“
connect-to-server = Mit Server verbinden…
disconnect = Trennen
connect-title = Mit Server verbinden
connect = Verbinden
connect-host = Server (z. B. dateien.beispiel.de)
connect-port = Port
connect-user = Benutzername
connect-password = Passwort
connect-hint-sftp = Zuerst werden Ihr SSH-Agent und die Schlüssel in ~/.ssh versucht, das Passwort ist also optional.
connect-hint-ftp = Lassen Sie den Benutzernamen leer, um sich anonym anzumelden. FTP sendet das Passwort unverschlüsselt: besser FTPS oder SFTP.
connect-hint-ftps = Mit TLS verschlüsseltes FTP. Der Server braucht ein gültiges Zertifikat.
connect-connecting = Verbindung wird hergestellt…
connect-error-host = Geben Sie den Namen oder die Adresse des Servers ein.
connect-error-port = Der Port muss eine Zahl sein.
connect-error-login = Anmeldung fehlgeschlagen: Prüfen Sie Benutzername und Passwort.
connect-unknown-host-title = Unbekannter Server
connect-unknown-host-body = 2fip war noch nie mit { $host } verbunden. Prüfen Sie, ob dieser Schlüssel-Fingerabdruck mit dem des Servers übereinstimmt, und vertrauen Sie ihm dann. Er wird in ~/.ssh/known_hosts gespeichert.
connect-trust = Vertrauen und verbinden
connect-key-changed-title = Der Schlüssel des Servers hat sich geändert
connect-key-changed-body = Der Schlüssel von { $host } passt nicht zu dem in ~/.ssh/known_hosts. Möglicherweise wird die Verbindung abgefangen, daher verbindet 2fip nicht. Wurde der Server neu installiert, entfernen Sie seine alte Zeile aus ~/.ssh/known_hosts.

## Seitenleiste Verbindungen
connections = Verbindungen
tooltip-connections = Verbindungen
folder-size = Ordnergröße
move-to-trash = In den Papierkorb verschieben
open-path-in-new-tab = Pfad in neuem Tab öffnen
folder-size-loading = Ordnergrößen werden berechnet…
folder-size-scanning = { $count } Dateien und Ordner, bisher { $size }
folder-size-drive = Laufwerk: { $used } von { $total } belegt · { $free } frei
folder-size-other = Nicht gezählt: { $size } (Systemdateien und Ordner, die 2fip nicht lesen kann, Dateisystemdaten)
folder-size-unreadable = { $count } Ordner konnten nicht gelesen werden (keine Berechtigung)
folder-size-empty = Dieser Ordner ist leer
folder-size-parent-share = % des übergeordneten Ordners
folder-size-root-share = % des analysierten Ordners
folder-size-graph = Belegung
folder-size-more = … { $count } weitere
folder-size-selected = { $count } Ordner ausgewählt
connection-add = Verbindung hinzufügen
connection-edit = Verbindung bearbeiten
connection-save-only = Speichern
connection-save = Verbindung speichern
connection-open = Öffnen
connection-reconnect = Neu verbinden
connection-status-busy = Verbindung wird hergestellt…
connection-status-connected = Verbunden
connection-status-lost = Verbindung verloren
connection-status-closed = Nicht verbunden
connections-saved = Gespeichert
connections-open-unsaved = Offen, nicht gespeichert
connections-empty = Noch keine gespeicherten Verbindungen.
connections-hint = Passwörter, die Sie merken lassen, werden verschlüsselt im Schlüsselbund des Systems gespeichert (GNOME Keyring, KWallet), nie in den Dateien von 2fip.
connect-name = Name (optional)
connect-password-stored = Im Schlüsselbund gespeichert (zum Ändern tippen)
connect-save = In Verbindungen speichern
connect-remember-password = Passwort im Schlüsselbund des Systems merken

# F3-Betrachter und Schnellansicht (Strg+Q)
quick-view = Schnellansicht
viewer-loading = Wird geladen…
viewer-unreadable = Diese Datei kann nicht gelesen werden.
viewer-folder = Das ist ein Ordner.
viewer-previous = Vorherige Datei (P)
viewer-next = Nächste Datei (N)
viewer-text = Text
viewer-hex = Hex
viewer-image = Bild
viewer-zoom-out = Verkleinern (−)
viewer-zoom-fit = An Fenster anpassen (0)
viewer-zoom-in = Vergrößern (+)
viewer-wrap = Zeilen umbrechen
viewer-find = Suchen
viewer-find-previous = Vorheriges suchen (Umschalt+F3)
viewer-find-next = Nächstes suchen (F3)
viewer-searching = Suche läuft…
viewer-not-found = Nicht gefunden
viewer-open-externally = Mit Standardprogramm öffnen
viewer-close = Schließen (Esc)

# F4: editor choice, Shift+F4, editing files on servers
new-file = Neue Datei
new-file-default-name = Neue Datei.txt
file-name = Dateiname
settings-programs = Programme
settings-editor = Editor (F4)
settings-editor-description = Die App, in der F4 Dateien öffnet. Dateien auf einem Server werden bei jedem Speichern wieder hochgeladen.
editor-system-default = Texteditor des Systems
notice-uploaded = { $name } wurde auf dem Server gespeichert.
notice-upload-failed = { $name } konnte nicht auf dem Server gespeichert werden: { $error }

# Favorites menu (Ctrl+D) and Configure dialog
favorites-remove-current = Aktuellen Ordner entfernen
favorites-configure = Konfigurieren…
favorites-configure-title = Favoriten konfigurieren
favorites-add-item = Aktuellen Ordner hinzufügen
favorites-delete-item = Löschen
favorites-up = Nach oben
favorites-down = Nach unten
favorites-title = Titel
favorites-folder = Ordner
favorites-use-current = Ordner des aktiven Bereichs verwenden
ok = OK

# Folder colors (right-click → Folder color)
folder-color-ellipsis = Ordnerfarbe…
folder-color-one = Farbe für „{ $name }“
folder-color-many = Farbe für { $count } Ordner
folder-color-default = Standard
color-red = Rot
color-orange = Orange
color-yellow = Gelb
color-green = Grün
color-blue = Blau
color-purple = Lila
color-pink = Rosa
color-grey = Grau

# Ctrl+C, Ctrl+V in the same folder: "readme - Copy.txt"
copy-suffix = Kopie

# Toolbar: list / grid view
tooltip-list-view = Listenansicht (Strg+1)
tooltip-grid-view = Rasteransicht (Strg+2)

# The ★ button in each panel's bar
tooltip-favorites = Favoriten (Strg+D)
