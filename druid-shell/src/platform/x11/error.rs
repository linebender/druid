use std::fmt;

#[derive(Debug, Clone)]
pub enum Error {
    // TODO(x11/errors): enumerate `Error`s for X11
    NoError
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        // TODO(x11/errors): implement Error::fmt
        unimplemented!();
    }
}