<script setup lang="ts">
import { useToggle } from '@vueuse/core';
import { useQuasar } from 'quasar';

const { notify } = useQuasar();
const [drawerOpen] = useToggle(false);

const handleDrawer = () => {
  notify({
    message: 'The feature is under construction',
    color: 'warning',
    position: 'top',
    icon: 'warning',
  });
};
</script>

<template>
  <q-layout view="hHh lpR fFf">
    <q-header class="bg-white text-subtitle2 text-black" elevated>
      <q-toolbar>
        <q-toolbar-title class="r-no-sel text-center">
          <q-avatar @click="handleDrawer">
            <q-icon name="play_circle" size="26px" />
          </q-avatar>
          UNI-REMOTE
        </q-toolbar-title>
      </q-toolbar>
    </q-header>

    <q-drawer v-model="drawerOpen" side="left" bordered>
      <q-list bordered separator>
        <q-item>
          <q-item-section avatar>
            <q-icon name="handyman" />
          </q-item-section>
          <q-item-section> Under Construction </q-item-section>
        </q-item>
      </q-list>
    </q-drawer>

    <q-page-container>
      <RouterView />
    </q-page-container>
  </q-layout>
</template>
