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

use crossbeam_channel::Sender;
use futures::sync::oneshot;
use parking_lot::Mutex;
use std::sync::Arc;
use std::thread::JoinHandle;

#[derive(Debug)]
pub enum SignalSender {
    Future(oneshot::Sender<()>),
    Crossbeam(Sender<()>),
}

impl SignalSender {
    pub fn send(self) {
        match self {
            SignalSender::Crossbeam(tx) => {
                if let Err(e) = tx.send(()) {
                    eprintln!("handler signal send error {:?}", e);
                };
            }
            SignalSender::Future(tx) => {
                if let Err(e) = tx.send(()) {
                    eprintln!("handler signal send error {:?}", e);
                };
            }
        }
    }
}

#[derive(Debug)]
struct Handler<T> {
    signal: SignalSender,
    thread: JoinHandle<T>,
}

//the outer Option take ownership for `Arc::try_unwrap`
//the inner Option take ownership for `JoinHandle` or `oneshot::Sender`
#[derive(Clone, Debug)]
pub struct StopHandler<T> {
    inner: Option<Arc<Mutex<Option<Handler<T>>>>>,
}

impl<T> StopHandler<T> {
    pub fn new(signal: SignalSender, thread: JoinHandle<T>) -> StopHandler<T> {
        let handler = Handler { signal, thread };
        StopHandler {
            inner: Some(Arc::new(Mutex::new(Some(handler)))),
        }
    }

    pub fn try_send(&mut self) {
        let inner = self
            .inner
            .take()
            .expect("Stop signal can only be sent once");
        if let Ok(lock) = Arc::try_unwrap(inner) {
            let handler = lock.lock().take().expect("Handler can only be taken once");
            let Handler { signal, thread } = handler;
            signal.send();
            if let Err(e) = thread.join() {
                eprintln!("handler thread join error {:?}", e);
            };
        };
    }
}
