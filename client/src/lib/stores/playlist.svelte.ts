import { type VideoEntry } from "$lib/api/video";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { type Snowflake, type UserReadinessView } from "$lib/api/types";

var entries = $state<VideoEntry[]>([]);
var selected = $state<Snowflake>();
var onDevice = $state<Snowflake[]>([]);
listen<VideoEntry[]>("playlist_updated", (event) => {
  entries = event.payload;
}).catch((error) => {
  console.error(`Failed to register playlist_updated listener: ${error}`);
});

listen<UserReadinessView>("readiness_updated", async (event) => {
  try {
    var files = (await invoke("load_local_files")) as Snowflake[];
    onDevice = files;
  } catch (error) {
    console.log(`Error when loading local files: ${error}`);
  }
}).catch((error) => {
  console.error(`Failed to register playlist_updated listener: ${error}`);
});

listen<Snowflake>("video_selected", async (event) => {
  selected = event.payload;
}).catch((error) => {
  console.error(`Failed to register video_selected listener: ${error}`);
});

export const playlistStore = {
  get entries() {
    return entries;
  },
  get selected() {
    return selected;
  },
  async requestPlaylist() {
    try {
      await invoke("request_playlist");
    } catch (error) {
      console.log(`Error when requesting playlist: ${error}`);
    }
  },
  async loadLocalFiles() {
    try {
      var files = (await invoke("load_local_files")) as Snowflake[];
      onDevice = files;
    } catch (error) {
      console.log(`Error when loading local files: ${error}`);
    }
  },
  async init() {
    try {
      await invoke("init_file_manager");
      await invoke("init_mpv_manager");
    } catch (error) {
      console.log(`Error when initializing file manager: ${error}`);
    }
  },
  async selectVideo(id: Snowflake) {
    try {
      await invoke("select_video", {
        videoId: id,
      });
    } catch (error) {
      console.log(`Error when selecting video: ${error}`);
    }
  },
  isSelected(id: Snowflake): boolean {
    return selected === id;
  },
  fileOnDevice(id: Snowflake): boolean {
    return onDevice.includes(id);
  },
  isLastItem(id: Snowflake): boolean {
    if (entries.length == 0) {
      return false;
    }
    return entries[entries.length - 1].id === id;
  },
};
