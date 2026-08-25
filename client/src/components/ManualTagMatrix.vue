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

    <div class='mtm-scroll'>
        <table class='mtm-table'>
            <thead>
                <tr>
                    <th class='mtm-field'>Field</th>
                    <th v-for='(m, i) in matches' :key='i' class='mtm-col'>
                        <div class='monospace text-caption'>{{ m.track.platform.toUpperCase() }}</div>
                        <div class='text-caption' :class='accuracyColor(m.accuracy)'>
                            {{ Math.round(m.accuracy * 100) }}%
                        </div>
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
                <tr v-for='f in SCALARS' :key='f.key'>
                    <td class='mtm-field'>{{ f.label }}</td>
                    <td v-for='(m, i) in matches' :key='i' class='mtm-cell'
                        :class="{ 'mtm-empty': !has(m.track, f.key) }">
                        <q-radio
                            v-if='has(m.track, f.key)'
                            dense
                            size='xs'
                            :model-value='scalar[f.key]'
                            :val='i'
                            @update:model-value='scalar[f.key] = i'
                        >
                            <span class='mtm-val'>
                                <img v-if="f.key === 'art'" :src='m.track.art' class='mtm-art'>
                                <span v-else>{{ display(m.track, f.key) }}</span>
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
                    <td :colspan='matches.length + 2'>
                        Lists — tick every source you want included. Today these are
                        unioned silently whenever you select a second match; here it
                        is a choice.
                    </td>
                </tr>

                <!-- Arrays: "pick one source" is the wrong verb, so checkboxes. -->
                <tr v-for='f in ARRAYS' :key='f.key'>
                    <td class='mtm-field'>{{ f.label }}</td>
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
        <span class='text-caption text-grey-6'>{{ chosenCount }} of {{ SCALARS.length + ARRAYS.length }} fields set</span>
    </div>
</div>
</template>

<script lang='ts' setup>
import { ref, reactive, computed, watch, onMounted, toRefs, PropType } from 'vue';
import { useQuasar } from 'quasar';
import { get1t } from '../scripts/onetagger';
import type { TrackMatch } from '../scripts/manualtag';
import type { AutotaggerConfig, Track } from '../scripts/autotagger';

const $1t = get1t();
const $q = useQuasar();

const props = defineProps({
    matches: { type: Array as PropType<TrackMatch[]>, required: true },
    path: { type: String, required: true },
    config: { type: Object, required: true },
});
const { matches, path } = toRefs(props);
const emit = defineEmits(['applied']);

/// Fields the matrix can arbitrate with a single winner.
///
/// `siblings` exists because a few of these are stored as more than one field
/// and only make sense together: a source offering a release *year* but no full
/// date would otherwise contribute half of a date, and artwork carries its own
/// thumbnail URL. Choosing a source for the row takes all of its siblings.
const SCALARS = [
    { key: 'title',          label: 'Title' },
    { key: 'version',        label: 'Version' },
    { key: 'album',          label: 'Album' },
    { key: 'label',          label: 'Label' },
    { key: 'catalog_number', label: 'Catalogue №' },
    { key: 'key',            label: 'Key' },
    { key: 'bpm',            label: 'BPM',           hint: 'e.g. 128' },
    { key: 'release_date',   label: 'Release date',  siblings: ['release_year'], hint: 'YYYY-MM-DD' },
    { key: 'publish_date',   label: 'Publish date',  siblings: ['publish_year'], hint: 'YYYY-MM-DD' },
    { key: 'isrc',           label: 'ISRC' },
    { key: 'mood',           label: 'Mood' },
    { key: 'explicit',       label: 'Explicit',      hint: 'yes / no' },
    { key: 'art',            label: 'Artwork',       siblings: ['thumbnail'], hint: 'image URL' },
    { key: 'url',            label: 'URL' },
];

const ARRAYS = [
    { key: 'artists',       label: 'Artists' },
    { key: 'album_artists', label: 'Album artists' },
    { key: 'genres',        label: 'Genres' },
    { key: 'styles',        label: 'Styles' },
    { key: 'remixers',      label: 'Remixers' },
];

// field -> match index, or 'custom', or undefined (meaning: do not write it)
const scalar = reactive<Record<string, number | 'custom' | undefined>>({});
// field -> list of match indices (and possibly 'custom') to union
const arr = reactive<Record<string, (number | 'custom')[]>>({});
const custom = reactive<Record<string, string>>({});

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

function takeAll(i: number) {
    for (const f of SCALARS) if (has(matches.value[i].track, f.key)) scalar[f.key] = i;
    for (const f of ARRAYS) if (has(matches.value[i].track, f.key)) arr[f.key] = [i];
}

function toggleArray(key: string, i: number | 'custom') {
    const at = arr[key].indexOf(i);
    if (at >= 0) arr[key].splice(at, 1); else arr[key].push(i);
}

function ensureArrayCustom(key: string) {
    if (!arr[key].includes('custom')) arr[key].push('custom');
}

const chosenCount = computed(() =>
    SCALARS.filter((f) => scalar[f.key] !== undefined).length +
    ARRAYS.filter((f) => arr[f.key].length > 0).length);

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
function compose(): Track {
    const base: any = JSON.parse(JSON.stringify(matches.value[0].track));

    for (const f of SCALARS) {
        const keys = [f.key, ...(f.siblings ?? [])];
        const choice = scalar[f.key];
        if (choice === undefined) {
            for (const k of keys) base[k] = undefined;
            continue;
        }
        if (choice === 'custom') {
            base[f.key] = parseCustom(f.key, custom[f.key]);
            // A typed date cannot also supply a year field, and a typed artwork
            // URL has no thumbnail -- clear the siblings rather than leaving the
            // primary match's, which would pair a new value with an old one.
            for (const k of keys.slice(1)) base[k] = undefined;
            continue;
        }
        const src: any = matches.value[choice].track;
        for (const k of keys) base[k] = src[k];
    }

    for (const f of ARRAYS) {
        const out: string[] = [];
        for (const c of arr[f.key]) {
            const vals: string[] = c === 'custom'
                ? (custom[f.key] ?? '').split(',').map((s) => s.trim()).filter(Boolean)
                : ((matches.value[c as number].track as any)[f.key] ?? []);
            for (const v of vals) if (!out.includes(v)) out.push(v);
        }
        base[f.key] = out;
    }

    return base as Track;
}

async function apply() {
    saving.value = true;
    const response: any = await $1t.manualTag.value
        .applyComposed(compose(), path.value, props.config as AutotaggerConfig);
    saving.value = false;

    if (response.status == 'ok') {
        $q.notify({ message: 'Track saved!', timeout: 3000, position: 'top-right' });
        emit('applied');
    } else {
        $q.dialog({ title: 'Failed to save track', message: response.error, ok: true, cancel: false });
    }
}

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
