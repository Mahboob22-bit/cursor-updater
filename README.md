# cursor-updater

A simple CLI tool that automates updating the [Cursor](https://cursor.sh) editor on Linux. You download the `.deb` file — cursor-updater handles the rest.

## What it does

1. Finds the latest `cursor_*.deb` in `~/Downloads`
2. Removes older `.deb` versions
3. Extracts the archive (ar → xz → tar)
4. Installs everything to `/opt/cursor/`
5. Updates the `.desktop` launcher
6. Cleans up temporary files

## Installation

### From GitHub Releases

Download the binary from the [latest release](https://github.com/Mahboob22-bit/cursor-updater/releases/latest) and place it in your PATH:

```bash
sudo cp cursor-updater /usr/local/bin/
```

### Build from source

```bash
git clone git@github.com:Mahboob22-bit/cursor-updater.git
cd cursor-updater
cargo build --release
sudo cp target/release/cursor-updater /usr/local/bin/
```

## Usage

1. Download the latest Cursor `.deb` from [cursor.sh](https://cursor.sh) to `~/Downloads/`
2. Run:

```bash
cursor-updater
```

The tool automatically requests `sudo` if needed.

```
[cursor-updater] Suche neueste Version in ~/Downloads...
[cursor-updater] Gefunden: cursor_2.6.21_amd64.deb
[cursor-updater] Lösche alte Versionen...
[cursor-updater] Extrahiere .deb...
[cursor-updater] Installiere nach /opt/cursor/...
[cursor-updater] Räume auf...
[cursor-updater] ✓ Cursor 2.6.21 erfolgreich installiert!
```

## Prerequisites

- Linux x86_64
- `/opt/cursor/` must exist (`sudo mkdir -p /opt/cursor`)
- Cursor `.deb` downloaded to `~/Downloads/`

## License

MIT
