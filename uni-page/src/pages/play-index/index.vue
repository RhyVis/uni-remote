<script setup lang="ts">
import { usePlayIndexStore } from '@/pages/play-index/store.ts';
import { onMounted } from 'vue';
import { storeToRefs } from 'pinia';
import { mapManageDisplayInfo } from '@/pages/play-index/function.ts';

const store = usePlayIndexStore();
const { infoList } = storeToRefs(store);

const handleJump = (id: string, sub_id: string) => {
  location.replace(`/play/${id}/${sub_id}/index-path`);
};

onMounted(() => {
  store.fetchInfo();
});
</script>

<template>
  <div class="q-ml-xs q-mr-md q-my-md q-gutter-md">
    <template v-for="(entry, entryIndex) in infoList" :key="entryIndex">
      <div class="row r-no-sel">
        <q-chip dense>
          <q-avatar icon="bookmark" color="secondary" />
          {{ entryIndex }}
        </q-chip>
        <q-chip dense v-if="entry.name">
          <q-avatar icon="key" color="primary" text-color="white" />
          {{ entry.name }}
        </q-chip>
        <q-chip dense v-else>
          <q-avatar icon="grid_3x3" color="primary" text-color="white" />
          '{{ entry.id }}'
        </q-chip>
        <q-chip dense>
          {{ mapManageDisplayInfo(entry) }}
        </q-chip>
      </div>
      <template v-if="'Plain' in entry.manage">
        <q-list bordered separator>
          <q-item>
            <q-item-section>
              <q-item-label>{{ entry.name ? entry.name : entry.id }}</q-item-label>
            </q-item-section>
            <q-item-section side>
              <q-btn
                round
                size="sm"
                color="primary"
                icon="play_arrow"
                @click="handleJump(entry.id, entry.manage.Plain)"
              />
            </q-item-section>
          </q-item>
        </q-list>
      </template>
      <template v-else-if="'SugarCube' in entry.manage">
        <q-list bordered separator>
          <q-item v-for="(instance, instanceIndex) in entry.manage.SugarCube" :key="instanceIndex">
            <q-item-section>
              <q-item-label>
                {{ instance.name ? instance.name : `'${instance.id}'` }}
              </q-item-label>
              <q-item-label caption>
                <div class="q-mx-xs q-mt-xs q-gutter-y-xs">
                  <div class="col-auto">Index: {{ instance.index }}</div>
                  <div class="col-auto">Layers:</div>
                  <div class="col-auto q-ml-xs">
                    <div v-for="(layer, layerIndex) in instance.layers" :key="layerIndex">
                      - {{ layer }}
                    </div>
                  </div>
                  <template v-if="instance.mods">
                    <div class="col-auto">Mods:</div>
                    <div class="col-auto q-ml-xs">
                      <div v-for="([mod, modSubId], modIndex) in instance.mods" :key="modIndex">
                        - {{ mod }} : {{ modSubId }}
                      </div>
                    </div>
                  </template>
                </div>
              </q-item-label>
            </q-item-section>
            <q-item-section side>
              <q-btn
                round
                size="sm"
                color="primary"
                icon="play_arrow"
                @click="handleJump(entry.id, instance.id)"
              />
            </q-item-section>
          </q-item>
        </q-list>
      </template>
    </template>
  </div>
</template>
