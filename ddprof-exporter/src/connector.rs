use std::error::Error;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

// Tokio doesn't handle unix sockets on windows
#[cfg(unix)]
pub(crate) mod uds {
    use pin_project_lite::pin_project;
    use std::error::Error;
    use std::ffi::OsString;
    use std::os::unix::ffi::{OsStrExt, OsStringExt};
    use std::path::{Path, PathBuf};

    /// Creates a new Uri, with the `unix` scheme, and the path to the socket
    /// encoded as a hex string, to prevent special characters in the url authority
    pub fn socket_path_to_uri(path: &Path) -> Result<hyper::Uri, Box<dyn Error>> {
        let path = hex::encode(path.as_os_str().as_bytes());
        Ok(hyper::Uri::builder()
            .scheme("unix")
            .authority(path)
            .path_and_query("")
            .build()?)
    }

    pub fn socket_path_from_uri(
        uri: &hyper::Uri,
    ) -> Result<PathBuf, Box<dyn Error + Sync + Send + 'static>> {
        if uri.scheme_str() != Some("unix") {
            return Err(crate::errors::Error::InvalidUrl.into());
        }
        let path = hex::decode(
            uri.authority()
                .ok_or(crate::errors::Error::InvalidUrl)?
                .as_str(),
        )
        .map_err(|_| crate::errors::Error::InvalidUrl)?;
        Ok(PathBuf::from(OsString::from_vec(path)))
    }

    #[test]
    fn test_encode_unix_socket_path_absolute() {
        let expected_path = "/path/to/a/socket.sock".as_ref();
        let uri = socket_path_to_uri(expected_path).unwrap();
        assert_eq!(uri.scheme_str(), Some("unix"));

        let actual_path = socket_path_from_uri(&uri).unwrap();
        assert_eq!(actual_path.as_path(), Path::new(expected_path))
    }

    #[test]
    fn test_encode_unix_socket_relative_path() {
        let expected_path = "relative/path/to/a/socket.sock".as_ref();
        let uri = socket_path_to_uri(expected_path).unwrap();
        let actual_path = socket_path_from_uri(&uri).unwrap();
        assert_eq!(actual_path.as_path(), Path::new(expected_path));

        let expected_path = "./relative/path/to/a/socket.sock".as_ref();
        let uri = socket_path_to_uri(expected_path).unwrap();
        let actual_path = socket_path_from_uri(&uri).unwrap();
        assert_eq!(actual_path.as_path(), Path::new(expected_path));
    }

    pin_project! {
        #[project = ConnStreamProj]
        pub enum ConnStream {
            Tcp{ #[pin] transport: tokio::net::TcpStream },
            Tls{ #[pin] transport: tokio_rustls::client::TlsStream<tokio::net::TcpStream>},
            Udp{ #[pin] transport: tokio::net::UnixStream },
        }
    }
}

use futures::{future, FutureExt};
use hyper::client::HttpConnector;
use hyper_rustls::MaybeHttpsStream;
use rustls::ClientConfig;

#[cfg(unix)]
use uds::{ConnStream, ConnStreamProj};

#[cfg(not(unix))]
pin_project_lite::pin_project! {
    #[project = ConnStreamProj]
    pub(crate) enum ConnStream {
        Tcp{ #[pin] transport: tokio::net::TcpStream },
        Tls{ #[pin] transport: tokio_rustls::client::TlsStream<tokio::net::TcpStream>},
    }
}

#[derive(Clone)]
pub enum MaybeHttpsConnector {
    Http(hyper::client::HttpConnector),
    Https(hyper_rustls::HttpsConnector<hyper::client::HttpConnector>),
}

impl MaybeHttpsConnector {
    pub(crate) fn new() -> Self {
        match build_https_connector() {
            Some(connector) => MaybeHttpsConnector::Https(connector),
            None => MaybeHttpsConnector::Http(HttpConnector::new()),
        }
    }
}

fn build_https_connector() -> Option<hyper_rustls::HttpsConnector<hyper::client::HttpConnector>> {
    let certs = load_root_certs()?;
    let client_config = ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(certs)
        .with_no_client_auth();
    Some(
        hyper_rustls::HttpsConnectorBuilder::new()
            .with_tls_config(client_config)
            .https_or_http()
            .enable_http1()
            .build(),
    )
}

fn load_root_certs() -> Option<rustls::RootCertStore> {
    let mut roots = rustls::RootCertStore::empty();
    let mut invalid_count = 0;

    for cert in rustls_native_certs::load_native_certs().ok()? {
        let cert = rustls::Certificate(cert.0);
        match roots.add(&cert) {
            Ok(_) => valid_count += 1,
            Err(err) => invalid_count += 1,
        }
    }
    if roots.is_empty() {
        return None;
    }
    Some(roots)
}

impl tokio::io::AsyncRead for ConnStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        match self.project() {
            ConnStreamProj::Tcp { transport } => transport.poll_read(cx, buf),
            ConnStreamProj::Tls { transport } => transport.poll_read(cx, buf),
            #[cfg(unix)]
            ConnStreamProj::Udp { transport } => transport.poll_read(cx, buf),
        }
    }
}

impl hyper::client::connect::Connection for ConnStream {
    fn connected(&self) -> hyper::client::connect::Connected {
        match self {
            Self::Tcp { transport } => transport.connected(),
            Self::Tls { transport } => {
                let (tcp, tls) = transport.get_ref();
                if tls.alpn_protocol() == Some(b"h2") {
                    // TODO/QUESTION: is it safe, future proof, to implement this ourselves ?
                    tcp.connected().negotiated_h2()
                } else {
                    tcp.connected()
                }
            }
            #[cfg(unix)]
            Self::Udp { transport: _ } => hyper::client::connect::Connected::new(),
        }
    }
}

impl tokio::io::AsyncWrite for ConnStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        match self.project() {
            ConnStreamProj::Tcp { transport } => transport.poll_write(cx, buf),
            ConnStreamProj::Tls { transport } => transport.poll_write(cx, buf),
            #[cfg(unix)]
            ConnStreamProj::Udp { transport } => transport.poll_write(cx, buf),
        }
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        match self.project() {
            ConnStreamProj::Tcp { transport } => transport.poll_shutdown(cx),
            ConnStreamProj::Tls { transport } => transport.poll_shutdown(cx),
            #[cfg(unix)]
            ConnStreamProj::Udp { transport } => transport.poll_shutdown(cx),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
        match self.project() {
            ConnStreamProj::Tcp { transport } => transport.poll_flush(cx),
            ConnStreamProj::Tls { transport } => transport.poll_flush(cx),
            #[cfg(unix)]
            ConnStreamProj::Udp { transport } => transport.poll_flush(cx),
        }
    }
}

impl hyper::service::Service<hyper::Uri> for MaybeHttpsConnector {
    type Response = ConnStream;
    type Error = Box<dyn Error + Sync + Send>;

    // This lint gets lifted in this place in a newer version, see:
    // https://github.com/rust-lang/rust-clippy/pull/8030
    #[allow(clippy::type_complexity)]
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&mut self, uri: hyper::Uri) -> Self::Future {
        match uri.scheme_str() {
            Some("unix") => Box::pin(async move {
                #[cfg(unix)]
                {
                    let path = uds::socket_path_from_uri(&uri)?;
                    Ok(ConnStream::Udp {
                        transport: tokio::net::UnixStream::connect(path).await?,
                    })
                }
                #[cfg(not(unix))]
                {
                    Err(crate::errors::Error::UnixSockeUnsuported.into())
                }
            }),
            Some("https") => match self {
                Self::Http(_) => future::err::<Self::Response, Self::Error>(
                    crate::errors::Error::CannotEstablishTlsConnection.into(),
                )
                .boxed(),
                Self::Https(c) => {
                    let fut = c.call(uri);
                    Box::pin(async {
                        match fut.await? {
                            MaybeHttpsStream::Http(_) => {
                                Err(crate::errors::Error::CannotEstablishTlsConnection.into())
                            }
                            MaybeHttpsStream::Https(t) => Ok(ConnStream::Tls { transport: t }),
                        }
                    })
                }
            },
            _ => match self {
                Self::Http(c) => {
                    let fut = c.call(uri);
                    Box::pin(async {
                        Ok(ConnStream::Tcp {
                            transport: fut.await?,
                        })
                    })
                }
                Self::Https(c) => {
                    let fut = c.call(uri);
                    Box::pin(async {
                        match fut.await? {
                            MaybeHttpsStream::Http(t) => Ok(ConnStream::Tcp { transport: t }),
                            MaybeHttpsStream::Https(t) => Ok(ConnStream::Tls { transport: t }),
                        }
                    })
                }
            },
        }
    }

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match self {
            MaybeHttpsConnector::Http(c) => c.poll_ready(cx).map_err(|e| e.into()),
            MaybeHttpsConnector::Https(c) => c.poll_ready(cx),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// Verify that the Connector type implements the correct bound Connect + Clone
    /// to be able to use the hyper::Client
    fn test_hyper_client_from_connector() {
        let _: hyper::Client<MaybeHttpsConnector> =
            hyper::Client::builder().build(MaybeHttpsConnector::new());
    }
}
