//! The web host lists external addons because a browser cannot scan a local directory.

#[cfg(target_arch = "wasm32")]
use super::addons::{Addon, Source};
#[cfg(target_arch = "wasm32")]
use benilla_ui::toc::Toc;

#[cfg(target_arch = "wasm32")]
pub(super) fn discover() -> Vec<Addon> {
    let Some(bytes) = super::reference_ui::read("Interface/AddOns/__addons.txt") else {
        return Vec::new();
    };
    let Some(root) = super::addons::root() else {
        return Vec::new();
    };
    let mut names = names(&String::from_utf8_lossy(&bytes));
    super::addons::sort_by_directory_order(&mut names);
    names
        .into_iter()
        .filter_map(|name| {
            let path = format!("Interface/AddOns/{name}/{name}.toc");
            let bytes = super::reference_ui::read(&path)?;
            Some(Addon::new(
                name,
                Toc::parse(&benilla_ui::source::decode(&bytes)),
                Source::Dir(root.clone()),
            ))
        })
        .collect()
}

fn names(index: &str) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    index
        .lines()
        .map(str::trim)
        .filter(|n| {
            !n.is_empty()
                && *n != "."
                && *n != ".."
                && !n.contains(['/', '\\', ':'])
                && !n.chars().any(char::is_control)
                && seen.insert(n.to_ascii_lowercase())
        })
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_preserves_folder_names_and_rejects_duplicates_and_paths() {
        assert_eq!(
            names("DungeonClear\npfQuest\npfQuest-turtle\nPFQUEST\n../escape\n..\na\\b\n"),
            ["DungeonClear", "pfQuest", "pfQuest-turtle"]
        );
        assert!(names("").is_empty());
    }
}
