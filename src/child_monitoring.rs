//! Child-local, non-invasive monitoring for an awaited review or check.
//!
//! This state machine deliberately has no messaging, interruption, or
//! replacement operation. Liveness observations preserve the child's active
//! goal and plan; only a material terminal result emits one parent delta and
//! transitions the child out of its awaiting state.

use std::{io, process::Child};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum AwaitedGate {
    CodexReview,
    Check,
    Sentinel,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum GateOutcome {
    Passed,
    Blocked,
    Failed,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ParentDelta {
    pub gate: AwaitedGate,
    pub outcome: GateOutcome,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ObservationEffect {
    RetainActive,
    BoundReached,
    Terminal,
    Transitioned,
}

#[derive(Debug)]
pub struct ChildLocalMonitor {
    gate: AwaitedGate,
    observation_limit: usize,
    observations: usize,
    goal_active: bool,
    parent_delta_sent: bool,
}

impl ChildLocalMonitor {
    pub fn new(gate: AwaitedGate, observation_limit: usize) -> Result<Self, &'static str> {
        if observation_limit == 0 {
            return Err("observation limit must be positive");
        }

        Ok(Self {
            gate,
            observation_limit,
            observations: 0,
            goal_active: true,
            parent_delta_sent: false,
        })
    }

    /// Records a local liveness observation without changing the child state.
    pub fn observe_liveness(&mut self) -> ObservationEffect {
        if !self.goal_active {
            return ObservationEffect::Transitioned;
        }

        if self.observations == self.observation_limit {
            return ObservationEffect::BoundReached;
        }

        self.observations += 1;
        ObservationEffect::RetainActive
    }

    /// Checks child-process liveness without emitting a parent delta.
    pub fn observe_process_liveness(&mut self, child: &mut Child) -> io::Result<ObservationEffect> {
        if !self.goal_active {
            return Ok(ObservationEffect::Transitioned);
        }

        if child.try_wait()?.is_some() {
            return Ok(ObservationEffect::Terminal);
        }

        Ok(self.observe_liveness())
    }

    /// Transitions once on a material result and returns the sole parent delta.
    pub fn observe_terminal(&mut self, outcome: GateOutcome) -> Option<ParentDelta> {
        if !self.goal_active || self.parent_delta_sent {
            return None;
        }

        self.goal_active = false;
        self.parent_delta_sent = true;
        Some(ParentDelta {
            gate: self.gate,
            outcome,
        })
    }

    pub fn goal_is_active(&self) -> bool {
        self.goal_active
    }

    pub fn plan_is_awaiting(&self) -> bool {
        self.goal_active
    }
}
