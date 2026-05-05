#![allow(clippy::missing_safety_doc)]

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;

use metricchrono_core::{
    adaptive_ladder_distance, geometric_ladder, ladder_distance, simple_weight_update,
    smooth_tick_distance, tick_distance, weighted_consensus, EventLog, Tier,
};

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MCStatus {
    Ok = 0,
    Null = 1,
    InvalidArgument = 2,
    BufferTooSmall = 3,
    Panic = 255,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MCTier {
    pub epsilon: f64,
    pub delta: f64,
    pub p: f64,
    pub epsilon_ref: f64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MCZoomDecision {
    pub evaluated_tiers: usize,
    pub first_inactive_tier: usize,
    pub has_first_inactive_tier: bool,
    pub stopped_early: bool,
}

pub struct MCEventLog {
    inner: EventLog<u64>,
}

impl From<MCTier> for Tier {
    fn from(value: MCTier) -> Self {
        Self {
            epsilon: value.epsilon,
            delta: value.delta,
            p: value.p,
            epsilon_ref: value.epsilon_ref,
        }
    }
}

impl From<Tier> for MCTier {
    fn from(value: Tier) -> Self {
        Self {
            epsilon: value.epsilon,
            delta: value.delta,
            p: value.p,
            epsilon_ref: value.epsilon_ref,
        }
    }
}

fn ffi_status<F>(func: F) -> MCStatus
where
    F: FnOnce() -> MCStatus,
{
    match catch_unwind(AssertUnwindSafe(func)) {
        Ok(status) => status,
        Err(_) => MCStatus::Panic,
    }
}

fn status_from_error(error: metricchrono_core::MetricChronoError) -> MCStatus {
    match error {
        metricchrono_core::MetricChronoError::OutputTooSmall { .. } => MCStatus::BufferTooSmall,
        _ => MCStatus::InvalidArgument,
    }
}

unsafe fn slice_from_ptr<'a, T>(ptr: *const T, len: usize) -> Option<&'a [T]> {
    if len == 0 {
        Some(&[])
    } else if ptr.is_null() {
        None
    } else {
        Some(std::slice::from_raw_parts(ptr, len))
    }
}

unsafe fn slice_from_mut_ptr<'a, T>(ptr: *mut T, len: usize) -> Option<&'a mut [T]> {
    if len == 0 {
        Some(&mut [])
    } else if ptr.is_null() {
        None
    } else {
        Some(std::slice::from_raw_parts_mut(ptr, len))
    }
}

#[no_mangle]
pub unsafe extern "C" fn mc_tick_distance(distance: f64, tier: MCTier, out: *mut f64) -> MCStatus {
    ffi_status(|| {
        let Some(out) = (unsafe { out.as_mut() }) else {
            return MCStatus::Null;
        };
        let tier = Tier::from(tier);
        match metricchrono_core::try_tick_distance(distance, tier) {
            Ok(value) => {
                *out = value;
                MCStatus::Ok
            }
            Err(error) => status_from_error(error),
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn mc_ladder_distance(
    distance: f64,
    tiers: *const MCTier,
    len: usize,
    out: *mut f64,
    out_len: usize,
) -> MCStatus {
    ffi_status(|| {
        let Some(tiers) = (unsafe { slice_from_ptr(tiers, len) }) else {
            return MCStatus::Null;
        };
        let Some(out) = (unsafe { slice_from_mut_ptr(out, out_len) }) else {
            return MCStatus::Null;
        };
        let tiers: Vec<Tier> = tiers.iter().copied().map(Tier::from).collect();
        ladder_distance(distance, &tiers, out).map_or_else(status_from_error, |_| MCStatus::Ok)
    })
}

#[no_mangle]
pub unsafe extern "C" fn mc_adaptive_ladder_distance(
    distance: f64,
    tiers: *const MCTier,
    len: usize,
    out: *mut f64,
    out_len: usize,
    decision: *mut MCZoomDecision,
) -> MCStatus {
    ffi_status(|| {
        let Some(tiers) = (unsafe { slice_from_ptr(tiers, len) }) else {
            return MCStatus::Null;
        };
        let Some(out) = (unsafe { slice_from_mut_ptr(out, out_len) }) else {
            return MCStatus::Null;
        };
        let Some(decision) = (unsafe { decision.as_mut() }) else {
            return MCStatus::Null;
        };
        let tiers: Vec<Tier> = tiers.iter().copied().map(Tier::from).collect();
        match adaptive_ladder_distance(distance, &tiers, out) {
            Ok(value) => {
                decision.evaluated_tiers = value.evaluated_tiers;
                decision.first_inactive_tier = value.first_inactive_tier.unwrap_or(0);
                decision.has_first_inactive_tier = value.first_inactive_tier.is_some();
                decision.stopped_early = value.stopped_early;
                MCStatus::Ok
            }
            Err(error) => status_from_error(error),
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn mc_smooth_tick_distance(
    distance: f64,
    tier: MCTier,
    sharpness: f64,
    out: *mut f64,
) -> MCStatus {
    ffi_status(|| {
        let Some(out) = (unsafe { out.as_mut() }) else {
            return MCStatus::Null;
        };
        match smooth_tick_distance(distance, Tier::from(tier), sharpness) {
            Ok(value) => {
                *out = value;
                MCStatus::Ok
            }
            Err(error) => status_from_error(error),
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn mc_geometric_ladder(
    epsilon0: f64,
    delta0: f64,
    ratio: f64,
    tiers: usize,
    p: f64,
    epsilon_ref: f64,
    out: *mut MCTier,
    out_len: usize,
) -> MCStatus {
    ffi_status(|| {
        if out_len < tiers {
            return MCStatus::BufferTooSmall;
        }
        let Some(out) = (unsafe { slice_from_mut_ptr(out, out_len) }) else {
            return MCStatus::Null;
        };
        match geometric_ladder(epsilon0, delta0, ratio, tiers, p, epsilon_ref) {
            Ok(values) => {
                for (slot, tier) in out.iter_mut().zip(values) {
                    *slot = tier.into();
                }
                MCStatus::Ok
            }
            Err(error) => status_from_error(error),
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn mc_weighted_consensus(
    vectors: *const f64,
    rows: usize,
    cols: usize,
    weights: *const f64,
    out: *mut f64,
    out_len: usize,
) -> MCStatus {
    ffi_status(|| {
        let Some(vectors) = (unsafe { slice_from_ptr(vectors, rows.saturating_mul(cols)) }) else {
            return MCStatus::Null;
        };
        let Some(weights) = (unsafe { slice_from_ptr(weights, rows) }) else {
            return MCStatus::Null;
        };
        let Some(out) = (unsafe { slice_from_mut_ptr(out, out_len) }) else {
            return MCStatus::Null;
        };
        let rows: Vec<&[f64]> = vectors.chunks(cols).collect();
        weighted_consensus(&rows, weights, out).map_or_else(status_from_error, |_| MCStatus::Ok)
    })
}

#[no_mangle]
pub unsafe extern "C" fn mc_simple_weight_update(
    weights: *mut f64,
    residuals: *const f64,
    len: usize,
    learning_rate: f64,
    floor: f64,
) -> MCStatus {
    ffi_status(|| {
        let Some(weights) = (unsafe { slice_from_mut_ptr(weights, len) }) else {
            return MCStatus::Null;
        };
        let Some(residuals) = (unsafe { slice_from_ptr(residuals, len) }) else {
            return MCStatus::Null;
        };
        simple_weight_update(weights, residuals, learning_rate, floor)
            .map_or_else(status_from_error, |_| MCStatus::Ok)
    })
}

#[no_mangle]
pub extern "C" fn mc_event_log_new(tier_count: usize) -> *mut MCEventLog {
    match catch_unwind(AssertUnwindSafe(|| {
        EventLog::new(tier_count)
            .map(|inner| Box::into_raw(Box::new(MCEventLog { inner })))
            .unwrap_or(ptr::null_mut())
    })) {
        Ok(ptr) => ptr,
        Err(_) => ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn mc_event_log_free(log: *mut MCEventLog) {
    if log.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| unsafe {
        drop(Box::from_raw(log));
    }));
}

#[no_mangle]
pub unsafe extern "C" fn mc_event_log_append(
    log: *mut MCEventLog,
    state_id: u64,
    ticks: *const f64,
    len: usize,
    out_index: *mut usize,
) -> MCStatus {
    ffi_status(|| {
        let Some(log) = (unsafe { log.as_mut() }) else {
            return MCStatus::Null;
        };
        let Some(ticks) = (unsafe { slice_from_ptr(ticks, len) }) else {
            return MCStatus::Null;
        };
        let Some(out_index) = (unsafe { out_index.as_mut() }) else {
            return MCStatus::Null;
        };
        match log.inner.append(state_id, ticks.to_vec()) {
            Ok(index) => {
                *out_index = index;
                MCStatus::Ok
            }
            Err(error) => status_from_error(error),
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn mc_event_log_next_event(
    log: *const MCEventLog,
    index: usize,
    tier: usize,
    out_index: *mut usize,
    has_event: *mut bool,
) -> MCStatus {
    ffi_status(|| {
        let Some(log) = (unsafe { log.as_ref() }) else {
            return MCStatus::Null;
        };
        let Some(out_index) = (unsafe { out_index.as_mut() }) else {
            return MCStatus::Null;
        };
        let Some(has_event) = (unsafe { has_event.as_mut() }) else {
            return MCStatus::Null;
        };
        if tier >= log.inner.tier_count() || index >= log.inner.len() {
            return MCStatus::InvalidArgument;
        }
        if let Some(next) = log.inner.next_event(index, tier) {
            *out_index = next;
            *has_event = true;
        } else {
            *out_index = 0;
            *has_event = false;
        }
        MCStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn mc_event_log_len(log: *const MCEventLog, out_len: *mut usize) -> MCStatus {
    ffi_status(|| {
        let Some(log) = (unsafe { log.as_ref() }) else {
            return MCStatus::Null;
        };
        let Some(out_len) = (unsafe { out_len.as_mut() }) else {
            return MCStatus::Null;
        };
        *out_len = log.inner.len();
        MCStatus::Ok
    })
}

#[no_mangle]
pub extern "C" fn mc_tick_distance_unchecked(distance: f64, tier: MCTier) -> f64 {
    tick_distance(distance, Tier::from(tier))
}

#[no_mangle]
pub extern "C" fn mc_tick_distance_raw(
    distance: f64,
    epsilon: f64,
    delta: f64,
    p: f64,
    epsilon_ref: f64,
) -> f64 {
    tick_distance(
        distance,
        Tier {
            epsilon,
            delta,
            p,
            epsilon_ref,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ffi_tick_and_ladder_return_stable_values() {
        let tier = MCTier {
            epsilon: 0.5,
            delta: 1.0,
            p: 0.5,
            epsilon_ref: 1.0,
        };
        let mut out = 0.0;
        assert_eq!(
            unsafe { mc_tick_distance(1.2, tier, &mut out) },
            MCStatus::Ok
        );
        assert_eq!(out, 2.0_f64.sqrt());

        let tiers = [
            tier,
            MCTier {
                epsilon: 1.0,
                delta: 2.0,
                p: 0.5,
                epsilon_ref: 1.0,
            },
        ];
        let mut values = [0.0; 2];
        assert_eq!(
            unsafe {
                mc_ladder_distance(
                    1.2,
                    tiers.as_ptr(),
                    tiers.len(),
                    values.as_mut_ptr(),
                    values.len(),
                )
            },
            MCStatus::Ok
        );
        assert!(values[0] > 0.0 && values[1] > 0.0);
    }

    #[test]
    fn ffi_event_log_exposes_next_pointers() {
        let log = mc_event_log_new(2);
        assert!(!log.is_null());
        let mut index = usize::MAX;
        assert_eq!(
            unsafe { mc_event_log_append(log, 10, [1.0, 0.0].as_ptr(), 2, &mut index) },
            MCStatus::Ok
        );
        assert_eq!(index, 0);
        assert_eq!(
            unsafe { mc_event_log_append(log, 11, [1.0, 1.0].as_ptr(), 2, &mut index) },
            MCStatus::Ok
        );
        let mut next = 0;
        let mut has = false;
        assert_eq!(
            unsafe { mc_event_log_next_event(log, 0, 0, &mut next, &mut has) },
            MCStatus::Ok
        );
        assert!(has);
        assert_eq!(next, 1);
        unsafe { mc_event_log_free(log) };
    }
}
