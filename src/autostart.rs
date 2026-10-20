//! Launch at login through `SMAppService` (macOS 13+).

use objc2_service_management::{SMAppService, SMAppServiceStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginState {
    Enabled,
    RequiresApproval,
    Disabled,
}

pub fn state() -> LoginState {
    let svc = unsafe { SMAppService::mainAppService() };
    match unsafe { svc.status() } {
        SMAppServiceStatus::Enabled => LoginState::Enabled,
        SMAppServiceStatus::RequiresApproval => LoginState::RequiresApproval,
        _ => LoginState::Disabled,
    }
}

/// Register or unregister the login item; returns the resulting state.
pub fn set_enabled(enable: bool) -> Result<LoginState, String> {
    let svc = unsafe { SMAppService::mainAppService() };
    let r = if enable {
        unsafe { svc.registerAndReturnError() }
    } else {
        unsafe { svc.unregisterAndReturnError() }
    };
    r.map_err(|e| e.localizedDescription().to_string())?;
    Ok(state())
}

pub fn open_login_items_settings() {
    unsafe { SMAppService::openSystemSettingsLoginItems() };
}
