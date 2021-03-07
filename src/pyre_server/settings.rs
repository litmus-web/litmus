use std::net::SocketAddr;


/// The HTTP schema.
///
/// The selection of whether or not this is HTTP or HTTPS depends on the
/// socket selected at creation.
#[derive(Debug, Copy, Clone)]
pub enum Schema {
    HTTP,
    HTTPS,
}

impl Schema {
    pub fn as_str(&self) -> &'static str {
        match self {
            &Self::HTTP => "http",
            &Self::HTTPS => "https",
        }
    }
}


/// The constant server settings that are used to construct a given
/// ASGI scope for the web server.
#[derive(Debug, Copy, Clone)]
pub struct Settings {
    pub schema: Schema,
    pub server_addr: SocketAddr,
}

impl Settings {
    /// Create a new settings instance with a given set of specs
    pub fn new(
        is_tls: bool,
        server_addr: SocketAddr,
    ) -> Self {

        let schema = if is_tls {
            Schema::HTTP
        } else {
            Schema::HTTPS
        };

        Self {
            schema,
            server_addr,
        }
    }
}