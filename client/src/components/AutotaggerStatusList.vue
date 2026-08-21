<template>
<q-list class='list text-left bg-dark q-py-sm'>
    <q-virtual-scroll :items='statuses' :class='{"status-list": !$1t.taggerStatus.value.done, "status-list-done": $1t.taggerStatus.value.done}'>
        <template v-slot="{item, index: i}">
            <q-item :key='i'>
                <q-item-section>
                    <q-item-label overline>
                        <span v-for='(i, index) in item'>
                            <span v-if='$1t.taggerStatus.value.type != "audioFeatures"' class='selectable text-white'>{{platformText(i.platform)}}</span>
                            <span v-if='$1t.taggerStatus.value.type == "audioFeatures"' class='selectable text-white'>AUDIO FEATURES</span>
                            <img width='16' height='16' class='q-ml-sm' style='margin-bottom: -3px;' v-if='i.status.usedShazam' svg-inline src='../assets/shazam_icon.svg' />
                            <q-icon size='xs' class='q-ml-sm q-mb-xs' :name='statusIcon(i.status.status)' :color='statusColor(i.status.status)'>
                                <q-tooltip v-if='i.status.message'>
                                    {{i.status.message}}
                                </q-tooltip>
                                <q-tooltip v-if='i.status.status == "ok"'>
                                    Accuracy: {{ (i.status.accuracy * 100).toFixed(2) }}%
                                    <span v-if='i.status.reason'>, Reason: {{ i.status.reason }}</span>
                                </q-tooltip>
                            </q-icon>
                            <span class='q-px-sm' v-if='index < item.length - 1'>|</span>
                        </span>
                    </q-item-label>
                    <span class='selectable text-grey-5'>{{item[0].status.path}}</span>
                </q-item-section>
            </q-item>
        </template>
    </q-virtual-scroll>
</q-list>
</template>

<script lang='ts' setup>
import type { PropType } from 'vue';
import { get1t } from '../scripts/onetagger.js';
import { TaggingStatusWrap } from '../scripts/autotagger';
import { platformText, statusIcon, statusColor } from '../scripts/autotaggerStatus';

defineProps({
    statuses: { type: Array as PropType<TaggingStatusWrap[][]>, required: true },
});

const $1t = get1t();
</script>

<style scoped>
.list {
    max-width: 80%;
    margin-left: 10%;
}

.status-list {
    height: calc(100vh - 248px);
}

.status-list-done {
    height: calc(100vh - 308px);
}
</style>
