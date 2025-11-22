use super::Errors;

pub fn parse_name(name: String) -> Result<(String, Option<String>), Errors> {
    let mut parts = name.split('@');
    let cell = parts.next().map(|s| s.to_string());
    let proj_name = parts.next().map(|s| s.to_string());

    let (cell, project_name) = match (cell, proj_name) {
        (Some(cell), Some(proj)) => (Some(cell), proj),
        (Some(proj), None) => (None, proj),
        _ => return Err(Errors::InvalidNameFormat(name)),
    };

    Ok((project_name, cell))
}

