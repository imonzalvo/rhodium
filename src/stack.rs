use super::*;
use crate::errors::{RhodError, RhodResult};
use crate::request::*;
use crate::response::*;
use async_trait::async_trait;

// A stack is a list of handlers/dynamic handlers and one service
pub struct RhodStack<C> {
    pub handlers: Vec<RhodHandlerInStack<C>>,
    pub service: Box<dyn RhodService<C>>,
}

impl<C> RhodStack<C> {
    pub fn new(
        handlers: Vec<RhodHandlerInStack<C>>,
        service: Box<dyn RhodService<C>>,
    ) -> RhodStack<C> {
        RhodStack { handlers, service }
    }
}

pub enum RhodHandlerInStack<C> {
    RhodHandler(Box<dyn RhodHandler<C>>),
    DynamicRhodHandler(Box<dyn DynamicRhodHandler<C>>),
}

//The generic type C refers to the type that will be used for communication between handlers and the service
#[async_trait]
pub trait RhodHandler<C>: Sync + Send {
    async fn handle_request(
        &self,
        conn: &RhodConnInfo,
        req: &mut RhodRequest,
        comm: &mut C,
    ) -> RhodResult<()>;
    async fn catch_request(
        &self,
        conn: &RhodConnInfo,
        req: &RhodRequest,
        err: &RhodError,
        comm: &C,
    );

    async fn handle_response(
        &self,
        conn: &RhodConnInfo,
        res: RhodResponse,
        comm: &mut C,
    ) -> (RhodResponse, RhodResult<()>);

    async fn catch_response(
        &self,
        conn: &RhodConnInfo,
        res: &RhodResponse,
        err: &RhodError,
        comm: &C,
    );
}

//Dynamic Handlers are handlers that are evaluated in runtime
#[async_trait]
pub trait DynamicRhodHandler<C>: Sync + Send {
    async fn get_handler<'a>(
        &'a self,
        conn: &RhodConnInfo,
        req: &RhodRequest,
        comm: &mut C,
    ) -> &'a dyn RhodHandler<C>;
}

#[async_trait]
pub trait RhodService<C>: Sync + Send {
    async fn serve(
        &self,
        conn: &RhodConnInfo,
        req: RhodRequest,
        comm: &mut C,
    ) -> RhodResult<RhodResponse>;
}
