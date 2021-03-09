use pyo3::Py;
use pyo3::types::PyBytes;

/// A set of headers.
///
/// Each header is a (name, value) pair.
type Headers<'a> = Vec<(&'a str, Py<PyBytes>)>;

/// A simple tuple containing the ip string and port
type SocketDetails = (String, u16);

/// The type of the scope call
pub const SCOPE_TYPE: &str = "http";

/// A temporary root path constant todo: allow this to be set
pub const TEMP_ROOT_PATH: &str = "";

/// The HTTP/1.0 specification
pub const HTTP_10: &str = "1.0";

/// The HTTP/1.1 specification
pub const HTTP_11: &str = "1.1";

/// The HTTP/2 specification
pub const _HTTP_2: &str = "2";


/// The PSGI scope that contains all state of the server and
/// request.
pub type PSGIScope<'a> = (
    // type
    //
    // The type of scope, for a request this is "http"
    &'static str,

    // http_version
    //
    // One of "1.0", "1.1" or "2", representing HTTP/1, HTTP/1.1, HTTP/2.
    &'static str,

    // method
    //
    // The HTTP method name, in uppercase.
    &'a str,

    // scheme
    //
    // URL scheme portion, either http or https.
    &'static str,

    // path
    //
    // HTTP request target excluding any query string,
    // with percent-encoded sequences and UTF-8 byte sequences
    // decoded into characters.
    &'a str,

    // query_string
    //
    // URL portion after the ?, percent-encoded.
    &'a str,

    // root_path
    //
    // The root path this application is mounted at
    &'a str,

    // headers
    //
    // iterable of `(name, value)` two-item iterables, where name
    // is the header name, and value is the header value.
    // Order of header values must be preserved from the original
    // HTTP request.
    Headers<'a>,

    // client
    //
    // A two-item iterable of (host, port), where host is the remote
    // host’s IPv4 or IPv6 address, and port is the remote port
    // as an u16.
    SocketDetails,

    // server
    //
    // A two-item iterable of (host, port), where host is the
    // listening address for this server.
    SocketDetails,
);
