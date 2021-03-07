

type Headers = ();
type SocketDetails = (String, u16);

const SCOPE_TYPE: &str = "http";
const SCOPE_VERSION: &str = "";
const SCOPE_SPEC_VERSION: &str = "";

const HTTP_10: &str = "1.0";
const HTTP_11: &str = "1.1";
const HTTP_2: &str = "2";


/// The ASGI specification.
const SCOPE_SPEC: ASGISpec = ASGISpec {
    /// Version of the ASGI spec
    version: SCOPE_VERSION,

    /// Version of the ASGI HTTP spec this server understands
    spec_version: SCOPE_SPEC_VERSION
};


/// The ASGI specification
pub struct ASGISpec {
    version: &'static str,
    spec_version: &'static str,
}


/// The asgi scope that contains all state of the server and
/// request.
pub type AsgiScopeArgs = (
    // type
    //
    // The type of scope, for a request this is "http"
    &'static str,

    // spec
    //
    // The ASGI specification
    ASGISpec,

    // http_version
    //
    // One of "1.0", "1.1" or "2", representing HTTP/1, HTTP/1.1, HTTP/2.
    &'static str,

    // method
    //
    // The HTTP method name, in uppercase.
    &'static str,

    // scheme
    //
    // URL scheme portion, either http or https.
    String,

    // path
    //
    // HTTP request target excluding any query string,
    // with percent-encoded sequences and UTF-8 byte sequences
    // decoded into characters.
    String,

    // raw_path
    //
    // The original HTTP path component unmodified from the bytes
    // that were received by the web server.
    String,

    // query_string
    //
    // URL portion after the ?, percent-encoded.
    String,

    // root_path
    //
    // The root path this application is mounted at
    String,

    // headers
    //
    // iterable of `(name, value)` two-item iterables, where name
    // is the header name, and value is the header value.
    // Order of header values must be preserved from the original
    // HTTP request.
    Headers,

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
