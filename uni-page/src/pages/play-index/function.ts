import type { PlayableInfo } from '@/pages/play-index/define.ts';

export function mapManageDisplayInfo(info: PlayableInfo) {
  if ('Plain' in info.manage) {
    return 'Plain HTML';
  } else if ('SugarCube' in info.manage) {
    return `SugarCube ML`;
  }
}
