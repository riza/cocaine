#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "windows")]
mod windows;

use std::fmt;

#[derive(Debug, Clone, Copy)]
pub enum AssertionType {
    PreventIdleSystemSleep,
    PreventIdleDisplaySleep,
    PreventSystemSleep,
}

impl fmt::Display for AssertionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PreventIdleSystemSleep => write!(f, "idle system sleep"),
            Self::PreventIdleDisplaySleep => write!(f, "idle display sleep"),
            Self::PreventSystemSleep => write!(f, "system sleep"),
        }
    }
}

pub struct PowerAssertion {
    assertion_type: AssertionType,
    #[allow(dead_code)]
    inner: PlatformAssertion,
}

impl PowerAssertion {
    pub fn new(assertion_type: AssertionType, reason: &str) -> Result<Self, String> {
        let inner = PlatformAssertion::create(assertion_type, reason)?;
        Ok(Self {
            assertion_type,
            inner,
        })
    }

    pub fn assertion_type(&self) -> AssertionType {
        self.assertion_type
    }
}

#[cfg(target_os = "macos")]
use macos::MacOSAssertion as PlatformAssertion;

#[cfg(target_os = "linux")]
use linux::LinuxInhibitor as PlatformAssertion;

#[cfg(target_os = "windows")]
use windows::WindowsAssertion as PlatformAssertion;
