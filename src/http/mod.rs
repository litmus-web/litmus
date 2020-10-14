use std::collections::HashMap;

// 1xx codes
static STATUS_100: &[u8] = b"100 Continue";
static STATUS_101: &[u8] = b"101 Switching Protocol";
static STATUS_102: &[u8] = b"102 Processing";
static STATUS_103: &[u8] = b"103 Early Hints";

// 2xx codes
static STATUS_200: &[u8] = b"200 OK";
static STATUS_201: &[u8] = b"201 Created";
static STATUS_202: &[u8] = b"202 Accepted";
static STATUS_203: &[u8] = b"203 Non-Authoritative Information";
static STATUS_204: &[u8] = b"204 No Content";
static STATUS_205: &[u8] = b"205 Reset Content";
static STATUS_206: &[u8] = b"206 Partial Content";
static STATUS_207: &[u8] = b"207 Multi-Status";
static STATUS_208: &[u8] = b"208 Already Reported";
static STATUS_226: &[u8] = b"226 IM Used";

// 3xx codes
static STATUS_300: &[u8] = b"300 Multiple Choice";
static STATUS_301: &[u8] = b"301 Moved Permanently";
static STATUS_302: &[u8] = b"302 Found";
static STATUS_303: &[u8] = b"303 See Other";
static STATUS_307: &[u8] = b"307 Temporary Redirect";
static STATUS_308: &[u8] = b"308 Permanent Redirect";

// 4xx codes
static STATUS_400: &[u8] = b"400 Bad Request";
static STATUS_401: &[u8] = b"401 Unauthorized";
static STATUS_402: &[u8] = b"402 Payment Required";
static STATUS_403: &[u8] = b"403 Forbidden";
static STATUS_404: &[u8] = b"404 Not Found";
static STATUS_405: &[u8] = b"405 Method Not Allowed";
static STATUS_406: &[u8] = b"406 Not Acceptable";
static STATUS_407: &[u8] = b"407 Proxy Authentication Required";
static STATUS_408: &[u8] = b"408 Request Timeout";
static STATUS_409: &[u8] = b"409 Conflict";
static STATUS_410: &[u8] = b"410 Gone";
static STATUS_411: &[u8] = b"411 Length Required";
static STATUS_412: &[u8] = b"412 Precondition Failed";
static STATUS_413: &[u8] = b"413 Payload Too Large";
static STATUS_414: &[u8] = b"414 URI Too Long";
static STATUS_415: &[u8] = b"415 Unsupported Media Type";
static STATUS_416: &[u8] = b"416 Range Not Satisfiable";
static STATUS_417: &[u8] = b"417 Expectation Failed";
static STATUS_418: &[u8] = b"418 I'm a teapot";
static STATUS_421: &[u8] = b"421 Misdirected Request";
static STATUS_422: &[u8] = b"422 Unprocessable Entity";
static STATUS_423: &[u8] = b"423 Locked";
static STATUS_424: &[u8] = b"424 Failed Dependency";
static STATUS_425: &[u8] = b"425 Too Early";
static STATUS_426: &[u8] = b"426 Upgrade Required";
static STATUS_428: &[u8] = b"428 Precondition Required";
static STATUS_429: &[u8] = b"429 Too Many Requests";
static STATUS_431: &[u8] = b"431 Request Header Fields Too Large";
static STATUS_451: &[u8] = b"451 Unavailable For Legal Reasons";

// 5xx codes
static STATUS_500: &[u8] = b"500 Internal Server Error";
static STATUS_501: &[u8] = b"501 Not Implemented";
static STATUS_502: &[u8] = b"502 Bad Gateway";
static STATUS_503: &[u8] = b"503 Service Unavailable";
static STATUS_504: &[u8] = b"504 Gateway Timeout";
static STATUS_505: &[u8] = b"505 HTTP Version Not Supported";
static STATUS_506: &[u8] = b"506 Variant Also Negotiates";
static STATUS_507: &[u8] = b"507 Insufficient Storage";
static STATUS_508: &[u8] = b"508 Loop Detected";
static STATUS_510: &[u8] = b"510 Not Extended";
static STATUS_511: &[u8] = b"511 Network Authentication Required";

static CODE_MAP: HashMap<u16, &[u8]> = {
    let mut map = HashMap::new();

    map.insert(100, STATUS_100);


    map
};

pub fn get_bytes_from_u16(status: u16) {

}