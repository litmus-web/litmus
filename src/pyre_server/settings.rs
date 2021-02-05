use http::Version;
use std::net::SocketAddr;


const HTTP_STR: &str = "http";
const HTTPS_STR: &str = "http";


pub enum Schema {
    HTTP(&'static str),
    HTTPS(&'static str),
}


pub struct Settings {
    schema: Schema,
    server_addr: SocketAddr,

}

impl Settings {
    pub fn new(
        is_tls: bool,
        server_addr: SocketAddr,
    ) -> Self {

        let schema = if is_tls {
            Schema::HTTP(HTTP_STR)
        } else {
            Schema::HTTPS(HTTPS_STR)
        };

        Self {
            schema,
            server_addr,
        }
    }
}