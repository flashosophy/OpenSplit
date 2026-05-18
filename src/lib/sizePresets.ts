export interface WindowSizePreset {
  id: string;
  group: "Square" | "Horizontal 16:9" | "Vertical 9:16";
  label: string;
  width: number;
  height: number;
}

export const WINDOW_SIZE_PRESETS: WindowSizePreset[] = [
  {
    id: "square-1024",
    group: "Square",
    label: "Square 1024 x 1024",
    width: 1024,
    height: 1024,
  },
  {
    id: "horizontal-720p",
    group: "Horizontal 16:9",
    label: "720p 1280 x 720",
    width: 1280,
    height: 720,
  },
  {
    id: "horizontal-1080p",
    group: "Horizontal 16:9",
    label: "1080p 1920 x 1080",
    width: 1920,
    height: 1080,
  },
  {
    id: "horizontal-1440p",
    group: "Horizontal 16:9",
    label: "1440p 2560 x 1440",
    width: 2560,
    height: 1440,
  },
  {
    id: "vertical-720p",
    group: "Vertical 9:16",
    label: "720p 720 x 1280",
    width: 720,
    height: 1280,
  },
  {
    id: "vertical-1080p",
    group: "Vertical 9:16",
    label: "1080p 1080 x 1920",
    width: 1080,
    height: 1920,
  },
  {
    id: "vertical-1440p",
    group: "Vertical 9:16",
    label: "1440p 1440 x 2560",
    width: 1440,
    height: 2560,
  },
];
