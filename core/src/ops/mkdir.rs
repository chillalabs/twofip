use std::path::PathBuf;

/// Creates a new directory named `name` inside `parent`, returning its path.
pub async fn create_dir(parent: PathBuf, name: String) -> Result<PathBuf, String> {
    if let Some(location) = crate::vfs::RemoteLocation::parse(&parent) {
        let path = location.join(&name).to_path();
        crate::vfs::create_dir(&path).await?;
        return Ok(path);
    }
    let path = parent.join(&name);
    tokio::fs::create_dir(&path)
        .await
        .map_err(|err| format!("failed to create {}: {err}", path.display()))?;
    Ok(path)
}

/// Creates the empty file `name` in `parent` (here or on a server). An
/// existing file is never replaced.
pub async fn create_file(parent: PathBuf, name: String) -> Result<PathBuf, String> {
    if let Some(location) = crate::vfs::RemoteLocation::parse(&parent) {
        let path = location.join(&name).to_path();
        if crate::vfs::stat(&path).await?.is_some() {
            return Err(format!("{name} already exists"));
        }
        // Servers take files by upload: an empty one from the cache.
        let empty = crate::sandbox::cache_home().join("2fip/empty-file");
        if let Some(folder) = empty.parent() {
            tokio::fs::create_dir_all(folder)
                .await
                .map_err(|err| format!("failed to create {}: {err}", folder.display()))?;
        }
        tokio::fs::write(&empty, b"")
            .await
            .map_err(|err| format!("failed to create {}: {err}", empty.display()))?;
        crate::vfs::upload(&empty, &path, &mut |_| {}, &|| false).await?;
        return Ok(path);
    }
    let path = parent.join(&name);
    tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .await
        .map_err(|err| format!("failed to create {}: {err}", path.display()))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn creates_a_directory() {
        let dir = tempfile::tempdir().unwrap();

        let created = create_dir(dir.path().to_path_buf(), "New folder".to_string())
            .await
            .unwrap();

        assert_eq!(created, dir.path().join("New folder"));
        assert!(created.is_dir());
    }

    #[tokio::test]
    async fn fails_when_the_name_already_exists() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("existing")).unwrap();

        let result = create_dir(dir.path().to_path_buf(), "existing".to_string()).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn creates_an_empty_file_but_never_replaces_one() {
        let dir = tempfile::tempdir().unwrap();
        let parent = dir.path().to_path_buf();

        let created = create_file(parent.clone(), "notes.txt".to_string())
            .await
            .unwrap();
        assert_eq!(std::fs::read(&created).unwrap(), b"");

        std::fs::write(&created, b"keep me").unwrap();
        assert!(create_file(parent, "notes.txt".to_string()).await.is_err());
        assert_eq!(std::fs::read(&created).unwrap(), b"keep me");
    }
}
