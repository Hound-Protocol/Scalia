#!/usr/bin/env python3
"""Splits the project lead's weapon recordings (one file per gun: handling, single shot, burst, auto) into one file per
piece by silence: writes sheet.png (waveforms with numbered cuts) and segs.json, then cuts the pieces named in
`plan` (read off the sheet) into web/sfx/weapons/. Run from a scratch directory."""
import os
import subprocess, sys, numpy as np, json
import matplotlib; matplotlib.use('Agg'); import matplotlib.pyplot as plt
files = {'m1911': 'c487f653-jigokukarano_sisya-m1911a1-168854.mp3', 'deagle': '15960e0f-jigokukarano_sisya-desert-eagle-168857.mp3',
         'ar15': '430bf042-jigokukarano_sisya-ar15-168855.mp3', 'mp5': '268753ab-jigokukarano_sisya-mp5-168858.mp3', 'ak47': '04af07ef-jigokukarano_sisya-ak47-168856.mp3'}
SR = 44100
out = {}
fig, axs = plt.subplots(len(files), 1, figsize=(16, 2.4 * len(files)))
for ax, (name, f) in zip(axs, files.items()):
    raw = subprocess.run(['ffmpeg', '-v', 'error', '-i', '/mnt/user-data/uploads/' + f, '-ac', '1', '-ar', str(SR), '-f', 'f32le', '-'], capture_output=True, check=True).stdout
    x = np.frombuffer(raw, dtype='<f4')
    win = int(0.005 * SR)
    env = np.sqrt(np.convolve(x * x, np.ones(win) / win, mode='same'))
    db = 20 * np.log10(env + 1e-9)
    on = db > db.max() - 42
    segs, i = [], 0
    while i < len(on):
        if on[i]:
            j = i
            gap = 0
            while j < len(on) and gap < int(0.07 * SR):
                gap = 0 if on[j] else gap + 1
                j += 1
            segs.append((i, j - gap))
            i = j
        else:
            i += 1
    info = []
    for k, (a, b) in enumerate(segs):
        s = x[a:b]
        peak = 20 * np.log10(np.abs(s).max() + 1e-9)
        sp = np.abs(np.fft.rfft(s * np.hanning(len(s)))) if len(s) > 32 else np.zeros(2)
        fr = np.fft.rfftfreq(len(s), 1 / SR) if len(s) > 32 else np.zeros(2)
        cen = float((sp * fr).sum() / (sp.sum() + 1e-9))
        # attack: time from start to peak
        att = (np.argmax(np.abs(s))) / SR * 1000
        info.append(dict(k=k, start=round(a / SR, 3), dur=round((b - a) / SR, 3), peak=round(float(peak), 1), centroid=int(cen), attack_ms=round(att, 1)))
    out[name] = info
    t = np.arange(len(x)) / SR
    ax.plot(t, x, lw=0.3, color='k')
    for d in info:
        ax.axvspan(d['start'], d['start'] + d['dur'], color='orange', alpha=0.25)
        ax.text(d['start'], 0.9 * np.abs(x).max(), str(d['k']), fontsize=8)
    ax.set_title(name); ax.set_xlim(0, t[-1])
plt.tight_layout(); plt.savefig('sheet.png', dpi=60)
json.dump(out, open('segs.json', 'w'), indent=1)
for n, i in out.items():
    print(n)
    for d in i: print('  ', d)
# What each cut is (read off the waveforms: mechanical handling first, then a single shot, a burst, full auto).
plan = {
 'ar15':  {'mag-out': [0], 'mag-in': [1], 'bolt': [2], 'shot': [3]},
 'ak47':  {'mag-out': [0], 'mag-in': [1], 'bolt': [2, 3], 'shot': [4]},
 'mp5':   {'mag-out': [0], 'mag-in': [1], 'bolt': [2, 3], 'shot': [5]},
 'm1911': {'dry': [0], 'mag': [1], 'slide': [2, 3], 'shot': [4]},
 'deagle': {'mag-out': [0], 'mag-in': [2, 3], 'slide': [4, 5, 6], 'shot': [7]},
}
out = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'web', 'sfx', 'weapons')
os.makedirs(out, exist_ok=True)
for gun, parts in plan.items():
    s = {d['k']: d for d in segs[gun]}
    for name, ks in parts.items():
        a = s[ks[0]]['start'] - 0.008
        b = s[ks[-1]]['start'] + s[ks[-1]]['dur'] + 0.04
        dur = b - a
        loud = '-1' if name in ('shot', 'burst', 'auto') else '-4'
        f = f'{out}/{gun}-{name}.mp3'
        af = f"atrim=start={a:.3f}:end={b:.3f},asetpts=PTS-STARTPTS,afade=t=in:d=0.004,afade=t=out:st={max(0, dur - 0.03):.3f}:d=0.03,loudnorm=I=-16:TP={loud}:LRA=11,aresample=44100"
        subprocess.run(['ffmpeg', '-v', 'error', '-y', '-i', '/mnt/user-data/uploads/' + src[gun], '-af', af, '-ac', '1', '-c:a', 'libmp3lame', '-q:a', '4', f], check=True)
        print(f, f'{dur:.2f}s', os.path.getsize(f))
