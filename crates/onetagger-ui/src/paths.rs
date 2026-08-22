//! Confining client-supplied paths to the library.
//!
//! Every path the UI sends is input, not an instruction. The socket and the
//! `/audio` and `/thumb` endpoints all take one and go straight to the
//! filesystem with it, so without a check the server reads, writes and deletes
//! anything its uid can reach on behalf of whoever can reach the port.
//!
//! The root is whatever `--path` the server was started on: the one path chosen
//! by the operator rather than by a client. It is process-global because it
//! genuinely is -- fixed from argv before the first request, never changed
//! after -- and a global spares every handler signature a parameter it would
//! only pass through. That matters beyond tidiness: a check that has to be
//! threaded into each new endpoint is a check a new endpoint will be written
//! without.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use anyhow::Error;
use dunce::canonicalize;

use crate::StartContext;

static SCOPE: OnceLock<Scope> = OnceLock::new();

#[derive(Debug)]
enum Scope {
    /// Confine to this directory. Server started with `--path`.
    Library(PathBuf),
    /// No confinement: a local run with no `--path`, where the user drove the
    /// file dialog themselves and already has a shell and a file manager.
    /// Upstream behaviour.
    Unrestricted,
    /// Exposed with no `--path`, so there is nothing to confine to. Refuse
    /// everything rather than serve as a remote read-write primitive; failing
    /// closed is the only safe reading of "reachable, and no library set".
    Sealed
}

/// Set the scope from the start context. Call once, before serving.
pub fn init(context: &StartContext) {
    let scope = match &context.start_path {
        Some(path) => match canonicalize(path) {
            Ok(root) => {
                info!("Confining file access to: {}", root.display());
                Scope::Library(root)
            },
            // A --path that does not resolve cannot confine anything. Do not
            // quietly widen to unrestricted: the operator asked for a root.
            Err(e) => {
                error!("Cannot resolve --path {path}: {e}. Refusing file access.");
                Scope::Sealed
            }
        },
        None if context.expose => {
            warn!("Exposed with no --path: file access is refused. Start with --path to enable it.");
            Scope::Sealed
        },
        None => Scope::Unrestricted
    };
    let _ = SCOPE.set(scope);
}

/// The library root, if one is set. For defaulting a browser to somewhere
/// inside the scope rather than wherever the process happens to stand.
pub fn root() -> Option<&'static Path> {
    match SCOPE.get() {
        Some(Scope::Library(root)) => Some(root.as_path()),
        _ => None
    }
}

/// Resolve an existing path and confine it. Returns the *resolved* path, which
/// is the one callers must then use: resolving one path and opening another is
/// the gap this exists to close.
pub fn confine(path: impl AsRef<Path>) -> Result<PathBuf, Error> {
    let path = path.as_ref();
    match SCOPE.get() {
        // Not initialised: only reachable if a caller runs before init, which
        // would be a bug. Refuse rather than guess.
        None => Err(Error::msg("File access is not configured")),
        Some(Scope::Unrestricted) => Ok(path.to_path_buf()),
        Some(Scope::Sealed) => Err(Error::msg(
            "Refusing file access: the server is exposed but was started without --path, \
             so there is no library to confine it to"
        )),
        Some(Scope::Library(root)) => {
            // realpath first. `..` segments, symlinks and repeated separators
            // all mean the string a client sent is not necessarily the file it
            // names.
            let resolved = canonicalize(path)
                .map_err(|e| Error::msg(format!("{}: {e}", path.display())))?;
            check(&resolved, root)
        }
    }
}

/// As [`confine`], for a path that does not exist yet -- a file about to be
/// written. The parent must exist and be inside the library; the leaf is then
/// appended to the resolved parent, so a symlinked parent cannot be used to
/// land the write outside.
pub fn confine_new(path: impl AsRef<Path>) -> Result<PathBuf, Error> {
    let path = path.as_ref();
    match SCOPE.get() {
        Some(Scope::Unrestricted) => return Ok(path.to_path_buf()),
        Some(Scope::Library(_)) => {},
        _ => { confine(path)?; }
    }
    let name = path.file_name()
        .ok_or_else(|| Error::msg(format!("{}: no filename", path.display())))?;
    let parent = path.parent()
        .ok_or_else(|| Error::msg(format!("{}: no parent directory", path.display())))?;
    // An empty parent means a bare filename, which is relative to a working
    // directory the client does not choose. Reject rather than resolve it.
    if parent.as_os_str().is_empty() {
        return Err(Error::msg(format!("{}: not an absolute path", path.display())));
    }
    Ok(confine(parent)?.join(name))
}

/// Confine a whole batch, failing on the first path outside the library. Used
/// where a partial result would be misleading rather than merely incomplete.
pub fn confine_all<P: AsRef<Path>>(paths: &[P]) -> Result<Vec<PathBuf>, Error> {
    paths.iter().map(confine).collect()
}

fn check(resolved: &Path, root: &Path) -> Result<PathBuf, Error> {
    // Component-wise, not a string prefix: as text, "/music" prefixes
    // "/musicians" too.
    if !resolved.starts_with(root) {
        warn!("Refusing access outside the library: {}", resolved.display());
        return Err(Error::msg(format!(
            "{}: outside the library", resolved.display()
        )));
    }
    Ok(resolved.to_path_buf())
}
