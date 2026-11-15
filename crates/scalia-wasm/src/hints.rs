//! Data-driven interaction hint metadata shared by world targets and the presentation layer.

/// Metadata published by an active world entity when the player can interact with it.
/// World coordinates are in metres; the local vertical offset is added above the ground anchor.
#[derive(Clone, Debug)]
pub struct HintData {
    pub id: String,
    pub target_transform: [f64; 3],
    pub local_vertical_offset: f32,
    pub caption_text: String,
    pub input_token: String,
    pub on_screen_icon: String,
    pub off_screen_icon: String,
    pub trigger_radius: f64,
    pub clamp_to_screen: bool,
    pub show_offscreen_arrow: bool,
    pub priority: i32,
    pub allow_multiple: bool,
    pub distance: f64,
}

/// Keep the highest-priority active target. Multiple prompts are opt-in per target.
pub fn prioritize(mut hints: Vec<HintData>) -> Vec<HintData> {
    hints.sort_by(|a, b| b.priority.cmp(&a.priority).then_with(|| a.distance.total_cmp(&b.distance)));
    if !hints.first().is_some_and(|hint| hint.allow_multiple) {
        hints.truncate(1);
    }
    hints
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hint(id: &str, priority: i32, distance: f64, allow_multiple: bool) -> HintData {
        HintData {
            id: id.into(),
            target_transform: [0.0, 0.0, 0.0],
            local_vertical_offset: 1.0,
            caption_text: id.into(),
            input_token: "{+use}".into(),
            on_screen_icon: "keycap".into(),
            off_screen_icon: "arrow".into(),
            trigger_radius: 2.0,
            clamp_to_screen: true,
            show_offscreen_arrow: false,
            priority,
            allow_multiple,
            distance,
        }
    }

    #[test]
    fn highest_priority_wins_when_multiple_is_not_enabled() {
        let active = prioritize(vec![hint("door", 50, 0.5, false), hint("jukebox", 100, 1.5, false)]);
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, "jukebox");
    }

    #[test]
    fn nearest_target_breaks_priority_ties() {
        let active = prioritize(vec![hint("far", 50, 1.5, false), hint("near", 50, 0.5, false)]);
        assert_eq!(active[0].id, "near");
    }

    #[test]
    fn multiple_targets_are_kept_when_explicitly_allowed() {
        let active = prioritize(vec![hint("one", 50, 0.5, true), hint("two", 40, 1.0, false)]);
        assert_eq!(active.len(), 2);
    }
}
