use crate::protocol::{ClientMessage, SessionId};
pub struct ConnectionGuard {
    session: SessionId,

    opened: bool,
    closed: bool,
}

impl ConnectionGuard {
    pub fn new(session: SessionId) -> Self {
        Self {
            session,
            opened: false,
            closed: false,
        }
    }

    pub fn session(&self) -> Option<SessionId> {
        self.opened.then_some(self.session)
    }

    pub fn authorize(&mut self, message: &mut ClientMessage) -> bool {
        if self.closed {
            return false;
        }
        let closing = matches!(message, ClientMessage::CloseSession { .. });
        let opening = matches!(message, ClientMessage::OpenSession { protocol, .. } if *protocol == crate::protocol::PROTOCOL_VERSION);
        if matches!(message, ClientMessage::OpenSession { .. }) && (!opening || self.opened) {
            return false;
        }
        let id = match message {
            ClientMessage::OpenSession { session, .. }
            | ClientMessage::Key { session, .. }
            | ClientMessage::Commit { session }
            | ClientMessage::Poll { session }
            | ClientMessage::Surrounding { session, .. }
            | ClientMessage::Privacy { session, .. }
            | ClientMessage::Selection { session, .. }
            | ClientMessage::PositionCandidates { session, .. }
            | ClientMessage::HideCandidates { session }
            | ClientMessage::ModeChanged { session, .. }
            | ClientMessage::SyncMode { session }
            | ClientMessage::ImeSwitched { session }
            | ClientMessage::Indicator { session, .. }
            | ClientMessage::CloseSession { session } => session,
        };
        if opening {
            *id = self.session;
            self.opened = true;
            return true;
        }
        let allowed = self.opened && *id == self.session;
        if allowed && closing {
            self.closed = true;
        }
        allowed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_forgery_and_requires_open() {
        let mut guard = ConnectionGuard::new(SessionId(41));
        assert!(!guard.authorize(&mut ClientMessage::Poll {
            session: SessionId(41)
        }));
        let mut opening = ClientMessage::OpenSession {
            session: SessionId(1),
            app: None,
            protocol: crate::protocol::PROTOCOL_VERSION,
        };
        assert!(guard.authorize(&mut opening));
        assert!(guard.authorize(&mut ClientMessage::Poll {
            session: SessionId(41)
        }));
        assert!(!guard.authorize(&mut ClientMessage::Privacy {
            session: SessionId(42),
            private: false
        }));
        assert!(!guard.authorize(&mut opening));
        let mut other = ConnectionGuard::new(SessionId(42));
        assert!(!other.authorize(&mut ClientMessage::Poll {
            session: SessionId(41)
        }));
        assert!(guard.authorize(&mut ClientMessage::CloseSession {
            session: SessionId(41)
        }));
        assert!(!guard.authorize(&mut ClientMessage::Poll {
            session: SessionId(41)
        }));
    }
}
