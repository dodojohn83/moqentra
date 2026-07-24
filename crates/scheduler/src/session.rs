//! Persistent agent session and command ledger for HA control-plane replicas.
//!
//! An Agent session survives individual control-plane replica restarts: commands
//! are persisted before they are sent, acks are idempotent, and an agent
//! reconnecting to a different replica resumes from the last acknowledged
//! sequence without duplicating completed side effects.

use moqentra_types::{TenantId, UtcTimestamp};
use std::collections::{BTreeMap, VecDeque};

/// A command sent to an agent/worker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    pub id: String,
    /// Monotonic sequence number within the session.
    pub sequence: u64,
    pub payload: String,
    pub state: CommandState,
    pub created_at: UtcTimestamp,
    /// Set when the command was sent over a connection.
    pub sent_at: Option<UtcTimestamp>,
    /// Set when the agent acknowledged the command.
    pub acked_at: Option<UtcTimestamp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandState {
    Pending,
    Sent,
    Acked,
    Failed,
}

/// Persistent agent session used across control-plane replicas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSession {
    pub agent_id: String,
    pub tenant_id: TenantId,
    /// The current connection owner (control-plane replica id).
    pub connection_owner: String,
    /// Session epoch used to fence stale connections.
    pub epoch: u64,
    pub lease_expires_at: UtcTimestamp,
    pub capabilities: BTreeMap<String, String>,
    pub last_received_seq: u64,
    pub last_sent_seq: u64,
    pub last_acked_seq: u64,
    commands: VecDeque<Command>,
}

impl AgentSession {
    pub fn new(
        agent_id: impl Into<String>,
        tenant_id: TenantId,
        connection_owner: impl Into<String>,
        epoch: u64,
        capabilities: BTreeMap<String, String>,
        lease_ttl_seconds: u64,
    ) -> Self {
        let now = UtcTimestamp::now();
        let expires = now
            .add_std_duration(std::time::Duration::from_secs(lease_ttl_seconds))
            .unwrap_or(now);
        Self {
            agent_id: agent_id.into(),
            tenant_id,
            connection_owner: connection_owner.into(),
            epoch,
            lease_expires_at: expires,
            capabilities,
            last_received_seq: 0,
            last_sent_seq: 0,
            last_acked_seq: 0,
            commands: VecDeque::new(),
        }
    }

    /// Replace an existing connection with a newer one.  Stale epochs are
    /// rejected so an old control-plane replica cannot continue to send on
    /// behalf of this agent.
    pub fn replace_connection(
        &mut self,
        new_owner: impl Into<String>,
        new_epoch: u64,
        now: UtcTimestamp,
        lease_ttl_seconds: u64,
    ) -> Result<(), moqentra_types::Error> {
        if new_epoch <= self.epoch {
            return Err(moqentra_types::Error::conflict(
                "stale session epoch; connection rejected",
            ));
        }
        self.connection_owner = new_owner.into();
        self.epoch = new_epoch;
        self.lease_expires_at = now
            .add_std_duration(std::time::Duration::from_secs(lease_ttl_seconds))
            .unwrap_or(now);
        Ok(())
    }

    /// Refresh the lease of the current connection owner.
    pub fn heartbeat(
        &mut self,
        owner: &str,
        epoch: u64,
        now: UtcTimestamp,
        ttl_seconds: u64,
    ) -> Result<(), moqentra_types::Error> {
        if epoch < self.epoch {
            return Err(moqentra_types::Error::conflict("stale epoch"));
        }
        if self.connection_owner != owner {
            return Err(moqentra_types::Error::conflict("connection owner mismatch"));
        }
        self.lease_expires_at =
            now.add_std_duration(std::time::Duration::from_secs(ttl_seconds)).unwrap_or(now);
        Ok(())
    }

    pub fn is_lease_expired(&self, now: UtcTimestamp) -> bool {
        now >= self.lease_expires_at
    }

    /// Enqueue a command and persist it before it is sent.  Returns the
    /// sequence number assigned to the command.
    pub fn enqueue_command(&mut self, id: impl Into<String>, payload: impl Into<String>) -> u64 {
        self.last_sent_seq = self.last_sent_seq.saturating_add(1);
        let seq = self.last_sent_seq;
        self.commands.push_back(Command {
            id: id.into(),
            sequence: seq,
            payload: payload.into(),
            state: CommandState::Pending,
            created_at: UtcTimestamp::now(),
            sent_at: None,
            acked_at: None,
        });
        seq
    }

    /// Mark a command as sent over the current connection.
    pub fn mark_sent(
        &mut self,
        sequence: u64,
        now: UtcTimestamp,
    ) -> Result<(), moqentra_types::Error> {
        let cmd = self
            .commands
            .iter_mut()
            .find(|c| c.sequence == sequence)
            .ok_or_else(|| moqentra_types::Error::not_found("command"))?;
        if !matches!(cmd.state, CommandState::Pending) {
            return Err(moqentra_types::Error::conflict(
                "command already sent or terminal",
            ));
        }
        cmd.state = CommandState::Sent;
        cmd.sent_at = Some(now);
        Ok(())
    }

    /// Acknowledge a command by sequence.  Idempotent; updates the session
    /// resume cursor only when the new ack is larger than the previous one.
    pub fn ack(&mut self, sequence: u64, now: UtcTimestamp) -> Result<(), moqentra_types::Error> {
        let cmd = self
            .commands
            .iter_mut()
            .find(|c| c.sequence == sequence)
            .ok_or_else(|| moqentra_types::Error::not_found("command"))?;
        if matches!(cmd.state, CommandState::Acked) {
            return Ok(());
        }
        if matches!(cmd.state, CommandState::Failed) {
            return Err(moqentra_types::Error::conflict(
                "cannot ack a failed command",
            ));
        }
        cmd.state = CommandState::Acked;
        cmd.acked_at = Some(now);
        if sequence > self.last_acked_seq {
            self.last_acked_seq = sequence;
        }
        Ok(())
    }

    /// Resume after an agent reconnects.  Returns all commands with sequence
    /// numbers greater than `resume_seq` that have not yet been acknowledged,
    /// plus the next expected sequence so the agent can detect gaps.
    pub fn resume_commands(&self, resume_seq: u64) -> (Vec<Command>, u64) {
        let next = self.last_sent_seq.saturating_add(1);
        let pending: Vec<_> = self
            .commands
            .iter()
            .filter(|c| c.sequence > resume_seq && !matches!(c.state, CommandState::Acked))
            .cloned()
            .collect();
        (pending, next)
    }

    /// Mark a command as failed, leaving it in the persistent queue for retry
    /// on a new connection.
    pub fn fail(&mut self, sequence: u64, now: UtcTimestamp) -> Result<(), moqentra_types::Error> {
        let cmd = self
            .commands
            .iter_mut()
            .find(|c| c.sequence == sequence)
            .ok_or_else(|| moqentra_types::Error::not_found("command"))?;
        if matches!(cmd.state, CommandState::Acked) {
            return Err(moqentra_types::Error::conflict(
                "cannot fail an acked command",
            ));
        }
        cmd.state = CommandState::Failed;
        cmd.acked_at = Some(now);
        Ok(())
    }

    /// Trim acknowledged commands up to the current resume cursor.
    pub fn compact(&mut self) {
        while let Some(cmd) = self.commands.front() {
            if matches!(cmd.state, CommandState::Acked) && cmd.sequence <= self.last_acked_seq {
                self.commands.pop_front();
            } else {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moqentra_types::RandomIdGenerator;

    fn session() -> AgentSession {
        let g = RandomIdGenerator;
        AgentSession::new(
            "agent-1",
            TenantId::new_v7(&g),
            "cp-1",
            1,
            BTreeMap::new(),
            60,
        )
    }

    #[test]
    fn enqueue_and_ack_updates_cursor() {
        let mut s = session();
        let seq = s.enqueue_command("cmd-1", "start");
        s.mark_sent(seq, UtcTimestamp::now()).unwrap();
        s.ack(seq, UtcTimestamp::now()).unwrap();
        assert_eq!(s.last_acked_seq, seq);
    }

    #[test]
    fn ack_is_idempotent() {
        let mut s = session();
        let seq = s.enqueue_command("cmd-1", "start");
        s.mark_sent(seq, UtcTimestamp::now()).unwrap();
        s.ack(seq, UtcTimestamp::now()).unwrap();
        s.ack(seq, UtcTimestamp::now()).unwrap();
        assert_eq!(s.last_acked_seq, seq);
    }

    #[test]
    fn resume_returns_unacked_commands() {
        let mut s = session();
        let a = s.enqueue_command("cmd-a", "A");
        let b = s.enqueue_command("cmd-b", "B");
        s.mark_sent(a, UtcTimestamp::now()).unwrap();
        s.mark_sent(b, UtcTimestamp::now()).unwrap();
        s.ack(a, UtcTimestamp::now()).unwrap();
        let (pending, next) = s.resume_commands(0);
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].sequence, b);
        assert_eq!(next, 3);
    }

    #[test]
    fn stale_connection_rejected() {
        let mut s = session();
        assert!(s.replace_connection("cp-2", 0, UtcTimestamp::now(), 60).is_err());
        s.replace_connection("cp-2", 2, UtcTimestamp::now(), 60).unwrap();
        assert_eq!(s.connection_owner, "cp-2");
    }

    #[test]
    fn heartbeat_requires_owner_and_epoch() {
        let mut s = session();
        assert!(s.heartbeat("cp-1", 1, UtcTimestamp::now(), 60).is_ok());
        assert!(s.heartbeat("cp-2", 1, UtcTimestamp::now(), 60).is_err());
        assert!(s.heartbeat("cp-1", 0, UtcTimestamp::now(), 60).is_err());
    }

    #[test]
    fn compact_removes_acked_prefix() {
        let mut s = session();
        let a = s.enqueue_command("cmd-a", "A");
        let b = s.enqueue_command("cmd-b", "B");
        s.mark_sent(a, UtcTimestamp::now()).unwrap();
        s.mark_sent(b, UtcTimestamp::now()).unwrap();
        s.ack(a, UtcTimestamp::now()).unwrap();
        s.compact();
        let (pending, _) = s.resume_commands(0);
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].sequence, b);
    }
}
