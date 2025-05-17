export type PlayableInfo = {
  id: string;
  name: string | undefined;
  manage: { Plain: string } | { SugarCube: SugarCubeLabel[] };
};

export type SugarCubeLabel = {
  id: string;
  name: string | undefined;
  index: string;
  layers: string[];
  mods: [string, string][] | undefined;
};
