//! External addon overlay. The game install remains read-only; names match Windows casing.

use std::path::{Path, PathBuf};

pub(crate) fn is_addon_path(name: &str) -> bool {
    name.replace('\\', "/")
        .to_ascii_lowercase()
        .starts_with("interface/addons/")
}

fn child(root: &Path, name: &str) -> Option<PathBuf> {
    if name.is_empty() || matches!(name, "." | "..") {
        return None;
    }
    std::fs::read_dir(root)
        .ok()?
        .flatten()
        .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(name))
        .map(|e| e.path())
}

pub(crate) fn read(root: &Path, name: &str) -> Option<Vec<u8>> {
    if !is_addon_path(name) {
        return None;
    }
    let normalized = name.replace('\\', "/");
    let tail = normalized.split('/').skip(2).collect::<Vec<_>>();
    let root = root.canonicalize().ok()?;
    if tail.len() == 1 && tail[0].eq_ignore_ascii_case("__addons.txt") {
        let mut names = Vec::new();
        for entry in std::fs::read_dir(&root).ok()?.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if !path
                .canonicalize()
                .ok()
                .is_some_and(|p| p.starts_with(&root))
            {
                continue;
            }
            if child(&path, &format!("{name}.toc")).is_some() {
                names.push(name);
            }
        }
        names.sort_by_key(|n| n.to_ascii_lowercase());
        return Some(names.join("\n").into_bytes());
    }
    let mut path = root.clone();
    for part in tail {
        path = child(&path, part)?;
    }
    let path = path.canonicalize().ok()?;
    if !path.starts_with(root) {
        return None;
    }
    std::fs::read(path).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovers_manifests_reads_mixed_case_and_blocks_escapes() {
        let root = std::env::temp_dir().join(format!("wenilla-addons-{}", std::process::id()));
        std::fs::create_dir_all(root.join("Example")).unwrap();
        std::fs::write(root.join("Example/example.TOC"), b"Example.lua").unwrap();
        std::fs::write(root.join("Example/Example.lua"), b"HELLO = true").unwrap();
        assert_eq!(
            read(&root, r"Interface\AddOns\__addons.txt").unwrap(),
            b"Example"
        );
        assert_eq!(
            read(&root, r"interface\addons\EXAMPLE\example.LUA").unwrap(),
            b"HELLO = true"
        );
        assert!(read(&root, r"Interface\AddOns\..\secret").is_none());
        assert!(read(&root, r"Interface\FrameXML\Example.lua").is_none());
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink("/etc/passwd", root.join("Example/escape.lua")).unwrap();
            assert!(read(&root, r"Interface\AddOns\Example\escape.lua").is_none());
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}
