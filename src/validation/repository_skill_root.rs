use std::path::Path;

pub(super) fn from_plugin_root(plugin_root: &Path) -> Option<&Path> {
    let plugins = plugin_root.parent()?;
    (plugin_root.file_name()?.to_str() == Some("codexy")
        && plugins.file_name()?.to_str() == Some("plugins"))
    .then(|| plugins.parent())
    .flatten()
}
