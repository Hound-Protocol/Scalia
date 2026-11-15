//! Data-driven interaction metadata for world entities.
//!
//! An entity opts into contextual prompts by implementing `Interactable`. The viewer
//! gathers those declarations; the HTML layer never needs object-specific prompt logic.

use scalia_ai::doors::{Door, DoorState};

#[derive(Clone, Debug)]
pub struct HintData {
    pub id: String,
    pub action: String,
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

/// Implement this for an entity to make it eligible for automatic interaction hints.
pub trait Interactable {
    fn interaction_hint(&self, id: String, player: (f64, f64)) -> Option<HintData>;
}

/// Static world target used for interactables that are not stored as a dedicated entity yet.
pub struct JukeboxTarget {
    pub at_mm: (i64, i64),
}

fn make_hint(
    id: String,
    action: &str,
    caption: &str,
    at_mm: (i64, i64),
    player: (f64, f64),
    radius: f64,
    priority: i32,
    vertical_offset: f32,
) -> Option<HintData> {
    let (x, y) = (at_mm.0 as f64 / 1000.0, at_mm.1 as f64 / 1000.0);
    let distance = (x - player.0).hypot(y - player.1);
    if distance > radius {
        return None;
    }
    Some(HintData {
        id,
        action: action.into(),
        target_transform: [x, y, 0.0],
        local_vertical_offset: vertical_offset,
        caption_text: caption.into(),
        input_token: "{+use}".into(),
        on_screen_icon: "keycap".into(),
        off_screen_icon: "arrow".into(),
        trigger_radius: radius,
        clamp_to_screen: true,
        show_offscreen_arrow: false,
        priority,
        allow_multiple: false,
        distance,
    })
}

impl Interactable for Door {
    fn interaction_hint(&self, id: String, player: (f64, f64)) -> Option<HintData> {
        let caption = match self.state {
            DoorState::Closed => "OPEN DOOR",
            DoorState::Open => "CLOSE DOOR",
            DoorState::Locked | DoorState::Breached => return None,
        };
        make_hint(id, "door", caption, self.at, player, 1.8, 80, 1.15)
    }
}

impl Interactable for JukeboxTarget {
    fn interaction_hint(&self, id: String, player: (f64, f64)) -> Option<HintData> {
        make_hint(id, "jukebox", "USE JUKEBOX", self.at_mm, player, 2.0, 100, 1.0)
    }
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
            action: "use".into(),
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

    #[test]
    fn locked_and_breached_doors_do_not_advertise_use() {
        let locked = Door::new((0, 0), true, 900, 2_000, DoorState::Locked);
        let breached = Door::new((0, 0), true, 900, 2_000, DoorState::Breached);
        assert!(locked.interaction_hint("locked".into(), (0.0, 0.0)).is_none());
        assert!(breached.interaction_hint("breached".into(), (0.0, 0.0)).is_none());
    }

    #[test]
    fn door_caption_tracks_its_state() {
        let closed = Door::new((0, 0), true, 900, 2_000, DoorState::Closed);
        let open = Door::new((0, 0), true, 900, 2_000, DoorState::Open);
        assert_eq!(closed.interaction_hint("door".into(), (0.0, 0.0)).unwrap().caption_text, "OPEN DOOR");
        assert_eq!(open.interaction_hint("door".into(), (0.0, 0.0)).unwrap().caption_text, "CLOSE DOOR");
    }
}
