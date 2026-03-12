use super::AssertionType;
use windows_sys::Win32::System::Power::{
    SetThreadExecutionState, ES_CONTINUOUS, ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED,
    EXECUTION_STATE,
};

fn execution_state_flags(assertion_type: AssertionType) -> EXECUTION_STATE {
    match assertion_type {
        AssertionType::IdleSystem => ES_CONTINUOUS | ES_SYSTEM_REQUIRED,
        AssertionType::IdleDisplay => {
            ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED
        }
        AssertionType::System => ES_CONTINUOUS | ES_SYSTEM_REQUIRED,
    }
}

pub struct WindowsAssertion {
    flags: EXECUTION_STATE,
}

impl WindowsAssertion {
    pub fn create(assertion_type: AssertionType, _reason: &str) -> Result<Self, String> {
        let flags = execution_state_flags(assertion_type);
        let prev = unsafe { SetThreadExecutionState(flags) };
        if prev == 0 {
            Err("SetThreadExecutionState failed".to_string())
        } else {
            Ok(Self { flags })
        }
    }
}

impl Drop for WindowsAssertion {
    fn drop(&mut self) {
        // Clear only the flags we set, keeping ES_CONTINUOUS to signal reset
        let _ = self.flags;
        unsafe {
            SetThreadExecutionState(ES_CONTINUOUS);
        }
    }
}
