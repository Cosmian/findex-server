use std::{
    pin::Pin,
    rc::Rc,
    sync::Arc,
    task::{Context, Poll},
};

use actix_service::{Service, Transform};
use actix_web::{
    body::{BoxBody, EitherBody},
    dev::{ServiceRequest, ServiceResponse},
    Error,
};
use futures::{
    future::{ok, Ready},
    Future,
};

use super::{jwt_token_auth::manage_jwt_request, JwtConfig};


#[derive(Clone)]
pub struct AuthTransformer {
    jwt_configurations: Option<Arc<Vec<JwtConfig>>>,
}

impl AuthTransformer {
    #[must_use]
    pub(crate) const fn new(
        jwt_configurations: Option<Arc<Vec<JwtConfig>>>,
    ) -> Self {
        Self {
            jwt_configurations,
        }
    }
}

impl<S, B> Transform<S, ServiceRequest> for AuthTransformer
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
{
    type Error = Error;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;
    type InitError = ();
    type Response = ServiceResponse<EitherBody<B, BoxBody>>;
    type Transform = AuthMiddleware<S>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(AuthMiddleware {
            service: Rc::new(service),
            jwt_configurations: self.jwt_configurations.clone(),
        })
    }
}

pub(crate) struct AuthMiddleware<S> {
    service: Rc<S>,
    jwt_configurations: Option<Arc<Vec<JwtConfig>>>,
}

impl<S, B> Service<ServiceRequest> for AuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
{
    type Error = Error;
    #[allow(clippy::type_complexity)]
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;
    type Response = ServiceResponse<EitherBody<B, BoxBody>>;

    fn poll_ready(&self, ctx: &mut Context) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(ctx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();

        // TODO : implement this
        // if req.extensions().contains::<PeerCommonName>() {
        //     debug!(
        //         "Request extension PeerCommonName found! Certificate client authentication has \
        //          already been done in success, no need to authenticate twice..."
        //     );
        //     return Box::pin(async move {
        //         let res = service.call(req).await?;
        //         Ok(res.map_into_left_body())
        //     });
        // }

        /*
         * There is a JWT config, treat the request as a jwt auth request
         */
        if let Some(configurations) = self.jwt_configurations.clone() {
            return Box::pin(async move { manage_jwt_request(service, configurations, req).await });
        }
        
        todo!("TODO: NOT IMPLEMENTED TOKEN AUTH")
        // Box::pin(async move { manage_api_token_request(service,  req).await })
    }
}
