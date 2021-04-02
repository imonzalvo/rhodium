use crate::errors::*;
use hyper::body::Body as HyperBody;
use hyper::http::Response as HyperResponse;
use hyper::{header::HeaderValue, HeaderMap};

// Extends HyperResponse
pub struct RhodResponse {
    res: Option<HyperResponse<HyperBody>>, // Is allways Some(..)
}

impl RhodResponse {
    pub fn new(res: HyperResponse<HyperBody>) -> RhodResponse {
        RhodResponse { res: Some(res) }
    }

    pub fn headers(&self) -> &HeaderMap<HeaderValue> {
        self.res.as_ref().unwrap().headers()
    }

    pub fn headers_mut(&mut self) -> &mut HeaderMap<HeaderValue> {
        self.res.as_mut().unwrap().headers_mut()
    }

    pub fn into_hyper_response(self) -> HyperResponse<HyperBody> {
        self.res.unwrap()
    }

    pub fn status_as_int(&self) -> u16 {
        self.res.as_ref().unwrap().status().as_u16()
    }

    pub async fn body(&mut self) -> RhodResult<Vec<u8>> {
        let r = self.res.take().unwrap();

        let (header, body) = r.into_parts();
        match hyper::body::to_bytes(body).await {
            Ok(b) => {
                let cloned = b.clone();
                self.res = Some(HyperResponse::from_parts(header, HyperBody::from(b)));
                Ok(cloned.to_vec())
            }
            Err(e) => {
                // If error, body cant be recovered.
                self.res = Some(HyperResponse::from_parts(header, HyperBody::empty()));

                Err(RhodError::from_string(
                    format!("Cant parse response body to bytes. {}", e),
                    RhodErrorLevel::Error,
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_headers() {
        let mut res = RhodResponse::new(
            HyperResponse::builder()
                .header("Foo", "Bar")
                .body(HyperBody::empty())
                .unwrap(),
        );

        assert_eq!(res.headers().get("Foo").unwrap(), "Bar");
        assert!(res.headers().get("Accept").is_none());

        res.headers_mut()
            .insert("Accept", "text/html".parse().unwrap());
        assert_eq!(res.headers().get("Accept").unwrap(), "text/html");
    }

    #[test]
    fn test_status() {
        let res = RhodResponse::new(
            HyperResponse::builder()
                .status(404)
                .body(HyperBody::empty())
                .unwrap(),
        );

        assert_eq!(res.status_as_int(), 404);
    }

    #[tokio::test]
    async fn test_body() {
        let mut response = RhodResponse::new(
            HyperResponse::builder()
                .body(HyperBody::from("response bodyy %% #"))
                .unwrap(),
        );

        assert_eq!(
            response.body().await.unwrap(),
            "response bodyy %% #".as_bytes().to_vec()
        );

        let mut response =
            RhodResponse::new(HyperResponse::builder().body(HyperBody::empty()).unwrap());

        assert_eq!(response.body().await.unwrap(), [])
    }
}
