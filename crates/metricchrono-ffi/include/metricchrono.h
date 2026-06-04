#ifndef METRICCHRONO_H
#define METRICCHRONO_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

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

const char *mc_error_message(MCStatus status);
MCStatus mc_tier_new(double epsilon,
                     double delta,
                     double p,
                     double epsilon_ref,
                     MCTier *out);
MCStatus mc_ladder_new(const MCTier *tiers, size_t len, MCLadder **out);
void mc_ladder_free(MCLadder *ladder);
MCStatus mc_ladder_len(const MCLadder *ladder, size_t *out_len);
MCStatus mc_ladder_distance_owned(const MCLadder *ladder,
                                  double distance,
                                  double *out,
                                  size_t out_len);

MCStatus mc_tick_distance(double distance, MCTier tier, double *out);
MCStatus mc_ladder_distance(double distance,
                            const MCTier *tiers,
                            size_t len,
                            double *out,
                            size_t out_len);
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

MCEventLog *mc_event_log_new(size_t tier_count);
void mc_event_log_free(MCEventLog *log);
MCStatus mc_event_log_append(MCEventLog *log,
                             uint64_t state_id,
                             const double *ticks,
                             size_t len,
                             size_t *out_index);
MCStatus mc_event_log_next_event(const MCEventLog *log,
                                 size_t index,
                                 size_t tier,
                                 size_t *out_index,
                                 bool *has_event);
MCStatus mc_event_log_len(const MCEventLog *log, size_t *out_len);

double mc_tick_distance_raw(double distance,
                            double epsilon,
                            double delta,
                            double p,
                            double epsilon_ref);

#ifdef __cplusplus
}
#endif

#endif
