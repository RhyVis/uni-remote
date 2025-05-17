import { defineStore } from 'pinia';
import type { PlayableInfo } from '@/pages/play-index/define.ts';
import axios from 'axios';

interface PlayIndexStoreState {
  infoList: PlayableInfo[];
}

export const usePlayIndexStore = defineStore('play-index', {
  state: (): PlayIndexStoreState => ({
    infoList: [],
  }),
  actions: {
    async fetchInfo() {
      try {
        this.infoList = (await axios.get('/api/list-all')).data as PlayableInfo[];
      } catch (e) {
        console.error(e);
      }
    },
  },
});
