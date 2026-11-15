// The barracks jukebox (E at it, on foot). Its music plays from where the jukebox stands, through the same sound
// engine as everything else (distance, walls, the room's ring, around the head): a test bench for how sounds
// that are not gunfire travel, and for tuning the engine. Starts switched off at 50%; volume up to 200%; bass,
// treble and highs 0 to 10.

const TRACKS = [
  ['Heavy Music', 'heavy-music'],
  ['Hard Rock Metal', 'hard-rock-metal'],
  ['Hard Rock Sport', 'hard-rock-sport'],
  ['Virus', 'virus'],
  ['Polet', 'polet'],
  ['Ransom', 'ransom'],
];

export function createJukebox({ audio, viewer }) {
  const box = document.getElementById('jukebox');
  const el = new Audio();
  el.preload = 'none';
  el.crossOrigin = 'anonymous';
  let voice = null, on = false, track = 0, vol = 0.5, tone = [5, 5, 5], timer = 0;
  const $ = (s) => box.querySelector(s);
  const list = $('.jb-list');
  TRACKS.forEach(([name], i) => {
    const b = document.createElement('button');
    b.textContent = `${i + 1}. ${name}`;
    b.addEventListener('click', () => { track = i; if (on) load(true); draw(); });
    list.appendChild(b);
  });
  const ensure = () => {
    if (voice) return voice;
    voice = audio.stream(el);
    if (voice) { voice.volume(vol); voice.tone(...tone); }
    return voice;
  };
  const load = (play) => {
    el.src = `./sfx/jukebox/${TRACKS[track][1]}.mp3`;
    if (play) el.play().catch(() => {});
  };
  // Where the music comes from, as heard now (re-asked a few times a second while it plays).
  const aim = () => {
    const v = viewer(), at = v && v.jukebox_at();
    if (!voice || !at || at.length < 2) return;
    try { voice.place(v.hear_at(at[0], at[1])); } catch (_) { /* engine busy */ }
  };
  const draw = () => {
    $('.jb-power').textContent = on ? 'On' : 'Off';
    $('.jb-power').classList.toggle('lit', on);
    $('.jb-play').textContent = on && !el.paused ? 'Pause' : 'Play';
    $('.jb-play').disabled = !on;
    $('.jb-now').textContent = on ? TRACKS[track][0] : 'Switched off';
    list.querySelectorAll('button').forEach((b, i) => b.classList.toggle('lit', i === track));
    $('.jb-vol').value = Math.round(vol * 100);
    $('.jb-vol-v').textContent = `${Math.round(vol * 100)}%`;
    ['bass', 'treble', 'highs'].forEach((k, i) => { $(`.jb-${k}`).value = tone[i]; $(`.jb-${k}-v`).textContent = tone[i]; });
  };
  el.addEventListener('ended', () => { track = (track + 1) % TRACKS.length; load(true); draw(); });
  $('.jb-power').addEventListener('click', () => {
    on = !on;
    if (on) { ensure(); aim(); load(true); timer = setInterval(aim, 120); }
    else { el.pause(); clearInterval(timer); }
    draw();
  });
  $('.jb-play').addEventListener('click', () => { if (!on) return; if (el.paused) el.play().catch(() => {}); else el.pause(); draw(); });
  $('.jb-prev').addEventListener('click', () => { track = (track + TRACKS.length - 1) % TRACKS.length; if (on) load(true); draw(); });
  $('.jb-next').addEventListener('click', () => { track = (track + 1) % TRACKS.length; if (on) load(true); draw(); });
  $('.jb-vol').addEventListener('input', (e) => { vol = e.target.value / 100; voice?.volume(vol); draw(); });
  ['bass', 'treble', 'highs'].forEach((k, i) => $(`.jb-${k}`).addEventListener('input', (e) => { tone[i] = +e.target.value; voice?.tone(...tone); draw(); }));
  $('.jb-close').addEventListener('click', () => close());
  const close = () => { box.hidden = true; };
  draw();
  return {
    get open() { return !box.hidden; },
    toggle() { box.hidden = !box.hidden; draw(); },
    close,
  };
}
