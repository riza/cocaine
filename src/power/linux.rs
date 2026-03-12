use super::AssertionType;
use std::os::fd::OwnedFd;
use zbus::blocking::Connection;
use zbus::zvariant;

fn inhibit_what(types: &[AssertionType]) -> String {
    let mut what = Vec::new();
    for t in types {
        match t {
            AssertionType::PreventIdleSystemSleep => {
                if !what.contains(&"idle") {
                    what.push("idle");
                }
            }
            AssertionType::PreventIdleDisplaySleep => {
                if !what.contains(&"idle") {
                    what.push("idle");
                }
            }
            AssertionType::PreventSystemSleep => {
                if !what.contains(&"sleep") {
                    what.push("sleep");
                }
            }
        }
    }
    what.join(":")
}

/// Holds the inhibit file descriptor from systemd-logind.
/// The inhibit lock is released when this fd is closed (dropped).
pub struct LinuxInhibitor {
    _fd: OwnedFd,
}

impl LinuxInhibitor {
    pub fn create(assertion_type: AssertionType, reason: &str) -> Result<Self, String> {
        let what = inhibit_what(&[assertion_type]);
        let fd = call_inhibit(&what, reason)?;
        Ok(Self { _fd: fd })
    }
}

fn call_inhibit(what: &str, reason: &str) -> Result<OwnedFd, String> {
    let conn = Connection::system().map_err(|e| format!("D-Bus connection failed: {e}"))?;

    let reply: zvariant::OwnedFd = conn
        .call_method(
            Some("org.freedesktop.login1"),
            "/org/freedesktop/login1",
            Some("org.freedesktop.login1.Manager"),
            "Inhibit",
            &(what, "cocaine", reason, "block"),
        )
        .map_err(|e| format!("Inhibit call failed: {e}"))?
        .body()
        .deserialize()
        .map_err(|e| format!("Failed to read inhibit fd: {e}"))?;

    Ok(reply.into())
}
