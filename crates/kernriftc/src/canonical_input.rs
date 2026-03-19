use std::fs;
use std::io::{self, Read};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CanonicalInput<'a> {
    File(&'a str),
    Stdin,
}

impl<'a> CanonicalInput<'a> {
    pub(crate) fn from_optional_path(stdin: bool, path: Option<&'a str>) -> Self {
        match (stdin, path) {
            (true, None) => Self::Stdin,
            (false, Some(path)) => Self::File(path),
            _ => panic!("invalid canonical input state"),
        }
    }

    pub(crate) fn label(self) -> &'a str {
        match self {
            Self::File(path) => path,
            Self::Stdin => "<stdin>",
        }
    }

    pub(crate) fn read_to_string(self) -> Result<String, Vec<String>> {
        match self {
            Self::File(path) => fs::read_to_string(path)
                .map_err(|err| vec![format!("failed to read '{}': {}", path, err)]),
            Self::Stdin => {
                let mut src = String::new();
                io::stdin()
                    .read_to_string(&mut src)
                    .map_err(|err| vec![format!("failed to read '<stdin>': {}", err)])?;
                Ok(src)
            }
        }
    }
}
