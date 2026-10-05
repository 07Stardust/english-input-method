use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use std::time::Duration;

use qingjian_core::{Prediction, PredictionPolicy, PredictionRequest, Predictor};

use crate::chat_client::ChatClient;
use crate::config::PredictConfig;
use crate::error::PredictError;
use crate::worker::Worker;

/// 走网络的联想器。构造时起一个后台线程，`submit` / `poll` 都只碰通道，不阻塞。
pub struct CloudPredictor {
    /// 观察窗口、条数上限与开关，来自配置。
    policy: PredictionPolicy,

    /// 往后台线程发请求。
    requests: Sender<(u64, PredictionRequest)>,

    epoch: Arc<AtomicU64>,

    /// 从后台线程收结果。
    responses: Receiver<Prediction>,
}

impl CloudPredictor {
    /// 没有密钥直接报错，让壳退回 [`qingjian_core::NoPredictor`] 并记日志。
    pub fn new(config: &PredictConfig) -> Result<Self, PredictError> {
        let api_key = config
            .resolve_api_key()
            .ok_or_else(|| PredictError::MissingApiKey(config.api_key_env.clone()))?;
        let client = ChatClient::new(config, api_key);
        let (requests, request_rx) = mpsc::channel();
        let (response_tx, responses) = mpsc::channel();
        let epoch = Arc::new(AtomicU64::new(0));
        let worker = Worker::new(
            request_rx,
            response_tx,
            client,
            Duration::from_millis(config.debounce_ms),
            epoch.clone(),
        );
        std::thread::Builder::new()
            .name("qingjian-predict".to_owned())
            .spawn(move || {
                if let Err(error) = worker.run() {
                    tracing::error!(%error, "联想线程退出");
                }
            })?;
        tracing::info!(
            base_url = %config.base_url,
            model = %config.model,
            lookback = config.lookback,
            lookahead = config.lookahead,
            "云联想已启用"
        );
        Ok(Self {
            policy: config.policy(),
            requests,
            responses,
            epoch,
        })
    }
}

impl Predictor for CloudPredictor {
    fn cancel(&mut self) {
        self.epoch.fetch_add(1, Ordering::AcqRel);
        while self.responses.try_recv().is_ok() {}
    }
    fn policy(&self) -> PredictionPolicy {
        self.policy
    }

    fn submit(&mut self, request: PredictionRequest) {
        if self
            .requests
            .send((self.epoch.load(Ordering::Acquire), request))
            .is_err()
        {
            tracing::warn!("联想线程已退出，请求被丢弃");
        }
    }

    fn poll(&mut self) -> Option<Prediction> {
        // Empty 与 Disconnected 都当没有：线程退出时已经记过日志
        self.responses.try_recv().ok()
    }
}

impl Drop for CloudPredictor {
    fn drop(&mut self) {
        self.cancel();
    }
}
