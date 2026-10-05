//! 私密输入：密码框、浏览器无痕窗口这类应用声明「别记」的地方。壳判定（macOS 的 Secure Input 直接不组句；
//! Windows 的 `KEYBOARD_DISABLED` compartment 同样不组句，`IS_PRIVATE` / 密码类输入范围则照常组句但走这里），
//! Core 这一侧：不学习、不记输入日志、不发云端（联想 / 翻译 / 释义兜底）；排序仍用已有的个人数据。

use super::Engine;

impl Engine {
    /// 进入 / 离开私密输入。壳在焦点落到私密输入框（或离开它）时调；跨会话切换焦点时按各会话的状态重设。
    /// 此方法只设置写入开关，不清组句；已确认的隐私能力边界应先调用 [`Self::discard_input`]，
    /// 避免私密缓存被普通输入继续使用。恢复另一个独立会话时不需要丢弃该会话的输入。
    pub fn set_private(&mut self, private: bool) {
        if self.private == private {
            return;
        }
        self.private = private;
        self.learner.set_private(private);
        self.logger.set_muted(private);
        // 两侧都清除前文与暂存学习链，避免私密上屏内容在恢复后进入云请求或输入日志。
        self.history.clear();
        self.chain.reset();
        self.recent_commits.clear();
        self.recording.clear();
        self.passthrough_pending.clear();
        self.retype_snapshot = None;
        self.last_prediction_scope.clear();
        self.last_question_guess.clear();
        *self.neural_cache.borrow_mut() = super::rescoring::NeuralCache::default();
        *self.last_query.borrow_mut() = None;
        if private {
            self.predictor.cancel();
            self.gloss_filler.cancel();
            self.displayed.clear();
            // 在飞的云结果不能再显示，前文也不能留
            self.prediction_sequence += 1;
            self.rescoring_before = None;
        }
    }

    pub fn is_private(&self) -> bool {
        self.private
    }
}
