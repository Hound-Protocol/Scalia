/* tslint:disable */
/* eslint-disable */

/**
 * A flight from the parking spot at one body to the parking spot at another.
 */
export class Flight {
    free(): void;
    [Symbol.dispose](): void;
    duration_ms(): number;
    from(): Float64Array;
    constructor(from_body: number, to_body: number, depart_ms: number, peak_mm_s: number);
    /**
     * `[x_mm, y_mm]` at time `t_ms`.
     */
    pos(t_ms: number): Float64Array;
    to(): Float64Array;
}

export class Sim {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Fills the output buffers and returns how many individuals were derived.
     * Positions are `f32` metres relative to the view centre.
     */
    derive(t_ms: number, region: number, min_x: number, max_x: number, min_y: number, max_y: number, cap: number): number;
    constructor(seed: bigint, groups: number, per_group: number);
    phases(): Uint8Array;
    phases_ptr(): number;
    tick(now_ms: number): void;
    xs(): Float32Array;
    xs_ptr(): number;
    ys(): Float32Array;
    ys_ptr(): number;
    zs(): Float32Array;
    zs_ptr(): number;
}

export class Viewer {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * What the selected units can do at a map point, as JSON [{id, label, ok, reason}]. Each unit's AI checks
     * the ground: inside the testing ground it must find a way there; what the radar shows, they know.
     */
    actions_at(x: number, y: number): string;
    assault_state(): string;
    /**
     * Puts a squad under a captain (captain -1: it answers to the commander again).
     */
    attach(squad: number, captain: number): string;
    /**
     * Attaches the tablet's map canvas: drawn with the map camera while the tablet is up.
     */
    attach_map(canvas: HTMLCanvasElement): void;
    /**
     * The space at the listener's spot on the ground (ambience and the console):
     * [enclosure 0..1, size (m), reverb time (s), corridor 0..1, through the tablet 0/1].
     */
    audio_env(): Float32Array;
    /**
     * Console: everyone on `team` (0 ours, 1 the enemy) drops dead; how many.
     */
    battle_kill(team: number): number;
    /**
     * Benchmark scene: `squads` squads of 12 around the commander (see `bench_army`). Returns the soldier count.
     */
    bench_army(squads: number): number;
    /**
     * Left click on the ground: selects the group under the cursor (or clears the selection).
     */
    click(px: number, py: number, now_ms: number): string;
    /**
     * Right click on the ground: the selected group marches there (redirecting if already marching).
     */
    command(px: number, py: number, now_ms: number, run: boolean): string;
    /**
     * Takes command from a unit: the commander moves to stand with it, facing the way it faces.
     */
    command_unit(id: number): string;
    commander_gait(): number;
    /**
     * Console: rounds pass through him harmlessly (true) or not; returns the new setting.
     */
    commander_god(): boolean;
    /**
     * Console: back on his feet at full health.
     */
    commander_heal(): void;
    /**
     * Slings the rifle on his back, or brings it back to his hands.
     */
    commander_holster(): boolean;
    /**
     * Console: where the commander is and what he is doing.
     */
    commander_info(): string;
    commander_phase(): number;
    /**
     * Console: a full magazine and `mags` spares.
     */
    commander_refill(mags: number): void;
    /**
     * A fresh magazine, if he has a spare.
     */
    commander_reload(): void;
    /**
     * The commander's gait phase (0..1), or -1 when he stands: a foot lands at 0.25 (left) and 0.75 (right).
     * Where the commander is on screen (fractions of the width and height from the top left; his chest).
     */
    commander_screen(): Float32Array;
    /**
     * Crouch (1) or go prone (2), or back up to standing when already so.
     */
    commander_stance(k: number): void;
    /**
     * One footfall of the commander's, `loud` 0..1, as one record of `take_sounds`: the room he is in rings
     * with it and nearby walls throw it back, like the soldiers' steps.
     */
    commander_step(loud: number): Float32Array;
    /**
     * The commander's gait for his footsteps: 0 standing, 1 walking, 2 sprinting.
     * What the commander walks on: 1 concrete, 0 grass.
     */
    commander_surface(): number;
    /**
     * Console: puts the commander at (x, y) metres on the ground (his man in the battle follows next frame).
     */
    commander_teleport(x: number, y: number): void;
    /**
     * Alt held: the commander walks instead of jogging.
     */
    commander_walk(walk: boolean): void;
    /**
     * The commander's trigger and aim (held or not), and where the mouse is (device px).
     */
    commander_weapon(trigger: boolean, aim: boolean, px: number, py: number): void;
    /**
     * The commander's weapon for the page, as JSON: rounds in the rifle and spare, reloading, slung, stance,
     * rounds fired so far, health (0..100), and the aim line [x0, y0, x1, y1] (screen fractions) while aiming.
     */
    commander_weapon_state(): string;
    /**
     * Cycles the army size; returns a description.
     */
    cycle_army(): string;
    /**
     * Cycles the speed of the day: 60-minute days (the game's real pace), 12-second days (to watch), paused.
     */
    cycle_day_speed(now_ms: number): string;
    /**
     * Cycles the selected group's idle stance: stand, crouch, cover aim, prone, standing aim.
     */
    cycle_stance(): string;
    /**
     * Drops one unit from the selection (it did not answer the radio).
     */
    deselect(id: number): void;
    /**
     * Console: list the doors, or set one ("door 3 open|close|lock|breach").
     */
    door_cmd(which: number, state: string): string;
    /**
     * The intro is over: the camera is the player's again, at the widest field zoom.
     */
    end_intro(): void;
    /**
     * The field battle in words (debugging: squads, positions, steps).
     */
    field_debug(): string;
    flying(): boolean;
    /**
     * Camera follows the units (ground) or the ship (system) again.
     */
    follow(): void;
    /**
     * Draws one frame and returns the status line.
     */
    frame(now_ms: number): string;
    /**
     * Carries out an order at a map point (after the radio exchange): 0 move, 1 defend, 2 assault, 3 support.
     * Assaults go at a sprint; on arrival a unit takes the stance that fits the order.
     */
    issue_at(action: number, x: number, y: number, now_ms: number): string;
    /**
     * AI lab: the feed of leader steps and shouts since the last call, one per line.
     */
    lab_feed(): string;
    /**
     * AI lab: every soldier on the testing ground on screen with what he is doing, as JSON
     * [{x, y (screen fractions, over his head), t (team), txt}].
     */
    lab_marks(): string;
    /**
     * AI lab: how fast battle time runs (0 pauses).
     */
    lab_set_speed(speed: number): void;
    /**
     * AI lab: clears the testing ground and sets up a scenario; the commander is placed to watch it. Returns
     * a line saying what to watch for.
     */
    lab_start(name: string): string;
    /**
     * Starts a flight from the docked body to the other one; ignored while flying.
     */
    launch(now_ms: number): void;
    /**
     * The tablet map glides back over the commander, close in (from wherever it was, the planet or
     * the star system included).
     */
    map_center_commander(): void;
    /**
     * The map point under a pixel (absolute metres), for orders issued after the radio exchange.
     */
    map_point(px: number, py: number): Float64Array;
    /**
     * Number of models the viewer can show (the blueprints, then the soldier last).
     */
    model_count(): number;
    /**
     * Holds pose `i` in the soldier preview (walk, run, stand, crouch, cover aim, prone, standing aim); -1 cycles.
     * Commander movement for this frame: x to the screen's right, y to its top (-1..1 each), and sprinting.
     */
    move_commander(x: number, y: number, sprint: boolean): void;
    /**
     * What is under a pixel (the active camera; the tablet map inside `use_map(true)`), as JSON:
     * {kind: unit|commander|barracks|tower|wall|sandbags|container|crate|rock|ground, id, grid, ...}.
     */
    object_at(px: number, py: number): string;
    /**
     * Drags the view by screen pixels: pans the ground and the system, turns ships.
     */
    pan(dx: number, dy: number): void;
    /**
     * Numbers for the performance panel: [fps, internal width, internal height,
     * gpu shadow ms, gpu scene ms, gpu upscale ms] (gpu values are -1 when the browser gives no timestamps).
     */
    perf(): Float32Array;
    preview_pose(i: number): void;
    /**
     * Radio traffic from the units since the last call, as a JSON array of {from, text, urgent}.
     */
    radio_events(): string;
    /**
     * Model viewer: back to the isometric angle, framed, spinning.
     */
    reset_model_view(): void;
    resize(w: number, h: number): void;
    resize_map(w: number, h: number): void;
    /**
     * The units under command, as JSON: [{id, name, soldiers, status, stance, selected}].
     */
    roster(): string;
    /**
     * Turns the camera by `d` radians.
     */
    rotate(d: number): void;
    scene_name(): string;
    /**
     * Shift + drag: selects every group whose centre lies inside the screen rectangle (projected onto the ground).
     */
    select_box(x0: number, y0: number, x1: number, y1: number, now_ms: number): string;
    /**
     * Selects exactly one unit without moving the camera (clicking its marker on the tablet map).
     */
    select_only(id: number): void;
    /**
     * Selects one unit (from the roster) and centres the tablet's map on it.
     */
    select_unit(id: number): void;
    /**
     * Names of the selected units (for the radio).
     * The selected units' ids, as JSON.
     */
    selection_ids(): string;
    selection_name(): string;
    set_full(on: boolean): void;
    /**
     * The menu's intro flight holds the camera: `view_m` metres of view height (from the star system all the
     * way down to the commander), looking down at `elev` radians, centred on his head; `fade` dissolves him.
     */
    set_intro(view_m: number, elev: number, fade: number): void;
    /**
     * How orders are carried out: 0 calm, 1 aggressive.
     */
    set_manner(m: number): void;
    /**
     * Model viewer: level of detail of the soldier (0 full, 1 mid, 2 far).
     */
    set_model_detail(d: number): void;
    /**
     * Model viewer: sun position as a time of day (0.25 sunrise, 0.5 noon, 0.75 sunset).
     */
    set_model_light(t: number): void;
    /**
     * Model viewer: colour variant (an index into `variant_name`).
     */
    set_model_variant(i: number): void;
    /**
     * How the next move is to be made, as chosen on the radio: 0 just move, 1 with caution (possible hostiles),
     * 2 garrison the area, 3 secure the area (hostiles confirmed).
     */
    set_order_mode(m: number): void;
    set_outline(on: boolean): void;
    /**
     * Screen pixels per internal pixel (the chunky look); 1 is full resolution.
     */
    set_pixel_scale(s: number): void;
    /**
     * The commander holds the radio to his ear (true) or lets it go.
     */
    set_radio(on: boolean): void;
    set_scene(name: string): void;
    /**
     * Replaces the selection (the radio hands each answering unit its order on its own).
     */
    set_selection(ids: Uint32Array | number[]): void;
    set_ship(i: number): void;
    /**
     * Raises or lowers the tablet. The field view stays on the commander; the map has its own camera, which
     * starts over the commander.
     */
    set_tablet(up: boolean): void;
    /**
     * The fraction of the screen's height the tablet close-up's box covers: the dive frames the tablet to match.
     */
    set_tablet_fit(f: number): void;
    /**
     * Scales the tablet close-up and shifts it sideways (fraction of the screen width, positive to the right).
     */
    set_tablet_frame(scale: number, shift: number): void;
    /**
     * Model viewer: spin on a turntable (true) or hold the angle it was left at.
     */
    set_turntable(on: boolean): void;
    /**
     * Starts the training assault: a squad of 12 forms up south of the killhouse and takes it from the 8
     * enemy soldiers inside, all on the battle AI. Starting again restarts it.
     */
    start_assault(_seed: number): void;
    /**
     * Ends the assault: the testing ground starts over (the enemy back on their posts).
     */
    stop_assault(): void;
    /**
     * A structure's scan overlay for the active camera: its faces, doors, windows and room partitions as screen
     * polygons (fractions of the canvas, y down), JSON {faces:[[x,y,...]], doors, windows, rooms} (rooms are lines).
     */
    structure_overlay(id: number): string;
    /**
     * Where the tablet cinematic is: 0 put away, 1 raising it (camera rising and diving), 2 in front of the eyes
     * (show the tablet's screen), 3 putting it away.
     */
    tablet_view(): number;
    /**
     * Sounds since the last call, 26 floats each, worked out by the acoustics for the ears at the camera:
     * kind (0 shot, 1 blast, 2 footfall), loudness (footfalls: 0 walk .. 1 sprint), delay from now (s, travel
     * time included), gain, low-pass (Hz), reverb of its own space 0..1, position from the ears (m: right,
     * ahead, below), distance (m), walls between, its space (enclosure 0..1, size m, corridor 0..1), then four
     * echoes off what stands around it (extra delay s, gain against the direct sound, pan -1..1; zeros when
     * there is nothing to bounce off).
     */
    take_sounds(): Float32Array;
    /**
     * The unit under a pixel of the current view, or -1.
     */
    unit_at(px: number, py: number): number;
    /**
     * Every unit as seen by the camera that is active right now (the field camera, or the tablet map's inside
     * `use_map(true)`): JSON [{id, cs, n, x, y, r, sel, you, men: [[x, y, readiness, health]], ready: [stable, wounded,
     * incapacitated, deceased], u}] with x, y as fractions of the canvas (0..1, y down), u a metre as a fraction of
     * its height and r
     * the unit's radius as a fraction of the canvas height. The commander is id -1. Units off screen are left out.
     */
    unit_marks(): string;
    /**
     * Routes input to the map's camera (true) or the field view (false). Calls between a `use_map(true)` and
     * `use_map(false)` (wheel, pan, click, command) act on the map.
     */
    use_map(on: boolean): void;
    variant_name(i: number): string | undefined;
    /**
     * Mouse wheel: one call is one step of the zoom ladder (positive zooms out); the camera glides there.
     * One zoom step in or out; true when the view actually changed (not already at the end of the ladder).
     */
    wheel(delta: number): boolean;
}

/**
 * The killhouse battle hash after `ms`: the same computation as the native `hash_battle` binary.
 */
export function battle_hash(seed: bigint, ms: number): bigint;

/**
 * Compact JSON summary: id, name, class, triangles and the mesh hash as a decimal string.
 */
export function blueprint_info(text: string): string;

/**
 * Flat-shaded triangle list for drawing: 4 floats per vertex (x, y, z in metres, palette index),
 * 3 vertices per triangle.
 */
export function blueprint_mesh(text: string): Float32Array;

/**
 * Parse and validate blueprint text, then draw its top-down SVG.
 */
export function blueprint_svg(text: string): string;

/**
 * Heights of one planet chunk, `(cells + 1)^2` values in millimetres.
 */
export function planet_chunk(seed: bigint, cx: number, cy: number, cells: number): Int32Array;

export function planet_chunk_hash(seed: bigint, cx: number, cy: number, cells: number): bigint;

/**
 * Same computation as the native `hash_scenario` binary.
 */
export function scenario_hash(seed: bigint): bigint;

/**
 * File text of shipped blueprint `index`, or undefined when there is no such blueprint.
 */
export function shipped_ron(index: number): string | undefined;

export function start_viewer(canvas: HTMLCanvasElement): Promise<Viewer>;

export function system_body_count(): number;

export function system_body_name(i: number): string | undefined;

/**
 * `[x_mm, y_mm]` of a body in the system region; empty for an unknown body.
 */
export function system_body_pos(i: number, t_ms: number): Float64Array;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_flight_free: (a: number, b: number) => void;
    readonly __wbg_sim_free: (a: number, b: number) => void;
    readonly __wbg_viewer_free: (a: number, b: number) => void;
    readonly battle_hash: (a: bigint, b: number) => bigint;
    readonly blueprint_info: (a: number, b: number) => [number, number, number, number];
    readonly blueprint_mesh: (a: number, b: number) => [number, number, number, number];
    readonly blueprint_svg: (a: number, b: number) => [number, number, number, number];
    readonly flight_duration_ms: (a: number) => number;
    readonly flight_from: (a: number) => [number, number];
    readonly flight_new: (a: number, b: number, c: number, d: number) => [number, number, number];
    readonly flight_pos: (a: number, b: number) => [number, number];
    readonly flight_to: (a: number) => [number, number];
    readonly planet_chunk: (a: bigint, b: number, c: number, d: number) => [number, number];
    readonly planet_chunk_hash: (a: bigint, b: number, c: number, d: number) => bigint;
    readonly scenario_hash: (a: bigint) => bigint;
    readonly shipped_ron: (a: number) => [number, number];
    readonly sim_derive: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => number;
    readonly sim_new: (a: bigint, b: number, c: number) => [number, number, number];
    readonly sim_phases: (a: number) => [number, number];
    readonly sim_phases_ptr: (a: number) => number;
    readonly sim_tick: (a: number, b: number) => [number, number];
    readonly sim_xs: (a: number) => [number, number];
    readonly sim_xs_ptr: (a: number) => number;
    readonly sim_ys: (a: number) => [number, number];
    readonly sim_ys_ptr: (a: number) => number;
    readonly sim_zs: (a: number) => [number, number];
    readonly sim_zs_ptr: (a: number) => number;
    readonly start_viewer: (a: any) => any;
    readonly system_body_count: () => number;
    readonly system_body_name: (a: number) => [number, number];
    readonly system_body_pos: (a: number, b: number) => [number, number];
    readonly viewer_actions_at: (a: number, b: number, c: number) => [number, number];
    readonly viewer_assault_state: (a: number) => [number, number];
    readonly viewer_attach: (a: number, b: number, c: number) => [number, number];
    readonly viewer_attach_map: (a: number, b: any) => [number, number];
    readonly viewer_audio_env: (a: number) => [number, number];
    readonly viewer_battle_kill: (a: number, b: number) => number;
    readonly viewer_bench_army: (a: number, b: number) => number;
    readonly viewer_click: (a: number, b: number, c: number, d: number) => [number, number];
    readonly viewer_command: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly viewer_command_unit: (a: number, b: number) => [number, number];
    readonly viewer_commander_gait: (a: number) => number;
    readonly viewer_commander_god: (a: number) => number;
    readonly viewer_commander_heal: (a: number) => void;
    readonly viewer_commander_holster: (a: number) => number;
    readonly viewer_commander_info: (a: number) => [number, number];
    readonly viewer_commander_phase: (a: number) => number;
    readonly viewer_commander_refill: (a: number, b: number) => void;
    readonly viewer_commander_reload: (a: number) => void;
    readonly viewer_commander_screen: (a: number) => [number, number];
    readonly viewer_commander_stance: (a: number, b: number) => void;
    readonly viewer_commander_step: (a: number, b: number) => [number, number];
    readonly viewer_commander_surface: (a: number) => number;
    readonly viewer_commander_teleport: (a: number, b: number, c: number) => void;
    readonly viewer_commander_walk: (a: number, b: number) => void;
    readonly viewer_commander_weapon: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly viewer_commander_weapon_state: (a: number) => [number, number];
    readonly viewer_cycle_army: (a: number) => [number, number];
    readonly viewer_cycle_day_speed: (a: number, b: number) => [number, number];
    readonly viewer_cycle_stance: (a: number) => [number, number];
    readonly viewer_deselect: (a: number, b: number) => void;
    readonly viewer_door_cmd: (a: number, b: number, c: number, d: number) => [number, number];
    readonly viewer_end_intro: (a: number) => void;
    readonly viewer_field_debug: (a: number) => [number, number];
    readonly viewer_flying: (a: number) => number;
    readonly viewer_follow: (a: number) => void;
    readonly viewer_frame: (a: number, b: number) => [number, number];
    readonly viewer_issue_at: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly viewer_lab_feed: (a: number) => [number, number];
    readonly viewer_lab_marks: (a: number) => [number, number];
    readonly viewer_lab_set_speed: (a: number, b: number) => void;
    readonly viewer_lab_start: (a: number, b: number, c: number) => [number, number];
    readonly viewer_launch: (a: number, b: number) => void;
    readonly viewer_map_center_commander: (a: number) => void;
    readonly viewer_map_point: (a: number, b: number, c: number) => [number, number];
    readonly viewer_model_count: (a: number) => number;
    readonly viewer_move_commander: (a: number, b: number, c: number, d: number) => void;
    readonly viewer_object_at: (a: number, b: number, c: number) => [number, number];
    readonly viewer_pan: (a: number, b: number, c: number) => void;
    readonly viewer_perf: (a: number) => [number, number];
    readonly viewer_preview_pose: (a: number, b: number) => void;
    readonly viewer_radio_events: (a: number) => [number, number];
    readonly viewer_reset_model_view: (a: number) => void;
    readonly viewer_resize: (a: number, b: number, c: number) => void;
    readonly viewer_resize_map: (a: number, b: number, c: number) => void;
    readonly viewer_roster: (a: number) => [number, number];
    readonly viewer_rotate: (a: number, b: number) => void;
    readonly viewer_scene_name: (a: number) => [number, number];
    readonly viewer_select_box: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number];
    readonly viewer_select_only: (a: number, b: number) => void;
    readonly viewer_select_unit: (a: number, b: number) => void;
    readonly viewer_selection_ids: (a: number) => [number, number];
    readonly viewer_selection_name: (a: number) => [number, number];
    readonly viewer_set_full: (a: number, b: number) => void;
    readonly viewer_set_intro: (a: number, b: number, c: number, d: number) => void;
    readonly viewer_set_manner: (a: number, b: number) => void;
    readonly viewer_set_model_detail: (a: number, b: number) => void;
    readonly viewer_set_model_light: (a: number, b: number) => void;
    readonly viewer_set_model_variant: (a: number, b: number) => void;
    readonly viewer_set_order_mode: (a: number, b: number) => void;
    readonly viewer_set_outline: (a: number, b: number) => void;
    readonly viewer_set_pixel_scale: (a: number, b: number) => void;
    readonly viewer_set_radio: (a: number, b: number) => void;
    readonly viewer_set_scene: (a: number, b: number, c: number) => void;
    readonly viewer_set_selection: (a: number, b: number, c: number) => void;
    readonly viewer_set_ship: (a: number, b: number) => void;
    readonly viewer_set_tablet: (a: number, b: number) => void;
    readonly viewer_set_tablet_fit: (a: number, b: number) => void;
    readonly viewer_set_tablet_frame: (a: number, b: number, c: number) => void;
    readonly viewer_set_turntable: (a: number, b: number) => void;
    readonly viewer_start_assault: (a: number, b: number) => void;
    readonly viewer_stop_assault: (a: number) => void;
    readonly viewer_structure_overlay: (a: number, b: number) => [number, number];
    readonly viewer_tablet_view: (a: number) => number;
    readonly viewer_take_sounds: (a: number) => [number, number];
    readonly viewer_unit_at: (a: number, b: number, c: number) => number;
    readonly viewer_unit_marks: (a: number) => [number, number];
    readonly viewer_use_map: (a: number, b: number) => void;
    readonly viewer_variant_name: (a: number, b: number) => [number, number];
    readonly viewer_wheel: (a: number, b: number) => number;
    readonly wasm_bindgen_afd209a88ce49cc3___convert__closures_____invoke___js_sys_4c0a7da8c3256c44___Function_fn_wasm_bindgen_afd209a88ce49cc3___JsValue_____wasm_bindgen_afd209a88ce49cc3___sys__Undefined___js_sys_4c0a7da8c3256c44___Function_fn_wasm_bindgen_afd209a88ce49cc3___JsValue_____wasm_bindgen_afd209a88ce49cc3___sys__Undefined_______true_: (a: number, b: number, c: any, d: any) => void;
    readonly wasm_bindgen_afd209a88ce49cc3___convert__closures_____invoke___wasm_bindgen_afd209a88ce49cc3___sys__JsNullable_wgpu_c01756cc203236d7___backend__webgpu__webgpu_sys__gen_GpuAdapter__GpuAdapter___core_37ee0ce0c4cd57d5___result__Result_____wasm_bindgen_afd209a88ce49cc3___JsError___true_: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen_afd209a88ce49cc3___convert__closures_____invoke___wasm_bindgen_afd209a88ce49cc3___sys__JsNullable_wgpu_c01756cc203236d7___backend__webgpu__webgpu_sys__gen_GpuAdapter__GpuAdapter___core_37ee0ce0c4cd57d5___result__Result_____wasm_bindgen_afd209a88ce49cc3___JsError___true__119: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen_afd209a88ce49cc3___convert__closures_____invoke___wasm_bindgen_afd209a88ce49cc3___sys__JsNullable_wgpu_c01756cc203236d7___backend__webgpu__webgpu_sys__gen_GpuAdapter__GpuAdapter___core_37ee0ce0c4cd57d5___result__Result_____wasm_bindgen_afd209a88ce49cc3___JsError___true__120: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen_afd209a88ce49cc3___convert__closures_____invoke___wasm_bindgen_afd209a88ce49cc3___sys__JsNullable_wgpu_c01756cc203236d7___backend__webgpu__webgpu_sys__gen_GpuAdapter__GpuAdapter___core_37ee0ce0c4cd57d5___result__Result_____wasm_bindgen_afd209a88ce49cc3___JsError___true__121: (a: number, b: number, c: any) => [number, number];
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_destroy_closure: (a: number, b: number) => void;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
