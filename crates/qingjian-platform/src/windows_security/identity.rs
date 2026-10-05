/// 内核查询得到的身份，不采用客户端发送的账户或会话号。
#[derive(Debug, PartialEq, Eq)]
pub struct Identity {
    pub sid: String,

    pub session: u32,
}
