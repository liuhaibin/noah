#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "windows")]
pub mod windows;

use crate::agent::tool_router::ToolRouter;

/// Register platform-specific tools with the router.
pub fn register_platform_tools(_router: &mut ToolRouter) {
    #[cfg(target_os = "macos")]
    macos::register_tools(_router);

    #[cfg(target_os = "windows")]
    windows::register_tools(_router);
}
