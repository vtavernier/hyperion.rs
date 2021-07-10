use std::{
    io,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy)]
enum ResolvedPaths {
    Production,
    Development,
}

const ROOT_MARKER: &str = "$ROOT";
const SYSTEM_MARKER: &str = "$SYSTEM";

#[derive(Clone)]
pub struct Paths {
    mode: ResolvedPaths,
    system_root: PathBuf,
    user_root: PathBuf,
}

impl Paths {
    fn find_dev_root(first_root: &Path) -> Option<PathBuf> {
        let bn = first_root.file_name().and_then(std::ffi::OsStr::to_str);

        if bn == Some("release") || bn == Some("debug") {
            // A Rust release dir?
            let mut current_root = first_root.parent();
            while let Some(root) = current_root {
                if root.file_name().and_then(std::ffi::OsStr::to_str) == Some("target") {
                    // We need the parent of this one
                    return root.parent().map(Path::to_owned);
                } else {
                    // Keep going up
                    current_root = root.parent();
                }
            }
        }

        None
    }

    fn find_bin_root(first_root: &Path) -> Option<PathBuf> {
        let bn = first_root.file_name().and_then(std::ffi::OsStr::to_str);

        if bn == Some("bin") {
            return first_root.parent().map(|path| {
                let mut p = path.to_owned();
                p.push("share");
                p.push("hyperion");
                p
            });
        }

        None
    }

    fn dev_user_root(user_root: Option<PathBuf>, dev_root: &Path) -> PathBuf {
        if let Some(user_root) = user_root {
            user_root
        } else {
            dev_root.to_owned()
        }
    }

    fn prod_user_root(user_root: Option<PathBuf>) -> PathBuf {
        if let Some(user_root) = user_root {
            user_root
        } else {
            dirs::config_dir()
                .map(|mut path| {
                    path.push("hyperion.rs");
                    path
                })
                .unwrap()
        }
    }

    pub fn new(user_root: Option<PathBuf>) -> io::Result<Self> {
        // Try to find the current exe
        let proc = std::env::current_exe()?;

        // Find the 2nd parent
        let first_root = proc.parent().unwrap();

        if let Some(dev_root) = Self::find_dev_root(first_root) {
            debug!(path = %dev_root.display(), "found development root");

            let user_root = Self::dev_user_root(user_root, &dev_root);
            debug!(path = %user_root.display(), "found user root");

            Ok(Self {
                mode: ResolvedPaths::Development,
                system_root: dev_root,
                user_root,
            })
        } else if let Some(bin_root) = Self::find_bin_root(first_root) {
            debug!(path = %bin_root.display(), "found production root");

            let user_root = Self::prod_user_root(user_root);
            debug!(path = %user_root.display(), "found user root");

            Ok(Self {
                mode: ResolvedPaths::Production,
                system_root: bin_root,
                user_root,
            })
        } else {
            debug!(path = %first_root.display(), "no root found, using binary");

            let user_root = Self::prod_user_root(user_root);
            debug!(path = %user_root.display(), "found user root");

            Ok(Self {
                mode: ResolvedPaths::Production,
                system_root: first_root.to_owned(),
                user_root,
            })
        }
    }

    pub fn resolve_path(&self, p: impl Into<PathBuf>) -> PathBuf {
        let p: PathBuf = p.into();

        if p.is_absolute() {
            // Don't transform absolute paths
            trace!(path = %p.display(), "left unchanged");
            p
        } else {
            let mut out_path = PathBuf::new();
            let mut components = p.components().peekable();

            if let Some(component) = components.peek() {
                let component = component.as_os_str().to_str();
                if component == Some(SYSTEM_MARKER) {
                    out_path.extend(&self.system_root);
                    components.next();

                    if let ResolvedPaths::Development = self.mode {
                        match components.peek().and_then(|cmp| cmp.as_os_str().to_str()) {
                            Some("webconfig") => {
                                // Webconfig mapping
                                components.next();
                                out_path.extend(&PathBuf::from("ext/hyperion.ng/assets/webconfig"));
                            }
                            _ => {
                                // No matching mapping
                            }
                        }
                    }
                } else if component == Some(ROOT_MARKER) {
                    out_path.extend(&self.user_root);
                    components.next();
                }
            }

            out_path.extend(components);

            trace!(src = %p.display(), dst = %out_path.display(), "remapped path");
            out_path
        }
    }
}
