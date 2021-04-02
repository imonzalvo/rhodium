use crate::errors::*;
use hyper::body::Body as HyperBody;
use hyper::http::Request as HyperRequest;
use hyper::{header::HeaderValue, HeaderMap, Method, Uri, Version};

#[derive(Debug, PartialEq, Eq)]
pub enum BodyProcessor {
    URLENCODED,
    XML,
    JSON,
    MULTIPART,
    Other,
}

// Extends HyperRequest
#[derive(Debug)]
pub struct RhodRequest {
    req: Option<HyperRequest<HyperBody>>, // Is allways Some(..)
}

impl RhodRequest {
    pub fn new(req: HyperRequest<HyperBody>) -> RhodRequest {
        RhodRequest { req: Some(req) }
    }

    pub fn uri(&self) -> &Uri {
        self.req.as_ref().unwrap().uri()
    }

    pub fn uri_mut(&mut self) -> &mut Uri {
        self.req.as_mut().unwrap().uri_mut()
    }

    pub fn method(&self) -> &Method {
        self.req.as_ref().unwrap().method()
    }

    pub fn is_post(&self) -> bool {
        self.method() == Method::POST
    }

    pub fn version(&self) -> Version {
        self.req.as_ref().unwrap().version()
    }

    pub fn version_mut(&mut self) -> &mut Version {
        self.req.as_mut().unwrap().version_mut()
    }

    pub fn headers(&self) -> &HeaderMap<HeaderValue> {
        self.req.as_ref().unwrap().headers()
    }

    pub fn headers_mut(&mut self) -> &mut HeaderMap<HeaderValue> {
        self.req.as_mut().unwrap().headers_mut()
    }

    pub async fn body(&mut self) -> RhodResult<Vec<u8>> {
        let r = self.req.take().unwrap();

        let (header, body) = r.into_parts();
        match hyper::body::to_bytes(body).await {
            Ok(b) => {
                let cloned = b.clone();
                self.req = Some(HyperRequest::from_parts(header, HyperBody::from(b)));
                Ok(cloned.to_vec())
            }
            Err(e) => {
                // If error, body cant be recovered.
                self.req = Some(HyperRequest::from_parts(header, HyperBody::empty()));

                Err(RhodError::from_string(
                    format!("Cant parse request body to bytes. {}", e),
                    RhodErrorLevel::Error,
                ))
            }
        }
    }

    pub fn body_processor(&self) -> Option<BodyProcessor> {
        match self.headers().get("Content-Type") {
            Some(c) => {
                let value = c.to_str();
                if let Ok(value) = value {
                    let value = value.to_lowercase();
                    if value.contains("application/x-www-form-urlencoded") {
                        Some(BodyProcessor::URLENCODED)
                    } else if value.contains("application/xml") {
                        Some(BodyProcessor::XML)
                    } else if value.contains("application/json") {
                        Some(BodyProcessor::JSON)
                    } else if value.contains("multipart/form-data") {
                        Some(BodyProcessor::MULTIPART)
                    } else {
                        Some(BodyProcessor::Other)
                    }
                } else {
                    None
                }
            }
            None => None,
        }
    }

    pub fn method_str(&self) -> &str {
        self.method().as_str()
    }
    pub fn version_string(&self) -> String {
        format!("{:?}", self.version())
    }

    pub fn request_line(&self) -> String {
        let method = self.method_str();
        let path = self.req.as_ref().unwrap().uri().path();
        let version = self.version_string();
        format!("{} {} {}", method, path, &version)
    }

    pub fn into_hyper_request(self) -> HyperRequest<HyperBody> {
        self.req.unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_uri() {
        let query = "par=value&par=value";
        let fragment = "2";
        let uri = format!("https://www.rust-lang.org/token?{}#{}", query, fragment);
        let mut request = RhodRequest::new(
            HyperRequest::builder()
                .uri(uri)
                .body(HyperBody::empty())
                .unwrap(),
        );

        //asserts
        assert_eq!(request.uri().query(), Some(query));
        assert_eq!(request.uri().host(), Some("www.rust-lang.org"));
        assert_eq!(request.uri_mut().query(), Some(query));
        assert_eq!(request.uri_mut().host(), Some("www.rust-lang.org"));
    }

    #[test]
    fn test_method() {
        let request = RhodRequest::new(
            HyperRequest::builder()
                .uri("https://www.rust-lang.org/")
                .body(HyperBody::empty())
                .unwrap(),
        );
        assert_eq!(request.method(), Method::GET);
        assert!(!request.is_post());

        let request = RhodRequest::new(
            HyperRequest::post("https://www.rust-lang.org/")
                .body(HyperBody::empty())
                .unwrap(),
        );
        assert_eq!(request.method(), Method::POST);
        assert!(request.is_post());
    }

    #[test]
    fn test_version() {
        let mut request = RhodRequest::new(
            HyperRequest::builder()
                .version(Version::HTTP_2)
                .uri("https://www.rust-lang.org/")
                .body(HyperBody::empty())
                .unwrap(),
        );
        assert_eq!(request.version(), Version::HTTP_2);
        *request.version_mut() = Version::HTTP_11;
        assert_eq!(request.version(), Version::HTTP_11);
    }

    #[test]
    fn test_headers() {
        let mut request = RhodRequest::new(
            HyperRequest::builder()
                .uri("https://www.rust.rs/")
                .header("User-Agent", "my-awesome-agent/1.0")
                .body(HyperBody::empty())
                .unwrap(),
        );

        assert_eq!(
            request.headers().get("User-Agent").unwrap(),
            "my-awesome-agent/1.0"
        );
        assert!(request.headers().get("Accept").is_none());

        request
            .headers_mut()
            .insert("Accept", "text/html".parse().unwrap());
        assert_eq!(request.headers().get("Accept").unwrap(), "text/html");
    }

    #[tokio::test]
    async fn test_body() {
        let mut request = RhodRequest::new(
            HyperRequest::builder()
                .uri("https://www.rust.rs/")
                .header("User-Agent", "my-awesome-agent/1.0")
                .body(HyperBody::from("key1=value1&key2=value2"))
                .unwrap(),
        );

        assert_eq!(
            request.body().await.unwrap(),
            "key1=value1&key2=value2".as_bytes().to_vec()
        );

        let mut request = RhodRequest::new(
            HyperRequest::builder()
                .uri("https://www.rust.rs/")
                .header("User-Agent", "my-awesome-agent/1.0")
                .body(HyperBody::empty())
                .unwrap(),
        );

        assert_eq!(request.body().await.unwrap(), [])
    }

    #[test]
    fn test_body_processor() {
        let mut request = RhodRequest::new(
            HyperRequest::builder()
                .uri("https://www.rust.rs/")
                .header("User-Agent", "my-awesome-agent/1.0")
                .body(HyperBody::empty())
                .unwrap(),
        );

        assert!(request.body_processor().is_none());

        request.headers_mut().insert(
            "Content-Type",
            "application/X-WWW-Form-Urlencoded".parse().unwrap(),
        );
        assert_eq!(request.body_processor().unwrap(), BodyProcessor::URLENCODED);

        request
            .headers_mut()
            .insert("content-type", "application/xml".parse().unwrap());
        assert_eq!(request.body_processor().unwrap(), BodyProcessor::XML);

        request
            .headers_mut()
            .insert("content-type", "application/json".parse().unwrap());
        assert_eq!(request.body_processor().unwrap(), BodyProcessor::JSON);

        request
            .headers_mut()
            .insert("content-type", "multipart/form-data".parse().unwrap());
        assert_eq!(request.body_processor().unwrap(), BodyProcessor::MULTIPART);

        request
            .headers_mut()
            .insert("content-type", "idk".parse().unwrap());
        assert_eq!(request.body_processor().unwrap(), BodyProcessor::Other);
    }

    #[test]
    fn test_request_line() {
        let request = RhodRequest::new(
            HyperRequest::builder()
                .version(Version::HTTP_2)
                .uri("https://www.rust-lang.org/folder/file.txt?query=val#2")
                .body(HyperBody::empty())
                .unwrap(),
        );

        assert_eq!(request.method_str(), "GET");
        assert_eq!(request.version_string(), "HTTP/2.0".to_string());
        assert_eq!(
            request.request_line(),
            "GET /folder/file.txt HTTP/2.0".to_string()
        );
    }
}
