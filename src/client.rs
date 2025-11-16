//! Client implementation for the `bore` service.

use std::{path::PathBuf, sync::Arc};

use anyhow::{bail, Context, Result};
use tokio::{
    io::AsyncWriteExt,
    net::{TcpStream, UnixStream},
    time::timeout,
};
use tracing::{error, info, info_span, warn, Instrument};
use uuid::Uuid;

use crate::auth::Authenticator;
use crate::shared::{ClientMessage, Delimited, ServerMessage, CONTROL_PORT, NETWORK_TIMEOUT};

/// Represents the local target to forward connections to.
#[derive(Clone)]
enum LocalTarget {
    /// TCP target with host and port.
    Tcp { host: String, port: u16 },
    /// Unix socket target with path.
    UnixSocket { path: PathBuf },
}

/// State structure for the client.
pub struct Client {
    /// Control connection to the server.
    conn: Option<Delimited<TcpStream>>,

    /// Destination address of the server.
    to: String,

    /// Local target to forward connections to.
    local_target: LocalTarget,

    /// Port that is publicly available on the remote.
    remote_port: u16,

    /// Optional secret used to authenticate clients.
    auth: Option<Authenticator>,
}

impl Client {
    /// Create a new client for TCP target.
    pub async fn new(
        local_host: &str,
        local_port: u16,
        to: &str,
        port: u16,
        secret: Option<&str>,
    ) -> Result<Self> {
        let local_target = LocalTarget::Tcp {
            host: local_host.to_string(),
            port: local_port,
        };
        Self::create(local_target, to, port, secret).await
    }

    /// Create a new client for Unix socket target.
    pub async fn new_unix_socket(
        socket_path: PathBuf,
        to: &str,
        port: u16,
        secret: Option<&str>,
    ) -> Result<Self> {
        let local_target = LocalTarget::UnixSocket { path: socket_path };
        Self::create(local_target, to, port, secret).await
    }

    /// Internal method to create a client with the given local target.
    async fn create(
        local_target: LocalTarget,
        to: &str,
        port: u16,
        secret: Option<&str>,
    ) -> Result<Self> {
        let mut stream = Delimited::new(connect_with_timeout(to, CONTROL_PORT).await?);
        let auth = secret.map(Authenticator::new);
        if let Some(auth) = &auth {
            auth.client_handshake(&mut stream).await?;
        }

        stream.send(ClientMessage::Hello(port)).await?;
        let remote_port = match stream.recv_timeout().await? {
            Some(ServerMessage::Hello(remote_port)) => remote_port,
            Some(ServerMessage::Error(message)) => bail!("server error: {message}"),
            Some(ServerMessage::Challenge(_)) => {
                bail!("server requires authentication, but no client secret was provided");
            }
            Some(_) => bail!("unexpected initial non-hello message"),
            None => bail!("unexpected EOF"),
        };

        let target_info = match &local_target {
            LocalTarget::Tcp { host, port } => format!("{host}:{port}"),
            LocalTarget::UnixSocket { path } => format!("unix:{}", path.display()),
        };
        info!(remote_port, %target_info, "connected to server");
        info!("listening at {to}:{remote_port}");

        Ok(Client {
            conn: Some(stream),
            to: to.to_string(),
            local_target,
            remote_port,
            auth,
        })
    }

    /// Returns the port publicly available on the remote.
    pub fn remote_port(&self) -> u16 {
        self.remote_port
    }

    /// Start the client, listening for new connections.
    pub async fn listen(mut self) -> Result<()> {
        let mut conn = self.conn.take().unwrap();
        let this = Arc::new(self);
        loop {
            match conn.recv().await? {
                Some(ServerMessage::Hello(_)) => warn!("unexpected hello"),
                Some(ServerMessage::Challenge(_)) => warn!("unexpected challenge"),
                Some(ServerMessage::Heartbeat) => (),
                Some(ServerMessage::Connection(id)) => {
                    let this = Arc::clone(&this);
                    tokio::spawn(
                        async move {
                            info!("new connection");
                            match this.handle_connection(id).await {
                                Ok(_) => info!("connection exited"),
                                Err(err) => warn!(%err, "connection exited with error"),
                            }
                        }
                        .instrument(info_span!("proxy", %id)),
                    );
                }
                Some(ServerMessage::Error(err)) => error!(%err, "server error"),
                None => return Ok(()),
            }
        }
    }

    async fn handle_connection(&self, id: Uuid) -> Result<()> {
        let mut remote_conn =
            Delimited::new(connect_with_timeout(&self.to[..], CONTROL_PORT).await?);
        if let Some(auth) = &self.auth {
            auth.client_handshake(&mut remote_conn).await?;
        }
        remote_conn.send(ClientMessage::Accept(id)).await?;

        match &self.local_target {
            LocalTarget::Tcp { host, port } => {
                let mut local_conn = connect_with_timeout(host, *port).await?;
                let mut parts = remote_conn.into_parts();
                debug_assert!(parts.write_buf.is_empty(), "framed write buffer not empty");
                local_conn.write_all(&parts.read_buf).await?;
                tokio::io::copy_bidirectional(&mut local_conn, &mut parts.io).await?;
            }
            LocalTarget::UnixSocket { path } => {
                let mut local_conn = connect_unix_socket_with_timeout(path).await?;
                let mut parts = remote_conn.into_parts();
                debug_assert!(parts.write_buf.is_empty(), "framed write buffer not empty");
                local_conn.write_all(&parts.read_buf).await?;
                tokio::io::copy_bidirectional(&mut local_conn, &mut parts.io).await?;
            }
        }
        Ok(())
    }
}

async fn connect_with_timeout(to: &str, port: u16) -> Result<TcpStream> {
    match timeout(NETWORK_TIMEOUT, TcpStream::connect((to, port))).await {
        Ok(res) => res,
        Err(err) => Err(err.into()),
    }
    .with_context(|| format!("could not connect to {to}:{port}"))
}

async fn connect_unix_socket_with_timeout(path: &PathBuf) -> Result<UnixStream> {
    match timeout(NETWORK_TIMEOUT, UnixStream::connect(path)).await {
        Ok(res) => res,
        Err(err) => Err(err.into()),
    }
    .with_context(|| format!("could not connect to unix socket {}", path.display()))
}
