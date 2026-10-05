//! Discovery of listening TCP ports and their owning processes.
//!
//! Shells out to `lsof` in field mode (`-F`) rather than table mode. Table mode truncates the
//! COMMAND column to 9 characters (`com.docker.backend` arrives as `com.docke`) and forces
//! column-position parsing; field mode emits one newline-delimited record per field, each
//! prefixed by a type character, and does not truncate.
//!
//! Parsing of that output lives in [`crate::lsof_parser`].

use std::fmt;
use std::process::Command;

use crate::lsof_parser::parse_lsof_fields;

/// One listening port together with the process holding it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortEntry {
    pub port: u16,
    pub pid: i32,
    pub process: String,
}

#[derive(Debug)]
pub enum PortsError {
    /// `lsof` is not on PATH.
    LsofMissing,
    /// `lsof` ran but failed in a way we cannot interpret.
    LsofFailed(String),
}

impl fmt::Display for PortsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PortsError::LsofMissing => write!(f, "lsof not found on PATH"),
            PortsError::LsofFailed(msg) => write!(f, "lsof failed: {msg}"),
        }
    }
}

/// Run `lsof` and return listening ports, deduplicated and sorted by port ascending.
pub fn list_listening_ports() -> Result<Vec<PortEntry>, PortsError> {
    let output = Command::new("lsof")
        .args(["-nP", "-iTCP", "-sTCP:LISTEN", "-F", "pcn"])
        .output()
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => PortsError::LsofMissing,
            _ => PortsError::LsofFailed(e.to_string()),
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    // lsof exits 1 when nothing matches the filter, which is the normal "no listening ports"
    // case rather than a failure. Only treat a non-zero exit as an error if it also produced
    // no usable output.
    if !output.status.success() && stdout.trim().is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.trim().is_empty() {
            return Err(PortsError::LsofFailed(stderr.trim().to_string()));
        }
        return Ok(Vec::new());
    }

    Ok(parse_lsof_fields(&stdout))
}
