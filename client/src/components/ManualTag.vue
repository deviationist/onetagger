<template>
<q-dialog v-model='show' persistent>
<q-card :style="view == 'matrix' ? 'min-width: 90vw; min-height: 50vh;' : 'min-width: 650px; min-height: 50vh;'" class='q-pa-lg'>

    <!-- Title -->
    <q-card-section>
        <div class='text-subtitle2 q-mb-xs text-bold text-center text-primary'>MANUAL TAG</div>
        <div class='monospace text-subtitle2 text-grey-6 text-center'>{{ path }}</div>
        <!-- Two renderings of the same results, not two features. The list
             picks a winning *match*; the matrix picks a winning *source per
             field*, which is the thing the list cannot express. -->
        <div class='text-center q-mt-md' v-if='$1t.manualTag.value.matches.length > 0'>
            <q-btn-toggle
                v-model='view'
                dense flat no-caps
                toggle-color='primary'
                :options="[{label: 'List', value: 'list'}, {label: 'Matrix', value: 'matrix'}]"
            />
        </div>
    </q-card-section>

    <!-- Body -->
    <q-card-section>
        <div class='manualtag-results bg-dark q-pt-md'>

            <!-- Matrix view -->
            <ManualTagMatrix
                v-if="view == 'matrix' && $1t.manualTag.value.matches.length > 0"
                :matches='$1t.manualTag.value.matches'
                :path='path!'
                :config='cachedConfig'
                @applied='exit'
            />

            <!-- Results list -->
            <q-list v-if="view == 'list' && ($1t.manualTag.value.busy || $1t.manualTag.value.done)">

                <!-- Empty results -->
                <div v-if='$1t.manualTag.value.done && $1t.manualTag.value.matches.length == 0' class='text-center'>
                    <div class='text-h6 text-grey-4 q-mt-md'>No results!</div>
                    <div class='text-subtitle2 text-grey-6 q-mt-md'>Try enabling more platforms or correcting <q-badge  outline color='grey-8'><span class='text-uppercase text-grey-4'>Title</span></q-badge> + <q-badge  outline color='grey-8'><span class='text-uppercase text-grey-4'>Artist</span></q-badge> tag</div>
                </div>

                <!-- Matches -->
                <q-item v-for='(match, i) in $1t.manualTag.value.matches' :key='i'>
                    <q-item-section avatar>
                        <div class='row items-center'>
                            <div>
                                <div class='q-pr-sm'>
                                    <q-checkbox
                                        :model-value="selected.includes(match)"
                                        @update:model-value="(v: any) => toggleMatch(match)"
                                    ></q-checkbox>
                                    <!-- Which position this match holds in the
                                         merge. Only shown once more than one is
                                         selected, because with a single match
                                         there is no precedence to explain. -->
                                    <q-badge
                                        v-if='selected.length > 1 && selected.includes(match)'
                                        :color='selectionIndex(match) === 1 ? "primary" : "grey-7"'
                                        :label='selectionIndex(match)'
                                        class='q-ml-xs'
                                    >
                                        <q-tooltip>{{ selectionIndex(match) === 1
                                            ? 'Base match — its fields win'
                                            : 'Fills only the fields still empty' }}</q-tooltip>
                                    </q-badge>
                                </div>
                                <!-- Open URL -->
                                <q-btn 
                                    icon='mdi-open-in-new' 
                                    flat 
                                    round 
                                    size='sm' 
                                    color='grey-7'
                                    style='margin-left: 5px;'
                                    @click='$1t.url(match.track.url)'                                    
                                    v-if='match.track.url'
                                ></q-btn>
                            </div>
                            <q-img 
                                width='48px' 
                                height='48px'
                                :src='match.track.thumbnail??match.track.art'
                                :placeholder-src="PLACEHOLDER_IMG"
                            ></q-img>
                        </div>
                    </q-item-section>
                    <q-item-section>
                        <q-item-label overline class='text-grey-4'>
                            <span>{{ match.track.platform.toUpperCase() }}</span>
                            <span class='q-px-sm' :class='accuracyColor(match.accuracy)'><span class='text-subtitle3'>{{ (match.accuracy * 100.0).toFixed(2) }}%</span></span>
                            <span v-if='match.reason != "fuzzy"'>{{ match.reason.toUpperCase() }}</span>
                        </q-item-label>
                        <q-item-label class='title-span text-grey-6 text-weight-medium'>{{ match.track.artists.join(", ") }}
                            <span class='text-grey-4 text-weight-medium'> {{ match.track.title }}</span>
                            <span class='text-grey-4' v-if='match.track.version'> ({{ match.track.version }})</span>
                        </q-item-label>
                        <q-item-label class='text-grey-6' v-if='match.track.album'><q-badge  outline color='grey-9'><span class='text-uppercase text-grey-6'>Album</span></q-badge> <span class='text-caption text-weight-medium text-grey-5'>{{ match.track.album }}</span></q-item-label>
                        <q-item-label class='text-grey-6'>
                            <span v-if='match.track.genres.length > 0'>
                                <q-badge outline color='grey-9' class='q-mr-xs'><span class='text-uppercase text-grey-6'>Genre</span></q-badge>
                                <span class='text-caption text-weight-bold text-grey-4'>{{ match.track.genres.join(", ") }}</span>
                            </span>
                            <span v-if='trackLength(match.track)'>
                                <q-badge outline color='grey-9' class='q-mx-xs'><span class='text-uppercase text-grey-6'>Length</span></q-badge>
                                <span class='text-caption monospace text-weight-medium text-grey-4'>{{ trackLength(match.track) }}</span>
                                <span v-if='lengthDelta(match.track) !== undefined'
                                      class='text-caption monospace text-weight-medium q-ml-xs'
                                      :class='deltaColor(lengthDelta(match.track)!)'>{{ deltaLabel(lengthDelta(match.track)!) }}</span>
                            </span>
                            <span v-if='match.track.bpm'>
                                <q-badge outline color='grey-9' class='q-mx-xs'><span class='text-uppercase text-grey-6'>BPM</span></q-badge>
                                <span class='text-caption monospace text-weight-medium text-grey-4'>{{ match.track.bpm }}</span>
                            </span>
                            <span v-if='match.track.key'>
                                <q-badge outline color='grey-9' class='q-mx-xs'><span class='text-uppercase text-grey-6'>Key</span></q-badge>
                                <span class='text-caption monospace text-weight-medium' :style='keyColor(match.track.key)'>{{ match.track.key }}</span>
                            </span>
                            <br>
                            <span v-if='match.track.release_date'>
                                <q-badge outline color='grey-9' class='q-mr-xs'><span class='text-uppercase text-grey-6'>Release Date</span></q-badge>
                                <span class='text-caption monospace text-weight-medium text-grey-4'>{{ match.track.release_date }}</span>
                            </span>
                        </q-item-label>
                    </q-item-section>
                </q-item>
            </q-list>

            <!-- Config -->
            <div v-else>
                <div class='q-mt-md text-subtitle2 text-bold text-center text-primary'>PLATFORMS</div>
                <autotagger-platforms dense></autotagger-platforms>
                <autotagger-tags manual-tag></autotagger-tags>
                <autotagger-platform-specific class='q-mt-lg q-px-lg'></autotagger-platform-specific>
            </div>

        </div>

        <!-- Errors -->
        <div v-if='$1t.manualTag.value.errors.length > 0' class='text-center text-red text-body2 q-pt-sm clickable' @click='errorList = true'>
            Some platforms failed to search.<span class='keybind-icon q-px-sm text-caption text-bold'>CLICK</span> here to see details.
        </div>

    </q-card-section>

    <!-- Actions -->    
    <q-card-section class='row'>
        <q-space></q-space>
        <!-- Cancel / close -->
        <div class='q-px-sm'>
            <q-btn flat color='red' @click='exit' v-if='!saving'>Close</q-btn>
        </div>
        <!-- Start tagging -->
        <div class='q-px-sm' v-if='!$1t.manualTag.value.done'>
            <q-btn 
                flat 
                color='primary' 
                @click='start' 
                :disable='$1t.manualTag.value.busy && !$1t.manualTag.value.done' 
                :loading='$1t.manualTag.value.busy'
            >Start</q-btn>
        </div>
        <!-- What a multi-select actually does. Merging has always worked; the
             dialog never said so, and selection order silently decided which
             source won a contested field. -->
        <div class='q-px-sm text-caption text-grey-6' v-if="view == 'list' && selected.length > 1"
             style='max-width: 420px; line-height: 1.35;'>
            Merging {{selected.length}} matches: fields come from
            <span class='text-primary'>#1</span>, and the rest fill only what it
            leaves empty. Untick and re-tick to change the order.
        </div>
        <!-- Apply -->
        <div class='q-px-sm' v-if="view == 'list' && selected.length > 0">
            <q-btn 
                flat 
                color='primary' 
                @click='apply'
                :disable='saving'
                :loading='saving'
            >Apply</q-btn>
        </div>

    </q-card-section>

</q-card>
</q-dialog>

<!-- Error list -->
<q-dialog v-model='errorList'>
<q-card style='min-width: 420px;'>
    <!-- Title -->
    <q-card-section>
        <div class='q-mt-md text-subtitle2 text-bold text-center text-red'>ERRORS</div>
    </q-card-section>

    <!-- Errors -->
    <q-card-section>
        <div v-for='error in $1t.manualTag.value.errors' class='text-subtitle2'>
            <span class="text-grey-5"><span class='text-grey-4 monospace'>{{ error.platform.toUpperCase() }}</span>: {{ error.error }}</span>
        </div>
    </q-card-section>

    <!-- Hide -->
    <q-card-section class='row'>
        <q-space></q-space>
        <q-btn flat color='red' @click='errorList = false'>Close</q-btn>
    </q-card-section>

</q-card>
</q-dialog>


</template>

<script lang='ts' setup>
import { ref, toRefs, watch, computed } from 'vue';
import { TrackMatch } from '../scripts/manualtag';
import { get1t } from '../scripts/onetagger';
import { AutotaggerConfig } from '../scripts/autotagger';
import { useQuasar } from 'quasar';
import AutotaggerPlatforms from './AutotaggerPlatforms.vue';
import AutotaggerTags from './AutotaggerTags.vue';
import AutotaggerPlatformSpecific from './AutotaggerPlatformSpecific.vue';
import ManualTagMatrix from './ManualTagMatrix.vue';
import { PLACEHOLDER_IMG, keyColor } from '../scripts/quicktag';

const $q = useQuasar();
const $1t = get1t();
const show = ref(false);
const emit = defineEmits(['exit']);
/// The length of the file being tagged, when it is known.
///
/// Taken from the player, which is told the duration by the backend when a file
/// is loaded -- and told it *before* the audio sink is touched, so this works
/// even where there is no audio device at all, as in the Docker image.
///
/// Guarded on the path, because the player holds whatever was last loaded and
/// that is not always the file this dialog was opened for. A delta measured
/// against the wrong file would be worse than no delta: it looks like an
/// answer.
const fileDuration = computed<number | undefined>(() => {
    const player = $1t.player.value;
    if (!player?.path || !path?.value) return undefined;
    if (player.path !== path.value) return undefined;
    const secs = Math.round(player.duration);
    return secs > 1 ? secs : undefined;
});

/// A platform result's length, as m:ss.
///
/// Absent on some platforms and zero-valued on others, and a length of 0:00 is
/// worse than no length at all -- it reads as a fact rather than a gap -- so
/// both are treated as unknown and the badge is left off.
function trackLength(track: any): string | undefined {
    const secs = track?.duration?.secs;
    if (!secs) return undefined;
    return `${Math.floor(secs / 60)}:${String(secs % 60).padStart(2, '0')}`;
}

/// How far a result is from the file being tagged, in seconds.
///
/// This is the number that actually settles a mix. Two remixes of one track
/// share a title, an artist and often an album, and differ by minutes -- so the
/// absolute length is useful and the *difference* is decisive.
function lengthDelta(track: any): number | undefined {
    const theirs = track?.duration?.secs;
    const ours = fileDuration.value;
    if (!theirs || !ours) return undefined;
    return theirs - ours;
}

function deltaLabel(delta: number): string {
    // An exact match is not "-0:00". Zero has no sign, and printing one makes a
    // perfect match look like a near miss.
    if (delta === 0) return 'exact';
    const sign = delta > 0 ? '+' : '-';
    const a = Math.abs(delta);
    return `${sign}${Math.floor(a / 60)}:${String(a % 60).padStart(2, '0')}`;
}

/// Two bands, and only one of them is ours to choose.
///
/// The outer bound is `max_duration_difference`, whose default is 30s: beyond
/// it the matcher would have rejected the candidate outright, so red means
/// "this would not have matched automatically" rather than a taste judgement.
///
/// The inner bound has no equivalent in OneTagger's config, so 15s is a
/// choice: wide enough to absorb the sloppy durations platforms report --
/// rounding, a counted fade, a silent tail -- and tight enough that two mixes
/// of one track rarely both land in it.
const CLOSE_ENOUGH_SECONDS = 15;
const MAX_DURATION_DIFFERENCE = 30;

function deltaColor(delta: number): string {
    const a = Math.abs(delta);
    if (a <= CLOSE_ENOUGH_SECONDS) return 'text-green-5';
    if (a <= MAX_DURATION_DIFFERENCE) return 'text-orange-5';
    return 'text-red-5';
}

const props = defineProps({
    path: { type: String, required: false }
});
const { path } = toRefs(props);
const saving = ref(false);
const selected = ref<TrackMatch[]>([]);
const view = ref<'list' | 'matrix'>('list');
const errorList = ref(false);
let cachedConfig = {};

/// Start manual tagger
function start() {
    $1t.manualTag.value.reset();

    // Generate config
    let config = JSON.parse(JSON.stringify($1t.config.value));
    config.path = '';
    if ($1t.spotify.value.clientId && $1t.spotify.value.clientSecret) {
        config.spotify = {
            clientId: $1t.spotify.value.clientId,
            clientSecret: $1t.spotify.value.clientSecret,
        }
    }
    cachedConfig = config;

    // Start
    $1t.manualTag.value.tagTrack(path!.value!, config);
}

/// Add or remove match
/// 1-based position in the merge, which is selection order rather than the
/// order shown: `toggleMatch` pushes on tick, and the server takes the first
/// as the base. That is invisible without this.
function selectionIndex(match: TrackMatch): number {
    return selected.value.indexOf(match) + 1;
}

function toggleMatch(match: TrackMatch) {
    let i = selected.value.indexOf(match);
    if (i != -1) {
        selected.value.splice(i, 1);
        return;
    }
    selected.value.push(match);
}

/// Exit manual tagger
function exit() {
    $1t.manualTag.value.reset();
    selected.value = [];
    saving.value = false;
    show.value = false;
    emit('exit');
}

/// Get accuracy color
function accuracyColor(acc: number) {
    if (acc == 1.0) return 'text-green';
    if (acc > 0.85) return 'text-yellow';
    return 'text-red';
}

/// Apply the matches
async function apply() {
    saving.value = true;
    let response: any = await $1t.manualTag.value.apply(selected.value, path!.value!, cachedConfig as AutotaggerConfig);
    // All ok
    if (response.status == 'ok') {
        $q.notify({
            message: "Track saved!",
            timeout: 3000,
            position: 'top-right'
        });
    // Show error
    } else {
        await new Promise((r, _) => {
            $q.dialog({
                title: 'Failed to save track',
                message: response.error,
                ok: true,
                cancel: false
            })
            .onOk(() => r(true));
        });
    }

    exit();
}

// Show / Hide
watch(path!, () => {
    // to bool
    show.value = !!(path!.value);
});


</script>

<style lang='scss' scoped>
.manualtag-results {
    min-height: 50vh;
    height: 50vh;
    overflow-y: scroll;
    overflow-x: hidden;
    border-radius: 8px;
    background-color: #99999910 !important
}
.keybind-icon {
    padding: 4px;
    border-radius: 2px;
    background: #262828;
    margin-bottom: 4px;
    margin-left: 4px;
}
</style>