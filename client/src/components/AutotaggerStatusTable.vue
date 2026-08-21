<template>
<div class='table-wrap' :class='{"status-list": !$1t.taggerStatus.value.done, "status-list-done": $1t.taggerStatus.value.done}'>
    <q-table
        class='status-table bg-dark'
        :rows='rows'
        :columns='columns'
        row-key='fullPath'
        virtual-scroll
        :rows-per-page-options='[0]'
        hide-pagination
        flat
        dark
        dense
        binary-state-sort
        :pagination='pagination'
        :virtual-scroll-sticky-size-start='48'
    >
        <template v-slot:body-cell='props'>
            <q-td :props='props'>
                <span v-if='props.col.name === "filename"' class='selectable text-white'>
                    {{ props.row.filename }}
                </span>
                <span v-else-if='props.col.name === "path"' class='selectable text-grey-5'>
                    {{ props.row.path }}
                </span>
                <span v-else>
                    <template v-if='props.row.platforms[props.col.name]'>
                        <img
                            v-if='props.row.platforms[props.col.name].status.usedShazam'
                            width='16'
                            height='16'
                            class='q-mr-xs'
                            style='margin-bottom: -3px;'
                            svg-inline
                            src='../assets/shazam_icon.svg'
                        />
                        <q-icon
                            size='xs'
                            :name='statusIcon(props.row.platforms[props.col.name].status.status)'
                            :color='statusColor(props.row.platforms[props.col.name].status.status)'
                        >
                            <q-tooltip v-if='props.row.platforms[props.col.name].status.message'>
                                {{ props.row.platforms[props.col.name].status.message }}
                            </q-tooltip>
                            <q-tooltip v-else-if='props.row.platforms[props.col.name].status.status === "ok"'>
                                Accuracy: {{ ((props.row.platforms[props.col.name].status.accuracy ?? 0) * 100).toFixed(2) }}%
                                <span v-if='props.row.platforms[props.col.name].status.reason'>, Reason: {{ props.row.platforms[props.col.name].status.reason }}</span>
                            </q-tooltip>
                        </q-icon>
                    </template>
                    <span v-else class='text-grey-8'>—</span>
                </span>
            </q-td>
        </template>
    </q-table>
</div>
</template>

<script lang='ts' setup>
import { computed, ref, type PropType } from 'vue';
import { get1t } from '../scripts/onetagger.js';
import { TaggingStatusWrap } from '../scripts/autotagger';
import { platformText, statusIcon, statusColor } from '../scripts/autotaggerStatus';

const props = defineProps({
    statuses: { type: Array as PropType<TaggingStatusWrap[][]>, required: true },
});

const $1t = get1t();

// Path helpers — handle both forward and backslashes for cross-platform paths
function basename(p: string): string {
    const parts = p.split(/[\\/]/);
    return parts[parts.length - 1] || p;
}

function dirname(p: string): string {
    const parts = p.split(/[\\/]/);
    parts.pop();
    return parts.join('/');
}

// Discover platforms from the unfiltered statuses so columns don't disappear when filtering
const platformList = computed<{ id: string; label: string }[]>(() => {
    if ($1t.taggerStatus.value.type === 'audioFeatures') {
        return [{ id: 'audioFeatures', label: 'AUDIO FEATURES' }];
    }
    const seen = new Set<string>();
    for (const row of $1t.taggerStatus.value.statuses) {
        for (const entry of row) seen.add(entry.platform);
    }
    const ordered: string[] = [];
    for (const p of $1t.config.value.platforms) {
        if (seen.has(p)) ordered.push(p);
    }
    for (const p of seen) {
        if (!ordered.includes(p)) ordered.push(p);
    }
    return ordered.map((id) => ({ id, label: platformText(id) }));
});

interface Row {
    filename: string;
    path: string;
    fullPath: string;
    platforms: Record<string, TaggingStatusWrap>;
}

const rows = computed<Row[]>(() => {
    const isAudio = $1t.taggerStatus.value.type === 'audioFeatures';
    return props.statuses.map((entries) => {
        const fullPath = entries[0]?.status.path ?? '';
        const platforms: Record<string, TaggingStatusWrap> = {};
        if (isAudio) {
            if (entries[0]) platforms['audioFeatures'] = entries[0];
        } else {
            for (const e of entries) platforms[e.platform] = e;
        }
        return {
            filename: basename(fullPath),
            path: dirname(fullPath),
            fullPath,
            platforms,
        };
    });
});

// Higher score = better match. Used as the sort key for platform columns.
function platformScore(w: TaggingStatusWrap | undefined): number {
    if (!w) return 0;
    switch (w.status.status) {
        case 'ok': return 3000 + (w.status.accuracy ?? 0) * 100;
        case 'skipped': return 200;
        case 'error': return 100;
        default: return 0;
    }
}

const columns = computed(() => {
    const cols: any[] = [
        { name: 'filename', label: 'Filename', field: 'filename', sortable: true, align: 'left' },
        { name: 'path', label: 'Path', field: 'path', sortable: true, align: 'left' },
    ];
    for (const p of platformList.value) {
        cols.push({
            name: p.id,
            label: p.label,
            field: (row: Row) => row.platforms[p.id],
            sortable: true,
            align: 'center',
            sort: (a: TaggingStatusWrap | undefined, b: TaggingStatusWrap | undefined) =>
                platformScore(a) - platformScore(b),
        });
    }
    return cols;
});

const pagination = ref({ sortBy: 'filename', descending: false, rowsPerPage: 0 });
</script>

<style scoped>
.table-wrap {
    margin: 0 16px;
    padding-bottom: 40px; /* Clear the floating Stop FAB so the last row stays visible */
    display: flex;
    flex-direction: column;
}

.status-table {
    flex: 1 1 auto;
    min-height: 0;
}

.status-table :deep(.q-table__top) {
    padding: 4px 12px;
}

.status-table :deep(thead tr th) {
    position: sticky;
    top: 0;
    z-index: 1;
    background-color: #1d1d1d;
}

.status-list {
    height: calc(100vh - 248px);
}

.status-list-done {
    height: calc(100vh - 308px);
}
</style>
