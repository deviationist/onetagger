import { describe, it, expect } from 'vitest';
import { compose, narrowTags, has, parseCustom, SCALARS, ARRAYS, ROWS, Selection } from './manualtagcompose';

/// A match with only the fields a test cares about; everything else is the
/// shape `Track` needs to survive the round trip.
function match(platform: string, over: Record<string, any> = {}) {
    return {
        track: {
            platform, title: `${platform} title`, version: null,
            artists: [`${platform} artist`], album_artists: [], album: `${platform} album`,
            key: null, bpm: null, genres: [], styles: [], art: null, url: '',
            label: null, catalog_number: null, other: [], track_id: null,
            release_id: '', duration: { secs: 0, nanos: 0 }, remixers: [],
            track_number: null, track_total: null, disc_number: null, isrc: null,
            mood: null, explicit: null, lyrics: null, release_year: null,
            release_date: null, publish_year: null, publish_date: null,
            thumbnail: null, custom: {}, ...over,
        } as any,
    };
}

/// The default the UI starts from: every row on, each pointing at the first
/// match that can fill it.
function defaults(matches: { track: any }[]): Selection {
    const sel: Selection = { enabled: {}, scalar: {}, arr: {}, custom: {} };
    for (const r of ROWS) { sel.enabled[r.key] = true; sel.custom[r.key] = ''; }
    for (const f of SCALARS) {
        const i = matches.findIndex((m) => has(m.track, f.key));
        sel.scalar[f.key] = i >= 0 ? i : undefined;
    }
    for (const f of ARRAYS) {
        const i = matches.findIndex((m) => has(m.track, f.key));
        sel.arr[f.key] = i >= 0 ? [i] : [];
    }
    return sel;
}

describe('compose — which tags are written', () => {
    it('writes only the rows that are ticked', () => {
        // The case the whole apply-column exists for: artwork alone.
        const m = [match('beatport', { art: null }), match('discogs', { art: 'http://art/1.jpg' })];
        const sel = defaults(m);
        for (const r of ROWS) sel.enabled[r.key] = false;
        sel.enabled['art'] = true;
        sel.scalar['art'] = 1;

        const { tags } = compose(m, sel);
        expect(tags).toEqual(['albumArt']);
    });

    it('never lists a tag for an unticked row, even when a source is selected', () => {
        const m = [match('beatport')];
        const sel = defaults(m);
        sel.enabled['album'] = false;              // still has scalar['album'] === 0
        const { tags } = compose(m, sel);
        expect(tags).not.toContain('album');
    });

    it('omits a row whose selected source has nothing', () => {
        const m = [match('beatport', { label: null })];
        const sel = defaults(m);
        sel.scalar['label'] = 0;                   // pointed at an empty cell
        expect(compose(m, sel).tags).not.toContain('label');
    });

    it('omits a row with no selection at all', () => {
        const m = [match('beatport')];
        const sel = defaults(m);
        sel.scalar['album'] = undefined;
        expect(compose(m, sel).tags).not.toContain('album');
    });

    // The failure this guards against is not cosmetic. write_to_file writes
    // Title and Artist UNGUARDED, so if a single-field apply left them empty
    // rather than unlisted, it would erase them from the file.
    it('does not list title or artist when their rows are off', () => {
        const m = [match('beatport', { bpm: 128 })];
        const sel = defaults(m);
        for (const r of ROWS) sel.enabled[r.key] = false;
        sel.enabled['bpm'] = true;
        sel.scalar['bpm'] = 0;
        const { tags } = compose(m, sel);
        expect(tags).toEqual(['bpm']);
        expect(tags).not.toContain('title');
        expect(tags).not.toContain('artist');
    });
});

describe('compose — which values are taken', () => {
    it('takes each field from its own source', () => {
        const m = [
            match('beatport', { album: 'Right Album', art: null }),
            match('discogs',  { album: 'Wrong Album', art: 'http://art/1.jpg' }),
        ];
        const sel = defaults(m);
        sel.scalar['album'] = 0;
        sel.scalar['art'] = 1;

        const { track, tags } = compose(m, sel);
        expect((track as any).album).toBe('Right Album');
        expect((track as any).art).toBe('http://art/1.jpg');
        expect(tags).toEqual(expect.arrayContaining(['album', 'albumArt']));
    });

    it('moves siblings with their field', () => {
        // A source with a year but no full date must not contribute half a date.
        const m = [
            match('a', { release_date: '2001-01-01', release_year: 2001 }),
            match('b', { release_date: '1998-09-22', release_year: 1998 }),
        ];
        const sel = defaults(m);
        sel.scalar['release_date'] = 1;
        const { track } = compose(m, sel);
        expect((track as any).release_date).toBe('1998-09-22');
        expect((track as any).release_year).toBe(1998);
    });

    it('clears siblings when the value is typed', () => {
        // A typed artwork URL has no thumbnail; keeping the primary match's
        // would pair a new cover with an old thumbnail.
        const m = [match('a', { art: 'http://a/1.jpg', thumbnail: 'http://a/thumb.jpg' })];
        const sel = defaults(m);
        sel.scalar['art'] = 'custom';
        sel.custom['art'] = 'http://typed/cover.jpg';
        const { track, tags } = compose(m, sel);
        expect((track as any).art).toBe('http://typed/cover.jpg');
        expect((track as any).thumbnail).toBeUndefined();
        expect(tags).toContain('albumArt');
    });

    it('leaves a field unwritten when the typed value cannot be parsed', () => {
        const m = [match('a', { bpm: 128 })];
        const sel = defaults(m);
        sel.scalar['bpm'] = 'custom';
        sel.custom['bpm'] = 'not a number';
        expect(compose(m, sel).tags).not.toContain('bpm');
    });
});

describe('compose — lists', () => {
    it('unions the ticked sources and dedupes', () => {
        const m = [
            match('a', { genres: ['House', 'Tech House'] }),
            match('b', { genres: ['Tech House', 'Progressive'] }),
        ];
        const sel = defaults(m);
        sel.arr['genres'] = [0, 1];
        const { track } = compose(m, sel);
        expect((track as any).genres).toEqual(['House', 'Tech House', 'Progressive']);
    });

    it('takes only the ticked source, not every match', () => {
        // The silent accumulation this view exists to stop: borrowing artwork
        // from a rejected match used to import its genres too.
        const m = [
            match('a', { genres: ['House'] }),
            match('b', { genres: ['Trance'] }),
        ];
        const sel = defaults(m);
        sel.arr['genres'] = [0];
        expect((compose(m, sel).track as any).genres).toEqual(['House']);
    });

    it('splits a typed list on commas and trims', () => {
        const m = [match('a', { genres: [] })];
        const sel = defaults(m);
        sel.arr['genres'] = ['custom'];
        sel.custom['genres'] = ' House , Tech House ,, ';
        const { track, tags } = compose(m, sel);
        expect((track as any).genres).toEqual(['House', 'Tech House']);
        expect(tags).toContain('genre');
    });

    it('omits an empty list rather than writing one', () => {
        const m = [match('a', { genres: [] })];
        const sel = defaults(m);
        sel.arr['genres'] = [0];
        expect(compose(m, sel).tags).not.toContain('genre');
    });
});

describe('narrowTags', () => {
    it('keeps only the applied rows among tags the matrix owns', () => {
        expect(narrowTags(['title', 'album', 'genre'], ['album'])).toEqual(['album']);
    });

    it('passes through tags the matrix has no row for', () => {
        // metaTags, lyrics and ids are not matrix rows and must survive.
        expect(narrowTags(['metaTags', 'unsyncedLyrics', 'title'], [])).toEqual(['metaTags', 'unsyncedLyrics']);
    });

    it('never widens: a tag disabled in settings stays disabled', () => {
        expect(narrowTags(['album'], ['album', 'title'])).toEqual(['album']);
    });

    it('survives a config with no tag list', () => {
        expect(narrowTags(undefined, ['album'])).toEqual([]);
    });
});

describe('has', () => {
    it('treats empty string and empty array as absent', () => {
        expect(has({ album: '' }, 'album')).toBe(false);
        expect(has({ genres: [] }, 'genres')).toBe(false);
        expect(has({ album: null }, 'album')).toBe(false);
        expect(has({ album: 'x' }, 'album')).toBe(true);
        expect(has({ genres: ['x'] }, 'genres')).toBe(true);
    });

    it('treats a false explicit flag as present', () => {
        // `explicit: false` is an answer, not a gap.
        expect(has({ explicit: false }, 'explicit')).toBe(true);
    });
});

describe('parseCustom', () => {
    it('parses bpm as an integer and rejects nonsense', () => {
        expect(parseCustom('bpm', ' 128 ')).toBe(128);
        expect(parseCustom('bpm', 'abc')).toBeUndefined();
    });

    it('parses explicit from several spellings', () => {
        for (const y of ['y', 'yes', 'true', '1']) expect(parseCustom('explicit', y)).toBe(true);
        for (const n of ['n', 'no', 'false', '0']) expect(parseCustom('explicit', n)).toBe(false);
        expect(parseCustom('explicit', 'maybe')).toBeUndefined();
    });

    it('returns undefined for blank input', () => {
        expect(parseCustom('album', '   ')).toBeUndefined();
    });
});

describe('field tables', () => {
    it('gives every row a unique key and a tag', () => {
        const keys = ROWS.map((r) => r.key);
        expect(new Set(keys).size).toBe(keys.length);
        for (const r of ROWS) expect(r.tag).toBeTruthy();
    });
});
