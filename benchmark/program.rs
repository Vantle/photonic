use frontend::source::Program;
use std::path::Path;

pub fn read(path: &Path) -> Result<Program, Box<dyn std::error::Error>> {
    let text = std::fs::read_to_string(path)?;
    if path
        .extension()
        .is_some_and(|extension| extension == "json")
    {
        return Ok(Program::read(&text)?);
    }
    Ok(frontend::lowering::parse(&text)?)
}
