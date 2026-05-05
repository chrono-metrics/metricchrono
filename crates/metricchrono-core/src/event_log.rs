use crate::ladder::{ensure_shape, sanitize_signed};
use crate::{MetricChronoError, Result};

/// Stable event identifier within an [`EventLog`].
pub type EventId = usize;

/// One event-log record.
#[derive(Clone, Debug, PartialEq)]
pub struct EventRecord<S = u64> {
    pub state_id: S,
    pub ticks: Vec<f64>,
    pub next_event: Vec<Option<usize>>,
}

/// Compact summary item for a tier.
#[derive(Clone, Debug, PartialEq)]
pub struct EventSummary<S = u64> {
    pub index: EventId,
    pub state_id: S,
    pub tick: f64,
}

/// Basic in-memory event skip-list.
#[derive(Clone, Debug, PartialEq)]
pub struct EventLog<S = u64> {
    tier_count: usize,
    records: Vec<EventRecord<S>>,
    first_by_tier: Vec<Option<usize>>,
    last_by_tier: Vec<Option<usize>>,
}

impl<S> EventLog<S> {
    pub fn new(tier_count: usize) -> Result<Self> {
        if tier_count == 0 {
            return Err(MetricChronoError::EmptyLadder);
        }
        Ok(Self {
            tier_count,
            records: Vec::new(),
            first_by_tier: vec![None; tier_count],
            last_by_tier: vec![None; tier_count],
        })
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn tier_count(&self) -> usize {
        self.tier_count
    }

    pub fn records(&self) -> &[EventRecord<S>] {
        &self.records
    }

    pub fn record(&self, index: EventId) -> Option<&EventRecord<S>> {
        self.records.get(index)
    }

    pub fn get(&self, index: EventId) -> Option<&EventRecord<S>> {
        self.record(index)
    }

    /// Append a record. A positive finite tick at tier `k` becomes an event at
    /// tier `k` and updates the skip pointer from the previous event.
    pub fn append(&mut self, state_id: S, tick_vector: impl Into<Vec<f64>>) -> Result<EventId> {
        let ticks = tick_vector.into();
        ensure_shape(self.tier_count, ticks.len(), "tick vector")?;

        let index = self.records.len();
        self.records.push(EventRecord {
            state_id,
            ticks,
            next_event: vec![None; self.tier_count],
        });

        for tier in 0..self.tier_count {
            if !is_event_tick(self.records[index].ticks[tier]) {
                continue;
            }
            if let Some(prev) = self.last_by_tier[tier] {
                self.records[prev].next_event[tier] = Some(index);
            } else {
                self.first_by_tier[tier] = Some(index);
            }
            self.last_by_tier[tier] = Some(index);
        }

        Ok(index)
    }

    pub fn next_event(&self, index: EventId, tier: usize) -> Option<EventId> {
        self.records
            .get(index)
            .and_then(|record| record.next_event.get(tier))
            .copied()
            .flatten()
    }

    pub fn first_event(&self, tier: usize) -> Option<EventId> {
        self.first_by_tier.get(tier).copied().flatten()
    }

    pub fn iter_events(&self, tier: usize) -> TierEventIter<'_, S> {
        TierEventIter {
            log: self,
            tier,
            next: self.first_event(tier),
        }
    }
}

impl<S: Clone> EventLog<S> {
    pub fn compact_summary(&self, tier: usize) -> Vec<EventSummary<S>> {
        self.iter_events(tier)
            .map(|(index, record)| EventSummary {
                index,
                state_id: record.state_id.clone(),
                tick: record.ticks[tier],
            })
            .collect()
    }
}

/// Iterator over tier-local event records.
pub struct TierEventIter<'a, S> {
    log: &'a EventLog<S>,
    tier: usize,
    next: Option<usize>,
}

impl<'a, S> Iterator for TierEventIter<'a, S> {
    type Item = (usize, &'a EventRecord<S>);

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.next?;
        let record = self.log.records.get(index)?;
        self.next = record.next_event.get(self.tier).copied().flatten();
        Some((index, record))
    }
}

fn is_event_tick(value: f64) -> bool {
    sanitize_signed(value) > 0.0
}
