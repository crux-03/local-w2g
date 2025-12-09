// Type definitions matching the Rust backend

export interface User {
  id: string;
  username: string | null;
  is_owner: boolean;
  is_ready: boolean;
  status: "ready" | "waiting" | "error";
}

export interface UserPermission {
  user_id: string;
  allow_pause: boolean;
  allow_seek: boolean;
  allow_subtitle: boolean;
  allow_audio: boolean;
}

export interface Video {
  id: string;
  filename: string;
  size_bytes: number;
  size_display: string;
  uploaded_at: string;
  uploader_id: string;
}

export interface LogEntry {
  timestamp: string;
  user_id: string;
  username: string | null;
  action: string;
  source: "server" | "client";
}

export interface ClientInfo {
  client_id: string | null;
  is_owner: boolean;
}

export interface Config {
  server_url: string | null;
  video_storage_path: string | null;
  mpv_binary_path: string | null;
}

// Server messages (received from WebSocket)
export type ServerMessage =
  | { type: "connected"; client_id: string; is_owner: boolean }
  | { type: "user_update"; users: User[] }
  | { type: "permissions_update"; permissions: UserPermission[] }
  | { type: "playlist_update"; videos: Video[]; current_index: number | null }
  | { type: "activity_log"; logs: LogEntry[] }
  | { type: "pause" }
  | { type: "play" }
  | { type: "seek"; position: number }
  | { type: "subtitle_track"; index: number }
  | { type: "audio_track"; index: number }
  | { type: "ready"; client_id: string; value: boolean }
  | { type: "video_uploaded"; video: Video }
  | { type: "all_ready" }
  | { type: "ownership_transferred"; new_owner_id: string }
  | { type: "error"; message: string };