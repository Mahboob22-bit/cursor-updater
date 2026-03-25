# cursor-updater – Anforderungsdokument

## Projektbeschreibung

Ein CLI-Tool in Rust, das den Cursor-Editor automatisch auf die neueste Version aktualisiert.
Die `.deb`-Datei wird manuell vom User heruntergeladen. Das Tool übernimmt danach alles:
Extraktion, Installation nach `/opt/cursor/`, Aufräumen.

---

## Kontext: Wie funktioniert das Update manuell?

Cursor wird als `.deb`-Datei von der Cursor-Website heruntergeladen (z.B. `cursor_2.6.13_amd64.deb`).

Eine `.deb`-Datei ist ein `ar`-Archiv mit folgender Struktur:
```
cursor_2.6.13_amd64.deb
├── debian-binary          ← Paketformat-Version
├── control.tar.xz         ← Metadaten (wird ignoriert)
└── data.tar.xz            ← Die eigentlichen Programmdateien
```

`data.tar.xz` enthält die Dateien so, wie sie auf dem System liegen sollen:
```
usr/
└── share/
    ├── cursor/            ← Das Programm selbst
    │   ├── cursor         ← Ausführbare Datei
    │   └── ...
    └── applications/
        └── cursor.desktop ← Desktop-Launcher-Eintrag
```

Cursor ist in `/opt/cursor/` installiert (nicht im Standard-Pfad `/usr/share/cursor/`),
weil es eine manuelle Drittanbieter-Software ohne APT-Repository ist.

---

## Ablauf der App

```
1. Prüfen ob Root → falls nicht: sich selbst mit sudo neu starten
2. ~/Downloads nach cursor*.deb durchsuchen
3. Neueste Version anhand der Versionsnummer ermitteln
4. Alte .deb-Dateien löschen (alle außer der neuesten)
5. .deb extrahieren:
   a. ar-Archiv öffnen → data.tar.xz extrahieren
   b. data.tar.xz dekomprimieren + tar entpacken → usr/ Ordner
6. Dateien installieren:
   a. /opt/cursor/ leeren
   b. usr/share/cursor/* → /opt/cursor/ kopieren
   c. chmod +x /opt/cursor/cursor
   d. usr/share/applications/cursor.desktop → /usr/share/applications/
7. Temporäre Extraktions-Dateien aufräumen (usr/, debian-binary, *.tar.xz)
8. Erfolgsmeldung ausgeben
```

---

## Technischer Stack

- **Sprache:** Rust (Edition 2021)
- **Zielplattform:** Linux x86_64

### Cargo-Abhängigkeiten

```toml
[dependencies]
ar = "0.9"       # Liest .deb-Dateien (ar-Archivformat)
tar = "0.4"      # Entpackt tar-Archive
xz2 = "0.1"     # Dekomprimiert .xz-Dateien
```

Kein async, kein CLI-Framework nötig – die App ist bewusst einfach gehalten.

---

## Projektstruktur

```
cursor-updater/
├── Cargo.toml
└── src/
    └── main.rs
```

Alles in einer Datei (`main.rs`) ist für dieses Projekt völlig ausreichend.

---

## Funktionen (grobe Aufteilung in main.rs)

```rust
fn main()
fn ensure_root()              // Prüft ob root, startet sich sonst mit sudo neu
fn find_latest_deb() -> PathBuf       // Sucht neueste cursor*.deb in ~/Downloads
fn delete_old_debs(latest: &Path)     // Löscht alle außer der neuesten
fn extract_deb(deb_path: &Path)       // ar + tar + xz → usr/ Ordner
fn install_cursor()                   // Kopiert Dateien nach /opt/cursor/
fn cleanup()                          // Löscht temporäre Dateien
```

---

## Root-Handling (Option 4)

Das Binary prüft beim Start ob es als root läuft.
Falls nicht, startet es sich selbst neu mit `sudo`:

```rust
fn ensure_root() {
    if !nix::unistd::Uid::effective().is_root() {
        let exe = std::env::current_exe().unwrap();
        std::process::Command::new("sudo")
            .arg(exe)
            .status()
            .unwrap();
        std::process::exit(0);
    }
}
```

Alternativ ohne `nix`-Crate: `std::env::var("USER") == "root"` oder
Lesen von `/proc/self/status` nach `Uid:`.

---

## Versionserkennung

Dateiname-Schema: `cursor_MAJOR.MINOR.PATCH_amd64.deb`

Die Versionsnummer wird aus dem Dateinamen geparst und als Tupel `(u32, u32, u32)` verglichen,
**nicht** als String-Vergleich (sonst würde `2.9` > `2.10` gelten).

---

## Fehlerbehandlung

- Keine `.deb`-Datei gefunden → Fehlermeldung + Exit
- `/opt/cursor/` existiert nicht → Fehlermeldung + Exit
- Extraktion schlägt fehl → Fehlermeldung + Exit
- Durchgehend mit `Result` und `?`-Operator arbeiten

---

## Gewünschte Terminal-Ausgabe

```
[cursor-updater] Suche neueste Version in ~/Downloads...
[cursor-updater] Gefunden: cursor_2.6.13_amd64.deb
[cursor-updater] Lösche alte Versionen...
[cursor-updater] Extrahiere .deb...
[cursor-updater] Installiere nach /opt/cursor/...
[cursor-updater] Räume auf...
[cursor-updater] ✓ Cursor 2.6.13 erfolgreich installiert!
```

---

## Was dieses Projekt lehrt

- `std::fs` – Dateien lesen, kopieren, löschen, Verzeichnisse leeren
- `std::path::{Path, PathBuf}` – Pfade manipulieren
- `std::process::Command` – externe Prozesse aufrufen (sudo)
- Externe Crates einbinden (`ar`, `tar`, `xz2`)
- String-Parsing (Versionsnummern extrahieren und vergleichen)
- Error Handling mit `Result`, `?`, eigene Fehlermeldungen
- Structs und Methoden
- Pattern Matching
