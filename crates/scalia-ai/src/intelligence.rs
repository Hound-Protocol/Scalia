//! Titan Intelligence: shared, time-stamped battlefield reports.
//!
//! This is an additive intelligence contract. Battle's existing squad-local Contact
//! model remains authoritative for tactical execution until a later integration work order.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct IntelKey {
    pub team: u8,
    pub enemy: usize,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntelSource {
    Visual,
    IncomingFire,
    Audio,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntelReport {
    pub key: IntelKey,
    pub position_mm: (i64, i64),
    /// Simulation time at which the enemy was observed or inferred.
    pub observed_at_ms: i64,
    /// Confidence at observation time, 0..=1000.
    pub confidence: u16,
    pub source: IntelSource,
    pub reporting_squad: u16,
}

impl IntelReport {
    /// Confidence after linear decay. Future-dated reports are not exposed.
    pub fn confidence_at(&self, now_ms: i64, fade_per_second: u16) -> Option<u16> {
        let age = now_ms.checked_sub(self.observed_at_ms)?;
        let decay = (age / 1_000).saturating_mul(fade_per_second as i64);
        Some((self.confidence as i64 - decay).clamp(0, 1_000) as u16)
    }
}

/// One best-known report per team/enemy pair. BTreeMap provides stable iteration order.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TitanIntelligence {
    reports: BTreeMap<IntelKey, IntelReport>,
}

impl TitanIntelligence {
    /// Accept a report only when it is newer, or equally recent and more confident.
    /// Confidence is clamped to the documented 0..=1000 range.
    pub fn publish(&mut self, mut report: IntelReport) -> bool {
        report.confidence = report.confidence.min(1_000);
        match self.reports.get(&report.key) {
            Some(old)
                if report.observed_at_ms < old.observed_at_ms
                    || (report.observed_at_ms == old.observed_at_ms
                        && report.confidence <= old.confidence) => false,
            _ => {
                self.reports.insert(report.key, report);
                true
            }
        }
    }

    pub fn get(&self, team: u8, enemy: usize) -> Option<&IntelReport> {
        self.reports.get(&IntelKey { team, enemy })
    }

    /// Deterministically ordered reports for one friendly team.
    pub fn for_team(&self, team: u8) -> impl Iterator<Item = &IntelReport> {
        self.reports.values().filter(move |r| r.key.team == team)
    }

    /// Remove reports older than the caller's retention window.
    pub fn expire_before(&mut self, cutoff_ms: i64) {
        self.reports.retain(|_, report| report.observed_at_ms >= cutoff_ms);
    }

    pub fn len(&self) -> usize {
        self.reports.len()
    }

    pub fn is_empty(&self) -> bool {
        self.reports.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(team: u8, enemy: usize, at: i64, confidence: u16, squad: u16) -> IntelReport {
        IntelReport {
            key: IntelKey { team, enemy },
            position_mm: (10_000, 20_000),
            observed_at_ms: at,
            confidence,
            source: IntelSource::Visual,
            reporting_squad: squad,
        }
    }

    #[test]
    fn newer_report_replaces_older_and_old_report_cannot_rewind_intel() {
        let mut intel = TitanIntelligence::default();
        assert!(intel.publish(report(1, 8, 2_000, 700, 2)));
        assert!(intel.publish(report(1, 8, 3_000, 300, 4)));
        assert!(!intel.publish(report(1, 8, 2_500, 1_000, 1)));
        assert_eq!(intel.get(1, 8).unwrap().observed_at_ms, 3_000);
        assert_eq!(intel.get(1, 8).unwrap().reporting_squad, 4);
    }

    #[test]
    fn equal_timestamp_requires_stronger_confidence() {
        let mut intel = TitanIntelligence::default();
        assert!(intel.publish(report(1, 8, 2_000, 500, 1)));
        assert!(!intel.publish(report(1, 8, 2_000, 400, 2)));
        assert!(intel.publish(report(1, 8, 2_000, 600, 2)));
    }

    #[test]
    fn confidence_decays_and_future_reports_are_hidden() {
        let r = report(1, 8, 1_000, 800, 1);
        assert_eq!(r.confidence_at(4_000, 100), Some(500));
        assert_eq!(r.confidence_at(12_000, 100), Some(0));
        assert_eq!(r.confidence_at(0, 100), None);
    }

    #[test]
    fn teams_are_isolated_and_expiry_is_deterministic() {
        let mut intel = TitanIntelligence::default();
        intel.publish(report(1, 8, 1_000, 800, 1));
        intel.publish(report(2, 8, 2_000, 800, 3));
        assert_eq!(intel.for_team(1).count(), 1);
        assert_eq!(intel.for_team(2).count(), 1);
        intel.expire_before(1_500);
        assert!(intel.get(1, 8).is_none());
        assert!(intel.get(2, 8).is_some());
    }
}
