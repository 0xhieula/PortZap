//! Tests for [`crate::lsof_parser`], kept in their own file for size.

use super::*;

/// Captured from `lsof -nP -iTCP -sTCP:LISTEN -F pcn` on macOS 15.7.7.
const FIXTURE: &str = "\
p606
crapportd
f8
n*:59089
f9
n*:59089
p797
cControlCenter
f8
n*:7000
f9
n*:7000
f10
n*:5000
f11
n*:5000
p40214
ccom.docker.backend
f148
n*:6379
f150
n*:5432
p45041
cRaycast
f37
n127.0.0.1:7265
";

#[test]
fn dedupes_ipv4_and_ipv6_rows_of_one_socket() {
    let entries = parse_lsof_fields(FIXTURE);
    let port_7000: Vec<_> = entries.iter().filter(|e| e.port == 7000).collect();
    assert_eq!(port_7000.len(), 1, "IPv4+IPv6 rows must collapse to one entry");
}

#[test]
fn keeps_command_names_untruncated() {
    let entries = parse_lsof_fields(FIXTURE);
    let docker = entries.iter().find(|e| e.port == 6379).unwrap();
    assert_eq!(docker.process, "com.docker.backend");
    let cc = entries.iter().find(|e| e.port == 7000).unwrap();
    assert_eq!(cc.process, "ControlCenter");
}

#[test]
fn attributes_each_port_to_its_own_process() {
    let entries = parse_lsof_fields(FIXTURE);
    let expected = [
        (5000, 797, "ControlCenter"),
        (5432, 40214, "com.docker.backend"),
        (6379, 40214, "com.docker.backend"),
        (7000, 797, "ControlCenter"),
        (7265, 45041, "Raycast"),
        (59089, 606, "rapportd"),
    ];
    assert_eq!(entries.len(), expected.len());
    for (entry, (port, pid, process)) in entries.iter().zip(expected) {
        assert_eq!(
            (entry.port, entry.pid, entry.process.as_str()),
            (port, pid, process)
        );
    }
}

#[test]
fn sorts_by_port_ascending() {
    let entries = parse_lsof_fields(FIXTURE);
    let ports: Vec<u16> = entries.iter().map(|e| e.port).collect();
    let mut sorted = ports.clone();
    sorted.sort();
    assert_eq!(ports, sorted);
}

#[test]
fn parses_wildcard_loopback_and_ipv6_address_forms() {
    assert_eq!(parse_port("*:3000"), Some(3000));
    assert_eq!(parse_port("127.0.0.1:7265"), Some(7265));
    assert_eq!(parse_port("[::1]:8080"), Some(8080));
    assert_eq!(parse_port("[fe80::1%lo0]:443"), Some(443));
}

#[test]
fn rejects_unparseable_addresses() {
    assert_eq!(parse_port(""), None);
    assert_eq!(parse_port("no-colon"), None);
    assert_eq!(parse_port("*:notaport"), None);
    assert_eq!(parse_port("*:99999"), None, "out of u16 range");
    assert_eq!(parse_port("127.0.0.1:50->1.2.3.4:80"), None, "connected socket");
}

#[test]
fn skips_addresses_with_no_preceding_process_set() {
    // `n` before any `p`/`c` has no owner and must not be attributed to anything.
    assert!(parse_lsof_fields("n*:3000\n").is_empty());
    // pid present but command missing is equally unusable.
    assert!(parse_lsof_fields("p123\nn*:3000\n").is_empty());
}

#[test]
fn malformed_pid_discards_its_process_set() {
    let entries = parse_lsof_fields("pnotanumber\ncfoo\nn*:3000\n");
    assert!(entries.is_empty());
}

#[test]
fn handles_empty_input() {
    assert!(parse_lsof_fields("").is_empty());
    assert!(parse_lsof_fields("\n\n").is_empty());
}

#[test]
fn same_port_on_different_pids_is_kept() {
    // Distinct processes can listen on the same port on different interfaces.
    let raw = "p100\ncalpha\nn127.0.0.1:8080\np200\ncbeta\nn192.168.1.5:8080\n";
    let entries = parse_lsof_fields(raw);
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].pid, 100);
    assert_eq!(entries[1].pid, 200);
}
