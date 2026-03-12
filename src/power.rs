use core_foundation::base::TCFType;
use core_foundation::string::CFString;
use std::fmt;

type CFStringRef = core_foundation::string::CFStringRef;

#[allow(non_upper_case_globals)]
const kIOPMAssertionLevelOn: u32 = 255;

unsafe extern "C" {
    fn IOPMAssertionCreateWithName(
        assertion_type: CFStringRef,
        assertion_level: u32,
        reason: CFStringRef,
        assertion_id: *mut u32,
    ) -> i32;

    fn IOPMAssertionRelease(assertion_id: u32) -> i32;
}

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

impl AssertionType {
    fn as_cfstring(&self) -> CFString {
        match self {
            Self::PreventIdleSystemSleep => CFString::new("PreventUserIdleSystemSleep"),
            Self::PreventIdleDisplaySleep => CFString::new("PreventUserIdleDisplaySleep"),
            Self::PreventSystemSleep => CFString::new("PreventSystemSleep"),
        }
    }
}

pub struct PowerAssertion {
    assertion_id: u32,
    assertion_type: AssertionType,
}

impl PowerAssertion {
    pub fn new(assertion_type: AssertionType, reason: &str) -> Result<Self, String> {
        let mut assertion_id: u32 = 0;
        let type_cf = assertion_type.as_cfstring();
        let reason_cf = CFString::new(reason);

        let result = unsafe {
            IOPMAssertionCreateWithName(
                type_cf.as_concrete_TypeRef(),
                kIOPMAssertionLevelOn,
                reason_cf.as_concrete_TypeRef(),
                &mut assertion_id,
            )
        };

        if result == 0 {
            Ok(Self {
                assertion_id,
                assertion_type,
            })
        } else {
            Err(format!(
                "Failed to create power assertion (IOReturn: {result})"
            ))
        }
    }

    pub fn assertion_type(&self) -> AssertionType {
        self.assertion_type
    }
}

impl Drop for PowerAssertion {
    fn drop(&mut self) {
        unsafe {
            IOPMAssertionRelease(self.assertion_id);
        }
    }
}
