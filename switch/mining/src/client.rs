// Copyright (C) 2019 Yee Foundation.
//
// This file is part of YeeChain.
//
// YeeChain is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// YeeChain is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with YeeChain.  If not, see <https://www.gnu.org/licenses/>.

use super::config::ClientConfig;
use crate::job_template::{Hash, JobResult, Job};
use yee_jsonrpc_types::{
    error::Error as RpcFail, id::Id, params::Params,
    request::MethodCall, response::Output, version::Version, };
use yee_stop_handler::{SignalSender, StopHandler};
use futures::sync::{mpsc, oneshot};
use hyper::error::Error as HyperError;
use hyper::header::{HeaderValue, CONTENT_TYPE};
use hyper::rt::{self, Future, Stream};
use hyper::Uri;
use hyper::{Body, Chunk, Client as HttpClient, Method, Request};
use serde_json::error::Error as JsonError;
use serde_json::{self, json, Value};
use std::thread;
use log::{info, error, warn, debug};

type RpcRequest = (oneshot::Sender<Result<Chunk, RpcError>>, MethodCall);

#[derive(Debug)]
pub enum RpcError {
    Http(HyperError),
    Canceled,
    //oneshot canceled
    Json(JsonError),
    Fail(RpcFail),
}

#[derive(Debug, Clone)]
pub struct Rpc {
    sender: mpsc::Sender<RpcRequest>,
    stop: StopHandler<()>,
}

impl Rpc {
    pub fn new(url: Uri) -> Rpc {
        let (sender, receiver) = mpsc::channel(65_535);
        let (stop, stop_rx) = oneshot::channel::<()>();
        let thread = thread::spawn(move || {
            let client = HttpClient::builder().keep_alive(true).build_http();
            let stream = receiver.for_each(move |(sender, call): RpcRequest| {
                let req_url = url.clone();
                let request_json = serde_json::to_vec(&call).expect("valid rpc call");
                let mut req = Request::new(Body::from(request_json));
                *req.method_mut() = Method::POST;
                *req.uri_mut() = req_url;
                req.headers_mut()
                    .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                let request = client
                    .request(req)
                    .and_then(|res| res.into_body().concat2())
                    .then(|res| sender.send(res.map_err(RpcError::Http)))
                    .map_err(|_| ());

                rt::spawn(request);
                Ok(())
            });

            rt::run(stream.select2(stop_rx).map(|_| ()).map_err(|_| ()));
        });

        Rpc {
            sender,
            stop: StopHandler::new(SignalSender::Future(stop), thread),
        }
    }

    pub fn request(
        &self,
        method: String,
        params: Vec<Value>,
    ) -> impl Future<Item=Output, Error=RpcError> {
        let (tx, rev) = oneshot::channel();

        let call = MethodCall {
            method,
            params: Params::Array(params),
            jsonrpc: Some(Version::V2),
            id: Id::Num(0),
        };
        let req = (tx, call);
        let mut sender = self.sender.clone();
        let _ = sender.try_send(req);
        rev.map_err(|_| RpcError::Canceled)
            .flatten()
            .and_then(|chunk| serde_json::from_slice(&chunk).map_err(RpcError::Json))
    }
}

impl Drop for Rpc {
    fn drop(&mut self) {
        self.stop.try_send();
    }
}

#[derive(Debug, Clone)]
pub struct Client {
    pub config: ClientConfig,
}

impl Client {
    pub fn new(config: ClientConfig) -> Client {
        Client {
            config,
        }
    }

    fn send_submit_job_request(
        &self,
        job: &JobResult,
        rpc: Rpc,
    ) -> impl Future<Item=Output, Error=RpcError> {
        let method = "mining_submitJob".to_owned();
        let params = vec![json!(job)];
        rpc.request(method, params)
    }
    pub fn submit_job(&self, job: &JobResult, rpc: Rpc) {
        let future = self.send_submit_job_request(job, rpc);
        if self.config.job_on_submit {
            let ret: Result<Option<Hash>, RpcError> = future.and_then(parse_response).wait();
            match ret {
                Ok(hash) => {
                    info!("Submit_job return Hash:{:?}", hash);
                    if hash.is_none() {
                        warn!(
                            "submit_job failed {}",
                            serde_json::to_string(job).unwrap()
                        );
                    }
                }
                Err(e) => {
                    error!("rpc call submit_job error: {:?}", e);
                }
            }
        }
    }

    pub fn get_job_template(&self, rpc: Rpc) -> impl Future<Item=Job, Error=RpcError> {
        let method = "mining_getJob".to_owned();
        let params = vec![];
        rpc.request(method, params).and_then(parse_response)
    }
}

fn parse_response<T: serde::de::DeserializeOwned>(output: Output) -> Result<T, RpcError> {
    match output {
        Output::Success(success) => {
            serde_json::from_value::<T>(success.result).map_err(RpcError::Json)
        }
        Output::Failure(failure) => Err(RpcError::Fail(failure.error)),
    }
}

