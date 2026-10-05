use std::collections::HashSet;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use std::time::{Duration, Instant};

use qingjian_core::{FilledGloss, Language};

use super::prompt;
use crate::chat_client::ChatClient;
use crate::error::PredictError;

/// 收到第一个词后最多等这么久再发，攒同一批。
pub const BATCH_WAIT: Duration = Duration::from_millis(1500);

/// 一批最多几个词。
pub const BATCH_SIZE: usize = 8;

/// 回复的 token 上限：8 个词各两条短译词绰绰有余。
const MAX_TOKENS: u32 = 600;

/// 后台线程：攒词、发请求、回释义。
pub struct GlossWorker {
    /// 请求入口。主线程 drop 掉发送端后线程自然退出。
    requests: Receiver<(u64, Language, String)>,

    epoch: Arc<AtomicU64>,

    /// 结果出口。
    responses: Sender<FilledGloss>,

    /// 网络客户端。
    client: ChatClient,

    /// 本进程内问过的 (语言, 词)，不再问。
    asked: HashSet<(u64, Language, String)>,
    pending: Option<(u64, Language, String)>,
}

impl GlossWorker {
    pub fn new(
        requests: Receiver<(u64, Language, String)>,
        responses: Sender<FilledGloss>,
        client: ChatClient,
        epoch: Arc<AtomicU64>,
    ) -> Self {
        Self {
            requests,
            responses,
            client,
            epoch,
            asked: HashSet::new(),
            pending: None,
        }
    }

    /// 阻塞运行直到发送端全部关闭。
    pub fn run(mut self) -> Result<(), PredictError> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        while let Some(first) = self.pending.take().or_else(|| self.requests.recv().ok()) {
            let epoch = first.0;
            if epoch != self.epoch.load(Ordering::Acquire) {
                continue;
            }
            let Some(batch) = self.collect(first) else {
                return Ok(());
            };
            // 一批里可能混着两种语言（中途切了学习语言）：按语言各发一次
            let mut languages: Vec<Language> = Vec::new();
            for (language, _) in &batch {
                if !languages.contains(language) {
                    languages.push(*language);
                }
            }
            for language in languages {
                let words: Vec<String> = batch
                    .iter()
                    .filter(|(l, _)| *l == language)
                    .map(|(_, w)| w.clone())
                    .collect();
                self.ask(&runtime, epoch, language, words);
            }
        }
        Ok(())
    }

    /// 从第一个词起攒一批：等到 [`BATCH_WAIT`] 或攒够 [`BATCH_SIZE`]；问过的跳过。发送端关闭返回 `None`。
    fn collect(&mut self, first: (u64, Language, String)) -> Option<Vec<(Language, String)>> {
        let epoch = first.0;
        self.asked.retain(|item| item.0 == epoch);
        let deadline = Instant::now() + BATCH_WAIT;
        let mut batch = Vec::with_capacity(BATCH_SIZE);
        self.take(&mut batch, first);
        while batch.len() < BATCH_SIZE {
            let remaining = deadline.saturating_duration_since(Instant::now());
            match self.requests.recv_timeout(remaining) {
                Ok(item) if item.0 != epoch => {
                    self.pending = Some(item);
                    break;
                }
                Ok(item) => self.take(&mut batch, item),
                Err(RecvTimeoutError::Timeout) => break,
                Err(RecvTimeoutError::Disconnected) => return None,
            }
        }
        Some(batch)
    }

    fn take(&mut self, batch: &mut Vec<(Language, String)>, item: (u64, Language, String)) {
        if item.0 == self.epoch.load(Ordering::Acquire) && self.asked.insert(item.clone()) {
            batch.push((item.1, item.2));
        }
    }

    fn ask(
        &self,
        runtime: &tokio::runtime::Runtime,
        epoch: u64,
        language: Language,
        words: Vec<String>,
    ) {
        if words.is_empty() {
            return;
        }
        let start = Instant::now();
        let system = prompt::system_prompt(language);
        let user = prompt::user_prompt(&words);
        match runtime.block_on(crate::cancellation::run(
            &self.epoch,
            epoch,
            self.client.chat(system, &user, MAX_TOKENS),
        )) {
            Some(Ok(content)) => {
                let filled = prompt::parse_reply(&content, language, &words);
                tracing::info!(
                    language = language.code(),
                    asked = words.len(),
                    filled = filled.len(),
                    elapsed_ms = start.elapsed().as_millis(),
                    "释义兜底完成"
                );
                for gloss in filled {
                    // 接收端没了说明 Filler 已经被 drop，线程随后也会退出
                    if self.responses.send(gloss).is_err() {
                        return;
                    }
                }
            }
            Some(Err(_)) => {
                tracing::warn!(
                    language = language.code(),
                    asked = words.len(),
                    "释义兜底失败"
                );
            }
            None => {}
        }
    }
}
