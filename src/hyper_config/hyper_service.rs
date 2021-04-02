use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Context;
use std::task::Poll;

use hyper;
use hyper::body::Body as HyperBody;
use hyper::http::Request as HyperRequest;
use hyper::http::Response as HyperResponse;
use hyper::service::Service as HyperService;

use crate::CommunicationChannel;
use crate::{errors::RhodError, RhodConnInfo, RhodHandlerInStack, RhodRequest, RhodStack};

type SecureFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

pub struct RhodHyperService<C> {
    stack: Arc<RhodStack<C>>,
    conn: RhodConnInfo,
}

impl<C> RhodHyperService<C> {
    pub fn new(stack: Arc<RhodStack<C>>, conn: RhodConnInfo) -> RhodHyperService<C> {
        RhodHyperService { stack, conn }
    }
}

impl<C: CommunicationChannel> HyperService<HyperRequest<HyperBody>> for RhodHyperService<C> {
    type Response = HyperResponse<HyperBody>;
    type Error = RhodError;
    type Future = SecureFuture<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, h_req: HyperRequest<HyperBody>) -> Self::Future {
        let stack = Arc::clone(&self.stack);
        let conn = self.conn.clone();
        Box::pin(async move {
            let mut req = RhodRequest::new(h_req);
            let mut err = None;

            let mut dyn_handlers = vec![];
            let mut counter: usize = 0;

            let mut communication = C::new();

            // call handle_request from handlers in order:
            for handler in stack.handlers.iter() {
                let handler = match handler {
                    // if is dynamic handler, gets it and saves in dyn handlers array
                    RhodHandlerInStack::DynamicRhodHandler(dyn_handler) => {
                        let aux = dyn_handler
                            .get_handler(&conn, &req, &mut communication)
                            .await;
                        dyn_handlers.push(aux);
                        counter += 1;
                        aux
                    }
                    RhodHandlerInStack::RhodHandler(handler) => &**handler,
                };

                match &err {
                    None => match handler
                        .handle_request(&conn, &mut req, &mut communication)
                        .await
                    {
                        Ok(()) => (),
                        Err(e) => {
                            e.log();
                            err = Some(e);
                        }
                    },
                    Some(e) => {
                        handler.catch_request(&conn, &req, e, &communication).await;
                    }
                }
            }

            if let Some(e) = err {
                return Err(e);
            }

            // call rhodium service:
            match stack.service.serve(&conn, req, &mut communication).await {
                Ok(mut res) => {
                    // call handle_response from handlers in reverse order:
                    for handler in stack.handlers.iter().rev() {
                        // if handler is dynamic, gets the handler from dyn handlers array
                        let handler = match handler {
                            RhodHandlerInStack::DynamicRhodHandler(_) => {
                                counter -= 1;
                                dyn_handlers[counter]
                            }
                            RhodHandlerInStack::RhodHandler(handler) => &**handler,
                        };

                        match &err {
                            None => match handler
                                .handle_response(&conn, res, &mut communication)
                                .await
                            {
                                (new_res, Ok(())) => res = new_res,
                                (new_res, Err(e)) => {
                                    res = new_res;
                                    e.log();
                                    err = Some(e);
                                }
                            },
                            Some(e) => {
                                handler.catch_response(&conn, &res, e, &communication).await;
                            }
                        }
                    }

                    if let Some(e) = err {
                        return Err(e);
                    }

                    Ok(res.into_hyper_response())
                }
                Err(e) => {
                    e.log();
                    Err(e)
                }
            }
        })
    }
}

// struct HyperServiceFactory {
//     stack: Arc<RhodStack>,
// }

// impl HyperServiceFactory {
//     fn new(stack: Arc<RhodStack>) -> HyperServiceFactory {
//         HyperServiceFactory { stack }
//     }
// }

// impl<T> HyperService<T> for HyperServiceFactory {
//     type Response = RhodHyperService;
//     type Error = RhodError;
//     type Future = future::Ready<Result<Self::Response, Self::Error>>;

//     fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
//         Ok(()).into()
//     }

//     fn call(&mut self, _: T) -> Self::Future {
//         future::ok(RhodHyperService::new(Arc::clone(&self.stack)))
//     }
// }
