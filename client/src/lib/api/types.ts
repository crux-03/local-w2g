// ─── Primitives ──────────────────────────────────────────────────────────────

/** 64-bit snowflake ID, serialised as a string to preserve precision. */
export type Snowflake = string;

// ─── Permissions (bitflags) ───────────────────────────────────────────────────

/** Serialised as a signed 64-bit integer. Combine flags with bitwise OR. */
export type Permissions = number;

export const Permissions = {
  MANAGE_PLAYBACK: 1 << 0,
  SEND_MESSAGE: 1 << 1,
  MANAGE_USERS: 1 << 2,
  SEND_STATE: 1 << 3,
  MANAGE_MEDIA: 1 << 4,
} as const;

// ─── WidgetState ──────────────────────────────────────────────────────────────

export interface WidgetStateUpload {
  kind: "upload";
  uploader: Snowflake;
  filename: string;
  target: Snowflake;
  bytes_done: number;
  bytes_total: number;
}

export interface WidgetStateDownload {
  kind: "download";
  reporter: Snowflake;
  filename: string;
  bytes_done: number;
  bytes_total: number;
}

export type WidgetState = WidgetStateUpload | WidgetStateDownload;

export type ChatEntry = Entry & { kind: EntryKindChat };
export type SystemEntry = Entry & { kind: EntryKindSystem };
export type WidgetEntry = Entry & { kind: EntryKindWidget };

// ─── EntryKind ────────────────────────────────────────────────────────────────

export interface EntryKindChat {
  type: "chat";
  sender: Snowflake;
  content: string;
}

export interface EntryKindSystem {
  type: "system";
  content: string;
}

export interface EntryKindWidget {
  type: "widget";
  state: WidgetState;
  done: boolean;
}

export type EntryKind = EntryKindChat | EntryKindSystem | EntryKindWidget;

// ─── Entry ────────────────────────────────────────────────────────────────────

export interface Entry {
  id: Snowflake;
  timestamp: number;
  kind: EntryKind;
}

// ─── VideoReadiness / Verdict / UserReadinessView ─────────────────────────────

export interface VideoReadinessOnDevice {
  status: "on_device";
}

export interface VideoReadinessNotStarted {
  status: "not_started";
}

export type VideoReadiness = VideoReadinessOnDevice | VideoReadinessNotStarted;

export type Verdict = "ready" | "partial" | "not_ready";

export interface UserReadinessView {
  videos: Record<Snowflake, VideoReadiness>;
  verdict: Verdict;
}

// ─── ServerMessage ────────────────────────────────────────────────────────────

export interface ServerMessageMessageCreated {
  type: "message_created";
  entry: Entry;
}

export interface ServerMessageWidgetUpdated {
  type: "widget_updated";
  entry: Entry;
}

export interface ServerMessageWidgetDone {
  type: "widget_done";
  entry: Entry;
}

export interface ServerMessageRequestResyncReport {
  type: "request_resync_report";
  id: Snowflake;
}

export interface ServerMessageCommitResync {
  type: "commit_resync";
  timestamp: number;
}

export interface ServerMessageReadinessUpdated {
  type: "readiness_updated";
  readiness: UserReadinessView;
}

export interface ServerMessageRequestReadyConfirmation {
  type: "request_ready_confirmation";
  request_id: Snowflake;
  video_id: Snowflake;
  /** Millisecond deadline — may exceed Number.MAX_SAFE_INTEGER for large u64 values. */
  deadline_ms: number;
}

export interface ServerMessagePlay {
  type: "play";
  request_id: Snowflake;
}

export interface ServerMessagePlayAborted {
  type: "play_aborted";
  request_id: Snowflake;
  non_confirmers: Snowflake[];
}

export interface ServerMessageVideoSelected {
  type: "video_selected";
  video_id: Snowflake;
}

export interface ServerMessageError {
  type: "error";
  message: string;
}

export type ServerMessage =
  | ServerMessageMessageCreated
  | ServerMessageWidgetUpdated
  | ServerMessageWidgetDone
  | ServerMessageRequestResyncReport
  | ServerMessageCommitResync
  | ServerMessageReadinessUpdated
  | ServerMessageRequestReadyConfirmation
  | ServerMessagePlay
  | ServerMessagePlayAborted
  | ServerMessageVideoSelected
  | ServerMessageError;

// ─── ClientMessage ────────────────────────────────────────────────────────────

export interface ClientMessageSendMessage {
  type: "send_message";
  content: string;
}

export interface ClientMessageStartResync {
  type: "start_resync";
}

export interface ClientMessageSendResyncReport {
  type: "send_resync_report";
  state_id: Snowflake;
  timestamp: number;
}

export interface ClientMessageDownloadProgress {
  type: "download_progress";
  widget_id: Snowflake;
  bytes_done: number;
}

export interface ClientMessageDownloadDone {
  type: "download_done";
  widget_id: Snowflake;
}

export interface ClientMessageAssertReady {
  type: "assert_ready";
  video_id: Snowflake;
  on_device: boolean;
}

export interface ClientMessageHeartbeat {
  type: "heartbeat";
}

export interface ClientMessageConfirmReadyForPlay {
  type: "confirm_ready_for_play";
  request_id: Snowflake;
}

/** Unit variant — no payload beyond the discriminant. */
export interface ClientMessagePlay {
  type: "play";
}

export interface ClientMessageSelectVideo {
  type: "select_video";
  video_id: Snowflake;
}

export type ClientMessage =
  | ClientMessageSendMessage
  | ClientMessageStartResync
  | ClientMessageSendResyncReport
  | ClientMessageDownloadProgress
  | ClientMessageDownloadDone
  | ClientMessageAssertReady
  | ClientMessageHeartbeat
  | ClientMessageConfirmReadyForPlay
  | ClientMessagePlay
  | ClientMessageSelectVideo;
