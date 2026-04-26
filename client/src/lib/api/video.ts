import type { Snowflake } from "./types";

export interface VideoEntry {
  id: Snowflake;
  display_name: String;
  audio_track: number;
  subtitle_track: number;
  order: number;
}
