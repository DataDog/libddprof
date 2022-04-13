// Unless explicitly stated otherwise all files in this repository are licensed under the Apache License Version 2.0.
// This product includes software developed at Datadog (https://www.datadoghq.com/). Copyright 2021-Present Datadog, Inc.

use std::error;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub(crate) enum Error {
    InvalidUrl,
    OperationTimedOut,
    UnixSockeUnsuported,
    CannotEstablishTlsConnection,
    NoValidCertifacteRootsFound,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidUrl => "invalid url",
            Self::OperationTimedOut => "operation timed out",
            Self::UnixSockeUnsuported => "unix sockets unsuported on windows",
            Self::CannotEstablishTlsConnection => {
                "cannot establish requested secure TLS connection"
            }
            Self::NoValidCertifacteRootsFound => {
                "native tls couldn't find any valid certifacte roots"
            }
        })
    }
}

impl error::Error for Error {}
