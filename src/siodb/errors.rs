// Copyright (C) 2019-2020 Siodb GmbH. All rights reserved.
// Use of this source code is governed by a license that can be found
// in the LICENSE file.

use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub struct DriverError {
    details: String,
}

impl DriverError {
    pub fn new(msg: &str) -> DriverError {
        DriverError {
            details: msg.to_string(),
        }
    }
}

impl fmt::Display for DriverError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl Error for DriverError {
    fn description(&self) -> &str {
        &self.details
    }
}

impl From<protobuf::error::ProtobufError> for DriverError {
    fn from(err: protobuf::error::ProtobufError) -> Self {
        DriverError::new(&err.to_string())
    }
}

impl From<openssl::error::ErrorStack> for DriverError {
    fn from(err: openssl::error::ErrorStack) -> Self {
        DriverError::new(&err.to_string())
    }
}

impl From<std::io::Error> for DriverError {
    fn from(err: std::io::Error) -> Self {
        DriverError::new(&err.to_string())
    }
}

pub fn debug(trace: bool, msg: &str) {
    if trace {
        println!("DEBUG: {}", msg);
    }
}
