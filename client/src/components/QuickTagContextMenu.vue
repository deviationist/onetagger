<template>
    <q-menu touch-position context-menu class='no-menu-shadow'>
        <q-list>

            <!-- Manual tag -->
            <q-item dense clickable @click='emit("manual-tag")' v-close-popup>
                <q-item-section avatar>
                    <q-icon name='mdi-magnify'></q-icon>
                </q-item-section>
                <q-item-section>
                    Manual Tag
                </q-item-section>
            </q-item>

            <!-- Edit tags -->
            <q-item dense clickable v-close-popup @click='tagEditor'>
                <q-item-section avatar>
                    <q-icon name='mdi-pencil'></q-icon>
                </q-item-section>
                <q-item-section>
                    Edit tags
                </q-item-section>
            </q-item>

            <!-- Delete file -->
            <q-item dense clickable v-close-popup @click='deleteFile'>
                <q-item-section avatar>
                    <q-icon name='mdi-delete' color='red'></q-icon>
                </q-item-section>
                <q-item-section class='text-red'>
                    Delete
                </q-item-section>
            </q-item>


        </q-list>
    </q-menu>
</template>

<script lang='ts' setup>
import { toRefs } from 'vue';
import { get1t } from '../scripts/onetagger';
import { useRouter } from 'vue-router';
import { useQuasar } from 'quasar';

const emit = defineEmits(['manual-tag']);
const props = defineProps({
    path: { type: String, required: true }
});
const { path } = toRefs(props);
const $1t = get1t();
const $q = useQuasar();
const $router = useRouter();

// Open tag editor
function tagEditor() {
    $1t.quickTag.value.toTagEditor = path.value;
    $router.push('/tageditor');
}

// Delete file option
function deleteFile() {
    // Confirm dialog
    $q.dialog({
        title: 'Delete File',
        message: 'Permanently delete the selected file from disk? This cannot be undone from OneTagger.',
        persistent: false,
        ok: {
            color: 'red'                        
        },
        cancel: {
            color: ''
        }
    }).onOk(() => {
        if ($1t.player.value.path == path.value)
            $1t.player.value.stop();
        // The list refresh is driven by the server's acknowledgement, handled
        // in the Quick Tag view -- see there for why not a timer.
        $1t.send('deleteFiles', { paths: [path.value] });
    });
}

</script>
