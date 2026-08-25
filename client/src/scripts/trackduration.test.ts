import { describe, it, expect, vi, beforeEach } from 'vitest';
import { ref } from 'vue';

/// `useFileDuration` reads the global OneTagger instance, so the test controls
/// it rather than the app. Mocked at the module boundary so the function under
/// test keeps the signature the components use.
///
/// Real `ref`s, not plain objects: the thing worth asserting is that the value
/// updates when the file info lands, and a plain object cannot be tracked by
/// the computed, so it would sit stale and the test would be measuring the
/// harness instead of the code.
const state = await vi.hoisted(async () => {
    const { ref } = await import('vue');
    return {
        manualTag: ref({ fileInfo: undefined as any }),
        player: ref({ path: undefined as string | undefined, duration: 0 }),
    };
});
vi.mock('./onetagger', () => ({ get1t: () => state }));

import {
    useFileDuration, trackLength, lengthDelta, deltaLabel, deltaColor,
    CLOSE_ENOUGH_SECONDS, MAX_DURATION_DIFFERENCE,
} from './trackduration';

beforeEach(() => {
    state.manualTag.value = { fileInfo: undefined };
    state.player.value = { path: undefined, duration: 0 };
});

const track = (secs: number | null) => ({ duration: secs === null ? null : { secs, nanos: 0 } });

describe('useFileDuration', () => {
    it('prefers the length read from the file', () => {
        state.manualTag.value = { fileInfo: { duration: 513 } };
        state.player.value = { path: '/music/other.aiff', duration: 999 };
        expect(useFileDuration(ref('/music/a.aiff')).value).toBe(513);
    });

    it('falls back to the player while the file info is still loading', () => {
        state.player.value = { path: '/music/a.aiff', duration: 200.4 };
        expect(useFileDuration(ref('/music/a.aiff')).value).toBe(200);
    });

    // The guard that matters. The player holds whatever was last played, which
    // is usually not the file the manual tagger was opened for -- and a delta
    // measured against the wrong file is worse than no delta, because it looks
    // like an answer.
    it('ignores the player when it holds a different file', () => {
        state.player.value = { path: '/music/something-else.aiff', duration: 200 };
        expect(useFileDuration(ref('/music/a.aiff')).value).toBeUndefined();
    });

    it('is undefined when nothing knows the length', () => {
        expect(useFileDuration(ref('/music/a.aiff')).value).toBeUndefined();
    });

    it('is undefined when no path is being tagged', () => {
        state.player.value = { path: '/music/a.aiff', duration: 200 };
        expect(useFileDuration(ref(undefined)).value).toBeUndefined();
    });

    // A zero or one-second length is a not-yet-loaded player, not a real track.
    it('rejects a degenerate length from either source', () => {
        state.manualTag.value = { fileInfo: { duration: 0 } };
        state.player.value = { path: '/music/a.aiff', duration: 1 };
        expect(useFileDuration(ref('/music/a.aiff')).value).toBeUndefined();
    });

    it('recomputes when the file info arrives', () => {
        const p = ref('/music/a.aiff');
        const d = useFileDuration(p);
        expect(d.value).toBeUndefined();
        state.manualTag.value = { fileInfo: { duration: 513 } };
        expect(d.value).toBe(513);
    });
});

describe('trackLength', () => {
    it('formats as m:ss with a padded seconds field', () => {
        expect(trackLength(track(513))).toBe('8:33');
        expect(trackLength(track(65))).toBe('1:05');
        expect(trackLength(track(600))).toBe('10:00');
    });

    // A length of 0:00 reads as a fact rather than a gap, which is worse than
    // showing nothing -- so both absent and zero are treated as unknown.
    it('treats absent and zero as unknown', () => {
        expect(trackLength(track(0))).toBeUndefined();
        expect(trackLength(track(null))).toBeUndefined();
        expect(trackLength({})).toBeUndefined();
        expect(trackLength(undefined)).toBeUndefined();
    });
});

describe('lengthDelta', () => {
    it('is signed: positive when the source is longer than the file', () => {
        expect(lengthDelta(track(520), 513)).toBe(7);
        expect(lengthDelta(track(500), 513)).toBe(-13);
        expect(lengthDelta(track(513), 513)).toBe(0);
    });

    it('is undefined when either side is unknown', () => {
        expect(lengthDelta(track(520), undefined)).toBeUndefined();
        expect(lengthDelta(track(null), 513)).toBeUndefined();
        expect(lengthDelta(track(0), 513)).toBeUndefined();
    });
});

describe('deltaLabel', () => {
    // Zero has no sign, and printing "-0:00" makes a perfect match look like a
    // near miss.
    it('calls an exact match exact, not -0:00', () => {
        expect(deltaLabel(0)).toBe('exact');
    });

    it('signs and formats in m:ss', () => {
        expect(deltaLabel(7)).toBe('+0:07');
        expect(deltaLabel(-13)).toBe('-0:13');
        expect(deltaLabel(125)).toBe('+2:05');
        expect(deltaLabel(-600)).toBe('-10:00');
    });
});

describe('deltaColor', () => {
    it('is green inside the close-enough band, in both directions', () => {
        expect(deltaColor(0)).toBe('text-green-5');
        expect(deltaColor(CLOSE_ENOUGH_SECONDS)).toBe('text-green-5');
        expect(deltaColor(-CLOSE_ENOUGH_SECONDS)).toBe('text-green-5');
    });

    it('is orange between the two bands', () => {
        expect(deltaColor(CLOSE_ENOUGH_SECONDS + 1)).toBe('text-orange-5');
        expect(deltaColor(MAX_DURATION_DIFFERENCE)).toBe('text-orange-5');
        expect(deltaColor(-MAX_DURATION_DIFFERENCE)).toBe('text-orange-5');
    });

    // Red is not a taste judgement: beyond max_duration_difference the matcher
    // would have rejected the candidate outright.
    it('is red beyond the matchercut-off', () => {
        expect(deltaColor(MAX_DURATION_DIFFERENCE + 1)).toBe('text-red-5');
        expect(deltaColor(-9999)).toBe('text-red-5');
    });

    it('keeps the bands ordered', () => {
        expect(CLOSE_ENOUGH_SECONDS).toBeLessThan(MAX_DURATION_DIFFERENCE);
    });
});
