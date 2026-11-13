#!/usr/bin/env python3
"""The radio's voices: every line the net can carry, spoken in advance by Chatterbox (Resemble AI, MIT licence),
one voice per speaker, and shipped as small clips (web/voice/). The game plays a clip through its radio chain
when a line's exact text is in web/voice/manifest.json, and falls back to the browser's speech voice otherwise.

Each speaker's voice is cloned from a short reference read by a Kokoro voice (Apache 2.0). Intensity per line:
calm (routine), tense (contact, urgent calls), scream (casualties, assaults).

Keep the texts here in step with web/game.js (radio procedure) and viewer.rs (unit reports).

Usage:  python3 -I scripts/radio_voices.py plan            # write the manifest, list what is missing
        python3 -I scripts/radio_voices.py refs  KOKORO_DIR # make the reference voices (kokoro-v1.0 onnx + voices)
        python3 -I scripts/radio_voices.py gen [N]          # generate up to N missing clips (resumable)
"""
import hashlib, json, os, re, subprocess, sys

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..')
OUT = os.path.join(ROOT, 'web', 'voice')
REFS = os.path.join(ROOT, 'assets', 'source', 'voice_refs')

SQUADS = ['Alpha', 'Bravo', 'Charlie', 'Delta', 'Echo', 'Foxtrot', 'Golf', 'Hotel']
CAPTAINS = ['Reyes', 'Okafor']
# Speaker voices: the commander, four squad voices (two squads each) and one per captain.
VOICE_OF = {'Alpha': 's1', 'Echo': 's1', 'Bravo': 's2', 'Foxtrot': 's2', 'Charlie': 's3', 'Golf': 's3',
            'Delta': 's4', 'Hotel': 's4', 'Reyes': 'c1', 'Okafor': 'c2'}
KOKORO = {'cmd': 'am_fenrir', 's1': 'am_michael', 's2': 'am_puck', 's3': 'bm_george', 's4': 'am_liam',
          'c1': 'am_onyx', 'c2': 'bm_lewis'}
REF_TEXT = ("Copy that, we are moving up to the tree line now. Two minutes out. "
            "Keep your heads down and watch the left side.")
TIER = {'calm': (0.5, 0.5), 'tense': (1.3, 0.35), 'scream': (2.0, 0.2)}  # exaggeration, cfg weight
VERBS = {'move to': 'moving to the coordinates', 'defend': 'defending the coordinates', 'assault': 'assaulting the coordinates',
         'support from': 'supporting from the coordinates', 'take': 'taking the coordinates', 'hold': 'holding the coordinates'}
SQUAD_REPORTS = ['Dug in at the coordinates, holding', 'Coordinates secured, we hold here', 'In cover at the coordinates, watching our front',
                 'Assault element at the coordinates. Awaiting orders', 'Reached the coordinates, area clear', 'At the objective, ready',
                 'Support set at the coordinates', 'In position at the coordinates, eyes on', 'Overwatch established at the coordinates',
                 'Arrived at the coordinates', 'We are at the coordinates, awaiting orders', 'At position, at the coordinates']
CAPTAIN_REPORTS = [('At the line of departure, no contact. Advancing on the coordinates', 'calm'),
                   ('On the objective. Digging in at the coordinates', 'calm'),
                   ('Objective taken, no casualties. Squads holding at the coordinates', 'calm'),
                   ('In position, holding the coordinates', 'calm')]
CAPTAIN_URGENT = [('Contact at the coordinates! Support element laying down fire, assault going in', 'tense'),
                  ('Assaulting now', 'scream')]


def catalogue():
    """[(voice, text, tier)]: the exact texts the game shows (and looks clips up by)."""
    lines = []
    add = lines.append
    units = [(f'{cs} {n:02d}', cs) for n, cs in enumerate(SQUADS, 1)] + [(f'{cs} {n:02d}', cs) for n, cs in enumerate(CAPTAINS, 9)]
    for name, cs in units:
        v = VOICE_OF[cs]
        add(('cmd', f'{name}, this is Commander. Over.', 'calm'))
        add(('cmd', f'{name}, {name}, this is Commander. Over.', 'tense'))
        add(('cmd', f'{name}, Commander. Send it. Over.', 'calm'))
        add(('cmd', f'{name}, Commander. Send it! Over!', 'tense'))
        add((v, f'Commander, {name}. Send it. Over.', 'calm'))
        add((v, f'Commander, this is {name}. Over.', 'calm'))
        add((v, f'Commander, this is {name}! Over!', 'tense'))
    for verb in VERBS:
        add(('cmd', f'{verb[0].upper() + verb[1:]} the coordinates transmitted. Over.', 'tense' if verb == 'assault' else 'calm'))
    for t in ['Send sitrep. Over.', 'Commander copies. Out.', 'Commander, out.']:
        add(('cmd', t, 'calm'))
    for v in ['s1', 's2', 's3', 's4']:
        for ing in VERBS.values():
            add((v, f'Roger, coordinates received, {ing}. Over.', 'tense' if ing.startswith('assault') else 'calm'))
        for doing in ['holding, no contact', 'on the move']:
            add((v, f'At position, 12 effective, {doing}. Over.', 'calm'))
        for body in SQUAD_REPORTS:
            add((v, f'{body}. Over.', 'calm'))
    for v in ['c1', 'c2']:
        for ing in [VERBS['take'], VERBS['hold']]:
            add((v, f'Roger, coordinates received, {ing}. Over.', 'calm'))
        for doing in ['holding, no contact', 'on the move']:
            add((v, f'At position, 1 effective, {doing}. Over.', 'calm'))
        for body, tier in CAPTAIN_REPORTS:
            add((v, f'{body}. Over.', tier))
        for body, tier in CAPTAIN_URGENT:
            add((v, f'{body}! Over!', tier))
    return lines


DIGITS = 'zero one two three four five six seven eight nine'.split()
CARDINAL = {'1': 'one', '12': 'twelve'}


def spoken(text):
    """How the line is read: call-sign numbers digit by digit (radio style), head counts as numbers."""
    text = re.sub(r'\b(\d+) effective', lambda m: f'{CARDINAL.get(m.group(1), m.group(1))} effective', text)
    return re.sub(r'\d+', lambda m: ' '.join(DIGITS[int(c)] for c in m.group(0)), text)


def clip_name(voice, text):
    return f'{voice}/{hashlib.sha1(text.encode()).hexdigest()[:12]}.mp3'


def plan():
    lines = catalogue()
    manifest = {}
    for v, t, _ in lines:
        manifest.setdefault(v, {})[t] = clip_name(v, t)
    os.makedirs(OUT, exist_ok=True)
    json.dump(manifest, open(os.path.join(OUT, 'manifest.json'), 'w'), indent=0, sort_keys=True)
    missing = [l for l in lines if not os.path.exists(os.path.join(OUT, clip_name(l[0], l[1])))]
    print(f'{len(lines)} lines, {len(missing)} still to generate')
    return missing


def refs(kokoro_dir):
    import soundfile as sf
    from kokoro_onnx import Kokoro
    k = Kokoro(os.path.join(kokoro_dir, 'kokoro-v1.0.int8.onnx'), os.path.join(kokoro_dir, 'voices-v1.0.bin'))
    os.makedirs(REFS, exist_ok=True)
    for v, kv in KOKORO.items():
        s, sr = k.create(REF_TEXT, voice=kv, speed=1.0, lang='en-gb' if kv.startswith('b') else 'en-us')
        sf.write(os.path.join(REFS, f'{v}.wav'), s, sr)
        print('ref', v, kv)


def gen(limit):
    import torchaudio as ta
    from chatterbox.tts import ChatterboxTTS
    todo = plan()[:limit]
    if not todo:
        return
    m = ChatterboxTTS.from_pretrained(device='cpu')
    for v, text, tier in todo:
        ex, cfg = TIER[tier]
        w = m.generate(spoken(text), audio_prompt_path=os.path.join(REFS, f'{v}.wav'), exaggeration=ex, cfg_weight=cfg)
        path = os.path.join(OUT, clip_name(v, text))
        os.makedirs(os.path.dirname(path), exist_ok=True)
        wav = path[:-4] + '.wav'
        ta.save(wav, w, m.sr)
        # Mono, trimmed of leading and trailing silence, small (the game band-limits it anyway).
        subprocess.run(['ffmpeg', '-y', '-loglevel', 'error', '-i', wav, '-af',
                        'silenceremove=start_periods=1:start_threshold=-45dB,areverse,silenceremove=start_periods=1:start_threshold=-45dB,areverse',
                        '-ac', '1', '-ar', '24000', '-b:a', '48k', path], check=True)
        os.remove(wav)
        print('made', v, tier, text, flush=True)


if __name__ == '__main__':
    cmd = sys.argv[1] if len(sys.argv) > 1 else 'plan'
    if cmd == 'plan':
        plan()
    elif cmd == 'refs':
        refs(sys.argv[2])
    elif cmd == 'gen':
        gen(int(sys.argv[2]) if len(sys.argv) > 2 else 10_000)
