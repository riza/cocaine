use super::AssertionType;
use core_foundation::base::TCFType;
use core_foundation::string::CFString;

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

fn assertion_type_string(t: AssertionType) -> CFString {
    match t {
        AssertionType::IdleSystem => CFString::new("PreventUserIdleSystemSleep"),
        AssertionType::IdleDisplay => CFString::new("PreventUserIdleDisplaySleep"),
        AssertionType::System => CFString::new("PreventSystemSleep"),
    }
}

pub struct MacOSAssertion {
    assertion_id: u32,
}

impl MacOSAssertion {
    pub fn create(assertion_type: AssertionType, reason: &str) -> Result<Self, String> {
        let mut assertion_id: u32 = 0;
        let type_cf = assertion_type_string(assertion_type);
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
            Ok(Self { assertion_id })
        } else {
            Err(format!(
                "Failed to create power assertion (IOReturn: {result})"
            ))
        }
    }
}

impl Drop for MacOSAssertion {
    fn drop(&mut self) {
        unsafe {
            IOPMAssertionRelease(self.assertion_id);
        }
    }
}
