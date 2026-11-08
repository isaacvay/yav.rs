//! Finite State Machines (XState equivalent) & Durable Saga Workflows (`yav::state_machine`).
//!
//! Provides deterministic, type-safe state orchestration for complex UIs and backend workflows:
//! - [`StateMachine<S, E, C>`]: Statechart with guards, transition actions, history tracking, and `.can(&event)` UI queries.
//! - [`SagaWorkflow<C>`]: Multi-step transactional workflow runner with automatic reverse-order compensation (rollback) on failure.

use std::fmt;

type GuardFn<C, E> = Box<dyn Fn(&C, &E) -> bool + Send + Sync>;
type ActionFn<C, E> = Box<dyn Fn(&mut C, &E) + Send + Sync>;

struct TransitionRule<S, E, C> {
    from: S,
    event: E,
    to: S,
    guard: Option<GuardFn<C, E>>,
    action: Option<ActionFn<C, E>>,
}

/// Deterministic Finite State Machine / Statechart (XState equivalent).
///
/// Parameterized over:
/// - `S`: State enum (`Clone + PartialEq + Eq`)
/// - `E`: Event enum (`Clone + PartialEq + Eq`)
/// - `C`: Extended state context (`Clone`)
pub struct StateMachine<S, E, C = ()> {
    current: S,
    context: C,
    rules: Vec<TransitionRule<S, E, C>>,
    history: Vec<S>,
}

impl<S: fmt::Debug, E: fmt::Debug, C: fmt::Debug> fmt::Debug for StateMachine<S, E, C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StateMachine")
            .field("current", &self.current)
            .field("context", &self.context)
            .field("history_len", &self.history.len())
            .finish()
    }
}

impl<S, E, C> StateMachine<S, E, C>
where
    S: Clone + PartialEq + Eq,
    E: Clone + PartialEq + Eq,
{
    /// Create a new state machine starting in `initial_state` with `initial_context`.
    pub fn new(initial_state: S, initial_context: C) -> Self {
        Self {
            history: vec![initial_state.clone()],
            current: initial_state,
            context: initial_context,
            rules: Vec::new(),
        }
    }

    /// Register a direct transition `from --(event)--> to`.
    #[must_use]
    pub fn on(mut self, from: S, event: E, to: S) -> Self {
        self.rules.push(TransitionRule {
            from,
            event,
            to,
            guard: None,
            action: None,
        });
        self
    }

    /// Register a guarded transition with a predicate `guard(&context, &event)` and side-effect `action(&mut context, &event)`.
    #[must_use]
    pub fn on_guarded<G, A>(mut self, from: S, event: E, to: S, guard: G, action: A) -> Self
    where
        G: Fn(&C, &E) -> bool + Send + Sync + 'static,
        A: Fn(&mut C, &E) + Send + Sync + 'static,
    {
        self.rules.push(TransitionRule {
            from,
            event,
            to,
            guard: Some(Box::new(guard)),
            action: Some(Box::new(action)),
        });
        self
    }

    /// Return a reference to the current active state.
    pub fn state(&self) -> &S {
        &self.current
    }

    /// Return a reference to the extended context.
    pub fn context(&self) -> &C {
        &self.context
    }

    /// Return a mutable reference to the extended context.
    pub fn context_mut(&mut self) -> &mut C {
        &mut self.context
    }

    /// Return the chronological history of visited states.
    pub fn history(&self) -> &[S] {
        &self.history
    }

    /// Check whether `event` can validly transition from the current state (evaluating any guards).
    ///
    /// Ideal for disabling/enabling buttons in `rsx!`: `disabled={!machine.can(&CheckoutEvent::Pay)}`.
    pub fn can(&self, event: &E) -> bool {
        self.rules.iter().any(|r| {
            r.from == self.current
                && &r.event == event
                && r.guard.as_ref().map_or(true, |g| g(&self.context, event))
        })
    }

    /// Dispatch `event` to the state machine. Returns `Ok(&S)` with the new state if a matching
    /// transition fired, or `Err(&S)` with the unchanged state if no transition matched.
    pub fn send(&mut self, event: E) -> Result<&S, &S> {
        for rule in &self.rules {
            if rule.from == self.current && rule.event == event {
                if let Some(ref g) = rule.guard {
                    if !g(&self.context, &event) {
                        continue;
                    }
                }
                if let Some(ref act) = rule.action {
                    act(&mut self.context, &event);
                }
                self.current = rule.to.clone();
                self.history.push(self.current.clone());
                return Ok(&self.current);
            }
        }
        Err(&self.current)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. SagaWorkflow (Durable Multi-Step Execution with Compensating Rollback)
// ─────────────────────────────────────────────────────────────────────────────

type StepExecFn<C> = Box<dyn Fn(&mut C) -> Result<(), String> + Send + Sync>;
type StepCompensateFn<C> = Box<dyn Fn(&mut C) + Send + Sync>;

struct SagaStep<C> {
    name: String,
    execute: StepExecFn<C>,
    compensate: StepCompensateFn<C>,
}

/// Outcome of running a [`SagaWorkflow`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SagaOutcome {
    /// All steps succeeded in order.
    Completed { executed_steps: Vec<String> },
    /// A step failed and all prior completed steps were compensated in reverse order.
    RolledBack {
        failed_step: String,
        error: String,
        compensated_steps: Vec<String>,
    },
}

/// Transactional Saga workflow orchestrator (Temporal / Restate pattern).
///
/// Executes a sequence of steps over mutable context `C`. If any step fails, all previously
/// completed steps are automatically rolled back in reverse order via their compensation closures.
pub struct SagaWorkflow<C> {
    steps: Vec<SagaStep<C>>,
}

impl<C> Default for SagaWorkflow<C> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C> SagaWorkflow<C> {
    /// Create a new empty [`SagaWorkflow`].
    pub fn new() -> Self {
        Self { steps: Vec::new() }
    }

    /// Add a forward step and its compensating rollback action.
    #[must_use]
    pub fn step<E, R>(mut self, name: impl Into<String>, execute: E, compensate: R) -> Self
    where
        E: Fn(&mut C) -> Result<(), String> + Send + Sync + 'static,
        R: Fn(&mut C) + Send + Sync + 'static,
    {
        self.steps.push(SagaStep {
            name: name.into(),
            execute: Box::new(execute),
            compensate: Box::new(compensate),
        });
        self
    }

    /// Execute the saga workflow against `context`.
    pub fn run(&self, context: &mut C) -> SagaOutcome {
        let mut completed_indices = Vec::with_capacity(self.steps.len());

        for (idx, step) in self.steps.iter().enumerate() {
            match (step.execute)(context) {
                Ok(()) => {
                    completed_indices.push(idx);
                }
                Err(err) => {
                    let mut compensated = Vec::new();
                    for &done_idx in completed_indices.iter().rev() {
                        let prior = &self.steps[done_idx];
                        (prior.compensate)(context);
                        compensated.push(prior.name.clone());
                    }
                    return SagaOutcome::RolledBack {
                        failed_step: step.name.clone(),
                        error: err,
                        compensated_steps: compensated,
                    };
                }
            }
        }

        SagaOutcome::Completed {
            executed_steps: self.steps.iter().map(|s| s.name.clone()).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum OrderState {
        Draft,
        Paid,
        Shipped,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum OrderEvent {
        Pay,
        Ship,
    }

    #[test]
    fn test_state_machine_guards_and_transitions() {
        let mut machine = StateMachine::new(OrderState::Draft, 150u32)
            .on_guarded(
                OrderState::Draft,
                OrderEvent::Pay,
                OrderState::Paid,
                |&balance, _| balance >= 100,
                |balance, _| *balance -= 100,
            )
            .on(OrderState::Paid, OrderEvent::Ship, OrderState::Shipped);

        assert!(machine.can(&OrderEvent::Pay));
        assert!(!machine.can(&OrderEvent::Ship));

        assert_eq!(machine.send(OrderEvent::Pay), Ok(&OrderState::Paid));
        assert_eq!(*machine.context(), 50);
        assert!(machine.can(&OrderEvent::Ship));

        assert_eq!(machine.send(OrderEvent::Ship), Ok(&OrderState::Shipped));
        assert_eq!(machine.history().len(), 3);
    }

    #[test]
    fn test_saga_workflow_automatic_rollback() {
        let saga = SagaWorkflow::<Vec<String>>::new()
            .step(
                "reserve_inventory",
                |log| {
                    log.push("reserved".to_string());
                    Ok(())
                },
                |log| log.push("unreserved".to_string()),
            )
            .step(
                "charge_card",
                |log| {
                    log.push("charged".to_string());
                    Ok(())
                },
                |log| log.push("refunded".to_string()),
            )
            .step(
                "dispatch_carrier",
                |_log| Err("carrier API down".to_string()),
                |_log| {},
            );

        let mut audit = Vec::new();
        let outcome = saga.run(&mut audit);
        assert_eq!(
            outcome,
            SagaOutcome::RolledBack {
                failed_step: "dispatch_carrier".to_string(),
                error: "carrier API down".to_string(),
                compensated_steps: vec!["charge_card".to_string(), "reserve_inventory".to_string()],
            }
        );
        assert_eq!(audit, vec!["reserved", "charged", "refunded", "unreserved"]);
    }
}
