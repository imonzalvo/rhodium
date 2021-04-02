use std::fmt;

// Http Protocols
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HttpProtocol {
    HTTP,
    HTTPS,
}

impl HttpProtocol {
    pub fn to_string(&self) -> &str {
        match &self {
            HttpProtocol::HTTP => "http",
            HttpProtocol::HTTPS => "https",
        }
    }
}

impl fmt::Display for HttpProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

// Used to configurate the Hyper server
#[derive(Debug, PartialEq, Eq)]
pub enum HttpProtocolConf {
    HTTP,
    HTTPS { cert_file: String, key_file: String },
}

impl HttpProtocolConf {
    pub fn to_string(&self) -> &str {
        match &self {
            HttpProtocolConf::HTTP => "http",
            HttpProtocolConf::HTTPS { .. } => "https",
        }
    }
}

impl Clone for HttpProtocolConf {
    fn clone(&self) -> HttpProtocolConf {
        match &self {
            HttpProtocolConf::HTTP => HttpProtocolConf::HTTP,
            HttpProtocolConf::HTTPS {
                cert_file,
                key_file,
            } => HttpProtocolConf::HTTPS {
                cert_file: cert_file.clone(),
                key_file: key_file.clone(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocols() {
        let http = HttpProtocol::HTTP;
        assert_eq!(http.to_string(), "http");

        let https = HttpProtocol::HTTPS;
        assert_eq!(https.to_string(), "https");

        let http = HttpProtocolConf::HTTP;
        assert_eq!(http.to_string(), "http");
        assert_eq!(http, http.clone());

        let https = HttpProtocolConf::HTTPS {
            cert_file: "".to_string(),
            key_file: "".to_string(),
        };
        assert_eq!(https.to_string(), "https");
        assert_eq!(https, https.clone());
    }
}
