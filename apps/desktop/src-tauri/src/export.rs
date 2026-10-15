use std::{fs::File, io, io::Write, path::Path};

pub fn save(path: &Path, json: &[u8]) -> io::Result<()> {
    write_then_publish(path, |file| file.write_all(json))
}

fn write_then_publish(
    path: &Path,
    write: impl FnOnce(&mut File) -> io::Result<()>,
) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("Export requires a destination directory"))?;
    let mut temporary = tempfile::Builder::new()
        .prefix(".threadmark-export-")
        .tempfile_in(parent)?;
    write(temporary.as_file_mut())?;
    temporary.as_file().sync_all()?;
    // The temporary file is on the same filesystem. Failed writes never truncate
    // the chosen destination, and tempfile removes abandoned staging files.
    temporary
        .persist(path)
        .map(|_| ())
        .map_err(|error| error.error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn successful_export_replaces_existing_file_without_leaving_temporary_files() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("export with spaces.json");
        std::fs::write(&path, "previous").unwrap();
        save(&path, b"{\"schema_version\":1}").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"{\"schema_version\":1}");
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
    }

    #[test]
    fn failed_export_preserves_destination_and_cleans_partial_staging_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("export.json");
        std::fs::write(&path, "previous").unwrap();
        let result = write_then_publish(&path, |file| {
            file.write_all(b"partial JSON")?;
            Err(io::Error::other("injected disk write failure"))
        });
        assert!(result.is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"previous");
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
    }
}
