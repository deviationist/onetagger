<template>
<div class='q-px-md'>

    <!-- One field for both ways in. A URL is recognised rather than declared:
         asking the operator to first say which of the two they are about to
         paste is a question the string itself already answers. -->
    <div class='row items-center q-gutter-sm q-mb-sm'>
        <q-select
            v-model='platform'
            :options='platformOptions'
            emit-value
            map-options
            dense
            outlined
            options-dense
            style='min-width: 150px;'
            label='Source'
        ></q-select>
        <q-input
            v-model='query'
            dense
            outlined
            clearable
            class='col'
            :label='inputLabel'
            :disable='busy'
            @keyup.enter='submit'
        >
            <template v-slot:append>
                <q-icon name='mdi-magnify'></q-icon>
            </template>
        </q-input>
        <q-btn
            flat
            color='primary'
            :loading='busy'
            :disable='!query || busy'
            @click='submit'
        >Find</q-btn>
    </div>

    <!-- What this does, said once rather than in a tooltip: the panel is new
         and the two inputs it accepts are not guessable from the field. -->
    <div class='text-caption text-grey-6 q-mb-md' v-if='!candidates.length && !tracks.length && !error'>
        Paste a release URL to go straight to it, or type artist and title to search.
        Picking a track adds it beside whatever the search found, so the matrix can
        take fields from both.
    </div>

    <div class='text-red text-body2 q-mb-sm' v-if='error'>{{ error }}</div>

    <!-- Releases -->
    <q-list v-if='candidates.length && !tracks.length'>
        <q-item
            v-for='c in candidates'
            :key='c.id'
            clickable
            @click='openRelease(c)'
        >
            <q-item-section avatar>
                <q-img width='48px' height='48px' :src='c.art' :placeholder-src='PLACEHOLDER_IMG'></q-img>
            </q-item-section>
            <q-item-section>
                <q-item-label class='text-grey-4 text-weight-medium'>{{ c.title }}</q-item-label>
                <q-item-label class='text-grey-6'>
                    <!-- The fields that separate two pressings of one record.
                         Anything absent is simply not drawn: Discogs omits
                         these per result and a row of dashes reads as data. -->
                    <span v-for='f in facts(c)' :key='f.label'>
                        <q-badge outline color='grey-9' class='q-mr-xs'>
                            <span class='text-uppercase text-grey-6'>{{ f.label }}</span>
                        </q-badge>
                        <span class='text-caption monospace text-weight-medium text-grey-4 q-mr-sm'>{{ f.value }}</span>
                    </span>
                </q-item-label>
            </q-item-section>
            <q-item-section side v-if='c.url'>
                <q-btn
                    icon='mdi-open-in-new'
                    flat round size='sm' color='grey-7'
                    @click.stop='$1t.url(c.url)'
                ></q-btn>
            </q-item-section>
        </q-item>
    </q-list>

    <!-- Tracklist -->
    <div v-if='tracks.length'>
        <div class='row items-center q-mb-sm'>
            <q-btn flat dense no-caps color='grey-5' icon='mdi-chevron-left' @click='back'>
                {{ candidates.length ? 'Back to results' : 'New search' }}
            </q-btn>
            <q-space></q-space>
            <span class='text-caption text-grey-6'>{{ release?.title }}</span>
        </div>
        <q-list>
            <q-item v-for='(m, i) in tracks' :key='i' clickable @click='toggle(i)'>
                <q-item-section avatar>
                    <q-checkbox :model-value='picked.includes(i)' @update:model-value='toggle(i)'></q-checkbox>
                </q-item-section>
                <q-item-section>
                    <q-item-label class='text-grey-6 text-weight-medium'>
                        <span class='text-grey-5'>{{ m.track.track_number }}</span>
                        {{ m.track.artists.join(", ") }}
                        <span class='text-grey-4'> {{ m.track.title }}</span>
                        <span class='text-grey-4' v-if='m.track.version'> ({{ m.track.version }})</span>
                    </q-item-label>
                    <q-item-label class='text-grey-6' v-if='trackLength(m.track)'>
                        <q-badge outline color='grey-9' class='q-mr-xs'>
                            <span class='text-uppercase text-grey-6'>Length</span>
                        </q-badge>
                        <span class='text-caption monospace text-weight-medium text-grey-4'>{{ trackLength(m.track) }}</span>
                        <!-- The one column that settles "is this the same
                             recording" between an original and an extended mix
                             carrying identical titles. -->
                        <span v-if='lengthDelta(m.track, fileDuration) !== undefined'
                              class='text-caption monospace text-weight-medium q-ml-xs'
                              :class='deltaColor(lengthDelta(m.track, fileDuration)!)'>{{ deltaLabel(lengthDelta(m.track, fileDuration)!) }}</span>
                    </q-item-label>
                </q-item-section>
            </q-item>
        </q-list>
        <div class='row q-mt-md'>
            <q-space></q-space>
            <q-btn flat color='primary' :disable='!picked.length' @click='add'>
                Add {{ picked.length || '' }} to results
            </q-btn>
        </div>
    </div>

</div>
</template>

<script lang='ts' setup>
import { ref, computed, toRefs, PropType } from 'vue';
import { get1t } from '../scripts/onetagger';
import type { TrackMatch } from '../scripts/manualtag';
import { PLACEHOLDER_IMG } from '../scripts/quicktag';
import { useFileDuration, trackLength, lengthDelta, deltaLabel, deltaColor } from '../scripts/trackduration';

const $1t = get1t();
const props = defineProps({
    path: { type: String, required: true },
});
const { path } = toRefs(props);
const fileDuration = useFileDuration(path);
const emit = defineEmits(['add']);

interface ReleaseCandidate {
    id: string;
    title: string;
    artists: string[];
    year?: number;
    label?: string;
    catalogNumber?: string;
    format?: string;
    country?: string;
    trackCount?: number;
    art?: string;
    url?: string;
}

/// Only platforms that said they can do this. A picker offering a source that
/// can only answer "not supported" is worse than one that does not list it.
const platformOptions = computed(() => $1t.info.value.platforms
    .filter((p: any) => p.supportsDirectMatch)
    .map((p: any) => ({ label: p.platform.name, value: p.platform.id })));

const platform = ref<string>(platformOptions.value[0]?.value ?? 'discogs');
const query = ref('');
const busy = ref(false);
const error = ref('');
const candidates = ref<ReleaseCandidate[]>([]);
const tracks = ref<TrackMatch[]>([]);
const release = ref<ReleaseCandidate | undefined>(undefined);
const picked = ref<number[]>([]);

const inputLabel = computed(() => 'Release URL, or artist and title');

/// A URL is anything with a scheme or a host in it. Deliberately loose: the
/// backend is the one that decides whether a URL is claimed by a platform, and
/// a false positive here costs one cheap round trip that falls back to search.
function looksLikeUrl(s: string): boolean {
    return /^https?:\/\//i.test(s) || /\w+\.\w{2,}\//.test(s);
}

function facts(c: ReleaseCandidate): { label: string, value: string }[] {
    const out: { label: string, value: string }[] = [];
    const push = (label: string, v: any) => {
        if (v !== undefined && v !== null && String(v).trim() !== '') out.push({ label, value: String(v) });
    };
    push('year', c.year);
    push('label', c.label);
    push('cat', c.catalogNumber);
    push('format', c.format);
    push('country', c.country);
    return out;
}

async function submit() {
    if (!query.value || busy.value) return;
    error.value = '';
    candidates.value = [];
    tracks.value = [];
    picked.value = [];
    busy.value = true;
    try {
        const q = query.value.trim();
        if (looksLikeUrl(q)) {
            const r = await $1t.manualTag.value.directUrl(q);
            if (r?.platform && r?.id) {
                // The URL names the source, so follow it rather than whatever
                // the dropdown happened to be left on.
                platform.value = r.platform;
                await fetchRelease({ id: r.id, title: q, artists: [] });
                return;
            }
            error.value = 'No platform recognised that URL. Paste a release link, or search by artist and title instead.';
            return;
        }
        const r = await $1t.manualTag.value.directSearch(platform.value, q);
        if (r?.status !== 'ok') {
            error.value = r?.error ?? 'Search failed';
            return;
        }
        candidates.value = r.candidates ?? [];
        if (!candidates.value.length) error.value = 'Nothing found. Try fewer words, or paste the release URL.';
    } finally {
        busy.value = false;
    }
}

async function openRelease(c: ReleaseCandidate) {
    busy.value = true;
    error.value = '';
    try {
        await fetchRelease(c);
    } finally {
        busy.value = false;
    }
}

async function fetchRelease(c: ReleaseCandidate) {
    const r = await $1t.manualTag.value.directFetch(platform.value, c.id);
    if (r?.status !== 'ok') {
        error.value = r?.error ?? 'Could not load that release';
        return;
    }
    tracks.value = r.matches ?? [];
    release.value = c;
    picked.value = [];
    if (!tracks.value.length) error.value = 'That release has no tracks listed.';
}

function toggle(i: number) {
    const at = picked.value.indexOf(i);
    if (at === -1) picked.value.push(i);
    else picked.value.splice(at, 1);
}

function back() {
    tracks.value = [];
    picked.value = [];
    release.value = undefined;
    error.value = '';
}

function add() {
    emit('add', picked.value.map((i) => tracks.value[i]));
    back();
}
</script>
