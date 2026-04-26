import type { MediaProbe } from "$lib/api/media";
import type { VideoEntry } from "$lib/api/video";

var entry = $state<VideoEntry | null>(null);
var probe = $state<MediaProbe | null>(null);

export const editStore = {
  get entry() {
    return entry;
  },
  get probe() {
    return probe;
  },
  get isOpen() {
    return entry !== null;
  },

  open(e: VideoEntry, p: MediaProbe) {
    entry = e;
    probe = p;
  },

  close() {
    entry = null;
    probe = null;
  },
};
