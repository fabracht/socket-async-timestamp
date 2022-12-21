#[derive(thiserror::Error, Debug)]
pub enum LibError {
    AddrParseError(#[from] std::net::AddrParseError),
    #[error(transparent)]
    IO(#[from] std::io::Error),
    OSError(#[from] nix::Error),
}

impl std::fmt::Display for LibError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", &self)
    }
}
