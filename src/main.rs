use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command};

fn main() {
    if let Err(e) = run() {
        eprintln!("[cursor-updater] Fehler: {e}");
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    ensure_root()?;

    println!("[cursor-updater] Suche neueste Version in ~/Downloads...");
    let latest = find_latest_deb()?;
    println!(
        "[cursor-updater] Gefunden: {}",
        latest.file_name().unwrap().to_string_lossy()
    );

    println!("[cursor-updater] Lösche alte Versionen...");
    delete_old_debs(&latest)?;

    println!("[cursor-updater] Extrahiere .deb...");
    extract_deb(&latest)?;

    println!("[cursor-updater] Installiere nach /opt/cursor/...");
    install_cursor()?;

    println!("[cursor-updater] Räume auf...");
    cleanup()?;

    let version = parse_version_string(&latest).unwrap_or_default();
    println!("[cursor-updater] \u{2713} Cursor {version} erfolgreich installiert!");

    Ok(())
}

// --- Root-Handling ---

fn ensure_root() -> Result<(), Box<dyn std::error::Error>> {
    // Check effective UID by reading /proc/self/status
    let status = fs::read_to_string("/proc/self/status")?;
    let euid = status
        .lines()
        .find(|line| line.starts_with("Uid:"))
        .and_then(|line| line.split_whitespace().nth(2)) // effective UID is 3rd field
        .and_then(|uid| uid.parse::<u32>().ok())
        .ok_or("Konnte effektive UID nicht lesen")?;

    if euid != 0 {
        let exe = env::current_exe()?;
        let status = Command::new("sudo").arg(exe).status()?;
        process::exit(status.code().unwrap_or(1));
    }

    Ok(())
}

// --- Home Directory ---

fn get_user_home() -> Result<String, Box<dyn std::error::Error>> {
    if let Ok(user) = env::var("SUDO_USER") {
        return Ok(format!("/home/{user}"));
    }
    env::var("HOME").map_err(|e| e.into())
}

// --- Version Parsing ---

/// Parses a version tuple (major, minor, patch) from a cursor .deb filename.
/// Expected format: cursor_MAJOR.MINOR.PATCH_amd64.deb
fn parse_version(path: &Path) -> Option<(u32, u32, u32)> {
    let name = path.file_name()?.to_str()?;
    let version_str = name.strip_prefix("cursor_")?.strip_suffix("_amd64.deb")?;
    let parts: Vec<&str> = version_str.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    Some((
        parts[0].parse().ok()?,
        parts[1].parse().ok()?,
        parts[2].parse().ok()?,
    ))
}

/// Returns the version string (e.g. "2.6.13") from a .deb path.
fn parse_version_string(path: &Path) -> Option<String> {
    let (major, minor, patch) = parse_version(path)?;
    Some(format!("{major}.{minor}.{patch}"))
}

// --- Find latest .deb ---

fn find_latest_deb() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let home = get_user_home()?;
    let downloads = PathBuf::from(&home).join("Downloads");

    if !downloads.is_dir() {
        return Err(format!("Verzeichnis {} existiert nicht", downloads.display()).into());
    }

    let mut best: Option<(PathBuf, (u32, u32, u32))> = None;

    for entry in fs::read_dir(&downloads)? {
        let entry = entry?;
        let path = entry.path();
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with("cursor_") && name.ends_with("_amd64.deb") {
                if let Some(version) = parse_version(&path) {
                    if best.as_ref().map_or(true, |(_, v)| version > *v) {
                        best = Some((path, version));
                    }
                }
            }
        }
    }

    best.map(|(path, _)| path)
        .ok_or_else(|| "Keine cursor*.deb Datei in ~/Downloads gefunden".into())
}

// --- Delete old .debs ---

fn delete_old_debs(latest: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let downloads = latest.parent().ok_or("Kein übergeordnetes Verzeichnis")?;

    for entry in fs::read_dir(downloads)? {
        let entry = entry?;
        let path = entry.path();
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with("cursor_") && name.ends_with("_amd64.deb") && path != latest {
                fs::remove_file(&path)?;
                println!("[cursor-updater]   Gelöscht: {name}");
            }
        }
    }

    Ok(())
}

// --- Extract .deb ---

fn extract_deb(deb_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let downloads = deb_path.parent().ok_or("Kein übergeordnetes Verzeichnis")?;
    let file = fs::File::open(deb_path)?;
    let mut archive = ar::Archive::new(file);

    // Find and extract data.tar.xz from the ar archive
    while let Some(entry) = archive.next_entry() {
        let entry = entry?;
        let name = std::str::from_utf8(entry.header().identifier())?.to_string();
        if name == "data.tar.xz" {
            // Decompress xz and unpack tar
            let decompressor = xz2::read::XzDecoder::new(entry);
            let mut tar = tar::Archive::new(decompressor);
            tar.unpack(downloads)?;
            return Ok(());
        }
    }

    Err("data.tar.xz nicht im .deb-Archiv gefunden".into())
}

// --- Install ---

fn install_cursor() -> Result<(), Box<dyn std::error::Error>> {
    let home = get_user_home()?;
    let downloads = PathBuf::from(&home).join("Downloads");
    let extracted_cursor = downloads.join("usr/share/cursor");
    let extracted_desktop = downloads.join("usr/share/applications/cursor.desktop");
    let install_dir = Path::new("/opt/cursor");

    if !install_dir.exists() {
        return Err(format!("{} existiert nicht", install_dir.display()).into());
    }

    if !extracted_cursor.is_dir() {
        return Err(format!(
            "Extrahierter Ordner {} nicht gefunden",
            extracted_cursor.display()
        )
        .into());
    }

    // Clear /opt/cursor/
    for entry in fs::read_dir(install_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            fs::remove_dir_all(&path)?;
        } else {
            fs::remove_file(&path)?;
        }
    }

    // Copy usr/share/cursor/* -> /opt/cursor/
    copy_dir_recursive(&extracted_cursor, install_dir)?;

    // chmod +x /opt/cursor/cursor
    let cursor_bin = install_dir.join("cursor");
    if cursor_bin.exists() {
        Command::new("chmod")
            .args(["+x", &cursor_bin.to_string_lossy()])
            .status()?;
    }

    // Copy .desktop file
    if extracted_desktop.exists() {
        fs::copy(&extracted_desktop, "/usr/share/applications/cursor.desktop")?;
    }

    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), Box<dyn std::error::Error>> {
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            fs::create_dir_all(&dst_path)?;
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

// --- Cleanup ---

fn cleanup() -> Result<(), Box<dyn std::error::Error>> {
    let home = get_user_home()?;
    let downloads = PathBuf::from(&home).join("Downloads");

    // Remove extracted directories and files
    let usr_dir = downloads.join("usr");
    if usr_dir.exists() {
        fs::remove_dir_all(&usr_dir)?;
    }

    let debian_binary = downloads.join("debian-binary");
    if debian_binary.exists() {
        fs::remove_file(&debian_binary)?;
    }

    // Remove any leftover .tar.xz files from extraction
    for entry in fs::read_dir(&downloads)? {
        let entry = entry?;
        let path = entry.path();
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.ends_with(".tar.xz") || name.ends_with(".tar.zst") {
                fs::remove_file(&path)?;
            }
        }
    }

    Ok(())
}
