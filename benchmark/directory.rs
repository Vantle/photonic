pub fn enter() -> std::io::Result<()> {
    let Some(directory) = std::env::var_os("BUILD_WORKING_DIRECTORY") else {
        return Ok(());
    };
    std::env::set_current_dir(directory)
}
