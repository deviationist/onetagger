<template>
<div class='mtm'>

    <!-- What this view is for, and the one thing that is not obvious about it:
         a field nothing is selected for is not written at all. -->
    <div class='q-px-sm q-pb-sm text-caption text-grey-6' style='line-height: 1.35;'>
        Pick a source per field. A row with nothing selected is
        <span class='text-grey-4'>not written</span>; your enabled-tags and
        overwrite settings still apply as usual.
        <span v-if='!extended && !extending'>
            &nbsp;<a class='mtm-link' @click='extend'>Load full values</a> —
            MusicBrainz and Traxsource only fetch artwork when a match is opened,
            so their artwork cells may read empty until then.
        </span>
        <span v-if='extending' class='text-primary'>&nbsp;Loading full values…</span>
        <span v-if='extended' class='text-green-5'>&nbsp;Full values loaded.</span>
    </div>

    <!-- Without this, "take all" quietly leaves behind every field the new
         source cannot fill, so you end up with a hybrid of the source you chose
         and the one you were trying to replace -- artwork being the usual one.
         Turning it on makes "take all" mean *only* this source. -->
    <div class='q-px-sm q-pb-sm'>
        <q-toggle dense size='xs' v-model='allowEmpty' color='primary'>
            <span class='text-caption text-grey-6'>
                “Take all” includes empty values — a field this source lacks is
                cleared rather than kept from the previous one
            </span>
        </q-toggle>
    </div>

    <div class='mtm-scroll'>
        <table class='mtm-table'>
            <thead>
                <tr>
                    <th class='mtm-field'>Field</th>
                    <th class='mtm-apply'>
                        <q-checkbox dense size='xs' v-model='allEnabled' toggle-indeterminate />
                        <div class='text-caption text-grey-6'>apply</div>
                    </th>
                    <th v-for='(m, i) in matches' :key='i' class='mtm-col'>
                        <div class='monospace text-caption'>{{ m.track.platform.toUpperCase() }}</div>
                        <div class='text-caption' :class='accuracyColor(m.accuracy)'>
                            {{ Math.round(m.accuracy * 100) }}%
                        </div>
                        <!-- Length is not a field you select -- it is how you
                             judge a column. Two remixes of one track share
                             title, artist and often album, and differ by
                             minutes, so the delta against the file is usually
                             what settles which source is even talking about
                             this recording. -->
                        <div class='text-caption' v-if='trackLength(m.track)'>
                            <span class='monospace text-grey-4'>{{ trackLength(m.track) }}</span>
                            <span v-if='lengthDelta(m.track, fileDuration) !== undefined'
                                  class='monospace q-ml-xs'
                                  :class='deltaColor(lengthDelta(m.track, fileDuration)!)'>
                                {{ deltaLabel(lengthDelta(m.track, fileDuration)!) }}
                            </span>
                        </div>
                        <div class='text-caption text-grey-7' v-else>no length</div>
                        <!-- Take this source for every row it can fill. The common
                             case is "this one, except the artwork", which is then
                             one click here plus one in the artwork row. -->
                        <q-btn dense flat size='sm' class='mtm-all' @click='takeAll(i)'>
                            take all
                        </q-btn>
                    </th>
                    <th class='mtm-col mtm-custom-col'>
                        <div class='monospace text-caption'>CUSTOM</div>
                        <div class='text-caption text-grey-6'>typed</div>
                    </th>
                </tr>
            </thead>

            <tbody>
                <!-- Scalars: exactly one source wins, so radios. -->
                <tr v-for='f in SCALARS' :key='f.key' :class="{ 'mtm-off': !enabled[f.key] }">
                    <td class='mtm-field'>{{ f.label }}</td>
                    <td class='mtm-apply'>
                        <q-checkbox dense size='xs' v-model='enabled[f.key]' />
                    </td>
                    <td v-for='(m, i) in matches' :key='i' class='mtm-cell'
                        :class="{ 'mtm-empty': !has(m.track, f.key) }">
                        <q-radio
                            v-if='has(m.track, f.key) || allowEmpty'
                            dense
                            size='xs'
                            :model-value='scalar[f.key]'
                            :val='i'
                            @update:model-value='scalar[f.key] = i'
                        >
                            <span class='mtm-val'>
                                <img v-if="f.key === 'art' && has(m.track, f.key)" :src='m.track.art' class='mtm-art'>
                                <span v-else-if='has(m.track, f.key)'>{{ display(m.track, f.key) }}</span>
                                <span v-else class='mtm-dash'>— none</span>
                            </span>
                        </q-radio>
                        <!-- Absent, not blank: an empty artwork cell is the thing
                             that sends you to another column, so it has to read
                             as a fact rather than as whitespace. -->
                        <span v-else class='mtm-dash'>—</span>
                    </td>
                    <td class='mtm-cell mtm-custom-col'>
                        <q-radio dense size='xs' :model-value='scalar[f.key]' val='custom'
                                 @update:model-value="scalar[f.key] = 'custom'" />
                        <q-input dense borderless class='mtm-input'
                                 v-model='custom[f.key]'
                                 :placeholder='f.hint || "type a value"'
                                 @update:model-value="scalar[f.key] = 'custom'" />
                    </td>
                </tr>

                <tr class='mtm-divider'>
                    <td :colspan='matches.length + 3'>
                        Lists — tick every source you want included. Today these are
                        unioned silently whenever you select a second match; here it
                        is a choice.
                    </td>
                </tr>

                <!-- Arrays: "pick one source" is the wrong verb, so checkboxes. -->
                <tr v-for='f in ARRAYS' :key='f.key' :class="{ 'mtm-off': !enabled[f.key] }">
                    <td class='mtm-field'>{{ f.label }}</td>
                    <td class='mtm-apply'>
                        <q-checkbox dense size='xs' v-model='enabled[f.key]' />
                    </td>
                    <td v-for='(m, i) in matches' :key='i' class='mtm-cell'
                        :class="{ 'mtm-empty': !has(m.track, f.key) }">
                        <q-checkbox
                            v-if='has(m.track, f.key)'
                            dense
                            size='xs'
                            :model-value='arr[f.key].includes(i)'
                            @update:model-value='toggleArray(f.key, i)'
                        >
                            <span class='mtm-val'>{{ display(m.track, f.key) }}</span>
                        </q-checkbox>
                        <span v-else class='mtm-dash'>—</span>
                    </td>
                    <td class='mtm-cell mtm-custom-col'>
                        <q-checkbox dense size='xs' :model-value="arr[f.key].includes('custom')"
                                    @update:model-value="toggleArray(f.key, 'custom')" />
                        <q-input dense borderless class='mtm-input'
                                 v-model='custom[f.key]'
                                 placeholder='comma separated'
                                 @update:model-value='ensureArrayCustom(f.key)' />
                    </td>
                </tr>
            </tbody>
        </table>
    </div>

    <div class='row items-center q-px-sm q-pt-sm'>
        <q-btn flat color='primary' @click='apply' :disable='saving' :loading='saving'>
            Apply composite
        </q-btn>
        <q-btn flat color='grey-6' @click='resetToDefaults'>Reset</q-btn>
        <q-space />
        <span class='text-caption text-grey-6'>{{ chosenCount }} of {{ ROWS.length }} fields will be written</span>
    </div>
</div>
</template>

<script lang='ts' setup>
import { ref, reactive, computed, watch, onMounted, toRefs, PropType } from 'vue';
import { useQuasar } from 'quasar';
import { get1t } from '../scripts/onetagger';
import type { TrackMatch } from '../scripts/manualtag';
import type { AutotaggerConfig, Track } from '../scripts/autotagger';
import { useFileDuration, trackLength, lengthDelta, deltaLabel, deltaColor } from '../scripts/trackduration';

const $1t = get1t();
const $q = useQuasar();

const props = defineProps({
    matches: { type: Array as PropType<TrackMatch[]>, required: true },
    path: { type: String, required: true },
    config: { type: Object, required: true },
});
const { matches, path } = toRefs(props);
const fileDuration = useFileDuration(path);
const emit = defineEmits(['applied']);

/// Fields the matrix can arbitrate with a single winner.
///
/// `siblings` exists because a few of these are stored as more than one field
/// and only make sense together: a source offering a release *year* but no full
/// date would otherwise contribute half of a date, and artwork carries its own
/// thumbnail URL. Choosing a source for the row takes all of its siblings.
/// `tag` is the SupportedTag this row writes, and it is what actually decides
/// whether the row is applied: an unticked row is dropped from the config sent
/// with the composite, so `write_to_file` skips it entirely.
///
/// That mechanism, rather than simply leaving the value empty, because Title and
/// Artist are the two fields OneTagger writes *unguarded* -- every other field
/// is behind an `is_some()` / `!is_empty()` check and an empty one is left
/// alone, but an empty title or artist is written, and writing an empty one
/// erases what the file had.
const SCALARS = [
    { key: 'title',          label: 'Title',         tag: 'title' },
    { key: 'version',        label: 'Version',       tag: 'version' },
    { key: 'album',          label: 'Album',         tag: 'album' },
    { key: 'label',          label: 'Label',         tag: 'label' },
    { key: 'catalog_number', label: 'Catalogue №',   tag: 'catalogNumber' },
    { key: 'key',            label: 'Key',           tag: 'key' },
    { key: 'bpm',            label: 'BPM',           tag: 'bpm',         hint: 'e.g. 128' },
    { key: 'release_date',   label: 'Release date',  tag: 'releaseDate', siblings: ['release_year'], hint: 'YYYY-MM-DD' },
    { key: 'publish_date',   label: 'Publish date',  tag: 'publishDate', siblings: ['publish_year'], hint: 'YYYY-MM-DD' },
    { key: 'isrc',           label: 'ISRC',          tag: 'isrc' },
    { key: 'mood',           label: 'Mood',          tag: 'mood' },
    { key: 'explicit',       label: 'Explicit',      tag: 'explicit',    hint: 'yes / no' },
    { key: 'art',            label: 'Artwork',       tag: 'albumArt',    siblings: ['thumbnail'], hint: 'image URL' },
    { key: 'url',            label: 'URL',           tag: 'url' },
];

const ARRAYS = [
    { key: 'artists',       label: 'Artists',       tag: 'artist' },
    { key: 'album_artists', label: 'Album artists', tag: 'albumArtist' },
    { key: 'genres',        label: 'Genres',        tag: 'genre' },
    { key: 'styles',        label: 'Styles',        tag: 'style' },
    { key: 'remixers',      label: 'Remixers',      tag: 'remixer' },
];

const ROWS = [...SCALARS, ...ARRAYS];

// field -> match index, or 'custom', or undefined (meaning: do not write it)
const scalar = reactive<Record<string, number | 'custom' | undefined>>({});
// field -> list of match indices (and possibly 'custom') to union
const arr = reactive<Record<string, (number | 'custom')[]>>({});
const custom = reactive<Record<string, string>>({});

// Which rows are applied at all. All on by default: the common case is still
// "take a match", and having to tick fourteen boxes to do it would be worse
// than the problem this solves.
const enabled = reactive<Record<string, boolean>>({});
const allowEmpty = ref(false);
const saving = ref(false);
const extending = ref(false);
const extended = computed(() => $1t.manualTag.value.extended);

function has(track: any, key: string): boolean {
    const v = track[key];
    if (v === undefined || v === null || v === '') return false;
    if (Array.isArray(v)) return v.length > 0;
    return true;
}

function display(track: any, key: string): string {
    const v = track[key];
    if (Array.isArray(v)) return v.join(', ');
    if (typeof v === 'boolean') return v ? 'Yes' : 'No';
    return String(v);
}

function accuracyColor(acc: number) {
    if (acc == 1.0) return 'text-green';
    if (acc > 0.85) return 'text-yellow';
    return 'text-red';
}

/// Default every row to the first match that can fill it.
///
/// That is exactly what happens today when several matches are ticked -- the
/// highest-accuracy source wins each field and the rest fill only the gaps. The
/// difference is that here it is visible and every cell of it can be overridden.
function resetToDefaults() {
    for (const r of ROWS) enabled[r.key] = true;
    for (const f of SCALARS) {
        const i = matches.value.findIndex((m) => has(m.track, f.key));
        scalar[f.key] = i >= 0 ? i : undefined;
        if (custom[f.key] === undefined) custom[f.key] = '';
    }
    for (const f of ARRAYS) {
        const i = matches.value.findIndex((m) => has(m.track, f.key));
        // Only the primary source, not the union. Inheriting every source's
        // genres is the thing this view exists to stop doing by accident.
        arr[f.key] = i >= 0 ? [i] : [];
        if (custom[f.key] === undefined) custom[f.key] = '';
    }
}

/// Take one source for every row it can fill -- or, with `allowEmpty`, for every
/// row full stop.
///
/// The default is conservative because it is usually what you want: most of the
/// time a second source is filling gaps. But it means the result is a hybrid,
/// and the fields it keeps are exactly the ones the chosen source is silent
/// about -- which is how you end up publishing artwork from a release you
/// rejected. Opting in makes the button mean "only this source", and a field it
/// lacks is then simply not written.
/// Header checkbox: all on, all off, and indeterminate while it is a mix.
const allEnabled = computed<boolean | null>({
    get() {
        const on = ROWS.filter((r) => enabled[r.key]).length;
        if (on === 0) return false;
        if (on === ROWS.length) return true;
        return null;
    },
    set(v) {
        for (const r of ROWS) enabled[r.key] = v === true;
    },
});

function takeAll(i: number) {
    for (const f of SCALARS) {
        if (allowEmpty.value || has(matches.value[i].track, f.key)) scalar[f.key] = i;
    }
    for (const f of ARRAYS) {
        // A ticked source that offers nothing contributes nothing, so there is
        // no empty checkbox to select here -- clearing the row is the same
        // statement, and it is one the UI can actually show.
        if (has(matches.value[i].track, f.key)) arr[f.key] = [i];
        else if (allowEmpty.value) arr[f.key] = [];
    }
}

function toggleArray(key: string, i: number | 'custom') {
    const at = arr[key].indexOf(i);
    if (at >= 0) arr[key].splice(at, 1); else arr[key].push(i);
}

function ensureArrayCustom(key: string) {
    if (!arr[key].includes('custom')) arr[key].push('custom');
}

// Counts what will actually be written, not what is merely selected -- an
// unticked row with a selection would otherwise inflate it.
const chosenCount = computed(() => compose().tags.length);

async function extend() {
    extending.value = true;
    await $1t.manualTag.value.extend(path.value, props.config as AutotaggerConfig);
    extending.value = false;
    // Values changed underneath the defaults, so rows that were empty may now
    // have a source. Recompute rather than leave stale selections.
    resetToDefaults();
}

/// Parse a typed value into the shape the field expects.
///
/// Returns undefined for anything unusable, which the caller treats as "leave
/// this field unwritten" -- silently writing 0 for an unparseable BPM would be
/// worse than writing nothing.
function parseCustom(key: string, raw: string): any {
    const v = (raw ?? '').trim();
    if (!v) return undefined;
    if (key === 'bpm') {
        const n = parseInt(v, 10);
        return Number.isFinite(n) ? n : undefined;
    }
    if (key === 'explicit') {
        if (/^(y|yes|true|1)$/i.test(v)) return true;
        if (/^(n|no|false|0)$/i.test(v)) return false;
        return undefined;
    }
    return v;
}

/// Build the track that will be written.
///
/// Starts from the highest-accuracy match so the fields the matrix does not
/// arbitrate -- platform, duration, ids, arbitrary `other` frames -- still carry
/// something coherent, then replaces every row from its chosen source.
/// Build the track to write, and the list of tags that may be written.
///
/// Two results because they answer different questions. The track carries the
/// values; the tag list decides which of them the backend is allowed to touch,
/// and it is the tag list that makes "only the artwork" safe. Leaving a field
/// empty is not equivalent: Title and Artist are written unguarded, so an empty
/// one erases what the file had.
///
/// A row contributes its tag only when it is ticked *and* resolves to a real
/// value. An unticked row, a row with nothing selected, and a row pointed at an
/// empty cell are all the same statement -- do not touch this field -- and the
/// values already on the file survive untouched.
function compose(): { track: Track, tags: string[] } {
    const base: any = JSON.parse(JSON.stringify(matches.value[0].track));
    const tags: string[] = [];

    for (const f of SCALARS) {
        if (!enabled[f.key]) continue;
        const keys = [f.key, ...(f.siblings ?? [])];
        const choice = scalar[f.key];
        if (choice === undefined) continue;

        if (choice === 'custom') {
            const parsed = parseCustom(f.key, custom[f.key]);
            if (parsed === undefined) continue;
            base[f.key] = parsed;
            // A typed date cannot also supply a year, and a typed artwork URL
            // has no thumbnail -- clear the siblings rather than leaving the
            // primary match's, which would pair a new value with an old one.
            for (const k of keys.slice(1)) base[k] = undefined;
            tags.push(f.tag);
            continue;
        }

        const src: any = matches.value[choice].track;
        if (!has(src, f.key)) continue;
        for (const k of keys) base[k] = src[k];
        tags.push(f.tag);
    }

    for (const f of ARRAYS) {
        if (!enabled[f.key]) continue;
        const out: string[] = [];
        for (const c of arr[f.key]) {
            const vals: string[] = c === 'custom'
                ? (custom[f.key] ?? '').split(',').map((s) => s.trim()).filter(Boolean)
                : ((matches.value[c as number].track as any)[f.key] ?? []);
            for (const v of vals) if (!out.includes(v)) out.push(v);
        }
        if (!out.length) continue;
        base[f.key] = out;
        tags.push(f.tag);
    }

    return { track: base as Track, tags };
}

async function apply() {
    const { track, tags } = compose();

    // Narrow the config's tag list to the rows being applied -- an intersection,
    // never a union: a tag the operator disabled in their own settings stays
    // disabled, and tags this matrix has no row for (metaTags, lyrics, ids) are
    // passed through untouched rather than silently dropped.
    const config: any = JSON.parse(JSON.stringify(props.config));
    const owned = new Set(ROWS.map((r) => r.tag));
    config.tags = (config.tags ?? []).filter((t: string) => !owned.has(t) || tags.includes(t));

    saving.value = true;
    const response: any = await $1t.manualTag.value
        .applyComposed(track, path.value, config as AutotaggerConfig);
    saving.value = false;

    if (response.status == 'ok') {
        $q.notify({ message: 'Track saved!', timeout: 3000, position: 'top-right' });
        emit('applied');
    } else {
        $q.dialog({ title: 'Failed to save track', message: response.error, ok: true, cancel: false });
    }
}

// Turning the opt-in back off hides the radios on empty cells, so a selection
// still pointing at one would leave the row looking unset while the counter
// said otherwise. The written result is identical either way -- an empty source
// and no selection both mean "not written" -- so this is purely about the
// display not contradicting itself.
watch(allowEmpty, (on) => {
    if (on) return;
    for (const f of SCALARS) {
        const c = scalar[f.key];
        if (typeof c === 'number' && !has(matches.value[c].track, f.key)) scalar[f.key] = undefined;
    }
});

onMounted(resetToDefaults);
watch(matches, resetToDefaults, { deep: false });
</script>

<style scoped>
.mtm-scroll { overflow-x: auto; max-width: 100%; }
.mtm-table { border-collapse: collapse; font-size: 12px; }
.mtm-table th, .mtm-table td {
    border-bottom: 1px solid rgba(255,255,255,0.06);
    padding: 2px 8px;
    vertical-align: middle;
    text-align: left;
}
/* The field name has to stay readable while the columns scroll under it --
   without this you lose track of which row you are choosing for. */
.mtm-field {
    position: sticky; left: 0; z-index: 2;
    background: var(--q-dark, #1d1d1d);
    white-space: nowrap;
    color: #bdbdbd;
    min-width: 110px;
}
.mtm-col { min-width: 150px; }
.mtm-custom-col { min-width: 190px; }
.mtm-cell { max-width: 320px; }
.mtm-val {
    display: inline-block; max-width: 250px;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
    vertical-align: middle;
}
.mtm-empty { background: rgba(255,255,255,0.02); }
.mtm-apply { min-width: 54px; text-align: center; }
/* Greyed rather than hidden: you still need to see what a row *would* take, to
   decide whether to turn it back on. */
.mtm-off td:not(.mtm-apply) { opacity: 0.32; pointer-events: none; }
.mtm-dash { color: #616161; }
.mtm-art { height: 34px; width: 34px; object-fit: cover; border-radius: 2px; vertical-align: middle; }
/* The control and the field are two separate decisions -- ticking the source
   and typing the value -- so they need to read as two things, not one. */
.mtm-input {
    max-width: 140px;
    display: inline-block;
    vertical-align: middle;
    margin-left: 10px;
}
.mtm-all { font-size: 10px; opacity: 0.7; }
.mtm-divider td {
    padding: 6px 8px; color: #9e9e9e; font-size: 11px;
    background: rgba(255,255,255,0.03);
}
.mtm-link { color: #00d2b4; cursor: pointer; text-decoration: underline; }
</style>
