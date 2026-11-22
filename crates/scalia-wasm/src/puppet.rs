//! Model viewer, Commander mode (viewer.rs `set_commander`): the soldier driven by keys.
use crate::motion::{CMD_AIM_WALK, CMD_CRAWL, CMD_CRAWL_FAST, CMD_CROUCH, CMD_FEET, CMD_JOG, CMD_SPRINT, CMD_WALK};

/// Model viewer, Commander mode: the soldier on the plinth driven by keys through the game's animation library
/// (anim.rs) and the commander's footing (motion.rs), on a treadmill: he moves his legs, not his place.
#[derive(Default)]
pub struct Puppet {
    pub keys: (f32, f32),
    pub sprint: bool,
    pub walk: bool,
    pub aim: bool,
    pub stance: crate::anim::Stance,
    fig: crate::anim::Animator,
    vel: (f32, f32),
    last: Option<f32>,
}

impl Puppet {
    /// One frame at clock `ts` (s): the instance record to draw and a line saying what he is doing.
    pub fn step(&mut self, ts: f32, colour: f32) -> ([f32; 12], String) {
        use crate::anim::{Drive, Face, Rifle, Stance};
        let dt = (ts - self.last.unwrap_or(ts)).clamp(0.0, 0.1);
        self.last = Some(ts);
        let dropping = self.fig.dropping();
        let len = (self.keys.0 * self.keys.0 + self.keys.1 * self.keys.1).sqrt();
        let wants = len > 0.01 && !dropping;
        let pace = match self.stance {
            Stance::Prone if self.sprint && !self.aim => CMD_CRAWL_FAST,
            Stance::Prone => CMD_CRAWL,
            Stance::Crouch => CMD_CROUCH,
            Stance::Stand if self.aim => CMD_AIM_WALK,
            Stance::Stand if self.sprint => CMD_SPRINT,
            Stance::Stand if self.walk => CMD_WALK,
            Stance::Stand => CMD_JOG,
        };
        let want = if wants { (self.keys.0 / len * pace, self.keys.1 / len * pace) } else { (0.0, 0.0) };
        self.vel = if dropping { crate::motion::drop_slide(self.vel, dt) } else { crate::motion::step_velocity(self.vel, want, dt, CMD_FEET) };
        let speed = (self.vel.0 * self.vel.0 + self.vel.1 * self.vel.1).sqrt();
        let mut d = Drive::new(dt, ts);
        d.vel = self.vel;
        d.want = Some(want);
        // Rifle up he keeps facing ahead (strafing, backpedalling); otherwise he turns the way he goes, as in game.
        d.face = if self.aim { Face::Aim { heading: 0.0 } } else if speed > 0.05 { Face::Turn { heading: (-self.vel.0).atan2(self.vel.1), top: 6.0 - 2.0 * (speed / CMD_SPRINT).min(1.0) } } else { Face::Keep };
        d.stance = self.stance;
        d.rifle = if self.aim { Rifle::Aim } else { Rifle::Ready };
        d.sprint = !self.aim;
        let f = self.fig.step(&d);
        let rec = self.fig.record(&f, (0.0, 0.0, 0.0), colour, 0.0, 0.0, 0.0);
        (rec, format!("{:?}, {:.1} m/s, {:?}", f.pose, speed, self.stance))
    }
}

#[cfg(test)]
mod tests {
    use super::Puppet;
    use crate::anim::Stance;

    /// The model viewer's Commander mode drives the soldier through the game's own gaits and drop.
    #[test]
    fn the_viewer_commander_walks_sprints_and_drops_to_prone() {
        let mut c = Puppet::default();
        let mut ts = 0.0;
        let mut run = |c: &mut Puppet, secs: f32| {
            let mut line = String::new();
            for _ in 0..(secs * 60.0) as usize {
                ts += 1.0 / 60.0;
                line = c.step(ts, 0.0).1;
            }
            line
        };
        (c.keys, c.walk) = ((0.0, 1.0), true);
        assert!(run(&mut c, 2.0).starts_with("Walk"), "W walks");
        (c.walk, c.sprint) = (false, true);
        assert!(run(&mut c, 2.0).starts_with("Sprint"), "shift sprints");
        (c.keys, c.sprint, c.stance) = ((0.0, 0.0), false, Stance::Prone);
        assert!(run(&mut c, 0.6).starts_with("Drop"), "Z drops through the game's drop");
        assert!(run(&mut c, 2.5).starts_with("Prone"), "and lies flat");
    }
}
