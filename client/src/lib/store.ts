import { writable, derived, get } from 'svelte/store';
import type { User, UserPermission, Video, LogEntry, ClientInfo } from './types';

// Connection state
export const connected = writable(false);
export const clientInfo = writable<ClientInfo>({ client_id: null, is_owner: false });

// Users and permissions
export const users = writable<User[]>([]);
export const permissions = writable<Map<string, UserPermission>>(new Map());

// Playlist
export const playlist = writable<Video[]>([]);
export const currentVideoIndex = writable<number | null>(null);

// Downloaded videos - maps video ID to local file path
export const downloadedVideos = writable<Map<string, string>>(new Map());

// Currently downloading videos - set of video IDs
export const downloadingVideos = writable<Set<string>>(new Set());

// Download progress - maps video ID to progress info
export interface DownloadProgress {
  video_id: string;
  filename: string;
  downloaded: number;
  total: number;
  progress: number; // 0-100
  speed: number; // bytes per second
  speed_display: string;
}
export const downloadProgress = writable<Map<string, DownloadProgress>>(new Map());

// Activity log
export const activityLog = writable<LogEntry[]>([]);

// Current user (derived from users and clientInfo)
export const currentUser = derived(
  [users, clientInfo],
  ([$users, $clientInfo]) => {
    if (!$clientInfo.client_id) return null;
    return $users.find(u => u.id === $clientInfo.client_id) || null;
  }
);

// Current user's permissions (derived)
export const myPermissions = derived(
  [permissions, clientInfo],
  ([$permissions, $clientInfo]) => {
    if (!$clientInfo.client_id) return null;
    return $permissions.get($clientInfo.client_id) || null;
  }
);

// Check if current user can perform actions (derived)
export const canPause = derived(
  [clientInfo, myPermissions],
  ([$clientInfo, $myPermissions]) => {
    if ($clientInfo.is_owner) return true;
    return $myPermissions?.allow_pause || false;
  }
);

export const canSeek = derived(
  [clientInfo, myPermissions],
  ([$clientInfo, $myPermissions]) => {
    if ($clientInfo.is_owner) return true;
    return $myPermissions?.allow_seek || false;
  }
);

export const canChangeSubtitle = derived(
  [clientInfo, myPermissions],
  ([$clientInfo, $myPermissions]) => {
    if ($clientInfo.is_owner) return true;
    return $myPermissions?.allow_subtitle || false;
  }
);

export const canChangeAudio = derived(
  [clientInfo, myPermissions],
  ([$clientInfo, $myPermissions]) => {
    if ($clientInfo.is_owner) return true;
    return $myPermissions?.allow_audio || false;
  }
);

// Current video (derived)
export const currentVideo = derived(
  [playlist, currentVideoIndex],
  ([$playlist, $currentVideoIndex]) => {
    if ($currentVideoIndex === null) return null;
    return $playlist[$currentVideoIndex] || null;
  }
);

// Helper functions
export function getPermissionForUser(userId: string): UserPermission | null {
  const perms = get(permissions);
  return perms.get(userId) || null;
}

export function getUserById(userId: string): User | null {
  const userList = get(users);
  return userList.find(u => u.id === userId) || null;
}

// Reset all state (on disconnect)
export function resetState() {
  connected.set(false);
  clientInfo.set({ client_id: null, is_owner: false });
  users.set([]);
  permissions.set(new Map());
  playlist.set([]);
  currentVideoIndex.set(null);
  downloadedVideos.set(new Map());
  downloadingVideos.set(new Set());
  downloadProgress.set(new Map());
  activityLog.set([]);
}