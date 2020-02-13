use std::fmt;

#[derive(Debug, Clone)]
pub enum Error {
    NoError // TODO
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        unimplemented!(); // TODO
    }
}