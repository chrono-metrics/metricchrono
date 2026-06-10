#![allow(clippy::missing_safety_doc)]

use std::cell::{Cell, RefCell};
use std::ffi::{c_char, c_int, c_void};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;

use metricchrono_core::{
    adaptive_ladder_distance, carry_rules, classify_regime, custom_ladder, geometric_ladder,
    ladder_distance, ladder_pair, normalize_ticks, progress_efficiency, simple_weight_update,
    smooth_tick_distance, tick_distance, tick_pair, validate_ladder, weighted_consensus, Absolute,
    CoverageMeter, Euclidean, EventLog, Metric, MetricChronoError, MetricFn, Normalization,
    OperatingRegime, PromotionCounter, SmoothParams, Tier,
};

const MC_METRIC_EUCLIDEAN: c_int = 0;
const MC_METRIC_ABSOLUTE: c_int = 1;

const MC_REGIME_QUIESCENT: c_int = 0;
const MC_REGIME_PROGRESS: c_int = 1;
const MC_REGIME_CHURN: c_int = 2;
const MC_REGIME_CREEP: c_int = 3;

const MC_NORMALIZATION_NONE: c_int = 0;
const MC_NORMALIZATION_UNIT_MAX: c_int = 1;
const MC_NORMALIZATION_TANH: c_int = 2;

thread_local! {
    static LAST_ERROR: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static LAST_ERROR_SET_IN_CALL: Cell<bool> = const { Cell::new(false) };
}

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

pub struct MCLadder {
    tiers: Vec<Tier>,
}

pub struct MCPromotionCounter {
    inner: PromotionCounter,
}

/// Caller-supplied distance function over two `dim`-length state vectors.
///
/// The callback must not unwind, must remain callable for the lifetime of the
/// meter it is registered with, and `user_data` (passed through verbatim) must
/// outlive the meter. Returning NaN rejects admission, which is the safe
/// failure mode for a callback that cannot compute a distance.
pub type MCDistanceFn =
    unsafe extern "C" fn(a: *const f64, b: *const f64, dim: usize, user_data: *mut c_void) -> f64;

#[derive(Clone, Copy)]
enum CoverageMetric {
    Builtin(c_int),
    Callback {
        callback: MCDistanceFn,
        user_data: *mut c_void,
    },
}

pub struct MCCoverageMeter {
    inner: CoverageMeter<Vec<f64>>,
    dim: usize,
    metric: CoverageMetric,
    /// Reusable state buffer so rejected observations allocate nothing.
    scratch: Vec<f64>,
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
    begin_ffi_call();
    match catch_unwind(AssertUnwindSafe(func)) {
        Ok(status) => finish_status(status),
        Err(_) => {
            set_last_error("panic");
            MCStatus::Panic
        }
    }
}

fn begin_ffi_call() {
    LAST_ERROR_SET_IN_CALL.with(|flag| flag.set(false));
}

fn set_last_error(message: impl AsRef<str>) {
    LAST_ERROR.with(|slot| {
        let mut slot = slot.borrow_mut();
        slot.clear();
        slot.extend_from_slice(message.as_ref().as_bytes());
    });
    LAST_ERROR_SET_IN_CALL.with(|flag| flag.set(true));
}

fn finish_status(status: MCStatus) -> MCStatus {
    if status != MCStatus::Ok {
        let already_set = LAST_ERROR_SET_IN_CALL.with(Cell::get);
        if !already_set {
            set_last_error(status_message(status));
        }
    }
    status
}

fn status_message(status: MCStatus) -> &'static str {
    match status {
        MCStatus::Ok => "ok",
        MCStatus::Null => "null pointer",
        MCStatus::InvalidArgument => "invalid argument",
        MCStatus::BufferTooSmall => "buffer too small",
        MCStatus::Panic => "panic",
    }
}

fn status_from_error(error: MetricChronoError) -> MCStatus {
    let status = match error {
        MetricChronoError::OutputTooSmall { .. } => MCStatus::BufferTooSmall,
        _ => MCStatus::InvalidArgument,
    };
    set_last_error(error.to_string());
    status
}

fn invalid_argument(message: &'static str) -> MCStatus {
    status_from_error(MetricChronoError::InvalidArgument(message))
}

fn buffer_too_small(needed: usize, actual: usize) -> MCStatus {
    status_from_error(MetricChronoError::OutputTooSmall { needed, actual })
}

fn normalization_from_id(id: c_int) -> Result<Normalization, MetricChronoError> {
    match id {
        MC_NORMALIZATION_NONE => Ok(Normalization::None),
        MC_NORMALIZATION_UNIT_MAX => Ok(Normalization::UnitMax),
        MC_NORMALIZATION_TANH => Ok(Normalization::Tanh),
        _ => Err(MetricChronoError::InvalidArgument(
            "unknown normalization id",
        )),
    }
}

#[no_mangle]
pub extern "C" fn mc_error_message(status: c_int) -> *const c_char {
    match status {
        0 => b"ok\0".as_ptr().cast(),
        1 => b"null pointer\0".as_ptr().cast(),
        2 => b"invalid argument\0".as_ptr().cast(),
        3 => b"buffer too small\0".as_ptr().cast(),
        255 => b"panic\0".as_ptr().cast(),
        _ => b"unknown status\0".as_ptr().cast(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn mc_last_error_message(
    buf: *mut c_char,
    cap: usize,
    out_len: *mut usize,
) -> MCStatus {
    match catch_unwind(AssertUnwindSafe(|| {
        let Some(out_len) = (unsafe { out_len.as_mut() }) else {
            set_last_error("null pointer");
            return MCStatus::Null;
        };

        let needed = LAST_ERROR.with(|slot| slot.borrow().len() + 1);
        *out_len = needed;
        if cap < needed {
            return MCStatus::BufferTooSmall;
        }

        if buf.is_null() {
            set_last_error("null pointer");
            return MCStatus::Null;
        }

        LAST_ERROR.with(|slot| {
            let message = slot.borrow();
            unsafe {
                ptr::copy_nonoverlapping(message.as_ptr().cast::<c_char>(), buf, message.len());
                *buf.add(message.len()) = 0;
            }
        });
        MCStatus::Ok
    })) {
        Ok(status) => status,
        Err(_) => {
            set_last_error("panic");
            MCStatus::Panic
        }
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

fn create_ladder(tiers: &[MCTier], out: &mut *mut MCLadder) -> MCStatus {
    let tiers: Vec<Tier> = tiers.iter().copied().map(Tier::from).collect();
    match custom_ladder(tiers) {
        Ok(tiers) => {
            *out = Box::into_raw(Box::new(MCLadder { tiers }));
            MCStatus::Ok
        }
        Err(error) => {
            *out = ptr::null_mut();
            status_from_error(error)
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn mc_tier_new(
    epsilon: f64,
    delta: f64,
    p: f64,
    epsilon_ref: f64,
    out: *mut MCTier,
) -> MCStatus {
    ffi_status(|| {
        let Some(out) = (unsafe { out.as_mut() }) else {
            return MCStatus::Null;
        };
        match Tier::new(epsilon, delta, p, epsilon_ref) {
            Ok(tier) => {
                *out = tier.into();
                MCStatus::Ok
            }
            Err(error) => status_from_error(error),
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn mc_ladder_new(
    tiers: *const MCTier,
    len: usize,
    out: *mut *mut MCLadder,
) -> MCStatus {
    ffi_status(|| {
        let Some(out) = (unsafe { out.as_mut() }) else {
            return MCStatus::Null;
        };
        let Some(tiers) = (unsafe { slice_from_ptr(tiers, len) }) else {
            return MCStatus::Null;
        };
        create_ladder(tiers, out)
    })
}

#[no_mangle]
pub unsafe extern "C" fn mc_custom_ladder(
    tiers: *const MCTier,
    len: usize,
    out: *mut *mut MCLadder,
) -> MCStatus {
    ffi_status(|| {
        let Some(out) = (unsafe { out.as_mut() }) else {
            return MCStatus::Null;
        };
        let Some(tiers) = (unsafe { slice_from_ptr(tiers, len) }) else {
            return MCStatus::Null;
        };
        create_ladder(tiers, out)
    })
}

#[no_mangle]
pub unsafe extern "C" fn mc_ladder_free(ladder: *mut MCLadder) {
    if ladder.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| unsafe {
        drop(Box::from_raw(ladder));
    }));
}

#[no_mangle]
pub unsafe extern "C" fn mc_ladder_len(ladder: *const MCLadder, out_len: *mut usize) -> MCStatus {
    ffi_status(|| {
        let Some(ladder) = (unsafe { ladder.as_ref() }) else {
            return MCStatus::Null;
        };
        let Some(out_len) = (unsafe { out_len.as_mut() }) else {
            return MCStatus::Null;
        };
        *out_len = ladder.tiers.len();
        MCStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn mc_validate_ladder(ladder: *const MCLadder) -> MCStatus {
    ffi_status(|| {
        let Some(ladder) = (unsafe { ladder.as_ref() }) else {
            return MCStatus::Null;
        };
        validate_ladder(&ladder.tiers).map_or_else(status_from_error, |_| MCStatus::Ok)
    })
}

#[no_mangle]
pub unsafe extern "C" fn mc_ladder_distance_owned(
    ladder: *const MCLadder,
    distance: f64,
    out: *mut f64,
    out_len: usize,
) -> MCStatus {
    ffi_status(|| {
        let Some(ladder) = (unsafe { ladder.as_ref() }) else {
            return MCStatus::Null;
        };
        let Some(out) = (unsafe { slice_from_mut_ptr(out, out_len) }) else {
            return MCStatus::Null;
        };
        ladder_distance(distance, &ladder.tiers, out)
            .map_or_else(status_from_error, |_| MCStatus::Ok)
    })
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
pub unsafe extern "C" fn mc_euclidean_distance(
    a: *const f64,
    b: *const f64,
    len: usize,
    out: *mut f64,
) -> MCStatus {
    ffi_status(|| {
        let Some(out) = (unsafe { out.as_mut() }) else {
            return MCStatus::Null;
        };
        let Some(a) = (unsafe { slice_from_ptr(a, len) }) else {
            return MCStatus::Null;
        };
        let Some(b) = (unsafe { slice_from_ptr(b, len) }) else {
            return MCStatus::Null;
        };
        *out = Euclidean.distance(a, b);
        MCStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn mc_absolute_distance(
    a: *const f64,
    b: *const f64,
    len: usize,
    out: *mut f64,
) -> MCStatus {
    ffi_status(|| {
        if len != 1 {
            return invalid_argument("absolute metric requires len == 1");
        }
        let Some(out) = (unsafe { out.as_mut() }) else {
            return MCStatus::Null;
        };
        let Some(a) = (unsafe { slice_from_ptr(a, len) }) else {
            return MCStatus::Null;
        };
        let Some(b) = (unsafe { slice_from_ptr(b, len) }) else {
            return MCStatus::Null;
        };
        *out = Absolute.distance(&a[0], &b[0]);
        MCStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn mc_tick_pair(
    metric_id: c_int,
    a: *const f64,
    b: *const f64,
    len: usize,
    tier: MCTier,
    out: *mut f64,
) -> MCStatus {
    ffi_status(|| {
        let Some(out) = (unsafe { out.as_mut() }) else {
            return MCStatus::Null;
        };
        match metric_id {
            MC_METRIC_EUCLIDEAN => {
                let Some(a) = (unsafe { slice_from_ptr(a, len) }) else {
                    return MCStatus::Null;
                };
                let Some(b) = (unsafe { slice_from_ptr(b, len) }) else {
                    return MCStatus::Null;
                };
                match tick_pair(a, b, &Euclidean, Tier::from(tier)) {
                    Ok(value) => {
                        *out = value;
                        MCStatus::Ok
                    }
                    Err(error) => status_from_error(error),
                }
            }
            MC_METRIC_ABSOLUTE => {
                if len != 1 {
                    return invalid_argument("absolute metric requires len == 1");
                }
                let Some(a) = (unsafe { slice_from_ptr(a, len) }) else {
                    return MCStatus::Null;
                };
                let Some(b) = (unsafe { slice_from_ptr(b, len) }) else {
                    return MCStatus::Null;
                };
                match tick_pair(&a[0], &b[0], &Absolute, Tier::from(tier)) {
                    Ok(value) => {
                        *out = value;
                        MCStatus::Ok
                    }
                    Err(error) => status_from_error(error),
                }
            }
            _ => invalid_argument("unknown metric id"),
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
pub unsafe extern "C" fn mc_ladder_pair(
    metric_id: c_int,
    a: *const f64,
    b: *const f64,
    len: usize,
    ladder: *const MCLadder,
    out: *mut f64,
    cap: usize,
    out_len: *mut usize,
) -> MCStatus {
    ffi_status(|| {
        if metric_id != MC_METRIC_EUCLIDEAN && metric_id != MC_METRIC_ABSOLUTE {
            return invalid_argument("unknown metric id");
        }
        if metric_id == MC_METRIC_ABSOLUTE && len != 1 {
            return invalid_argument("absolute metric requires len == 1");
        }
        let Some(ladder) = (unsafe { ladder.as_ref() }) else {
            return MCStatus::Null;
        };
        let Some(out_len) = (unsafe { out_len.as_mut() }) else {
            return MCStatus::Null;
        };
        let needed = ladder.tiers.len();
        *out_len = needed;
        if cap < needed {
            return buffer_too_small(needed, cap);
        }
        let Some(out) = (unsafe { slice_from_mut_ptr(out, cap) }) else {
            return MCStatus::Null;
        };
        let Some(a) = (unsafe { slice_from_ptr(a, len) }) else {
            return MCStatus::Null;
        };
        let Some(b) = (unsafe { slice_from_ptr(b, len) }) else {
            return MCStatus::Null;
        };
        let values = match metric_id {
            MC_METRIC_EUCLIDEAN => ladder_pair(a, b, &Euclidean, &ladder.tiers),
            MC_METRIC_ABSOLUTE => ladder_pair(&a[0], &b[0], &Absolute, &ladder.tiers),
            _ => unreachable!(),
        };
        match values {
            Ok(values) => {
                out[..needed].copy_from_slice(&values);
                MCStatus::Ok
            }
            Err(error) => status_from_error(error),
        }
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
        let params = match SmoothParams::sharpness(sharpness) {
            Ok(params) => params,
            Err(error) => return status_from_error(error),
        };
        match smooth_tick_distance(distance, Tier::from(tier), params) {
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
            return buffer_too_small(tiers, out_len);
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
pub unsafe extern "C" fn mc_normalize_ticks(
    ticks: *const f64,
    len: usize,
    normalization_id: c_int,
    out: *mut f64,
) -> MCStatus {
    ffi_status(|| {
        let mode = match normalization_from_id(normalization_id) {
            Ok(mode) => mode,
            Err(error) => return status_from_error(error),
        };
        let Some(ticks) = (unsafe { slice_from_ptr(ticks, len) }) else {
            return MCStatus::Null;
        };
        let Some(out) = (unsafe { slice_from_mut_ptr(out, len) }) else {
            return MCStatus::Null;
        };
        normalize_ticks(ticks, mode, out).map_or_else(status_from_error, |_| MCStatus::Ok)
    })
}

#[no_mangle]
pub unsafe extern "C" fn mc_carry_rules(
    epsilons: *const f64,
    len: usize,
    out: *mut u64,
    cap: usize,
    out_len: *mut usize,
) -> MCStatus {
    ffi_status(|| {
        let Some(epsilons) = (unsafe { slice_from_ptr(epsilons, len) }) else {
            return MCStatus::Null;
        };
        let Some(out_len) = (unsafe { out_len.as_mut() }) else {
            return MCStatus::Null;
        };
        let rules = match carry_rules(epsilons) {
            Ok(rules) => rules,
            Err(error) => return status_from_error(error),
        };
        let needed = rules.len();
        *out_len = needed;
        if cap < needed {
            return buffer_too_small(needed, cap);
        }
        let Some(out) = (unsafe { slice_from_mut_ptr(out, cap) }) else {
            return MCStatus::Null;
        };
        out[..needed].copy_from_slice(&rules);
        MCStatus::Ok
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
        if rows == 0 || cols == 0 {
            return invalid_argument("rows and cols must be > 0");
        }
        let Some(vector_len) = rows.checked_mul(cols) else {
            return invalid_argument("rows * cols overflow");
        };
        const MAX_F64_SLICE_LEN: usize = isize::MAX as usize / std::mem::size_of::<f64>();
        if vector_len > MAX_F64_SLICE_LEN || out_len > MAX_F64_SLICE_LEN {
            return invalid_argument("slice byte length exceeds isize::MAX");
        }
        let Some(vectors) = (unsafe { slice_from_ptr(vectors, vector_len) }) else {
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
pub unsafe extern "C" fn mc_promotion_counter_new(
    quotas: *const u64,
    len: usize,
    out: *mut *mut MCPromotionCounter,
) -> MCStatus {
    ffi_status(|| {
        let Some(out) = (unsafe { out.as_mut() }) else {
            return MCStatus::Null;
        };
        let Some(quotas) = (unsafe { slice_from_ptr(quotas, len) }) else {
            return MCStatus::Null;
        };
        match PromotionCounter::new(quotas.to_vec()) {
            Ok(inner) => {
                *out = Box::into_raw(Box::new(MCPromotionCounter { inner }));
                MCStatus::Ok
            }
            Err(error) => {
                *out = ptr::null_mut();
                status_from_error(error)
            }
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn mc_promotion_counter_from_epsilons(
    epsilons: *const f64,
    len: usize,
    out: *mut *mut MCPromotionCounter,
) -> MCStatus {
    ffi_status(|| {
        let Some(out) = (unsafe { out.as_mut() }) else {
            return MCStatus::Null;
        };
        let Some(epsilons) = (unsafe { slice_from_ptr(epsilons, len) }) else {
            return MCStatus::Null;
        };
        match PromotionCounter::from_epsilons(epsilons) {
            Ok(inner) => {
                *out = Box::into_raw(Box::new(MCPromotionCounter { inner }));
                MCStatus::Ok
            }
            Err(error) => {
                *out = ptr::null_mut();
                status_from_error(error)
            }
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn mc_promotion_counter_step(
    counter: *mut MCPromotionCounter,
    event_flags: *const bool,
    flags_len: usize,
    out: *mut bool,
    cap: usize,
    out_len: *mut usize,
) -> MCStatus {
    ffi_status(|| {
        let Some(counter) = (unsafe { counter.as_mut() }) else {
            return MCStatus::Null;
        };
        let Some(out_len) = (unsafe { out_len.as_mut() }) else {
            return MCStatus::Null;
        };
        let needed = counter.inner.len();
        *out_len = needed;
        if cap < needed {
            return buffer_too_small(needed, cap);
        }
        let flags = if event_flags.is_null() && flags_len == 0 {
            None
        } else {
            let Some(flags) = (unsafe { slice_from_ptr(event_flags, flags_len) }) else {
                return MCStatus::Null;
            };
            Some(flags)
        };
        let Some(out) = (unsafe { slice_from_mut_ptr(out, cap) }) else {
            return MCStatus::Null;
        };
        counter
            .inner
            .step(flags, out)
            .map_or_else(status_from_error, |_| MCStatus::Ok)
    })
}

#[no_mangle]
pub unsafe extern "C" fn mc_promotion_counter_counters(
    counter: *const MCPromotionCounter,
    out: *mut u64,
    cap: usize,
    out_len: *mut usize,
) -> MCStatus {
    ffi_status(|| {
        let Some(counter) = (unsafe { counter.as_ref() }) else {
            return MCStatus::Null;
        };
        let Some(out_len) = (unsafe { out_len.as_mut() }) else {
            return MCStatus::Null;
        };
        let counters = counter.inner.counters();
        let needed = counters.len();
        *out_len = needed;
        if cap < needed {
            return buffer_too_small(needed, cap);
        }
        let Some(out) = (unsafe { slice_from_mut_ptr(out, cap) }) else {
            return MCStatus::Null;
        };
        out[..needed].copy_from_slice(counters);
        MCStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn mc_promotion_counter_quotas(
    counter: *const MCPromotionCounter,
    out: *mut u64,
    cap: usize,
    out_len: *mut usize,
) -> MCStatus {
    ffi_status(|| {
        let Some(counter) = (unsafe { counter.as_ref() }) else {
            return MCStatus::Null;
        };
        let Some(out_len) = (unsafe { out_len.as_mut() }) else {
            return MCStatus::Null;
        };
        let quotas = counter.inner.quotas();
        let needed = quotas.len();
        *out_len = needed;
        if cap < needed {
            return buffer_too_small(needed, cap);
        }
        let Some(out) = (unsafe { slice_from_mut_ptr(out, cap) }) else {
            return MCStatus::Null;
        };
        out[..needed].copy_from_slice(quotas);
        MCStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn mc_promotion_counter_reset(counter: *mut MCPromotionCounter) -> MCStatus {
    ffi_status(|| {
        let Some(counter) = (unsafe { counter.as_mut() }) else {
            return MCStatus::Null;
        };
        counter.inner.reset();
        MCStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn mc_promotion_counter_free(counter: *mut MCPromotionCounter) {
    if counter.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| unsafe {
        drop(Box::from_raw(counter));
    }));
}

fn coverage_distance(metric: CoverageMetric, a: &Vec<f64>, b: &Vec<f64>) -> f64 {
    match metric {
        CoverageMetric::Builtin(MC_METRIC_ABSOLUTE) => (a[0] - b[0]).abs(),
        CoverageMetric::Builtin(_) => Euclidean.distance(a.as_slice(), b.as_slice()),
        CoverageMetric::Callback {
            callback,
            user_data,
        } => unsafe { callback(a.as_ptr(), b.as_ptr(), a.len(), user_data) },
    }
}

#[no_mangle]
pub unsafe extern "C" fn mc_coverage_meter_new(
    epsilons: *const f64,
    len: usize,
    dim: usize,
    metric: c_int,
    out: *mut *mut MCCoverageMeter,
) -> MCStatus {
    ffi_status(|| {
        let Some(out) = (unsafe { out.as_mut() }) else {
            return MCStatus::Null;
        };
        *out = ptr::null_mut();
        let Some(epsilons) = (unsafe { slice_from_ptr(epsilons, len) }) else {
            return MCStatus::Null;
        };
        if dim == 0 {
            set_last_error("coverage state dimension must be > 0");
            return MCStatus::InvalidArgument;
        }
        if metric != MC_METRIC_EUCLIDEAN && metric != MC_METRIC_ABSOLUTE {
            set_last_error("unknown metric id");
            return MCStatus::InvalidArgument;
        }
        if metric == MC_METRIC_ABSOLUTE && dim != 1 {
            set_last_error("absolute metric requires dimension 1");
            return MCStatus::InvalidArgument;
        }
        match CoverageMeter::from_epsilons(epsilons) {
            Ok(inner) => {
                *out = Box::into_raw(Box::new(MCCoverageMeter {
                    inner,
                    dim,
                    metric: CoverageMetric::Builtin(metric),
                    scratch: Vec::with_capacity(dim),
                }));
                MCStatus::Ok
            }
            Err(error) => status_from_error(error),
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn mc_coverage_meter_new_with_callback(
    epsilons: *const f64,
    len: usize,
    dim: usize,
    callback: Option<MCDistanceFn>,
    user_data: *mut c_void,
    out: *mut *mut MCCoverageMeter,
) -> MCStatus {
    ffi_status(|| {
        let Some(out) = (unsafe { out.as_mut() }) else {
            return MCStatus::Null;
        };
        *out = ptr::null_mut();
        let Some(epsilons) = (unsafe { slice_from_ptr(epsilons, len) }) else {
            return MCStatus::Null;
        };
        let Some(callback) = callback else {
            return MCStatus::Null;
        };
        if dim == 0 {
            set_last_error("coverage state dimension must be > 0");
            return MCStatus::InvalidArgument;
        }
        match CoverageMeter::from_epsilons(epsilons) {
            Ok(inner) => {
                *out = Box::into_raw(Box::new(MCCoverageMeter {
                    inner,
                    dim,
                    metric: CoverageMetric::Callback {
                        callback,
                        user_data,
                    },
                    scratch: Vec::with_capacity(dim),
                }));
                MCStatus::Ok
            }
            Err(error) => status_from_error(error),
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn mc_coverage_meter_observe(
    meter: *mut MCCoverageMeter,
    state: *const f64,
    state_len: usize,
    out: *mut bool,
    cap: usize,
    out_len: *mut usize,
) -> MCStatus {
    ffi_status(|| {
        let Some(meter) = (unsafe { meter.as_mut() }) else {
            return MCStatus::Null;
        };
        let Some(out_len) = (unsafe { out_len.as_mut() }) else {
            return MCStatus::Null;
        };
        let needed = meter.inner.tier_count();
        *out_len = needed;
        if cap < needed {
            return buffer_too_small(needed, cap);
        }
        let Some(state) = (unsafe { slice_from_ptr(state, state_len) }) else {
            return MCStatus::Null;
        };
        if state_len != meter.dim {
            return status_from_error(MetricChronoError::ShapeMismatch {
                expected: meter.dim,
                actual: state_len,
                context: "coverage state dimension",
            });
        }
        let Some(out) = (unsafe { slice_from_mut_ptr(out, cap) }) else {
            return MCStatus::Null;
        };
        let metric_kind = meter.metric;
        let metric =
            MetricFn(move |a: &Vec<f64>, b: &Vec<f64>| coverage_distance(metric_kind, a, b));
        let MCCoverageMeter { inner, scratch, .. } = meter;
        scratch.clear();
        scratch.extend_from_slice(state);
        inner
            .observe_into(&metric, scratch, &mut out[..needed])
            .map_or_else(status_from_error, |_| MCStatus::Ok)
    })
}

#[no_mangle]
pub unsafe extern "C" fn mc_coverage_meter_counts(
    meter: *const MCCoverageMeter,
    out: *mut u64,
    cap: usize,
    out_len: *mut usize,
) -> MCStatus {
    ffi_status(|| {
        let Some(meter) = (unsafe { meter.as_ref() }) else {
            return MCStatus::Null;
        };
        let Some(out_len) = (unsafe { out_len.as_mut() }) else {
            return MCStatus::Null;
        };
        let needed = meter.inner.tier_count();
        *out_len = needed;
        if cap < needed {
            return buffer_too_small(needed, cap);
        }
        let Some(out) = (unsafe { slice_from_mut_ptr(out, cap) }) else {
            return MCStatus::Null;
        };
        for (slot, count) in out.iter_mut().zip(meter.inner.counts()) {
            *slot = count as u64;
        }
        MCStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn mc_coverage_meter_unique_representatives(
    meter: *const MCCoverageMeter,
    out: *mut u64,
) -> MCStatus {
    ffi_status(|| {
        let Some(meter) = (unsafe { meter.as_ref() }) else {
            return MCStatus::Null;
        };
        let Some(out) = (unsafe { out.as_mut() }) else {
            return MCStatus::Null;
        };
        *out = meter.inner.unique_representatives() as u64;
        MCStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn mc_coverage_meter_tier_count(
    meter: *const MCCoverageMeter,
    out: *mut usize,
) -> MCStatus {
    ffi_status(|| {
        let Some(meter) = (unsafe { meter.as_ref() }) else {
            return MCStatus::Null;
        };
        let Some(out) = (unsafe { out.as_mut() }) else {
            return MCStatus::Null;
        };
        *out = meter.inner.tier_count();
        MCStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn mc_coverage_meter_free(meter: *mut MCCoverageMeter) {
    if meter.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| unsafe {
        drop(Box::from_raw(meter));
    }));
}

#[no_mangle]
pub unsafe extern "C" fn mc_progress_efficiency(
    coverage: u64,
    epsilon: f64,
    path_length: f64,
    out: *mut f64,
) -> MCStatus {
    ffi_status(|| {
        let Some(out) = (unsafe { out.as_mut() }) else {
            return MCStatus::Null;
        };
        match progress_efficiency(coverage as usize, epsilon, path_length) {
            Some(value) => {
                *out = value;
                MCStatus::Ok
            }
            None => {
                set_last_error("path_length must be finite and positive");
                MCStatus::InvalidArgument
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn mc_classify_regime(throughput_delta: f64, coverage_delta: u64) -> c_int {
    match classify_regime(throughput_delta, coverage_delta as usize) {
        OperatingRegime::Quiescent => MC_REGIME_QUIESCENT,
        OperatingRegime::Progress => MC_REGIME_PROGRESS,
        OperatingRegime::Churn => MC_REGIME_CHURN,
        OperatingRegime::Creep => MC_REGIME_CREEP,
    }
}

#[no_mangle]
pub extern "C" fn mc_event_log_new(tier_count: usize) -> *mut MCEventLog {
    begin_ffi_call();
    match catch_unwind(AssertUnwindSafe(|| match EventLog::new(tier_count) {
        Ok(inner) => Box::into_raw(Box::new(MCEventLog { inner })),
        Err(error) => {
            status_from_error(error);
            ptr::null_mut()
        }
    })) {
        Ok(ptr) => ptr,
        Err(_) => {
            set_last_error("panic");
            ptr::null_mut()
        }
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
pub unsafe extern "C" fn mc_event_log_first_event(
    log: *const MCEventLog,
    tier: usize,
    out_index: *mut usize,
    out_has: *mut bool,
) -> MCStatus {
    ffi_status(|| {
        let Some(log) = (unsafe { log.as_ref() }) else {
            return MCStatus::Null;
        };
        let Some(out_index) = (unsafe { out_index.as_mut() }) else {
            return MCStatus::Null;
        };
        let Some(out_has) = (unsafe { out_has.as_mut() }) else {
            return MCStatus::Null;
        };
        if tier >= log.inner.tier_count() {
            return invalid_argument("event log tier is out of bounds");
        }
        if let Some(index) = log.inner.first_event(tier) {
            *out_index = index;
            *out_has = true;
        } else {
            *out_index = 0;
            *out_has = false;
        }
        MCStatus::Ok
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
            return invalid_argument("event log index or tier is out of bounds");
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
pub unsafe extern "C" fn mc_event_log_record(
    log: *const MCEventLog,
    index: usize,
    out_state_id: *mut u64,
    ticks_out: *mut f64,
    ticks_cap: usize,
    out_ticks_len: *mut usize,
) -> MCStatus {
    ffi_status(|| {
        let Some(log) = (unsafe { log.as_ref() }) else {
            return MCStatus::Null;
        };
        let Some(out_ticks_len) = (unsafe { out_ticks_len.as_mut() }) else {
            return MCStatus::Null;
        };
        let Some(record) = log.inner.record(index) else {
            return invalid_argument("event log index is out of bounds");
        };
        let needed = record.ticks.len();
        *out_ticks_len = needed;
        if ticks_cap < needed {
            return buffer_too_small(needed, ticks_cap);
        }
        let Some(out_state_id) = (unsafe { out_state_id.as_mut() }) else {
            return MCStatus::Null;
        };
        let Some(ticks_out) = (unsafe { slice_from_mut_ptr(ticks_out, ticks_cap) }) else {
            return MCStatus::Null;
        };
        *out_state_id = record.state_id;
        ticks_out[..needed].copy_from_slice(&record.ticks);
        MCStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn mc_event_log_compact_summary(
    log: *const MCEventLog,
    tier: usize,
    idx_out: *mut usize,
    state_out: *mut u64,
    tick_out: *mut f64,
    cap: usize,
    out_len: *mut usize,
) -> MCStatus {
    ffi_status(|| {
        let Some(log) = (unsafe { log.as_ref() }) else {
            return MCStatus::Null;
        };
        let Some(out_len) = (unsafe { out_len.as_mut() }) else {
            return MCStatus::Null;
        };
        if tier >= log.inner.tier_count() {
            return invalid_argument("event log tier is out of bounds");
        }
        let summary = log.inner.compact_summary(tier);
        let needed = summary.len();
        *out_len = needed;
        if cap < needed {
            return buffer_too_small(needed, cap);
        }
        let Some(idx_out) = (unsafe { slice_from_mut_ptr(idx_out, cap) }) else {
            return MCStatus::Null;
        };
        let Some(state_out) = (unsafe { slice_from_mut_ptr(state_out, cap) }) else {
            return MCStatus::Null;
        };
        let Some(tick_out) = (unsafe { slice_from_mut_ptr(tick_out, cap) }) else {
            return MCStatus::Null;
        };
        for (offset, item) in summary.iter().enumerate() {
            idx_out[offset] = item.index;
            state_out[offset] = item.state_id;
            tick_out[offset] = item.tick;
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
pub unsafe extern "C" fn mc_event_log_tier_count(
    log: *const MCEventLog,
    out: *mut usize,
) -> MCStatus {
    ffi_status(|| {
        let Some(log) = (unsafe { log.as_ref() }) else {
            return MCStatus::Null;
        };
        let Some(out) = (unsafe { out.as_mut() }) else {
            return MCStatus::Null;
        };
        *out = log.inner.tier_count();
        MCStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn mc_event_log_is_empty(log: *const MCEventLog, out: *mut bool) -> MCStatus {
    ffi_status(|| {
        let Some(log) = (unsafe { log.as_ref() }) else {
            return MCStatus::Null;
        };
        let Some(out) = (unsafe { out.as_mut() }) else {
            return MCStatus::Null;
        };
        *out = log.inner.is_empty();
        MCStatus::Ok
    })
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
        let mut tier = MCTier {
            epsilon: 0.0,
            delta: 0.0,
            p: 0.0,
            epsilon_ref: 0.0,
        };
        assert_eq!(
            unsafe { mc_tier_new(0.5, 1.0, 0.5, 1.0, &mut tier) },
            MCStatus::Ok
        );
        assert_eq!(
            unsafe { mc_tier_new(1.0, 1.0, 0.0, 1.0, &mut tier) },
            MCStatus::InvalidArgument
        );
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

        let mut ladder = std::ptr::null_mut();
        assert_eq!(
            unsafe { mc_ladder_new(tiers.as_ptr(), tiers.len(), &mut ladder) },
            MCStatus::Ok
        );
        assert!(!ladder.is_null());
        let mut len = 0;
        assert_eq!(unsafe { mc_ladder_len(ladder, &mut len) }, MCStatus::Ok);
        assert_eq!(len, 2);
        let mut owned_values = [0.0; 2];
        assert_eq!(
            unsafe {
                mc_ladder_distance_owned(ladder, 1.2, owned_values.as_mut_ptr(), owned_values.len())
            },
            MCStatus::Ok
        );
        assert_eq!(values, owned_values);
        unsafe { mc_ladder_free(ladder) };
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

    #[test]
    fn ffi_coverage_meter_round_trip() {
        let epsilons = [0.1, 0.2];
        let mut meter: *mut MCCoverageMeter = ptr::null_mut();
        assert_eq!(
            unsafe {
                mc_coverage_meter_new(epsilons.as_ptr(), 2, 2, MC_METRIC_EUCLIDEAN, &mut meter)
            },
            MCStatus::Ok
        );
        assert!(!meter.is_null());

        let mut admitted = [false; 2];
        let mut out_len = 0usize;
        // first sample is always admitted at every tier
        assert_eq!(
            unsafe {
                mc_coverage_meter_observe(
                    meter,
                    [0.0, 0.0].as_ptr(),
                    2,
                    admitted.as_mut_ptr(),
                    2,
                    &mut out_len,
                )
            },
            MCStatus::Ok
        );
        assert_eq!(out_len, 2);
        assert_eq!(admitted, [true, true]);
        // 0.15 away: admitted at tier 0 (>= 0.1) but not tier 1 (< 0.2)
        assert_eq!(
            unsafe {
                mc_coverage_meter_observe(
                    meter,
                    [0.15, 0.0].as_ptr(),
                    2,
                    admitted.as_mut_ptr(),
                    2,
                    &mut out_len,
                )
            },
            MCStatus::Ok
        );
        assert_eq!(admitted, [true, false]);

        let mut counts = [0u64; 2];
        assert_eq!(
            unsafe { mc_coverage_meter_counts(meter, counts.as_mut_ptr(), 2, &mut out_len) },
            MCStatus::Ok
        );
        assert_eq!(counts, [2, 1]);

        let mut unique = 0u64;
        assert_eq!(
            unsafe { mc_coverage_meter_unique_representatives(meter, &mut unique) },
            MCStatus::Ok
        );
        assert_eq!(unique, 2);

        // wrong state dimension is a shape error
        assert_eq!(
            unsafe {
                mc_coverage_meter_observe(
                    meter,
                    [0.0].as_ptr(),
                    1,
                    admitted.as_mut_ptr(),
                    2,
                    &mut out_len,
                )
            },
            MCStatus::InvalidArgument
        );
        unsafe { mc_coverage_meter_free(meter) };

        // invalid constructions
        let mut bad: *mut MCCoverageMeter = ptr::null_mut();
        assert_eq!(
            unsafe {
                mc_coverage_meter_new(epsilons.as_ptr(), 2, 0, MC_METRIC_EUCLIDEAN, &mut bad)
            },
            MCStatus::InvalidArgument
        );
        assert_eq!(
            unsafe { mc_coverage_meter_new(epsilons.as_ptr(), 2, 3, MC_METRIC_ABSOLUTE, &mut bad) },
            MCStatus::InvalidArgument
        );

        // callback-metric constructor: Chebyshev distinguishes itself from
        // euclidean on the pair ((0,0), (0.05, 0.09)): euclidean ~0.103 would
        // admit at eps=0.1, chebyshev 0.09 must reject
        unsafe extern "C" fn chebyshev(
            a: *const f64,
            b: *const f64,
            dim: usize,
            _user_data: *mut c_void,
        ) -> f64 {
            let a = unsafe { std::slice::from_raw_parts(a, dim) };
            let b = unsafe { std::slice::from_raw_parts(b, dim) };
            a.iter()
                .zip(b)
                .map(|(left, right)| (left - right).abs())
                .fold(0.0, f64::max)
        }
        let mut cb_meter: *mut MCCoverageMeter = ptr::null_mut();
        assert_eq!(
            unsafe {
                mc_coverage_meter_new_with_callback(
                    [0.1].as_ptr(),
                    1,
                    2,
                    Some(chebyshev),
                    ptr::null_mut(),
                    &mut cb_meter,
                )
            },
            MCStatus::Ok
        );
        let mut flag = [false; 1];
        assert_eq!(
            unsafe {
                mc_coverage_meter_observe(
                    cb_meter,
                    [0.0, 0.0].as_ptr(),
                    2,
                    flag.as_mut_ptr(),
                    1,
                    &mut out_len,
                )
            },
            MCStatus::Ok
        );
        assert_eq!(
            unsafe {
                mc_coverage_meter_observe(
                    cb_meter,
                    [0.05, 0.09].as_ptr(),
                    2,
                    flag.as_mut_ptr(),
                    1,
                    &mut out_len,
                )
            },
            MCStatus::Ok
        );
        assert_eq!(flag, [false], "chebyshev 0.09 < 0.1 must reject");
        unsafe { mc_coverage_meter_free(cb_meter) };
        // a null callback is rejected
        assert_eq!(
            unsafe {
                mc_coverage_meter_new_with_callback(
                    [0.1].as_ptr(),
                    1,
                    2,
                    None,
                    ptr::null_mut(),
                    &mut cb_meter,
                )
            },
            MCStatus::Null
        );

        // pure helpers
        assert_eq!(mc_classify_regime(0.0, 0), MC_REGIME_QUIESCENT);
        assert_eq!(mc_classify_regime(1.0, 1), MC_REGIME_PROGRESS);
        assert_eq!(mc_classify_regime(1.0, 0), MC_REGIME_CHURN);
        assert_eq!(mc_classify_regime(0.0, 1), MC_REGIME_CREEP);
        let mut efficiency = -1.0;
        assert_eq!(
            unsafe { mc_progress_efficiency(11, 0.1, 2.0, &mut efficiency) },
            MCStatus::Ok
        );
        assert!((efficiency - 0.5).abs() < 1e-12);
        assert_eq!(
            unsafe { mc_progress_efficiency(11, 0.1, 0.0, &mut efficiency) },
            MCStatus::InvalidArgument
        );
    }
}
