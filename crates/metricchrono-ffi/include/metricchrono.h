#ifndef METRICCHRONO_H
#define METRICCHRONO_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#define METRICCHRONO_ABI_VERSION 1

#ifdef __cplusplus
extern "C" {
#endif

typedef enum MCStatus {
  MC_STATUS_OK = 0,
  MC_STATUS_NULL = 1,
  MC_STATUS_INVALID_ARGUMENT = 2,
  MC_STATUS_BUFFER_TOO_SMALL = 3,
  MC_STATUS_PANIC = 255
} MCStatus;

typedef enum MCMetricId {
  MC_METRIC_EUCLIDEAN = 0,
  MC_METRIC_ABSOLUTE = 1
} MCMetricId;

typedef enum MCNormalization {
  MC_NORMALIZATION_NONE = 0,
  MC_NORMALIZATION_UNIT_MAX = 1,
  MC_NORMALIZATION_TANH = 2
} MCNormalization;

typedef struct MCTier {
  double epsilon;
  double delta;
  double p;
  double epsilon_ref;
} MCTier;

typedef struct MCZoomDecision {
  size_t evaluated_tiers;
  size_t first_inactive_tier;
  bool has_first_inactive_tier;
  bool stopped_early;
} MCZoomDecision;

typedef struct MCLadder MCLadder;
typedef struct MCEventLog MCEventLog;
typedef struct MCPromotionCounter MCPromotionCounter;

const char *mc_error_message(MCStatus status);
MCStatus mc_last_error_message(char *buf, size_t cap, size_t *out_len);
MCStatus mc_tier_new(double epsilon,
                     double delta,
                     double p,
                     double epsilon_ref,
                     MCTier *out);
MCStatus mc_ladder_new(const MCTier *tiers, size_t len, MCLadder **out);
MCStatus mc_custom_ladder(const MCTier *tiers, size_t len, MCLadder **out);
void mc_ladder_free(MCLadder *ladder);
MCStatus mc_ladder_len(const MCLadder *ladder, size_t *out_len);
MCStatus mc_validate_ladder(const MCLadder *ladder);
MCStatus mc_ladder_distance_owned(const MCLadder *ladder,
                                  double distance,
                                  double *out,
                                  size_t out_len);

MCStatus mc_tick_distance(double distance, MCTier tier, double *out);
MCStatus mc_euclidean_distance(const double *a,
                               const double *b,
                               size_t len,
                               double *out);
MCStatus mc_absolute_distance(const double *a,
                              const double *b,
                              size_t len,
                              double *out);
MCStatus mc_tick_pair(MCMetricId metric_id,
                      const double *a,
                      const double *b,
                      size_t len,
                      MCTier tier,
                      double *out);
MCStatus mc_ladder_distance(double distance,
                            const MCTier *tiers,
                            size_t len,
                            double *out,
                            size_t out_len);
MCStatus mc_ladder_pair(MCMetricId metric_id,
                        const double *a,
                        const double *b,
                        size_t len,
                        const MCLadder *ladder,
                        double *out,
                        size_t cap,
                        size_t *out_len);
MCStatus mc_adaptive_ladder_distance(double distance,
                                     const MCTier *tiers,
                                     size_t len,
                                     double *out,
                                     size_t out_len,
                                     MCZoomDecision *decision);
MCStatus mc_smooth_tick_distance(double distance,
                                 MCTier tier,
                                 double sharpness,
                                 double *out);
MCStatus mc_geometric_ladder(double epsilon0,
                             double delta0,
                             double ratio,
                             size_t tiers,
                             double p,
                             double epsilon_ref,
                             MCTier *out,
                             size_t out_len);
MCStatus mc_normalize_ticks(const double *ticks,
                            size_t len,
                            MCNormalization normalization_id,
                            double *out);
MCStatus mc_carry_rules(const double *epsilons,
                        size_t len,
                        uint64_t *out,
                        size_t cap,
                        size_t *out_len);
MCStatus mc_weighted_consensus(const double *vectors,
                               size_t rows,
                               size_t cols,
                               const double *weights,
                               double *out,
                               size_t out_len);
MCStatus mc_simple_weight_update(double *weights,
                                 const double *residuals,
                                 size_t len,
                                 double learning_rate,
                                 double floor);

MCStatus mc_promotion_counter_new(const uint64_t *quotas,
                                  size_t len,
                                  MCPromotionCounter **out);
MCStatus mc_promotion_counter_from_epsilons(const double *epsilons,
                                            size_t len,
                                            MCPromotionCounter **out);
MCStatus mc_promotion_counter_step(MCPromotionCounter *counter,
                                   const bool *event_flags,
                                   size_t flags_len,
                                   bool *out,
                                   size_t cap,
                                   size_t *out_len);
MCStatus mc_promotion_counter_counters(const MCPromotionCounter *counter,
                                       uint64_t *out,
                                       size_t cap,
                                       size_t *out_len);
MCStatus mc_promotion_counter_quotas(const MCPromotionCounter *counter,
                                     uint64_t *out,
                                     size_t cap,
                                     size_t *out_len);
MCStatus mc_promotion_counter_reset(MCPromotionCounter *counter);
void mc_promotion_counter_free(MCPromotionCounter *counter);

MCEventLog *mc_event_log_new(size_t tier_count);
void mc_event_log_free(MCEventLog *log);
MCStatus mc_event_log_append(MCEventLog *log,
                             uint64_t state_id,
                             const double *ticks,
                             size_t len,
                             size_t *out_index);
MCStatus mc_event_log_first_event(const MCEventLog *log,
                                  size_t tier,
                                  size_t *out_index,
                                  bool *out_has);
MCStatus mc_event_log_next_event(const MCEventLog *log,
                                 size_t index,
                                 size_t tier,
                                 size_t *out_index,
                                 bool *has_event);
MCStatus mc_event_log_record(const MCEventLog *log,
                             size_t index,
                             uint64_t *out_state_id,
                             double *ticks_out,
                             size_t ticks_cap,
                             size_t *out_ticks_len);
MCStatus mc_event_log_compact_summary(const MCEventLog *log,
                                      size_t tier,
                                      size_t *idx_out,
                                      uint64_t *state_out,
                                      double *tick_out,
                                      size_t cap,
                                      size_t *out_len);
MCStatus mc_event_log_len(const MCEventLog *log, size_t *out_len);
MCStatus mc_event_log_tier_count(const MCEventLog *log, size_t *out);
MCStatus mc_event_log_is_empty(const MCEventLog *log, bool *out);

double mc_tick_distance_raw(double distance,
                            double epsilon,
                            double delta,
                            double p,
                            double epsilon_ref);

#ifdef __cplusplus
}
#endif

#endif
