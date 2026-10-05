//! Parsing of `lsof` field-mode output.
//!
//! Separated from process execution in [`crate::ports`] so it can be exercised against captured
//! fixtures without a live system.

use std::collections::HashSet;

use crate::ports::PortEntry;

/// Parse `lsof -F pcn` output.
///
/// Field mode is a stream of `<type><value>` lines where `p` (pid) and `c` (command) open a
/// process set and apply to every subsequent address until the next set begins:
///
/// ```text
/// p797
/// cControlCenter
/// f8
/// n*:7000
/// f9
/// n*:7000        <- same socket, IPv6 row
/// ```
///
/// A single socket bound on both IPv4 and IPv6 yields two `n` lines, so results are
/// deduplicated by `(pid, port)`. Output is sorted by port ascending.
pub fn parse_lsof_fields(raw: &str) -> Vec<PortEntry> {
    let mut entries = Vec::new();
    let mut seen: HashSet<(i32, u16)> = HashSet::new();
    let mut cur_pid: Option<i32> = None;
    let mut cur_cmd: Option<String> = None;

    for line in raw.lines() {
        let Some(kind) = line.chars().next() else {
            continue;
        };
        let value = &line[kind.len_utf8()..];

        match kind {
            // New process set. A malformed pid invalidates the set rather than letting the
            // following addresses be attributed to the previous process.
            'p' => {
                cur_pid = value.trim().parse::<i32>().ok();
                cur_cmd = None;
            }
            'c' => cur_cmd = Some(value.trim().to_string()),
            'n' => {
                let (Some(pid), Some(cmd)) = (cur_pid, cur_cmd.as_ref()) else {
                    continue;
                };
                let Some(port) = parse_port(value) else {
                    continue;
                };
                if seen.insert((pid, port)) {
                    entries.push(PortEntry {
                        port,
                        pid,
                        process: cmd.clone(),
                    });
                }
            }
            _ => {}
        }
    }

    entries.sort_by_key(|e| (e.port, e.pid));
    entries
}

/// Extract the port from an lsof address field.
///
/// Handles `*:3000`, `127.0.0.1:7265` and `[::1]:8080` by taking the segment after the final
/// colon — correct for IPv6 literals, where earlier colons belong to the address.
fn parse_port(addr: &str) -> Option<u16> {
    let addr = addr.trim();
    // Skip connected sockets (`local->peer`); only listening addresses are of interest.
    if addr.contains("->") {
        return None;
    }
    addr.rsplit_once(':')?.1.parse::<u16>().ok()
}

#[cfg(test)]
#[path = "lsof_parser_tests.rs"]
mod tests;
