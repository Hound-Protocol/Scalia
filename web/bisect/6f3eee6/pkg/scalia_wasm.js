/* @ts-self-types="./scalia_wasm.d.ts" */

/**
 * A flight from the parking spot at one body to the parking spot at another.
 */
export class Flight {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        FlightFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_flight_free(ptr, 0);
    }
    /**
     * @returns {number}
     */
    duration_ms() {
        const ret = wasm.flight_duration_ms(this.__wbg_ptr);
        return ret;
    }
    /**
     * @returns {Float64Array}
     */
    from() {
        const ret = wasm.flight_from(this.__wbg_ptr);
        var v1 = getArrayF64FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 8, 8);
        return v1;
    }
    /**
     * @param {number} from_body
     * @param {number} to_body
     * @param {number} depart_ms
     * @param {number} peak_mm_s
     */
    constructor(from_body, to_body, depart_ms, peak_mm_s) {
        const ret = wasm.flight_new(from_body, to_body, depart_ms, peak_mm_s);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        this.__wbg_ptr = ret[0];
        FlightFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * `[x_mm, y_mm]` at time `t_ms`.
     * @param {number} t_ms
     * @returns {Float64Array}
     */
    pos(t_ms) {
        const ret = wasm.flight_pos(this.__wbg_ptr, t_ms);
        var v1 = getArrayF64FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 8, 8);
        return v1;
    }
    /**
     * @returns {Float64Array}
     */
    to() {
        const ret = wasm.flight_to(this.__wbg_ptr);
        var v1 = getArrayF64FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 8, 8);
        return v1;
    }
}
if (Symbol.dispose) Flight.prototype[Symbol.dispose] = Flight.prototype.free;

export class Sim {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        SimFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_sim_free(ptr, 0);
    }
    /**
     * Fills the output buffers and returns how many individuals were derived.
     * Positions are `f32` metres relative to the view centre.
     * @param {number} t_ms
     * @param {number} region
     * @param {number} min_x
     * @param {number} max_x
     * @param {number} min_y
     * @param {number} max_y
     * @param {number} cap
     * @returns {number}
     */
    derive(t_ms, region, min_x, max_x, min_y, max_y, cap) {
        const ret = wasm.sim_derive(this.__wbg_ptr, t_ms, region, min_x, max_x, min_y, max_y, cap);
        return ret >>> 0;
    }
    /**
     * @param {bigint} seed
     * @param {number} groups
     * @param {number} per_group
     */
    constructor(seed, groups, per_group) {
        const ret = wasm.sim_new(seed, groups, per_group);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        this.__wbg_ptr = ret[0];
        SimFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * @returns {Uint8Array}
     */
    phases() {
        const ret = wasm.sim_phases(this.__wbg_ptr);
        var v1 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        return v1;
    }
    /**
     * @returns {number}
     */
    phases_ptr() {
        const ret = wasm.sim_phases_ptr(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @param {number} now_ms
     */
    tick(now_ms) {
        const ret = wasm.sim_tick(this.__wbg_ptr, now_ms);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * @returns {Float32Array}
     */
    xs() {
        const ret = wasm.sim_xs(this.__wbg_ptr);
        var v1 = getArrayF32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * @returns {number}
     */
    xs_ptr() {
        const ret = wasm.sim_xs_ptr(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @returns {Float32Array}
     */
    ys() {
        const ret = wasm.sim_ys(this.__wbg_ptr);
        var v1 = getArrayF32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * @returns {number}
     */
    ys_ptr() {
        const ret = wasm.sim_ys_ptr(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @returns {Float32Array}
     */
    zs() {
        const ret = wasm.sim_zs(this.__wbg_ptr);
        var v1 = getArrayF32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * @returns {number}
     */
    zs_ptr() {
        const ret = wasm.sim_zs_ptr(this.__wbg_ptr);
        return ret >>> 0;
    }
}
if (Symbol.dispose) Sim.prototype[Symbol.dispose] = Sim.prototype.free;

export class Viewer {
    static __wrap(ptr) {
        const obj = Object.create(Viewer.prototype);
        obj.__wbg_ptr = ptr;
        ViewerFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        ViewerFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_viewer_free(ptr, 0);
    }
    /**
     * What the selected units can do at a map point, as JSON [{id, label, ok, reason}]. Each unit's AI checks
     * the ground: inside the testing ground it must find a way there; what the radar shows, they know.
     * @param {number} x
     * @param {number} y
     * @returns {string}
     */
    actions_at(x, y) {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.viewer_actions_at(this.__wbg_ptr, x, y);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @returns {string}
     */
    assault_state() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.viewer_assault_state(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Puts a squad under a captain (captain -1: it answers to the commander again).
     * @param {number} squad
     * @param {number} captain
     * @returns {string}
     */
    attach(squad, captain) {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.viewer_attach(this.__wbg_ptr, squad, captain);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Attaches the tablet's map canvas: drawn with the map camera while the tablet is up.
     * @param {HTMLCanvasElement} canvas
     */
    attach_map(canvas) {
        const ret = wasm.viewer_attach_map(this.__wbg_ptr, canvas);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * The space at the listener's spot on the ground (ambience and the console):
     * [enclosure 0..1, size (m), reverb time (s), corridor 0..1, through the tablet 0/1].
     * @returns {Float32Array}
     */
    audio_env() {
        const ret = wasm.viewer_audio_env(this.__wbg_ptr);
        var v1 = getArrayF32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * Console: everyone on `team` (0 ours, 1 the enemy) drops dead; how many.
     * @param {number} team
     * @returns {number}
     */
    battle_kill(team) {
        const ret = wasm.viewer_battle_kill(this.__wbg_ptr, team);
        return ret >>> 0;
    }
    /**
     * Benchmark scene: `squads` squads of 12 around the commander (see `bench_army`). Returns the soldier count.
     * @param {number} squads
     * @returns {number}
     */
    bench_army(squads) {
        const ret = wasm.viewer_bench_army(this.__wbg_ptr, squads);
        return ret >>> 0;
    }
    /**
     * Left click on the ground: selects the group under the cursor (or clears the selection).
     * @param {number} px
     * @param {number} py
     * @param {number} now_ms
     * @returns {string}
     */
    click(px, py, now_ms) {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.viewer_click(this.__wbg_ptr, px, py, now_ms);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Right click on the ground: the selected group marches there (redirecting if already marching).
     * @param {number} px
     * @param {number} py
     * @param {number} now_ms
     * @param {boolean} run
     * @returns {string}
     */
    command(px, py, now_ms, run) {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.viewer_command(this.__wbg_ptr, px, py, now_ms, run);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Takes command from a unit: the commander moves to stand with it, facing the way it faces.
     * @param {number} id
     * @returns {string}
     */
    command_unit(id) {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.viewer_command_unit(this.__wbg_ptr, id);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @returns {number}
     */
    commander_gait() {
        const ret = wasm.viewer_commander_gait(this.__wbg_ptr);
        return ret;
    }
    /**
     * Console: rounds pass through him harmlessly (true) or not; returns the new setting.
     * @returns {boolean}
     */
    commander_god() {
        const ret = wasm.viewer_commander_god(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Console: back on his feet at full health.
     */
    commander_heal() {
        wasm.viewer_commander_heal(this.__wbg_ptr);
    }
    /**
     * Slings the rifle on his back, or brings it back to his hands.
     * @returns {boolean}
     */
    commander_holster() {
        const ret = wasm.viewer_commander_holster(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Console: where the commander is and what he is doing.
     * @returns {string}
     */
    commander_info() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.viewer_commander_info(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @returns {number}
     */
    commander_phase() {
        const ret = wasm.viewer_commander_phase(this.__wbg_ptr);
        return ret;
    }
    /**
     * Console: a full magazine and `mags` spares.
     * @param {number} mags
     */
    commander_refill(mags) {
        wasm.viewer_commander_refill(this.__wbg_ptr, mags);
    }
    /**
     * A fresh magazine, if he has a spare.
     */
    commander_reload() {
        wasm.viewer_commander_reload(this.__wbg_ptr);
    }
    /**
     * The commander's gait phase (0..1), or -1 when he stands: a foot lands at 0.25 (left) and 0.75 (right).
     * Where the commander is on screen (fractions of the width and height from the top left; his chest).
     * @returns {Float32Array}
     */
    commander_screen() {
        const ret = wasm.viewer_commander_screen(this.__wbg_ptr);
        var v1 = getArrayF32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * Crouch (1) or go prone (2), or back up to standing when already so.
     * @param {number} k
     */
    commander_stance(k) {
        wasm.viewer_commander_stance(this.__wbg_ptr, k);
    }
    /**
     * One footfall of the commander's, `loud` 0..1, as one record of `take_sounds`: the room he is in rings
     * with it and nearby walls throw it back, like the soldiers' steps.
     * @param {number} loud
     * @returns {Float32Array}
     */
    commander_step(loud) {
        const ret = wasm.viewer_commander_step(this.__wbg_ptr, loud);
        var v1 = getArrayF32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * The commander's gait for his footsteps: 0 standing, 1 walking, 2 sprinting.
     * What the commander walks on: 1 concrete, 0 grass.
     * @returns {number}
     */
    commander_surface() {
        const ret = wasm.viewer_commander_surface(this.__wbg_ptr);
        return ret;
    }
    /**
     * Console: puts the commander at (x, y) metres on the ground (his man in the battle follows next frame).
     * @param {number} x
     * @param {number} y
     */
    commander_teleport(x, y) {
        wasm.viewer_commander_teleport(this.__wbg_ptr, x, y);
    }
    /**
     * Alt held: the commander walks instead of jogging.
     * @param {boolean} walk
     */
    commander_walk(walk) {
        wasm.viewer_commander_walk(this.__wbg_ptr, walk);
    }
    /**
     * The commander's trigger and aim (held or not), and where the mouse is (device px).
     * @param {boolean} trigger
     * @param {boolean} aim
     * @param {number} px
     * @param {number} py
     */
    commander_weapon(trigger, aim, px, py) {
        wasm.viewer_commander_weapon(this.__wbg_ptr, trigger, aim, px, py);
    }
    /**
     * The commander's weapon for the page, as JSON: rounds in the rifle and spare, reloading, slung, stance,
     * rounds fired so far, health (0..100), and the aim line [x0, y0, x1, y1] (screen fractions) while aiming.
     * @returns {string}
     */
    commander_weapon_state() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.viewer_commander_weapon_state(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Cycles the army size; returns a description.
     * @returns {string}
     */
    cycle_army() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.viewer_cycle_army(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Cycles the speed of the day: 60-minute days (the game's real pace), 12-second days (to watch), paused.
     * @param {number} now_ms
     * @returns {string}
     */
    cycle_day_speed(now_ms) {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.viewer_cycle_day_speed(this.__wbg_ptr, now_ms);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Cycles the selected group's idle stance: stand, crouch, cover aim, prone, standing aim.
     * @returns {string}
     */
    cycle_stance() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.viewer_cycle_stance(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Drops one unit from the selection (it did not answer the radio).
     * @param {number} id
     */
    deselect(id) {
        wasm.viewer_deselect(this.__wbg_ptr, id);
    }
    /**
     * Console: list the doors, or set one ("door 3 open|close|lock|breach").
     * @param {number} which
     * @param {string} state
     * @returns {string}
     */
    door_cmd(which, state) {
        let deferred3_0;
        let deferred3_1;
        try {
            const val2 = +which;
            const ptr0 = passStringToWasm0(state, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.viewer_door_cmd(this.__wbg_ptr, val2, ptr0, len0);
            deferred3_0 = ret[0];
            deferred3_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * The intro is over: the camera is the player's again, at the widest field zoom.
     */
    end_intro() {
        wasm.viewer_end_intro(this.__wbg_ptr);
    }
    /**
     * The field battle in words (debugging: squads, positions, steps).
     * @returns {string}
     */
    field_debug() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.viewer_field_debug(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @returns {boolean}
     */
    flying() {
        const ret = wasm.viewer_flying(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Camera follows the units (ground) or the ship (system) again.
     */
    follow() {
        wasm.viewer_follow(this.__wbg_ptr);
    }
    /**
     * Draws one frame and returns the status line.
     * @param {number} now_ms
     * @returns {string}
     */
    frame(now_ms) {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.viewer_frame(this.__wbg_ptr, now_ms);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Carries out an order at a map point (after the radio exchange): 0 move, 1 defend, 2 assault, 3 support.
     * Assaults go at a sprint; on arrival a unit takes the stance that fits the order.
     * @param {number} action
     * @param {number} x
     * @param {number} y
     * @param {number} now_ms
     * @returns {string}
     */
    issue_at(action, x, y, now_ms) {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.viewer_issue_at(this.__wbg_ptr, action, x, y, now_ms);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * AI lab: the feed of leader steps and shouts since the last call, one per line.
     * @returns {string}
     */
    lab_feed() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.viewer_lab_feed(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * AI lab: every soldier on the testing ground on screen with what he is doing, as JSON
     * [{x, y (screen fractions, over his head), t (team), txt}].
     * @returns {string}
     */
    lab_marks() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.viewer_lab_marks(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * AI lab: how fast battle time runs (0 pauses).
     * @param {number} speed
     */
    lab_set_speed(speed) {
        wasm.viewer_lab_set_speed(this.__wbg_ptr, speed);
    }
    /**
     * AI lab: clears the testing ground and sets up a scenario; the commander is placed to watch it. Returns
     * a line saying what to watch for.
     * @param {string} name
     * @returns {string}
     */
    lab_start(name) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.viewer_lab_start(this.__wbg_ptr, ptr0, len0);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Starts a flight from the docked body to the other one; ignored while flying.
     * @param {number} now_ms
     */
    launch(now_ms) {
        wasm.viewer_launch(this.__wbg_ptr, now_ms);
    }
    /**
     * The tablet map glides back over the commander, close in (from wherever it was, the planet or
     * the star system included).
     */
    map_center_commander() {
        wasm.viewer_map_center_commander(this.__wbg_ptr);
    }
    /**
     * The map point under a pixel (absolute metres), for orders issued after the radio exchange.
     * @param {number} px
     * @param {number} py
     * @returns {Float64Array}
     */
    map_point(px, py) {
        const ret = wasm.viewer_map_point(this.__wbg_ptr, px, py);
        var v1 = getArrayF64FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 8, 8);
        return v1;
    }
    /**
     * Number of models the viewer can show (the blueprints, then the soldier last).
     * @returns {number}
     */
    model_count() {
        const ret = wasm.viewer_model_count(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Holds pose `i` in the soldier preview (walk, run, stand, crouch, cover aim, prone, standing aim); -1 cycles.
     * Commander movement for this frame: x to the screen's right, y to its top (-1..1 each), and sprinting.
     * @param {number} x
     * @param {number} y
     * @param {boolean} sprint
     */
    move_commander(x, y, sprint) {
        wasm.viewer_move_commander(this.__wbg_ptr, x, y, sprint);
    }
    /**
     * What is under a pixel (the active camera; the tablet map inside `use_map(true)`), as JSON:
     * {kind: unit|commander|barracks|tower|wall|sandbags|container|crate|rock|ground, id, grid, ...}.
     * @param {number} px
     * @param {number} py
     * @returns {string}
     */
    object_at(px, py) {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.viewer_object_at(this.__wbg_ptr, px, py);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Drags the view by screen pixels: pans the ground and the system, turns ships.
     * @param {number} dx
     * @param {number} dy
     */
    pan(dx, dy) {
        wasm.viewer_pan(this.__wbg_ptr, dx, dy);
    }
    /**
     * Numbers for the performance panel: [fps, internal width, internal height,
     * gpu shadow ms, gpu scene ms, gpu upscale ms] (gpu values are -1 when the browser gives no timestamps).
     * @returns {Float32Array}
     */
    perf() {
        const ret = wasm.viewer_perf(this.__wbg_ptr);
        var v1 = getArrayF32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * @param {number} i
     */
    preview_pose(i) {
        wasm.viewer_preview_pose(this.__wbg_ptr, i);
    }
    /**
     * Radio traffic from the units since the last call, as a JSON array of {from, text, urgent}.
     * @returns {string}
     */
    radio_events() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.viewer_radio_events(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Model viewer: back to the isometric angle, framed, spinning.
     */
    reset_model_view() {
        wasm.viewer_reset_model_view(this.__wbg_ptr);
    }
    /**
     * @param {number} w
     * @param {number} h
     */
    resize(w, h) {
        wasm.viewer_resize(this.__wbg_ptr, w, h);
    }
    /**
     * @param {number} w
     * @param {number} h
     */
    resize_map(w, h) {
        wasm.viewer_resize_map(this.__wbg_ptr, w, h);
    }
    /**
     * The units under command, as JSON: [{id, name, soldiers, status, stance, selected}].
     * @returns {string}
     */
    roster() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.viewer_roster(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Turns the camera by `d` radians.
     * @param {number} d
     */
    rotate(d) {
        wasm.viewer_rotate(this.__wbg_ptr, d);
    }
    /**
     * @returns {string}
     */
    scene_name() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.viewer_scene_name(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Shift + drag: selects every group whose centre lies inside the screen rectangle (projected onto the ground).
     * @param {number} x0
     * @param {number} y0
     * @param {number} x1
     * @param {number} y1
     * @param {number} now_ms
     * @returns {string}
     */
    select_box(x0, y0, x1, y1, now_ms) {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.viewer_select_box(this.__wbg_ptr, x0, y0, x1, y1, now_ms);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Selects exactly one unit without moving the camera (clicking its marker on the tablet map).
     * @param {number} id
     */
    select_only(id) {
        wasm.viewer_select_only(this.__wbg_ptr, id);
    }
    /**
     * Selects one unit (from the roster) and centres the tablet's map on it.
     * @param {number} id
     */
    select_unit(id) {
        wasm.viewer_select_unit(this.__wbg_ptr, id);
    }
    /**
     * Names of the selected units (for the radio).
     * The selected units' ids, as JSON.
     * @returns {string}
     */
    selection_ids() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.viewer_selection_ids(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @returns {string}
     */
    selection_name() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.viewer_selection_name(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @param {boolean} on
     */
    set_full(on) {
        wasm.viewer_set_full(this.__wbg_ptr, on);
    }
    /**
     * The menu's intro flight holds the camera: `view_m` metres of view height (from the star system all the
     * way down to the commander), looking down at `elev` radians, centred on his head; `fade` dissolves him.
     * @param {number} view_m
     * @param {number} elev
     * @param {number} fade
     */
    set_intro(view_m, elev, fade) {
        wasm.viewer_set_intro(this.__wbg_ptr, view_m, elev, fade);
    }
    /**
     * How orders are carried out: 0 calm, 1 aggressive.
     * @param {number} m
     */
    set_manner(m) {
        wasm.viewer_set_manner(this.__wbg_ptr, m);
    }
    /**
     * Model viewer: level of detail of the soldier (0 full, 1 mid, 2 far).
     * @param {number} d
     */
    set_model_detail(d) {
        wasm.viewer_set_model_detail(this.__wbg_ptr, d);
    }
    /**
     * Model viewer: sun position as a time of day (0.25 sunrise, 0.5 noon, 0.75 sunset).
     * @param {number} t
     */
    set_model_light(t) {
        wasm.viewer_set_model_light(this.__wbg_ptr, t);
    }
    /**
     * Model viewer: colour variant (an index into `variant_name`).
     * @param {number} i
     */
    set_model_variant(i) {
        wasm.viewer_set_model_variant(this.__wbg_ptr, i);
    }
    /**
     * How the next move is to be made, as chosen on the radio: 0 just move, 1 with caution (possible hostiles),
     * 2 garrison the area, 3 secure the area (hostiles confirmed).
     * @param {number} m
     */
    set_order_mode(m) {
        wasm.viewer_set_order_mode(this.__wbg_ptr, m);
    }
    /**
     * @param {boolean} on
     */
    set_outline(on) {
        wasm.viewer_set_outline(this.__wbg_ptr, on);
    }
    /**
     * Screen pixels per internal pixel (the chunky look); 1 is full resolution.
     * @param {number} s
     */
    set_pixel_scale(s) {
        wasm.viewer_set_pixel_scale(this.__wbg_ptr, s);
    }
    /**
     * The commander holds the radio to his ear (true) or lets it go.
     * @param {boolean} on
     */
    set_radio(on) {
        wasm.viewer_set_radio(this.__wbg_ptr, on);
    }
    /**
     * @param {string} name
     */
    set_scene(name) {
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.viewer_set_scene(this.__wbg_ptr, ptr0, len0);
    }
    /**
     * Replaces the selection (the radio hands each answering unit its order on its own).
     * @param {Uint32Array | number[]} ids
     */
    set_selection(ids) {
        const ptr0 = passArray32ToWasm0(ids, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.viewer_set_selection(this.__wbg_ptr, ptr0, len0);
    }
    /**
     * @param {number} i
     */
    set_ship(i) {
        wasm.viewer_set_ship(this.__wbg_ptr, i);
    }
    /**
     * Raises or lowers the tablet. The field view stays on the commander; the map has its own camera, which
     * starts over the commander.
     * @param {boolean} up
     */
    set_tablet(up) {
        wasm.viewer_set_tablet(this.__wbg_ptr, up);
    }
    /**
     * The fraction of the screen's height the tablet close-up's box covers: the dive frames the tablet to match.
     * @param {number} f
     */
    set_tablet_fit(f) {
        wasm.viewer_set_tablet_fit(this.__wbg_ptr, f);
    }
    /**
     * Scales the tablet close-up and shifts it sideways (fraction of the screen width, positive to the right).
     * @param {number} scale
     * @param {number} shift
     */
    set_tablet_frame(scale, shift) {
        wasm.viewer_set_tablet_frame(this.__wbg_ptr, scale, shift);
    }
    /**
     * Model viewer: spin on a turntable (true) or hold the angle it was left at.
     * @param {boolean} on
     */
    set_turntable(on) {
        wasm.viewer_set_turntable(this.__wbg_ptr, on);
    }
    /**
     * Starts the training assault: a squad of 12 forms up south of the killhouse and takes it from the 8
     * enemy soldiers inside, all on the battle AI. Starting again restarts it.
     * @param {number} _seed
     */
    start_assault(_seed) {
        wasm.viewer_start_assault(this.__wbg_ptr, _seed);
    }
    /**
     * Ends the assault: the testing ground starts over (the enemy back on their posts).
     */
    stop_assault() {
        wasm.viewer_stop_assault(this.__wbg_ptr);
    }
    /**
     * A structure's scan overlay for the active camera: its faces, doors, windows and room partitions as screen
     * polygons (fractions of the canvas, y down), JSON {faces:[[x,y,...]], doors, windows, rooms} (rooms are lines).
     * @param {number} id
     * @returns {string}
     */
    structure_overlay(id) {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.viewer_structure_overlay(this.__wbg_ptr, id);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Where the tablet cinematic is: 0 put away, 1 raising it (camera rising and diving), 2 in front of the eyes
     * (show the tablet's screen), 3 putting it away.
     * @returns {number}
     */
    tablet_view() {
        const ret = wasm.viewer_tablet_view(this.__wbg_ptr);
        return ret;
    }
    /**
     * Sounds since the last call, 26 floats each, worked out by the acoustics for the ears at the camera:
     * kind (0 shot, 1 blast, 2 footfall), loudness (footfalls: 0 walk .. 1 sprint), delay from now (s, travel
     * time included), gain, low-pass (Hz), reverb of its own space 0..1, position from the ears (m: right,
     * ahead, below), distance (m), walls between, its space (enclosure 0..1, size m, corridor 0..1), then four
     * echoes off what stands around it (extra delay s, gain against the direct sound, pan -1..1; zeros when
     * there is nothing to bounce off).
     * @returns {Float32Array}
     */
    take_sounds() {
        const ret = wasm.viewer_take_sounds(this.__wbg_ptr);
        var v1 = getArrayF32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * The unit under a pixel of the current view, or -1.
     * @param {number} px
     * @param {number} py
     * @returns {number}
     */
    unit_at(px, py) {
        const ret = wasm.viewer_unit_at(this.__wbg_ptr, px, py);
        return ret;
    }
    /**
     * Every unit as seen by the camera that is active right now (the field camera, or the tablet map's inside
     * `use_map(true)`): JSON [{id, cs, n, x, y, r, sel, you, men: [[x, y, readiness, health]], ready: [stable, wounded,
     * incapacitated, deceased], u}] with x, y as fractions of the canvas (0..1, y down), u a metre as a fraction of
     * its height and r
     * the unit's radius as a fraction of the canvas height. The commander is id -1. Units off screen are left out.
     * @returns {string}
     */
    unit_marks() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.viewer_unit_marks(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Routes input to the map's camera (true) or the field view (false). Calls between a `use_map(true)` and
     * `use_map(false)` (wheel, pan, click, command) act on the map.
     * @param {boolean} on
     */
    use_map(on) {
        wasm.viewer_use_map(this.__wbg_ptr, on);
    }
    /**
     * @param {number} i
     * @returns {string | undefined}
     */
    variant_name(i) {
        const ret = wasm.viewer_variant_name(this.__wbg_ptr, i);
        let v1;
        if (ret[0] !== 0) {
            v1 = getStringFromWasm0(ret[0], ret[1]);
            wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        }
        return v1;
    }
    /**
     * Mouse wheel: one call is one step of the zoom ladder (positive zooms out); the camera glides there.
     * One zoom step in or out; true when the view actually changed (not already at the end of the ladder).
     * @param {number} delta
     * @returns {boolean}
     */
    wheel(delta) {
        const ret = wasm.viewer_wheel(this.__wbg_ptr, delta);
        return ret !== 0;
    }
}
if (Symbol.dispose) Viewer.prototype[Symbol.dispose] = Viewer.prototype.free;

/**
 * The killhouse battle hash after `ms`: the same computation as the native `hash_battle` binary.
 * @param {bigint} seed
 * @param {number} ms
 * @returns {bigint}
 */
export function battle_hash(seed, ms) {
    const ret = wasm.battle_hash(seed, ms);
    return BigInt.asUintN(64, ret);
}

/**
 * Compact JSON summary: id, name, class, triangles and the mesh hash as a decimal string.
 * @param {string} text
 * @returns {string}
 */
export function blueprint_info(text) {
    let deferred3_0;
    let deferred3_1;
    try {
        const ptr0 = passStringToWasm0(text, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.blueprint_info(ptr0, len0);
        var ptr2 = ret[0];
        var len2 = ret[1];
        if (ret[3]) {
            ptr2 = 0; len2 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred3_0 = ptr2;
        deferred3_1 = len2;
        return getStringFromWasm0(ptr2, len2);
    } finally {
        wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
    }
}

/**
 * Flat-shaded triangle list for drawing: 4 floats per vertex (x, y, z in metres, palette index),
 * 3 vertices per triangle.
 * @param {string} text
 * @returns {Float32Array}
 */
export function blueprint_mesh(text) {
    const ptr0 = passStringToWasm0(text, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.blueprint_mesh(ptr0, len0);
    if (ret[3]) {
        throw takeFromExternrefTable0(ret[2]);
    }
    var v2 = getArrayF32FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
    return v2;
}

/**
 * Parse and validate blueprint text, then draw its top-down SVG.
 * @param {string} text
 * @returns {string}
 */
export function blueprint_svg(text) {
    let deferred3_0;
    let deferred3_1;
    try {
        const ptr0 = passStringToWasm0(text, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.blueprint_svg(ptr0, len0);
        var ptr2 = ret[0];
        var len2 = ret[1];
        if (ret[3]) {
            ptr2 = 0; len2 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred3_0 = ptr2;
        deferred3_1 = len2;
        return getStringFromWasm0(ptr2, len2);
    } finally {
        wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
    }
}

/**
 * Heights of one planet chunk, `(cells + 1)^2` values in millimetres.
 * @param {bigint} seed
 * @param {number} cx
 * @param {number} cy
 * @param {number} cells
 * @returns {Int32Array}
 */
export function planet_chunk(seed, cx, cy, cells) {
    const ret = wasm.planet_chunk(seed, cx, cy, cells);
    var v1 = getArrayI32FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
    return v1;
}

/**
 * @param {bigint} seed
 * @param {number} cx
 * @param {number} cy
 * @param {number} cells
 * @returns {bigint}
 */
export function planet_chunk_hash(seed, cx, cy, cells) {
    const ret = wasm.planet_chunk_hash(seed, cx, cy, cells);
    return BigInt.asUintN(64, ret);
}

/**
 * Same computation as the native `hash_scenario` binary.
 * @param {bigint} seed
 * @returns {bigint}
 */
export function scenario_hash(seed) {
    const ret = wasm.scenario_hash(seed);
    return BigInt.asUintN(64, ret);
}

/**
 * File text of shipped blueprint `index`, or undefined when there is no such blueprint.
 * @param {number} index
 * @returns {string | undefined}
 */
export function shipped_ron(index) {
    const ret = wasm.shipped_ron(index);
    let v1;
    if (ret[0] !== 0) {
        v1 = getStringFromWasm0(ret[0], ret[1]);
        wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
    }
    return v1;
}

/**
 * @param {HTMLCanvasElement} canvas
 * @returns {Promise<Viewer>}
 */
export function start_viewer(canvas) {
    const ret = wasm.start_viewer(canvas);
    return ret;
}

/**
 * @returns {number}
 */
export function system_body_count() {
    const ret = wasm.system_body_count();
    return ret >>> 0;
}

/**
 * @param {number} i
 * @returns {string | undefined}
 */
export function system_body_name(i) {
    const ret = wasm.system_body_name(i);
    let v1;
    if (ret[0] !== 0) {
        v1 = getStringFromWasm0(ret[0], ret[1]);
        wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
    }
    return v1;
}

/**
 * `[x_mm, y_mm]` of a body in the system region; empty for an unknown body.
 * @param {number} i
 * @param {number} t_ms
 * @returns {Float64Array}
 */
export function system_body_pos(i, t_ms) {
    const ret = wasm.system_body_pos(i, t_ms);
    var v1 = getArrayF64FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 8, 8);
    return v1;
}
function __wbg_get_imports() {
    const import0 = {
        __proto__: null,
        __wbg_Error_208446807d0d1e30: function(arg0, arg1) {
            const ret = Error(getStringFromWasm0(arg0, arg1));
            return ret;
        },
        __wbg_Window_2805651c516cb7aa: function(arg0) {
            const ret = arg0.Window;
            return ret;
        },
        __wbg_WorkerGlobalScope_174390815d0a4893: function(arg0) {
            const ret = arg0.WorkerGlobalScope;
            return ret;
        },
        __wbg___wbindgen_debug_string_f249a2a76caa9a55: function(arg0, arg1) {
            const ret = debugString(arg1);
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 1, len1, true);
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 0, ptr1, true);
        },
        __wbg___wbindgen_is_function_741ff2bb612836f5: function(arg0) {
            const ret = typeof(arg0) === 'function';
            return ret;
        },
        __wbg___wbindgen_is_null_c22311b1cc228e3a: function(arg0) {
            const ret = arg0 === null;
            return ret;
        },
        __wbg___wbindgen_is_string_fc456fee5d3317de: function(arg0) {
            const ret = typeof(arg0) === 'string';
            return ret;
        },
        __wbg___wbindgen_is_undefined_be5b34249ff1fc15: function(arg0) {
            const ret = arg0 === undefined;
            return ret;
        },
        __wbg___wbindgen_string_get_7ce8eee7fd327545: function(arg0, arg1) {
            const obj = arg1;
            const ret = typeof(obj) === 'string' ? obj : undefined;
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 1, len1, true);
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 0, ptr1, true);
        },
        __wbg___wbindgen_throw_4c88d8bf264fd984: function(arg0, arg1) {
            throw new Error(getStringFromWasm0(arg0, arg1));
        },
        __wbg__wbg_cb_unref_09736879a3f67559: function(arg0) {
            arg0._wbg_cb_unref();
        },
        __wbg_beginRenderPass_597e3973321fd369: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.beginRenderPass(arg1);
            return ret;
        }, arguments); },
        __wbg_call_a410eb4b59ed5965: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.call(arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_configure_8e04ab1a867d31e1: function() { return handleError(function (arg0, arg1) {
            arg0.configure(arg1);
        }, arguments); },
        __wbg_copyBufferToBuffer_0c93ff5b22a585e9: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5) {
            arg0.copyBufferToBuffer(arg1, arg2, arg3, arg4, arg5);
        }, arguments); },
        __wbg_createBindGroup_f539b26ca341308f: function(arg0, arg1) {
            const ret = arg0.createBindGroup(arg1);
            return ret;
        },
        __wbg_createBuffer_b8c5b520d2c822b8: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.createBuffer(arg1);
            return ret;
        }, arguments); },
        __wbg_createCommandEncoder_3352d1ffc36c6fc0: function(arg0, arg1) {
            const ret = arg0.createCommandEncoder(arg1);
            return ret;
        },
        __wbg_createQuerySet_4a3a1eb942cab7ca: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.createQuerySet(arg1);
            return ret;
        }, arguments); },
        __wbg_createRenderPipeline_58390ba302107691: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.createRenderPipeline(arg1);
            return ret;
        }, arguments); },
        __wbg_createSampler_9bd91d7e928c0060: function(arg0, arg1) {
            const ret = arg0.createSampler(arg1);
            return ret;
        },
        __wbg_createShaderModule_cefa51336cb288ae: function(arg0, arg1) {
            const ret = arg0.createShaderModule(arg1);
            return ret;
        },
        __wbg_createTexture_2c95e64f50008aea: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.createTexture(arg1);
            return ret;
        }, arguments); },
        __wbg_createView_96e5e405cc3c01e8: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.createView(arg1);
            return ret;
        }, arguments); },
        __wbg_description_7a568e202a00293f: function(arg0, arg1) {
            const ret = arg1.description;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 1, len1, true);
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 0, ptr1, true);
        },
        __wbg_document_3b5c3071b43e97ee: function(arg0) {
            const ret = arg0.document;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_draw_086a9578fc9898c2: function(arg0, arg1, arg2, arg3, arg4) {
            arg0.draw(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, arg4 >>> 0);
        },
        __wbg_end_b57473834b877409: function(arg0) {
            arg0.end();
        },
        __wbg_features_7b0a0ba10b61a5e4: function(arg0) {
            const ret = arg0.features;
            return ret;
        },
        __wbg_features_d5d767fa7757f783: function(arg0) {
            const ret = arg0.features;
            return ret;
        },
        __wbg_finish_09ec094c10f41e7b: function(arg0) {
            const ret = arg0.finish();
            return ret;
        },
        __wbg_finish_ec1c191f66a895b1: function(arg0, arg1) {
            const ret = arg0.finish(arg1);
            return ret;
        },
        __wbg_getBindGroupLayout_fb40c3b93d37f466: function(arg0, arg1) {
            const ret = arg0.getBindGroupLayout(arg1 >>> 0);
            return ret;
        },
        __wbg_getContext_1938edb9a972f63d: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.getContext(getStringFromWasm0(arg1, arg2));
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments); },
        __wbg_getContext_d93f904b714852ae: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.getContext(getStringFromWasm0(arg1, arg2));
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments); },
        __wbg_getCurrentTexture_c3b1727224d7f6e6: function() { return handleError(function (arg0) {
            const ret = arg0.getCurrentTexture();
            return ret;
        }, arguments); },
        __wbg_getMappedRange_28e816a763520e01: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.getMappedRange(arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_getPreferredCanvasFormat_0ef5034c8902201b: function(arg0) {
            const ret = arg0.getPreferredCanvasFormat();
            return (__wbindgen_enum_GpuTextureFormat.indexOf(ret) + 1 || 102) - 1;
        },
        __wbg_get_0c55869fe53c7d14: function(arg0, arg1) {
            const ret = arg0[arg1 >>> 0];
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_gpu_514705c6dd1f0dab: function(arg0) {
            const ret = arg0.gpu;
            return ret;
        },
        __wbg_has_eafa12e457ea88fb: function(arg0, arg1, arg2) {
            const ret = arg0.has(getStringFromWasm0(arg1, arg2));
            return ret;
        },
        __wbg_height_ac6f1c3f49c13fe2: function(arg0) {
            const ret = arg0.height;
            return ret;
        },
        __wbg_info_8554cb7af37427b6: function(arg0) {
            const ret = arg0.info;
            return ret;
        },
        __wbg_instanceof_Window_5cf453f5639aa1a2: function(arg0) {
            let result;
            try {
                result = arg0 instanceof Window;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_isFallbackAdapter_6a4fa7356d18310b: function(arg0) {
            const ret = arg0.isFallbackAdapter;
            return ret;
        },
        __wbg_label_fceb40a43a358e62: function(arg0, arg1) {
            const ret = arg1.label;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 1, len1, true);
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 0, ptr1, true);
        },
        __wbg_length_186163212ffe79c3: function(arg0) {
            const ret = arg0.length;
            return ret;
        },
        __wbg_limits_4e444eb8c2e61c45: function(arg0) {
            const ret = arg0.limits;
            return ret;
        },
        __wbg_mapAsync_b0597127f5037286: function(arg0, arg1, arg2, arg3) {
            const ret = arg0.mapAsync(arg1 >>> 0, arg2, arg3);
            return ret;
        },
        __wbg_maxBindGroupsPlusVertexBuffers_1face4afb5363937: function(arg0) {
            const ret = arg0.maxBindGroupsPlusVertexBuffers;
            return ret;
        },
        __wbg_maxBindGroups_a0b7e55762f095b9: function(arg0) {
            const ret = arg0.maxBindGroups;
            return ret;
        },
        __wbg_maxBindingsPerBindGroup_ec9067be38d6ec72: function(arg0) {
            const ret = arg0.maxBindingsPerBindGroup;
            return ret;
        },
        __wbg_maxBufferSize_a9ce15e541c64cd2: function(arg0) {
            const ret = arg0.maxBufferSize;
            return ret;
        },
        __wbg_maxColorAttachmentBytesPerSample_44089e129e1a2a1d: function(arg0) {
            const ret = arg0.maxColorAttachmentBytesPerSample;
            return ret;
        },
        __wbg_maxColorAttachments_6bd8156394ba618c: function(arg0) {
            const ret = arg0.maxColorAttachments;
            return ret;
        },
        __wbg_maxComputeInvocationsPerWorkgroup_7b086f15ef868934: function(arg0) {
            const ret = arg0.maxComputeInvocationsPerWorkgroup;
            return ret;
        },
        __wbg_maxComputeWorkgroupSizeX_8bbe505e0ac8dc18: function(arg0) {
            const ret = arg0.maxComputeWorkgroupSizeX;
            return ret;
        },
        __wbg_maxComputeWorkgroupSizeY_7228ed3935eecc31: function(arg0) {
            const ret = arg0.maxComputeWorkgroupSizeY;
            return ret;
        },
        __wbg_maxComputeWorkgroupSizeZ_6092b56a967f6f8d: function(arg0) {
            const ret = arg0.maxComputeWorkgroupSizeZ;
            return ret;
        },
        __wbg_maxComputeWorkgroupStorageSize_4773e3bd3936df36: function(arg0) {
            const ret = arg0.maxComputeWorkgroupStorageSize;
            return ret;
        },
        __wbg_maxComputeWorkgroupsPerDimension_79222bf8e8a90fbd: function(arg0) {
            const ret = arg0.maxComputeWorkgroupsPerDimension;
            return ret;
        },
        __wbg_maxDynamicStorageBuffersPerPipelineLayout_3ab5762b1b2c7fc2: function(arg0) {
            const ret = arg0.maxDynamicStorageBuffersPerPipelineLayout;
            return ret;
        },
        __wbg_maxDynamicUniformBuffersPerPipelineLayout_722da31d0de1c256: function(arg0) {
            const ret = arg0.maxDynamicUniformBuffersPerPipelineLayout;
            return ret;
        },
        __wbg_maxInterStageShaderVariables_821096d4b3020715: function(arg0) {
            const ret = arg0.maxInterStageShaderVariables;
            return ret;
        },
        __wbg_maxSampledTexturesPerShaderStage_c6aea9e05c0a02ca: function(arg0) {
            const ret = arg0.maxSampledTexturesPerShaderStage;
            return ret;
        },
        __wbg_maxSamplersPerShaderStage_3aa30f6c002d3a99: function(arg0) {
            const ret = arg0.maxSamplersPerShaderStage;
            return ret;
        },
        __wbg_maxStorageBufferBindingSize_84a2c2433627fca2: function(arg0) {
            const ret = arg0.maxStorageBufferBindingSize;
            return ret;
        },
        __wbg_maxStorageBuffersPerShaderStage_25b96748df28dc20: function(arg0) {
            const ret = arg0.maxStorageBuffersPerShaderStage;
            return ret;
        },
        __wbg_maxStorageTexturesPerShaderStage_53ed734f26e6c1ad: function(arg0) {
            const ret = arg0.maxStorageTexturesPerShaderStage;
            return ret;
        },
        __wbg_maxTextureArrayLayers_9ec487f1ce102992: function(arg0) {
            const ret = arg0.maxTextureArrayLayers;
            return ret;
        },
        __wbg_maxTextureDimension1D_faef91b2e9fa6740: function(arg0) {
            const ret = arg0.maxTextureDimension1D;
            return ret;
        },
        __wbg_maxTextureDimension2D_3a923eb2df4d9124: function(arg0) {
            const ret = arg0.maxTextureDimension2D;
            return ret;
        },
        __wbg_maxTextureDimension3D_943321fe5945464e: function(arg0) {
            const ret = arg0.maxTextureDimension3D;
            return ret;
        },
        __wbg_maxUniformBufferBindingSize_21e89ff761d576db: function(arg0) {
            const ret = arg0.maxUniformBufferBindingSize;
            return ret;
        },
        __wbg_maxUniformBuffersPerShaderStage_19a0bfb0189f7254: function(arg0) {
            const ret = arg0.maxUniformBuffersPerShaderStage;
            return ret;
        },
        __wbg_maxVertexAttributes_49aef6c51d07a280: function(arg0) {
            const ret = arg0.maxVertexAttributes;
            return ret;
        },
        __wbg_maxVertexBufferArrayStride_f68c4029e6f047ec: function(arg0) {
            const ret = arg0.maxVertexBufferArrayStride;
            return ret;
        },
        __wbg_maxVertexBuffers_ce9f15a623f9aac7: function(arg0) {
            const ret = arg0.maxVertexBuffers;
            return ret;
        },
        __wbg_minStorageBufferOffsetAlignment_529f1f52e61188fc: function(arg0) {
            const ret = arg0.minStorageBufferOffsetAlignment;
            return ret;
        },
        __wbg_minUniformBufferOffsetAlignment_519e26287e1996a3: function(arg0) {
            const ret = arg0.minUniformBufferOffsetAlignment;
            return ret;
        },
        __wbg_navigator_9641ab32fa612dd1: function(arg0) {
            const ret = arg0.navigator;
            return ret;
        },
        __wbg_navigator_c17f443a6fe5d38d: function(arg0) {
            const ret = arg0.navigator;
            return ret;
        },
        __wbg_new_2bfd0f45e60518b1: function() { return handleError(function (arg0, arg1) {
            const ret = new OffscreenCanvas(arg0 >>> 0, arg1 >>> 0);
            return ret;
        }, arguments); },
        __wbg_new_cd4348290afae586: function() {
            const ret = new Object();
            return ret;
        },
        __wbg_new_typed_6b60b99917da27d6: function() {
            const ret = new Object();
            return ret;
        },
        __wbg_new_typed_e09da9dae6ee5935: function(arg0, arg1) {
            try {
                var state0 = {a: arg0, b: arg1};
                var cb0 = (arg0, arg1) => {
                    const a = state0.a;
                    state0.a = 0;
                    try {
                        return wasm_bindgen_afd209a88ce49cc3___convert__closures_____invoke___js_sys_4c0a7da8c3256c44___Function_fn_wasm_bindgen_afd209a88ce49cc3___JsValue_____wasm_bindgen_afd209a88ce49cc3___sys__Undefined___js_sys_4c0a7da8c3256c44___Function_fn_wasm_bindgen_afd209a88ce49cc3___JsValue_____wasm_bindgen_afd209a88ce49cc3___sys__Undefined_______true_(a, state0.b, arg0, arg1);
                    } finally {
                        state0.a = a;
                    }
                };
                const ret = new Promise(cb0);
                return ret;
            } finally {
                state0.a = 0;
            }
        },
        __wbg_new_with_byte_offset_and_length_3cd32055a417f095: function(arg0, arg1, arg2) {
            const ret = new Uint8Array(arg0, arg1 >>> 0, arg2 >>> 0);
            return ret;
        },
        __wbg_onSubmittedWorkDone_1190213cee1ecf7e: function(arg0) {
            const ret = arg0.onSubmittedWorkDone();
            return ret;
        },
        __wbg_prototypesetcall_13162965d1fe7aa3: function(arg0, arg1, arg2) {
            Uint8Array.prototype.set.call(getArrayU8FromWasm0(arg0, arg1), arg2);
        },
        __wbg_querySelectorAll_6c1bd81d448fb480: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.querySelectorAll(getStringFromWasm0(arg1, arg2));
            return ret;
        }, arguments); },
        __wbg_queueMicrotask_1e8503a8a690f0a3: function(arg0) {
            const ret = arg0.queueMicrotask;
            return ret;
        },
        __wbg_queueMicrotask_4a1a4d8dd332373b: function(arg0) {
            queueMicrotask(arg0);
        },
        __wbg_queue_e250e2ebc49b39e0: function(arg0) {
            const ret = arg0.queue;
            return ret;
        },
        __wbg_requestAdapter_a539af006419f2e9: function(arg0, arg1) {
            const ret = arg0.requestAdapter(arg1);
            return ret;
        },
        __wbg_requestDevice_5cb8a582e55d08cb: function(arg0, arg1) {
            const ret = arg0.requestDevice(arg1);
            return ret;
        },
        __wbg_resolveQuerySet_770f23fabac49845: function(arg0, arg1, arg2, arg3, arg4, arg5) {
            arg0.resolveQuerySet(arg1, arg2 >>> 0, arg3 >>> 0, arg4, arg5 >>> 0);
        },
        __wbg_resolve_dbf78ab4942478b0: function(arg0) {
            const ret = Promise.resolve(arg0);
            return ret;
        },
        __wbg_scaliaPanic_d7ce75e61e075897: function() { return handleError(function (arg0, arg1) {
            window.scaliaPanic(getStringFromWasm0(arg0, arg1));
        }, arguments); },
        __wbg_setBindGroup_418c3e0eb6943ce0: function(arg0, arg1, arg2) {
            arg0.setBindGroup(arg1 >>> 0, arg2);
        },
        __wbg_setPipeline_b6f981027e02cd16: function(arg0, arg1) {
            arg0.setPipeline(arg1);
        },
        __wbg_setVertexBuffer_cbf4ca1627c02f4c: function(arg0, arg1, arg2, arg3, arg4) {
            arg0.setVertexBuffer(arg1 >>> 0, arg2, arg3, arg4);
        },
        __wbg_set_118fefd528ab0a79: function(arg0, arg1, arg2) {
            arg0.set(arg1, arg2 >>> 0);
        },
        __wbg_set_2793479888329cb6: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = Reflect.set(arg0, arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_set_a_1c94f1413a06c773: function(arg0, arg1) {
            arg0.a = arg1;
        },
        __wbg_set_address_mode_u_a39aefdf030758ae: function(arg0, arg1) {
            arg0.addressModeU = __wbindgen_enum_GpuAddressMode[arg1];
        },
        __wbg_set_address_mode_v_999ec6ed4d587e8c: function(arg0, arg1) {
            arg0.addressModeV = __wbindgen_enum_GpuAddressMode[arg1];
        },
        __wbg_set_address_mode_w_a936c884ccf08234: function(arg0, arg1) {
            arg0.addressModeW = __wbindgen_enum_GpuAddressMode[arg1];
        },
        __wbg_set_alpha_84214d12347c6647: function(arg0, arg1) {
            arg0.alpha = arg1;
        },
        __wbg_set_alpha_mode_278c6649365aff36: function(arg0, arg1) {
            arg0.alphaMode = __wbindgen_enum_GpuCanvasAlphaMode[arg1];
        },
        __wbg_set_alpha_to_coverage_enabled_1e879520cab4bb05: function(arg0, arg1) {
            arg0.alphaToCoverageEnabled = arg1 !== 0;
        },
        __wbg_set_array_layer_count_bd7a074559a0abaa: function(arg0, arg1) {
            arg0.arrayLayerCount = arg1 >>> 0;
        },
        __wbg_set_array_stride_f64_b132c6a79afeae43: function(arg0, arg1) {
            arg0.arrayStride = arg1;
        },
        __wbg_set_aspect_8c866a3891032753: function(arg0, arg1) {
            arg0.aspect = __wbindgen_enum_GpuTextureAspect[arg1];
        },
        __wbg_set_attributes_67bbc44b08e038d6: function(arg0, arg1, arg2) {
            arg0.attributes = getArrayJsValueViewFromWasm0(arg1, arg2);
        },
        __wbg_set_b_a93bcfdd1c29a0bd: function(arg0, arg1) {
            arg0.b = arg1;
        },
        __wbg_set_base_array_layer_e745f84c2e5437e2: function(arg0, arg1) {
            arg0.baseArrayLayer = arg1 >>> 0;
        },
        __wbg_set_base_mip_level_fe08854fab09de85: function(arg0, arg1) {
            arg0.baseMipLevel = arg1 >>> 0;
        },
        __wbg_set_beginning_of_pass_write_index_a13b608f19d6a2f0: function(arg0, arg1) {
            arg0.beginningOfPassWriteIndex = arg1 >>> 0;
        },
        __wbg_set_binding_bd7d6b01bb08823a: function(arg0, arg1) {
            arg0.binding = arg1 >>> 0;
        },
        __wbg_set_blend_1f3e5b36f9c246cb: function(arg0, arg1) {
            arg0.blend = arg1;
        },
        __wbg_set_buffer_91f4e8e05066c37a: function(arg0, arg1) {
            arg0.buffer = arg1;
        },
        __wbg_set_buffers_47665c552e27672c: function(arg0, arg1, arg2) {
            arg0.buffers = getArrayJsValueViewFromWasm0(arg1, arg2);
        },
        __wbg_set_clear_value_gpu_color_dict_303e0009591e5d26: function(arg0, arg1) {
            arg0.clearValue = arg1;
        },
        __wbg_set_code_130aebc49c4cadd3: function(arg0, arg1, arg2) {
            arg0.code = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_color_6d2de56a2224ec57: function(arg0, arg1) {
            arg0.color = arg1;
        },
        __wbg_set_color_attachments_e83f79badceda8b4: function(arg0, arg1, arg2) {
            arg0.colorAttachments = getArrayJsValueViewFromWasm0(arg1, arg2);
        },
        __wbg_set_compare_159af15eb77dbcd2: function(arg0, arg1) {
            arg0.compare = __wbindgen_enum_GpuCompareFunction[arg1];
        },
        __wbg_set_compare_4db27ced1edc0322: function(arg0, arg1) {
            arg0.compare = __wbindgen_enum_GpuCompareFunction[arg1];
        },
        __wbg_set_count_4d1e9559a7f1b993: function(arg0, arg1) {
            arg0.count = arg1 >>> 0;
        },
        __wbg_set_count_dc87e26bec5f6dc3: function(arg0, arg1) {
            arg0.count = arg1 >>> 0;
        },
        __wbg_set_cull_mode_31293d13bc064271: function(arg0, arg1) {
            arg0.cullMode = __wbindgen_enum_GpuCullMode[arg1];
        },
        __wbg_set_depth_bias_b30728f3ab3f643e: function(arg0, arg1) {
            arg0.depthBias = arg1;
        },
        __wbg_set_depth_bias_clamp_5b58fd4819283038: function(arg0, arg1) {
            arg0.depthBiasClamp = arg1;
        },
        __wbg_set_depth_bias_slope_scale_8b8b20db6b43c43b: function(arg0, arg1) {
            arg0.depthBiasSlopeScale = arg1;
        },
        __wbg_set_depth_clear_value_1165941523836019: function(arg0, arg1) {
            arg0.depthClearValue = arg1;
        },
        __wbg_set_depth_compare_81dd71de7e433cc2: function(arg0, arg1) {
            arg0.depthCompare = __wbindgen_enum_GpuCompareFunction[arg1];
        },
        __wbg_set_depth_fail_op_ddf822c573cdf0c1: function(arg0, arg1) {
            arg0.depthFailOp = __wbindgen_enum_GpuStencilOperation[arg1];
        },
        __wbg_set_depth_load_op_ca270acab0c1ca69: function(arg0, arg1) {
            arg0.depthLoadOp = __wbindgen_enum_GpuLoadOp[arg1];
        },
        __wbg_set_depth_or_array_layers_e59f3988a095ccac: function(arg0, arg1) {
            arg0.depthOrArrayLayers = arg1 >>> 0;
        },
        __wbg_set_depth_read_only_49ca399f98b97955: function(arg0, arg1) {
            arg0.depthReadOnly = arg1 !== 0;
        },
        __wbg_set_depth_stencil_41e94739225cb2c3: function(arg0, arg1) {
            arg0.depthStencil = arg1;
        },
        __wbg_set_depth_stencil_attachment_36ce3695dfcb6b85: function(arg0, arg1) {
            arg0.depthStencilAttachment = arg1;
        },
        __wbg_set_depth_store_op_6cd6bdde49f9050d: function(arg0, arg1) {
            arg0.depthStoreOp = __wbindgen_enum_GpuStoreOp[arg1];
        },
        __wbg_set_depth_write_enabled_4b5da11c77fbdab7: function(arg0, arg1) {
            arg0.depthWriteEnabled = arg1 !== 0;
        },
        __wbg_set_device_1940fd5f9e2a7fee: function(arg0, arg1) {
            arg0.device = arg1;
        },
        __wbg_set_dimension_88d0c9f16bb079fb: function(arg0, arg1) {
            arg0.dimension = __wbindgen_enum_GpuTextureDimension[arg1];
        },
        __wbg_set_dimension_bab9530fa3e311d8: function(arg0, arg1) {
            arg0.dimension = __wbindgen_enum_GpuTextureViewDimension[arg1];
        },
        __wbg_set_dst_factor_80c13f29a3e117cb: function(arg0, arg1) {
            arg0.dstFactor = __wbindgen_enum_GpuBlendFactor[arg1];
        },
        __wbg_set_end_of_pass_write_index_793fcfe3141dc357: function(arg0, arg1) {
            arg0.endOfPassWriteIndex = arg1 >>> 0;
        },
        __wbg_set_entries_d9929dbfa2aaa9a7: function(arg0, arg1, arg2) {
            arg0.entries = getArrayJsValueViewFromWasm0(arg1, arg2);
        },
        __wbg_set_entry_point_0eacf7b26b3637b0: function(arg0, arg1, arg2) {
            arg0.entryPoint = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_entry_point_bb3692db4a6065d0: function(arg0, arg1, arg2) {
            arg0.entryPoint = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_fail_op_739cc3815d38c95f: function(arg0, arg1) {
            arg0.failOp = __wbindgen_enum_GpuStencilOperation[arg1];
        },
        __wbg_set_format_1119b9930bd324e1: function(arg0, arg1) {
            arg0.format = __wbindgen_enum_GpuTextureFormat[arg1];
        },
        __wbg_set_format_443ea19e2d25a1e2: function(arg0, arg1) {
            arg0.format = __wbindgen_enum_GpuTextureFormat[arg1];
        },
        __wbg_set_format_459792306baadae5: function(arg0, arg1) {
            arg0.format = __wbindgen_enum_GpuTextureFormat[arg1];
        },
        __wbg_set_format_46b1c6b3acc10b24: function(arg0, arg1) {
            arg0.format = __wbindgen_enum_GpuTextureFormat[arg1];
        },
        __wbg_set_format_5a62f31a4690f935: function(arg0, arg1) {
            arg0.format = __wbindgen_enum_GpuVertexFormat[arg1];
        },
        __wbg_set_format_efec4f7d9471ddbb: function(arg0, arg1) {
            arg0.format = __wbindgen_enum_GpuTextureFormat[arg1];
        },
        __wbg_set_fragment_bea523f8973c326e: function(arg0, arg1) {
            arg0.fragment = arg1;
        },
        __wbg_set_front_face_30951d0404e9e6d4: function(arg0, arg1) {
            arg0.frontFace = __wbindgen_enum_GpuFrontFace[arg1];
        },
        __wbg_set_g_90eacc6608e549f8: function(arg0, arg1) {
            arg0.g = arg1;
        },
        __wbg_set_height_3f64e19acf29f5fb: function(arg0, arg1) {
            arg0.height = arg1 >>> 0;
        },
        __wbg_set_height_ae0610d318c3428c: function(arg0, arg1) {
            arg0.height = arg1 >>> 0;
        },
        __wbg_set_height_ae24c2f7d580ae0a: function(arg0, arg1) {
            arg0.height = arg1 >>> 0;
        },
        __wbg_set_label_20c42b4ebeb250d5: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_5347fc976d15e94c: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_644bc1b32c7917b0: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_73c99dab6708c962: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_85a21b5fdd57fd91: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_89d2ffaf598df6a4: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_9e68078b9533426b: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_cba03ffef7ae90a3: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_d0d693a97f5be626: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_f654bcb731d4d9ca: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_layout_200fc3b143da0a9e: function(arg0, arg1) {
            arg0.layout = arg1;
        },
        __wbg_set_layout_3f0cb99635c33123: function(arg0, arg1) {
            arg0.layout = arg1;
        },
        __wbg_set_layout_gpu_auto_layout_mode_2e8a53f8643b4f69: function(arg0, arg1) {
            arg0.layout = __wbindgen_enum_GpuAutoLayoutMode[arg1];
        },
        __wbg_set_load_op_e8c33d4bdd2772aa: function(arg0, arg1) {
            arg0.loadOp = __wbindgen_enum_GpuLoadOp[arg1];
        },
        __wbg_set_lod_max_clamp_fcbef26e87120fac: function(arg0, arg1) {
            arg0.lodMaxClamp = arg1;
        },
        __wbg_set_lod_min_clamp_f5b6ddca1e4d813f: function(arg0, arg1) {
            arg0.lodMinClamp = arg1;
        },
        __wbg_set_mag_filter_ba0a2f25ad41eea8: function(arg0, arg1) {
            arg0.magFilter = __wbindgen_enum_GpuFilterMode[arg1];
        },
        __wbg_set_mapped_at_creation_8af5b87ccb898bbe: function(arg0, arg1) {
            arg0.mappedAtCreation = arg1 !== 0;
        },
        __wbg_set_mask_0de0e782abec3c25: function(arg0, arg1) {
            arg0.mask = arg1 >>> 0;
        },
        __wbg_set_max_anisotropy_8361213d3df93532: function(arg0, arg1) {
            arg0.maxAnisotropy = arg1;
        },
        __wbg_set_min_filter_8aa8b0f7523a0281: function(arg0, arg1) {
            arg0.minFilter = __wbindgen_enum_GpuFilterMode[arg1];
        },
        __wbg_set_mip_level_count_c0ac46de264d83ec: function(arg0, arg1) {
            arg0.mipLevelCount = arg1 >>> 0;
        },
        __wbg_set_mip_level_count_cd650972803ed9eb: function(arg0, arg1) {
            arg0.mipLevelCount = arg1 >>> 0;
        },
        __wbg_set_mipmap_filter_1cbdec28d1b54d9d: function(arg0, arg1) {
            arg0.mipmapFilter = __wbindgen_enum_GpuMipmapFilterMode[arg1];
        },
        __wbg_set_mode_51d2c5a5ac0f7ac9: function(arg0, arg1) {
            arg0.mode = __wbindgen_enum_GpuCanvasToneMappingMode[arg1];
        },
        __wbg_set_module_3dec1a50bca7870a: function(arg0, arg1) {
            arg0.module = arg1;
        },
        __wbg_set_module_b42baf3925e2a94a: function(arg0, arg1) {
            arg0.module = arg1;
        },
        __wbg_set_multisample_6af76792fae6efb1: function(arg0, arg1) {
            arg0.multisample = arg1;
        },
        __wbg_set_offset_f64_52f728fc5c98a52c: function(arg0, arg1) {
            arg0.offset = arg1;
        },
        __wbg_set_offset_f64_9374d9e912985773: function(arg0, arg1) {
            arg0.offset = arg1;
        },
        __wbg_set_operation_e103287df7ac9cf6: function(arg0, arg1) {
            arg0.operation = __wbindgen_enum_GpuBlendOperation[arg1];
        },
        __wbg_set_pass_op_4d08c3105f64a17a: function(arg0, arg1) {
            arg0.passOp = __wbindgen_enum_GpuStencilOperation[arg1];
        },
        __wbg_set_power_preference_6bc1d2697a0e7374: function(arg0, arg1) {
            arg0.powerPreference = __wbindgen_enum_GpuPowerPreference[arg1];
        },
        __wbg_set_primitive_ef867d733c4efb1c: function(arg0, arg1) {
            arg0.primitive = arg1;
        },
        __wbg_set_query_set_765c4c9ab99342cb: function(arg0, arg1) {
            arg0.querySet = arg1;
        },
        __wbg_set_r_1116d6a5f656263d: function(arg0, arg1) {
            arg0.r = arg1;
        },
        __wbg_set_required_features_5bda1420e19461b2: function(arg0, arg1, arg2) {
            arg0.requiredFeatures = getArrayJsValueViewFromWasm0(arg1, arg2);
        },
        __wbg_set_required_limits_e78b923d73f9b596: function(arg0, arg1) {
            arg0.requiredLimits = arg1;
        },
        __wbg_set_resolve_target_gpu_texture_view_a7a97a7c04d84698: function(arg0, arg1) {
            arg0.resolveTarget = arg1;
        },
        __wbg_set_resource_0f960a4279e7e400: function(arg0, arg1) {
            arg0.resource = arg1;
        },
        __wbg_set_resource_gpu_buffer_binding_53a2189a77dd38be: function(arg0, arg1) {
            arg0.resource = arg1;
        },
        __wbg_set_resource_gpu_texture_view_9153a2356b958667: function(arg0, arg1) {
            arg0.resource = arg1;
        },
        __wbg_set_sample_count_8262aacc91878e4d: function(arg0, arg1) {
            arg0.sampleCount = arg1 >>> 0;
        },
        __wbg_set_shader_location_31dc427c303a32aa: function(arg0, arg1) {
            arg0.shaderLocation = arg1 >>> 0;
        },
        __wbg_set_size_f64_3ff72c0d736c42a1: function(arg0, arg1) {
            arg0.size = arg1;
        },
        __wbg_set_size_f64_9b65e4916b2eb111: function(arg0, arg1) {
            arg0.size = arg1;
        },
        __wbg_set_size_gpu_extent_3d_dict_bba6669c79c26190: function(arg0, arg1) {
            arg0.size = arg1;
        },
        __wbg_set_src_factor_7e90d4b7d8227fc5: function(arg0, arg1) {
            arg0.srcFactor = __wbindgen_enum_GpuBlendFactor[arg1];
        },
        __wbg_set_stencil_back_6b5aeefbb90ae3ae: function(arg0, arg1) {
            arg0.stencilBack = arg1;
        },
        __wbg_set_stencil_clear_value_56fa848198a9b430: function(arg0, arg1) {
            arg0.stencilClearValue = arg1 >>> 0;
        },
        __wbg_set_stencil_front_4ffeca9473bca095: function(arg0, arg1) {
            arg0.stencilFront = arg1;
        },
        __wbg_set_stencil_load_op_b7a9fe3ad789b6c6: function(arg0, arg1) {
            arg0.stencilLoadOp = __wbindgen_enum_GpuLoadOp[arg1];
        },
        __wbg_set_stencil_read_mask_7cfe0945679de134: function(arg0, arg1) {
            arg0.stencilReadMask = arg1 >>> 0;
        },
        __wbg_set_stencil_read_only_0dde0c073192b5fa: function(arg0, arg1) {
            arg0.stencilReadOnly = arg1 !== 0;
        },
        __wbg_set_stencil_store_op_d320bcbb72456a4c: function(arg0, arg1) {
            arg0.stencilStoreOp = __wbindgen_enum_GpuStoreOp[arg1];
        },
        __wbg_set_stencil_write_mask_765017d5830ee98a: function(arg0, arg1) {
            arg0.stencilWriteMask = arg1 >>> 0;
        },
        __wbg_set_step_mode_2b1318f00e2ddc02: function(arg0, arg1) {
            arg0.stepMode = __wbindgen_enum_GpuVertexStepMode[arg1];
        },
        __wbg_set_store_op_e2b740eddfd2713f: function(arg0, arg1) {
            arg0.storeOp = __wbindgen_enum_GpuStoreOp[arg1];
        },
        __wbg_set_strip_index_format_22ddab1b8f352c1d: function(arg0, arg1) {
            arg0.stripIndexFormat = __wbindgen_enum_GpuIndexFormat[arg1];
        },
        __wbg_set_targets_07e15f3cca640faf: function(arg0, arg1, arg2) {
            arg0.targets = getArrayJsValueViewFromWasm0(arg1, arg2);
        },
        __wbg_set_timestamp_writes_1d304b9eb09ff227: function(arg0, arg1) {
            arg0.timestampWrites = arg1;
        },
        __wbg_set_tone_mapping_23fcd4f846a61048: function(arg0, arg1) {
            arg0.toneMapping = arg1;
        },
        __wbg_set_topology_b29804e5549370f5: function(arg0, arg1) {
            arg0.topology = __wbindgen_enum_GpuPrimitiveTopology[arg1];
        },
        __wbg_set_type_813f0d8c7822259c: function(arg0, arg1) {
            arg0.type = __wbindgen_enum_GpuQueryType[arg1];
        },
        __wbg_set_unclipped_depth_59155eb615f193fe: function(arg0, arg1) {
            arg0.unclippedDepth = arg1 !== 0;
        },
        __wbg_set_usage_5ba672bf81ba9859: function(arg0, arg1) {
            arg0.usage = arg1 >>> 0;
        },
        __wbg_set_usage_b0070c2ce47149a6: function(arg0, arg1) {
            arg0.usage = arg1 >>> 0;
        },
        __wbg_set_usage_bac3cabe90090275: function(arg0, arg1) {
            arg0.usage = arg1 >>> 0;
        },
        __wbg_set_usage_f3e1a48118ff483a: function(arg0, arg1) {
            arg0.usage = arg1 >>> 0;
        },
        __wbg_set_vertex_00b1e17dc3fe7fb3: function(arg0, arg1) {
            arg0.vertex = arg1;
        },
        __wbg_set_view_formats_2540cdb8dde31055: function(arg0, arg1, arg2) {
            arg0.viewFormats = getArrayJsValueViewFromWasm0(arg1, arg2);
        },
        __wbg_set_view_formats_845b425c1d19e1af: function(arg0, arg1, arg2) {
            arg0.viewFormats = getArrayJsValueViewFromWasm0(arg1, arg2);
        },
        __wbg_set_view_gpu_texture_view_9a3c34ba4d1969b8: function(arg0, arg1) {
            arg0.view = arg1;
        },
        __wbg_set_view_gpu_texture_view_efa644c1ce3c4deb: function(arg0, arg1) {
            arg0.view = arg1;
        },
        __wbg_set_width_5f89a551b5472887: function(arg0, arg1) {
            arg0.width = arg1 >>> 0;
        },
        __wbg_set_width_a2410cafbdefeb5a: function(arg0, arg1) {
            arg0.width = arg1 >>> 0;
        },
        __wbg_set_width_eb3001e1dcabb909: function(arg0, arg1) {
            arg0.width = arg1 >>> 0;
        },
        __wbg_set_write_mask_39ce121635fd4296: function(arg0, arg1) {
            arg0.writeMask = arg1 >>> 0;
        },
        __wbg_static_accessor_GLOBAL_THIS_003ee833af9699fc: function() {
            const ret = typeof globalThis === 'undefined' ? undefined : globalThis;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_GLOBAL_e4293fb22cbe77da: function() {
            const ret = typeof global === 'undefined' ? undefined : global;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_SELF_53bb20457401453d: function() {
            const ret = typeof self === 'undefined' ? undefined : self;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_WINDOW_b5e0810bfecb7feb: function() {
            const ret = typeof window === 'undefined' ? undefined : window;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_subgroupMaxSize_3586e822dc3603f6: function(arg0) {
            const ret = arg0.subgroupMaxSize;
            return ret;
        },
        __wbg_subgroupMinSize_17f758985cdd1dab: function(arg0) {
            const ret = arg0.subgroupMinSize;
            return ret;
        },
        __wbg_submit_077c85cc28e36892: function(arg0, arg1, arg2) {
            arg0.submit(getArrayJsValueViewFromWasm0(arg1, arg2));
        },
        __wbg_then_656f253a5dd7d8ea: function(arg0, arg1, arg2) {
            const ret = arg0.then(arg1, arg2);
            return ret;
        },
        __wbg_then_e3f7f2d365708be8: function(arg0, arg1) {
            const ret = arg0.then(arg1);
            return ret;
        },
        __wbg_unconfigure_835307f58dc68d80: function(arg0) {
            arg0.unconfigure();
        },
        __wbg_unmap_6a96b14c9ef5f7f5: function(arg0) {
            arg0.unmap();
        },
        __wbg_viewer_new: function(arg0) {
            const ret = Viewer.__wrap(arg0);
            return ret;
        },
        __wbg_width_babc5dc7654771b1: function(arg0) {
            const ret = arg0.width;
            return ret;
        },
        __wbg_writeBuffer_d7640e64ceb69371: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
            arg0.writeBuffer(arg1, arg2, getArrayU8FromWasm0(arg3, arg4), arg5, arg6);
        }, arguments); },
        __wbindgen_generic_0000000000000001: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [Externref], shim_idx: 7, ret: Result(Unit), inner_ret: Some(Result(Unit)) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen_afd209a88ce49cc3___convert__closures_____invoke___wasm_bindgen_afd209a88ce49cc3___sys__JsNullable_wgpu_c01756cc203236d7___backend__webgpu__webgpu_sys__gen_GpuAdapter__GpuAdapter___core_37ee0ce0c4cd57d5___result__Result_____wasm_bindgen_afd209a88ce49cc3___JsError___true_);
            return ret;
        },
        __wbindgen_generic_0000000000000002: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [NamedExternref("GPUDevice")], shim_idx: 7, ret: Result(Unit), inner_ret: Some(Result(Unit)) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen_afd209a88ce49cc3___convert__closures_____invoke___wasm_bindgen_afd209a88ce49cc3___sys__JsNullable_wgpu_c01756cc203236d7___backend__webgpu__webgpu_sys__gen_GpuAdapter__GpuAdapter___core_37ee0ce0c4cd57d5___result__Result_____wasm_bindgen_afd209a88ce49cc3___JsError___true__119);
            return ret;
        },
        __wbindgen_generic_0000000000000003: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [NamedExternref("any")], shim_idx: 7, ret: Result(Unit), inner_ret: Some(Result(Unit)) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen_afd209a88ce49cc3___convert__closures_____invoke___wasm_bindgen_afd209a88ce49cc3___sys__JsNullable_wgpu_c01756cc203236d7___backend__webgpu__webgpu_sys__gen_GpuAdapter__GpuAdapter___core_37ee0ce0c4cd57d5___result__Result_____wasm_bindgen_afd209a88ce49cc3___JsError___true__120);
            return ret;
        },
        __wbindgen_generic_0000000000000004: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [NamedExternref("undefined")], shim_idx: 7, ret: Result(Unit), inner_ret: Some(Result(Unit)) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen_afd209a88ce49cc3___convert__closures_____invoke___wasm_bindgen_afd209a88ce49cc3___sys__JsNullable_wgpu_c01756cc203236d7___backend__webgpu__webgpu_sys__gen_GpuAdapter__GpuAdapter___core_37ee0ce0c4cd57d5___result__Result_____wasm_bindgen_afd209a88ce49cc3___JsError___true__121);
            return ret;
        },
        __wbindgen_generic_0000000000000005: function(arg0) {
            // Cast intrinsic for `F64 -> Externref`.
            const ret = arg0;
            return ret;
        },
        __wbindgen_generic_0000000000000006: function(arg0, arg1) {
            // Cast intrinsic for `Ref(Slice(U8)) -> NamedExternref("Uint8Array")`.
            const ret = getArrayU8FromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_generic_0000000000000007: function(arg0, arg1) {
            // Cast intrinsic for `Ref(String) -> Externref`.
            const ret = getStringFromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_init_externref_table: function() {
            const table = wasm.__wbindgen_externrefs;
            table.set(0, undefined);
            table.set(1024, undefined);
            table.set(1025, null);
            table.set(1026, true);
            table.set(1027, false);
        },
    };
    return {
        __proto__: null,
        "./scalia_wasm_bg.js": import0,
    };
}

function wasm_bindgen_afd209a88ce49cc3___convert__closures_____invoke___wasm_bindgen_afd209a88ce49cc3___sys__JsNullable_wgpu_c01756cc203236d7___backend__webgpu__webgpu_sys__gen_GpuAdapter__GpuAdapter___core_37ee0ce0c4cd57d5___result__Result_____wasm_bindgen_afd209a88ce49cc3___JsError___true_(arg0, arg1, arg2) {
    const ret = wasm.wasm_bindgen_afd209a88ce49cc3___convert__closures_____invoke___wasm_bindgen_afd209a88ce49cc3___sys__JsNullable_wgpu_c01756cc203236d7___backend__webgpu__webgpu_sys__gen_GpuAdapter__GpuAdapter___core_37ee0ce0c4cd57d5___result__Result_____wasm_bindgen_afd209a88ce49cc3___JsError___true_(arg0, arg1, arg2);
    if (ret[1]) {
        throw takeFromExternrefTable0(ret[0]);
    }
}

function wasm_bindgen_afd209a88ce49cc3___convert__closures_____invoke___wasm_bindgen_afd209a88ce49cc3___sys__JsNullable_wgpu_c01756cc203236d7___backend__webgpu__webgpu_sys__gen_GpuAdapter__GpuAdapter___core_37ee0ce0c4cd57d5___result__Result_____wasm_bindgen_afd209a88ce49cc3___JsError___true__119(arg0, arg1, arg2) {
    const ret = wasm.wasm_bindgen_afd209a88ce49cc3___convert__closures_____invoke___wasm_bindgen_afd209a88ce49cc3___sys__JsNullable_wgpu_c01756cc203236d7___backend__webgpu__webgpu_sys__gen_GpuAdapter__GpuAdapter___core_37ee0ce0c4cd57d5___result__Result_____wasm_bindgen_afd209a88ce49cc3___JsError___true__119(arg0, arg1, arg2);
    if (ret[1]) {
        throw takeFromExternrefTable0(ret[0]);
    }
}

function wasm_bindgen_afd209a88ce49cc3___convert__closures_____invoke___wasm_bindgen_afd209a88ce49cc3___sys__JsNullable_wgpu_c01756cc203236d7___backend__webgpu__webgpu_sys__gen_GpuAdapter__GpuAdapter___core_37ee0ce0c4cd57d5___result__Result_____wasm_bindgen_afd209a88ce49cc3___JsError___true__120(arg0, arg1, arg2) {
    const ret = wasm.wasm_bindgen_afd209a88ce49cc3___convert__closures_____invoke___wasm_bindgen_afd209a88ce49cc3___sys__JsNullable_wgpu_c01756cc203236d7___backend__webgpu__webgpu_sys__gen_GpuAdapter__GpuAdapter___core_37ee0ce0c4cd57d5___result__Result_____wasm_bindgen_afd209a88ce49cc3___JsError___true__120(arg0, arg1, arg2);
    if (ret[1]) {
        throw takeFromExternrefTable0(ret[0]);
    }
}

function wasm_bindgen_afd209a88ce49cc3___convert__closures_____invoke___wasm_bindgen_afd209a88ce49cc3___sys__JsNullable_wgpu_c01756cc203236d7___backend__webgpu__webgpu_sys__gen_GpuAdapter__GpuAdapter___core_37ee0ce0c4cd57d5___result__Result_____wasm_bindgen_afd209a88ce49cc3___JsError___true__121(arg0, arg1, arg2) {
    const ret = wasm.wasm_bindgen_afd209a88ce49cc3___convert__closures_____invoke___wasm_bindgen_afd209a88ce49cc3___sys__JsNullable_wgpu_c01756cc203236d7___backend__webgpu__webgpu_sys__gen_GpuAdapter__GpuAdapter___core_37ee0ce0c4cd57d5___result__Result_____wasm_bindgen_afd209a88ce49cc3___JsError___true__121(arg0, arg1, arg2);
    if (ret[1]) {
        throw takeFromExternrefTable0(ret[0]);
    }
}

function wasm_bindgen_afd209a88ce49cc3___convert__closures_____invoke___js_sys_4c0a7da8c3256c44___Function_fn_wasm_bindgen_afd209a88ce49cc3___JsValue_____wasm_bindgen_afd209a88ce49cc3___sys__Undefined___js_sys_4c0a7da8c3256c44___Function_fn_wasm_bindgen_afd209a88ce49cc3___JsValue_____wasm_bindgen_afd209a88ce49cc3___sys__Undefined_______true_(arg0, arg1, arg2, arg3) {
    wasm.wasm_bindgen_afd209a88ce49cc3___convert__closures_____invoke___js_sys_4c0a7da8c3256c44___Function_fn_wasm_bindgen_afd209a88ce49cc3___JsValue_____wasm_bindgen_afd209a88ce49cc3___sys__Undefined___js_sys_4c0a7da8c3256c44___Function_fn_wasm_bindgen_afd209a88ce49cc3___JsValue_____wasm_bindgen_afd209a88ce49cc3___sys__Undefined_______true_(arg0, arg1, arg2, arg3);
}


const __wbindgen_enum_GpuAddressMode = ["clamp-to-edge", "repeat", "mirror-repeat"];


const __wbindgen_enum_GpuAutoLayoutMode = ["auto"];


const __wbindgen_enum_GpuBlendFactor = ["zero", "one", "src", "one-minus-src", "src-alpha", "one-minus-src-alpha", "dst", "one-minus-dst", "dst-alpha", "one-minus-dst-alpha", "src-alpha-saturated", "constant", "one-minus-constant", "src1", "one-minus-src1", "src1-alpha", "one-minus-src1-alpha"];


const __wbindgen_enum_GpuBlendOperation = ["add", "subtract", "reverse-subtract", "min", "max"];


const __wbindgen_enum_GpuCanvasAlphaMode = ["opaque", "premultiplied"];


const __wbindgen_enum_GpuCanvasToneMappingMode = ["standard", "extended"];


const __wbindgen_enum_GpuCompareFunction = ["never", "less", "equal", "less-equal", "greater", "not-equal", "greater-equal", "always"];


const __wbindgen_enum_GpuCullMode = ["none", "front", "back"];


const __wbindgen_enum_GpuFilterMode = ["nearest", "linear"];


const __wbindgen_enum_GpuFrontFace = ["ccw", "cw"];


const __wbindgen_enum_GpuIndexFormat = ["uint16", "uint32"];


const __wbindgen_enum_GpuLoadOp = ["load", "clear"];


const __wbindgen_enum_GpuMipmapFilterMode = ["nearest", "linear"];


const __wbindgen_enum_GpuPowerPreference = ["low-power", "high-performance"];


const __wbindgen_enum_GpuPrimitiveTopology = ["point-list", "line-list", "line-strip", "triangle-list", "triangle-strip"];


const __wbindgen_enum_GpuQueryType = ["occlusion", "timestamp"];


const __wbindgen_enum_GpuStencilOperation = ["keep", "zero", "replace", "invert", "increment-clamp", "decrement-clamp", "increment-wrap", "decrement-wrap"];


const __wbindgen_enum_GpuStoreOp = ["store", "discard"];


const __wbindgen_enum_GpuTextureAspect = ["all", "stencil-only", "depth-only"];


const __wbindgen_enum_GpuTextureDimension = ["1d", "2d", "3d"];


const __wbindgen_enum_GpuTextureFormat = ["r8unorm", "r8snorm", "r8uint", "r8sint", "r16unorm", "r16snorm", "r16uint", "r16sint", "r16float", "rg8unorm", "rg8snorm", "rg8uint", "rg8sint", "r32uint", "r32sint", "r32float", "rg16unorm", "rg16snorm", "rg16uint", "rg16sint", "rg16float", "rgba8unorm", "rgba8unorm-srgb", "rgba8snorm", "rgba8uint", "rgba8sint", "bgra8unorm", "bgra8unorm-srgb", "rgb9e5ufloat", "rgb10a2uint", "rgb10a2unorm", "rg11b10ufloat", "rg32uint", "rg32sint", "rg32float", "rgba16unorm", "rgba16snorm", "rgba16uint", "rgba16sint", "rgba16float", "rgba32uint", "rgba32sint", "rgba32float", "stencil8", "depth16unorm", "depth24plus", "depth24plus-stencil8", "depth32float", "depth32float-stencil8", "bc1-rgba-unorm", "bc1-rgba-unorm-srgb", "bc2-rgba-unorm", "bc2-rgba-unorm-srgb", "bc3-rgba-unorm", "bc3-rgba-unorm-srgb", "bc4-r-unorm", "bc4-r-snorm", "bc5-rg-unorm", "bc5-rg-snorm", "bc6h-rgb-ufloat", "bc6h-rgb-float", "bc7-rgba-unorm", "bc7-rgba-unorm-srgb", "etc2-rgb8unorm", "etc2-rgb8unorm-srgb", "etc2-rgb8a1unorm", "etc2-rgb8a1unorm-srgb", "etc2-rgba8unorm", "etc2-rgba8unorm-srgb", "eac-r11unorm", "eac-r11snorm", "eac-rg11unorm", "eac-rg11snorm", "astc-4x4-unorm", "astc-4x4-unorm-srgb", "astc-5x4-unorm", "astc-5x4-unorm-srgb", "astc-5x5-unorm", "astc-5x5-unorm-srgb", "astc-6x5-unorm", "astc-6x5-unorm-srgb", "astc-6x6-unorm", "astc-6x6-unorm-srgb", "astc-8x5-unorm", "astc-8x5-unorm-srgb", "astc-8x6-unorm", "astc-8x6-unorm-srgb", "astc-8x8-unorm", "astc-8x8-unorm-srgb", "astc-10x5-unorm", "astc-10x5-unorm-srgb", "astc-10x6-unorm", "astc-10x6-unorm-srgb", "astc-10x8-unorm", "astc-10x8-unorm-srgb", "astc-10x10-unorm", "astc-10x10-unorm-srgb", "astc-12x10-unorm", "astc-12x10-unorm-srgb", "astc-12x12-unorm", "astc-12x12-unorm-srgb"];


const __wbindgen_enum_GpuTextureViewDimension = ["1d", "2d", "2d-array", "cube", "cube-array", "3d"];


const __wbindgen_enum_GpuVertexFormat = ["uint8", "uint8x2", "uint8x4", "sint8", "sint8x2", "sint8x4", "unorm8", "unorm8x2", "unorm8x4", "snorm8", "snorm8x2", "snorm8x4", "uint16", "uint16x2", "uint16x4", "sint16", "sint16x2", "sint16x4", "unorm16", "unorm16x2", "unorm16x4", "snorm16", "snorm16x2", "snorm16x4", "float16", "float16x2", "float16x4", "float32", "float32x2", "float32x3", "float32x4", "uint32", "uint32x2", "uint32x3", "uint32x4", "sint32", "sint32x2", "sint32x3", "sint32x4", "unorm10-10-10-2", "unorm8x4-bgra"];


const __wbindgen_enum_GpuVertexStepMode = ["vertex", "instance"];
const FlightFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_flight_free(ptr, 1));
const SimFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_sim_free(ptr, 1));
const ViewerFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_viewer_free(ptr, 1));

function addToExternrefTable0(obj) {
    const idx = wasm.__externref_table_alloc();
    wasm.__wbindgen_externrefs.set(idx, obj);
    return idx;
}

const CLOSURE_DTORS = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(state => wasm.__wbindgen_destroy_closure(state.a, state.b));

function debugString(val) {
    switch (typeof val) {
        // primitive types
        case 'number':
        case 'boolean':
        case 'undefined':
        return `${val}`;
        case 'string':
        return JSON.stringify(val);
        case 'bigint':
        return `${val}n`;
        case 'symbol':
        return val.description !== undefined
            ? `Symbol(${val.description})`
            : 'Symbol';
        case 'function':
        return typeof val.name === 'string' && val.name !== ''
            ? `Function(${val.name})`
            : 'Function';
        case 'object': {
            if (val === null) {
                return 'null';
            }
            // objects
            if (Array.isArray(val)) {
                return '[' + Array.from(val, debugString).join(', ') + ']';
            }
            // errors
            if (val instanceof Error) {
                return `${val.name}: ${val.message}\n${val.stack}`;
            }

            const toStringOutput = toString.call(val);
            // Test for built-in
            const className = /\[object ([^\]]+)\]/.exec(toStringOutput)?.[1];

            // Failed to match the standard '[object ClassName]'
            if (className === undefined) {
                return toStringOutput;
            }
            if (className === 'Object') {
                // we're a user defined class or Object
                // JSON.stringify avoids problems with cycles, and is generally much
                // easier than looping through ownProperties of `val`.
                try {
                    return 'Object(' + JSON.stringify(val) + ')';
                } catch (_) {
                    return 'Object';
                }
            }
            // TODO we could test for more things here, like `Set`s and `Map`s.
            return className;
        }
        // Per MDN: 'This list of values is exhaustive.'
        // https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Operators/typeof#description
    }
}

function getArrayF32FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getFloat32ArrayMemory0().subarray(ptr / 4, ptr / 4 + len);
}

function getArrayF64FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getFloat64ArrayMemory0().subarray(ptr / 8, ptr / 8 + len);
}

function getArrayI32FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getInt32ArrayMemory0().subarray(ptr / 4, ptr / 4 + len);
}

function getArrayJsValueViewFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    const mem = getDataViewMemory0();
    const result = [];
    for (let i = ptr; i < ptr + 4 * len; i += 4) {
        result.push(wasm.__wbindgen_externrefs.get(mem.getUint32(i, true)));
    }
    return result;
}

function getArrayU8FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
}

let cachedDataViewMemory0 = null;
function getDataViewMemory0() {
    if (cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer.detached === true || (cachedDataViewMemory0.buffer.detached === undefined && cachedDataViewMemory0.buffer !== wasm.memory.buffer)) {
        cachedDataViewMemory0 = new DataView(wasm.memory.buffer);
    }
    return cachedDataViewMemory0;
}

let cachedFloat32ArrayMemory0 = null;
function getFloat32ArrayMemory0() {
    if (cachedFloat32ArrayMemory0 === null || cachedFloat32ArrayMemory0.byteLength === 0) {
        cachedFloat32ArrayMemory0 = new Float32Array(wasm.memory.buffer);
    }
    return cachedFloat32ArrayMemory0;
}

let cachedFloat64ArrayMemory0 = null;
function getFloat64ArrayMemory0() {
    if (cachedFloat64ArrayMemory0 === null || cachedFloat64ArrayMemory0.byteLength === 0) {
        cachedFloat64ArrayMemory0 = new Float64Array(wasm.memory.buffer);
    }
    return cachedFloat64ArrayMemory0;
}

let cachedInt32ArrayMemory0 = null;
function getInt32ArrayMemory0() {
    if (cachedInt32ArrayMemory0 === null || cachedInt32ArrayMemory0.byteLength === 0) {
        cachedInt32ArrayMemory0 = new Int32Array(wasm.memory.buffer);
    }
    return cachedInt32ArrayMemory0;
}

function getStringFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return decodeText(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

let cachedUint32ArrayMemory0 = null;
function getUint32ArrayMemory0() {
    if (cachedUint32ArrayMemory0 === null || cachedUint32ArrayMemory0.byteLength === 0) {
        cachedUint32ArrayMemory0 = new Uint32Array(wasm.memory.buffer);
    }
    return cachedUint32ArrayMemory0;
}

let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function handleError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        const idx = addToExternrefTable0(e);
        wasm.__wbindgen_exn_store(idx);
    }
}

function isLikeNone(x) {
    return x === undefined || x === null;
}

function makeMutClosure(arg0, arg1, f) {
    const state = { a: arg0, b: arg1, cnt: 1 };
    const real = (...args) => {

        // First up with a closure we increment the internal reference
        // count. This ensures that the Rust closure environment won't
        // be deallocated while we're invoking it.
        state.cnt++;
        const a = state.a;
        state.a = 0;
        try {
            return f(a, state.b, ...args);
        } finally {
            state.a = a;
            real._wbg_cb_unref();
        }
    };
    real._wbg_cb_unref = () => {
        if (--state.cnt === 0) {
            wasm.__wbindgen_destroy_closure(state.a, state.b);
            state.a = 0;
            CLOSURE_DTORS.unregister(state);
        }
    };
    CLOSURE_DTORS.register(real, state, state);
    return real;
}

function passArray32ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 4, 4) >>> 0;
    getUint32ArrayMemory0().set(arg, ptr / 4);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}

function passStringToWasm0(arg, malloc, realloc) {
    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }
    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = cachedTextEncoder.encodeInto(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

function takeFromExternrefTable0(idx) {
    const value = wasm.__wbindgen_externrefs.get(idx);
    wasm.__externref_table_dealloc(idx);
    return value;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true });
cachedTextDecoder.decode();
const MAX_SAFARI_DECODE_BYTES = 2146435072;
let numBytesDecoded = 0;
function decodeText(bytes) {
    numBytesDecoded += bytes.length;
    if (numBytesDecoded >= MAX_SAFARI_DECODE_BYTES) {
        cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true });
        cachedTextDecoder.decode();
        numBytesDecoded = bytes.length;
    }
    return cachedTextDecoder.decode(bytes);
}

const cachedTextEncoder = new TextEncoder();

if (!('encodeInto' in cachedTextEncoder)) {
    cachedTextEncoder.encodeInto = function (arg, view) {
        const buf = cachedTextEncoder.encode(arg);
        view.set(buf);
        return {
            read: arg.length,
            written: buf.length
        };
    };
}

let WASM_VECTOR_LEN = 0;

let wasmModule, wasmInstance, wasm;
function __wbg_finalize_init(instance, module) {
    wasmInstance = instance;
    wasm = instance.exports;
    wasmModule = module;
    cachedDataViewMemory0 = null;
    cachedFloat32ArrayMemory0 = null;
    cachedFloat64ArrayMemory0 = null;
    cachedInt32ArrayMemory0 = null;
    cachedUint32ArrayMemory0 = null;
    cachedUint8ArrayMemory0 = null;
    wasm.__wbindgen_start();
    return wasm;
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (!module.ok) {
            throw new Error(`failed to fetch Wasm: ${module.status} ${module.statusText} fetching '${module.url}'`);
        }

        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);
            } catch (e) {
                const validResponse = expectedResponseType(module.type);

                if (validResponse && module.headers.get('Content-Type') !== 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else { throw e; }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);
    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };
        } else {
            return instance;
        }
    }

    function expectedResponseType(type) {
        switch (type) {
            case 'basic': case 'cors': case 'default': return true;
        }
        return false;
    }
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (module !== undefined) {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();
    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }
    const instance = new WebAssembly.Instance(module, imports);
    return __wbg_finalize_init(instance, module);
}

let __wbg_init_promise;
async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;
    if (__wbg_init_promise === undefined) {
        __wbg_init_promise = __wbg_init_once(module_or_path);
        __wbg_init_promise.catch(() => { __wbg_init_promise = undefined; });
    }
    return __wbg_init_promise;
}

async function __wbg_init_once(module_or_path) {

    if (module_or_path !== undefined) {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (module_or_path === undefined) {
        module_or_path = new URL('scalia_wasm_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync, __wbg_init as default };
