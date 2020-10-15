// 1xx codes
static STATUS_100: &str = "100 Continue";
static STATUS_101: &str = "101 Switching Protocol";
static STATUS_102: &str = "102 Processing";
static STATUS_103: &str = "103 Early Hints";

// 2xx codes
static STATUS_200: &str = "200 OK";
static STATUS_201: &str = "201 Created";
static STATUS_202: &str = "202 Accepted";
static STATUS_203: &str = "203 Non-Authoritative Information";
static STATUS_204: &str = "204 No Content";
static STATUS_205: &str = "205 Reset Content";
static STATUS_206: &str = "206 Partial Content";
static STATUS_207: &str = "207 Multi-Status";
static STATUS_208: &str = "208 Already Reported";
static STATUS_226: &str = "226 IM Used";

// 3xx codes
static STATUS_300: &str = "300 Multiple Choice";
static STATUS_301: &str = "301 Moved Permanently";
static STATUS_302: &str = "302 Found";
static STATUS_303: &str = "303 See Other";
static STATUS_307: &str = "307 Temporary Redirect";
static STATUS_308: &str = "308 Permanent Redirect";

// 4xx codes
static STATUS_400: &str = "400 Bad Request";
static STATUS_401: &str = "401 Unauthorized";
static STATUS_402: &str = "402 Payment Required";
static STATUS_403: &str = "403 Forbidden";
static STATUS_404: &str = "404 Not Found";
static STATUS_405: &str = "405 Method Not Allowed";
static STATUS_406: &str = "406 Not Acceptable";
static STATUS_407: &str = "407 Proxy Authentication Required";
static STATUS_408: &str = "408 Request Timeout";
static STATUS_409: &str = "409 Conflict";
static STATUS_410: &str = "410 Gone";
static STATUS_411: &str = "411 Length Required";
static STATUS_412: &str = "412 Precondition Failed";
static STATUS_413: &str = "413 Payload Too Large";
static STATUS_414: &str = "414 URI Too Long";
static STATUS_415: &str = "415 Unsupported Media Type";
static STATUS_416: &str = "416 Range Not Satisfiable";
static STATUS_417: &str = "417 Expectation Failed";
static STATUS_418: &str = "418 I'm a teapot";
static STATUS_421: &str = "421 Misdirected Request";
static STATUS_422: &str = "422 Unprocessable Entity";
static STATUS_423: &str = "423 Locked";
static STATUS_424: &str = "424 Failed Dependency";
static STATUS_425: &str = "425 Too Early";
static STATUS_426: &str = "426 Upgrade Required";
static STATUS_428: &str = "428 Precondition Required";
static STATUS_429: &str = "429 Too Many Requests";
static STATUS_431: &str = "431 Request Header Fields Too Large";
static STATUS_451: &str = "451 Unavailable For Legal Reasons";

// 5xx codes
static STATUS_500: &str = "500 Internal Server Error";
static STATUS_501: &str = "501 Not Implemented";
static STATUS_502: &str = "502 Bad Gateway";
static STATUS_503: &str = "503 Service Unavailable";
static STATUS_504: &str = "504 Gateway Timeout";
static STATUS_505: &str = "505 HTTP Version Not Supported";
static STATUS_506: &str = "506 Variant Also Negotiates";
static STATUS_507: &str = "507 Insufficient Storage";
static STATUS_508: &str = "508 Loop Detected";
static STATUS_510: &str = "510 Not Extended";
static STATUS_511: &str = "511 Network Authentication Required";

// default
static STATUS_UNKNOWN: &str = "";


pub fn get_status_from_u16(status: u16) -> &'static str {
    return match status {
        // 1xx codes
        100 => STATUS_100,
        101 => STATUS_101,
        102 => STATUS_102,
        103 => STATUS_103,

        // 2xx codes
        200 => STATUS_200,
        201 => STATUS_201,
        202 => STATUS_202,
        203 => STATUS_203,
        204 => STATUS_204,
        205 => STATUS_205,
        206 => STATUS_206,
        207 => STATUS_207,
        208 => STATUS_208,
        226 => STATUS_226,

        // 3xx codes
        300 => STATUS_300,
        302 => STATUS_301,
        303 => STATUS_302,
        304 => STATUS_303,
        307 => STATUS_307,
        308 => STATUS_308,

        // 4xx codes
        400 => STATUS_400,
        401 => STATUS_401,
        402 => STATUS_402,
        403 => STATUS_403,
        404 => STATUS_404,
        405 => STATUS_405,
        406 => STATUS_406,
        407 => STATUS_407,
        408 => STATUS_408,
        409 => STATUS_409,
        410 => STATUS_410,
        411 => STATUS_411,
        412 => STATUS_412,
        413 => STATUS_413,
        414 => STATUS_414,
        415 => STATUS_415,
        416 => STATUS_416,
        417 => STATUS_417,
        418 => STATUS_418,
        421 => STATUS_421,
        422 => STATUS_422,
        423 => STATUS_423,
        424 => STATUS_424,
        425 => STATUS_425,
        426 => STATUS_426,
        428 => STATUS_428,
        429 => STATUS_429,
        431 => STATUS_431,
        451 => STATUS_451,

        // 5xx codes
        500 => STATUS_500,
        501 => STATUS_501,
        502 => STATUS_502,
        503 => STATUS_503,
        504 => STATUS_504,
        505 => STATUS_505,
        506 => STATUS_506,
        507 => STATUS_507,
        508 => STATUS_508,
        510 => STATUS_510,
        511 => STATUS_511,

        // Default
        _ => STATUS_UNKNOWN,
    }
}