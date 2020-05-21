use distinst_chroot::Command;
use std::{
    collections::HashSet,
    hash::{Hash, Hasher},
    io,
    str::FromStr,
};

/// Fetch a vector of APT URIs required for the given `apt-get` operation.
pub fn apt_uris(args: &[&str]) -> Result<HashSet<AptUri>, AptUriError> {
    let mut cmd = Command::new("apt-get");

    cmd.env("DEBIAN_FRONTEND", "noninteractive");

    let output =
        cmd.args(&["--print-uris"]).args(args).run_with_stdout().map_err(AptUriError::Command)?;

    let mut packages = HashSet::new();
    for line in output.lines() {
        if !line.starts_with('\'') {
            continue;
        }

        packages.insert(line.parse::<AptUri>()?);
    }

    Ok(packages)
}

#[derive(Debug, Error)]
pub enum AptUriError {
    #[error(display = "apt command failed: {}", _0)]
    Command(io::Error),
    #[error(display = "uri not found in output: {}", _0)]
    UriNotFound(String),
    #[error(display = "invalid URI value: {}", _0)]
    UriInvalid(String),
    #[error(display = "name not found in output: {}", _0)]
    NameNotFound(String),
    #[error(display = "size not found in output: {}", _0)]
    SizeNotFound(String),
    #[error(display = "size in output could not be parsed as an integer: {}", _0)]
    SizeParse(String),
    #[error(display = "md5sum not found in output: {}", _0)]
    Md5NotFound(String),
    #[error(display = "md5 prefix (MD5Sum:) not found in md5sum: {}", _0)]
    Md5Prefix(String),
}

#[derive(Debug, Clone, Eq)]
pub struct AptUri {
    pub uri:    String,
    pub name:   String,
    pub size:   u64,
    pub md5sum: String,
}

impl PartialEq for AptUri {
    fn eq(&self, other: &Self) -> bool { self.uri == other.uri }
}

impl Hash for AptUri {
    fn hash<H: Hasher>(&self, state: &mut H) { self.uri.hash(state); }
}

impl FromStr for AptUri {
    type Err = AptUriError;

    fn from_str(line: &str) -> Result<Self, Self::Err> {
        let mut words = line.split_whitespace();

        let mut uri = words.next().ok_or_else(|| AptUriError::UriNotFound(line.into()))?;

        // We need to remove the single quotes that apt-get encloses the URI within.
        if uri.len() <= 3 {
            return Err(AptUriError::UriInvalid(uri.into()));
        } else {
            uri = &uri[1..uri.len() - 1];
        }

        let name = words.next().ok_or_else(|| AptUriError::NameNotFound(line.into()))?;
        let size = words.next().ok_or_else(|| AptUriError::SizeNotFound(line.into()))?;
        let size = size.parse::<u64>().map_err(|_| AptUriError::SizeParse(size.into()))?;
        let mut md5sum = words.next().ok_or_else(|| AptUriError::Md5NotFound(line.into()))?;

        if md5sum.starts_with("MD5Sum:") {
            md5sum = &md5sum[7..];
        } else {
            return Err(AptUriError::Md5Prefix(md5sum.into()));
        }

        Ok(AptUri { uri: uri.into(), name: name.into(), size, md5sum: md5sum.into() })
    }
}
