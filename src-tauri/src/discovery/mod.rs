pub mod epic;
pub mod gog;
pub mod manager;
pub mod models;
pub mod provider;
pub mod scanner;
pub mod steam;

/// Runs `reg query` with the given arguments. In release builds the app has no
/// console, so without CREATE_NO_WINDOW every query would flash a console window.
pub(crate) fn reg_query(args: &[&str]) -> std::io::Result<std::process::Output> {
    let mut command = std::process::Command::new("reg");
    command.arg("query").args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command.output()
}
