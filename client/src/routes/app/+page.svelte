<script lang="ts">
    import "$lib/assets/styles/main.css";
    import UserItem from "$lib/components/UserItem.svelte";
    import PlaylistItem from "$lib/components/PlaylistItem.svelte";
    import ContextMenu from "$lib/components/ContextMenu.svelte";
    import HostControls from "$lib/components/HostControls.svelte";
    import { goto } from "$app/navigation";
    import { invoke } from "@tauri-apps/api/core";
    import { listen } from "@tauri-apps/api/event";
    import { onMount, onDestroy } from "svelte";
    import {
        connected,
        clientInfo,
        users,
        permissions,
        playlist,
        currentVideoIndex,
        downloadedVideos,
        downloadingVideos,
        downloadProgress,
        activityLog,
        currentUser,
        resetState,
    } from "$lib/store";
    import type {
        ServerMessage,
        DownloadProgress,
        ClientInfo,
        Config,
    } from "$lib/types";

    let username = $state("");
    let videoStoragePath = $state("");
    let mpvBinaryPath = $state("");

    // Context menu state
    let contextMenuUserId: string | null = $state(null);
    let contextMenuX = $state(0);
    let contextMenuY = $state(0);

    // WebSocket event unlisteners
    let unlisteners: Array<() => void> = [];
    
    // Track pending playback (videos waiting to start after download)
    let pendingPlayback = $state<Set<string>>(new Set());

    onMount(async () => {
        // Check connection status
        try {
            const isConnected = await invoke("is_connected");
            if (!isConnected) {
                goto("/");
                return;
            }
            $connected = true;

            // Get client info
            const info = (await invoke("get_client_info")) as ClientInfo;
            $clientInfo = info;

            // Load config
            try {
                const config = (await invoke("get_config")) as Config;
                videoStoragePath = config.video_storage_path || "";
                mpvBinaryPath = config.mpv_binary_path || "";
            } catch (error) {
                console.error("Failed to load config:", error);
            }
        } catch (error) {
            console.error("Connection check failed:", error);
            goto("/");
            return;
        }

        // Set up WebSocket event listeners first
        await setupEventListeners();

        // Then request current state from server to ensure we have the latest data
        // This handles the race condition where initial messages were sent before listeners were ready
        try {
            await invoke("send_message", {
                messageType: "request_state",
                data: null,
            });
        } catch (error) {
            console.error("Failed to request state:", error);
        }

        // Check which videos are already downloaded
        await checkDownloadedVideos();
    });

    async function checkDownloadedVideos() {
        // Wait a moment for playlist to load
        await new Promise((resolve) => setTimeout(resolve, 500));

        const downloaded = new Map<string, string>();

        for (const video of $playlist) {
            try {
                const localPath = (await invoke("check_video_downloaded", {
                    filename: video.filename,
                })) as string;

                if (localPath) {
                    downloaded.set(video.id, localPath);
                }
            } catch (error) {
                console.error(
                    `Failed to check video ${video.filename}:`,
                    error,
                );
            }
        }

        $downloadedVideos = downloaded;
    }

    onDestroy(() => {
        unlisteners.forEach((unlisten) => unlisten());
    });

    async function setupEventListeners() {
        // User updates
        const unlistenUserUpdate = await listen<ServerMessage>(
            "ws-user-update",
            (event) => {
                if (event.payload.type === "user_update") {
                    $users = event.payload.users;
                }
            },
        );
        unlisteners.push(unlistenUserUpdate);

        // Permissions updates
        const unlistenPermissions = await listen<ServerMessage>(
            "ws-permissions-update",
            (event) => {
                if (event.payload.type === "permissions_update") {
                    const permsMap = new Map();
                    event.payload.permissions.forEach((p) => {
                        permsMap.set(p.user_id, p);
                    });
                    $permissions = permsMap;
                }
            },
        );
        unlisteners.push(unlistenPermissions);

        // Playlist updates
        const unlistenPlaylist = await listen<ServerMessage>(
            "ws-playlist-update",
            (event) => {
                if (event.payload.type === "playlist_update") {
                    $playlist = event.payload.videos;
                    $currentVideoIndex = event.payload.current_index;
                }
            },
        );
        unlisteners.push(unlistenPlaylist);

        // Activity log updates
        const unlistenActivityLog = await listen<ServerMessage>(
            "ws-activity-log",
            (event) => {
                if (event.payload.type === "activity_log") {
                    $activityLog = event.payload.logs;
                }
            },
        );
        unlisteners.push(unlistenActivityLog);

        // Client log updates
        const unlistenClientLog = await listen("client-log", (event) => {
            // Client logs come in correct format already
            const log = event.payload as any;
            $activityLog = [...$activityLog, log];
        });
        unlisteners.push(unlistenClientLog);

        // Download progress updates from server (broadcast to all clients)
        const unlistenServerDownloadProgress = await listen<ServerMessage>(
            "ws-download-progress",
            (event) => {
                if (event.payload.type === "download_progress") {
                    const serverProgress = event.payload as any;
                    const progress = {
                        client_id: serverProgress.client_id,
                        video_id: serverProgress.video_id,
                        filename: serverProgress.filename,
                        downloaded: serverProgress.downloaded,
                        total: serverProgress.total,
                        progress: serverProgress.progress,
                        speed: serverProgress.speed,
                        speed_display: serverProgress.speed_display,
                    };
                    
                    // Use composite key: client_id-video_id
                    const key = `${progress.client_id}-${progress.video_id}`;
                    $downloadProgress.set(key, progress);
                    $downloadProgress = $downloadProgress;
                    
                    // Clean up completed downloads after delay
                    if (progress.progress >= 100) {
                        setTimeout(() => {
                            $downloadProgress.delete(key);
                            $downloadProgress = $downloadProgress;
                        }, 2000);
                    }
                }
            },
        );
        unlisteners.push(unlistenServerDownloadProgress);

        // Local download progress updates (for immediate feedback before server broadcast)
        const unlistenDownloadProgress = await listen<DownloadProgress>(
            "download-progress",
            async (event) => {
                const progress = event.payload;
                // Also store with composite key for consistency
                const key = `${$clientInfo.client_id}-${progress.video_id}`;
                $downloadProgress.set(key, {
                    ...progress,
                    client_id: $clientInfo.client_id || "",
                });
                $downloadProgress = $downloadProgress;
                
                // When download completes (progress >= 100), clean up and start playback
                if (progress.progress >= 100) {
                    console.log("Download complete for:", progress.video_id);
                    
                    // Remove from downloading set
                    $downloadingVideos.delete(progress.video_id);
                    $downloadingVideos = $downloadingVideos;
                    
                    // Get the video info from the playlist
                    const video = $playlist.find(v => v.id === progress.video_id);
                    if (video) {
                        // Construct the expected file path
                        const localPath = `${videoStoragePath}/${video.filename}`;
                        
                        // Verify the file actually exists before adding to downloaded
                        try {
                            const verifiedPath = await invoke("check_video_downloaded", {
                                filename: video.filename,
                            }) as string | null;
                            
                            if (verifiedPath) {
                                console.log("Verified downloaded file at:", verifiedPath);
                                $downloadedVideos.set(progress.video_id, verifiedPath);
                                $downloadedVideos = $downloadedVideos;
                                
                                // If this video was pending playback, start it now
                                if (pendingPlayback.has(progress.video_id)) {
                                    console.log("Starting playback for:", video.filename);
                                    pendingPlayback.delete(progress.video_id);
                                    pendingPlayback = pendingPlayback;
                                    
                                    try {
                                        await invoke("start_mpv", { videoPath: verifiedPath });
                                        console.log("MPV started successfully");
                                    } catch (error) {
                                        console.error("Failed to start MPV:", error);
                                    }
                                }
                            } else {
                                console.error("File verification failed - file doesn't exist:", localPath);
                            }
                        } catch (error) {
                            console.error("Error verifying downloaded file:", error);
                        }
                    }
                    
                    // Set ready status
                    invoke("send_message", {
                        messageType: "ready",
                        data: true,
                    }).catch((error) => {
                        console.error("Failed to set ready status:", error);
                    });
                    
                    // Clean up local progress after a short delay (server broadcast will handle the key)
                    setTimeout(() => {
                        const localKey = `${$clientInfo.client_id}-${progress.video_id}`;
                        $downloadProgress.delete(localKey);
                        $downloadProgress = $downloadProgress;
                    }, 2000);
                }
            },
        );
        unlisteners.push(unlistenDownloadProgress);

        // Play/Pause/Seek handlers
        const unlistenPlay = await listen("ws-play", async () => {
            try {
                await invoke("mpv_play");
            } catch (error) {
                console.error("Failed to play:", error);
            }
        });
        unlisteners.push(unlistenPlay);

        const unlistenPause = await listen("ws-pause", async () => {
            try {
                await invoke("mpv_pause");
            } catch (error) {
                console.error("Failed to pause:", error);
            }
        });
        unlisteners.push(unlistenPause);

        const unlistenSeek = await listen<number>("ws-seek", async (event) => {
            try {
                await invoke("mpv_seek", { position: event.payload });
            } catch (error) {
                console.error("Failed to seek:", error);
            }
        });
        unlisteners.push(unlistenSeek);

        // Ownership transferred
        const unlistenOwnership = await listen<ServerMessage>(
            "ws-ownership-transferred",
            async (event) => {
                if (event.payload.type === "ownership_transferred") {
                    const info = (await invoke(
                        "get_client_info",
                    )) as ClientInfo;
                    $clientInfo = info;
                }
            },
        );
        unlisteners.push(unlistenOwnership);

        const unlistenError = await listen<string>("ws-error", (event) => {
            console.error("Server error:", event.payload);
        });
        unlisteners.push(unlistenError);

        const unlistenDisconnected = await listen("ws-disconnected", () => {
            resetState();
            goto("/");
        });
        unlisteners.push(unlistenDisconnected);
    }

    async function setUsername() {
        if (!username.trim()) return;

        try {
            await invoke("send_message", {
                messageType: "set_username",
                data: username,
            });
        } catch (error) {
            console.error("Failed to set username:", error);
        }
    }

    async function disconnect() {
        try {
            await invoke("disconnect");
            resetState();
            goto("/");
        } catch (error) {
            console.error("Failed to disconnect:", error);
        }
    }

    async function uploadVideo() {
        if (!$clientInfo.is_owner) {
            console.error("Only the owner can upload videos");
            return;
        }

        try {
            const filters = [
                ["Video", ["mp4", "mkv", "avi", "mov", "webm", "flv", "wmv"]],
            ];

            const filePath = await invoke("pick_file", { filters });

            if (filePath) {
                await invoke("upload_video", {
                    filePath: filePath,
                });
            }
        } catch (error) {
            console.error("Upload failed:", error);
        }
    }

    async function handlePlaylistClick(videoId: string, filename: string) {
        // Prevent multiple clicks while downloading
        if ($downloadingVideos.has(videoId)) {
            console.log("Already downloading, please wait...");
            return;
        }

        try {
            // Check if video is already downloaded
            let localPath = $downloadedVideos.get(videoId);

            if (!localPath) {
                // Check if it exists on disk
                localPath = await invoke("check_video_downloaded", {
                    filename,
                });

                if (localPath) {
                    console.log("Found video on disk:", localPath);
                    // Found on disk, add to store
                    $downloadedVideos = $downloadedVideos.set(
                        videoId,
                        localPath,
                    );
                } else {
                    // Need to download first
                    console.log("Video not found locally, starting download...");
                    
                    // Add to downloading set
                    $downloadingVideos = $downloadingVideos.add(videoId);
                    
                    // Add to pending playback so we know to start MPV when download completes
                    pendingPlayback.add(videoId);
                    pendingPlayback = pendingPlayback;
                    
                    // Start download (returns immediately, download happens in background)
                    await invoke("download_video_to_storage", {
                        videoId,
                        filename,
                    });

                    console.log("Download initiated, will start playback when complete");
                    
                    // Exit - download progress listener will start MPV when ready
                    return;
                }
            }

            // Video is downloaded, verify it exists before starting MPV
            console.log("Attempting to start MPV with:", localPath);
            
            // Double-check the file exists
            const exists = await invoke("check_video_downloaded", {
                filename,
            });
            
            if (!exists) {
                throw new Error(`File not found at expected location: ${localPath}`);
            }
            
            // Start MPV with local file
            await invoke("start_mpv", { videoPath: localPath });
            console.log("MPV started successfully");
        } catch (error) {
            console.error("Failed to start video:", error);
            // Remove from downloading on error
            $downloadingVideos.delete(videoId);
            $downloadingVideos = $downloadingVideos;
            // Remove from pending playback
            pendingPlayback.delete(videoId);
            pendingPlayback = pendingPlayback;
            // Clear download progress
            $downloadProgress.delete(videoId);
            $downloadProgress = $downloadProgress;
        }
    }

    async function selectVideo(index: number) {
        if (!$clientInfo.is_owner) return;

        try {
            await invoke("send_message", {
                messageType: "select_video",
                data: index,
            });
        } catch (error) {
            console.error("Failed to select video:", error);
        }
    }

    async function downloadVideo(videoId: string, filename: string) {
        try {
            const savePath = await invoke("save_file", {
                defaultName: filename,
            });

            if (savePath) {
                await invoke("download_video", {
                    videoId,
                    savePath: savePath,
                });
            }
        } catch (error) {
            console.error("Download failed:", error);
        }
    }

    async function browseVideoPath() {
        try {
            const path = (await invoke("pick_folder")) as string;
            if (path) {
                videoStoragePath = path;
                await saveConfig();
            }
        } catch (error) {
            console.error("Failed to select folder:", error);
        }
    }

    async function browseMpvBinary() {
        try {
            const path = (await invoke("pick_file")) as string;
            if (path) {
                mpvBinaryPath = path;
                await saveConfig();
            }
        } catch (error) {
            console.error("Failed to select file:", error);
        }
    }

    async function saveConfig() {
        try {
            await invoke("set_config", {
                serverUrl: null,
                videoPath: videoStoragePath || null,
                mpvPath: mpvBinaryPath || null,
            });
        } catch (error) {
            console.error("Failed to save config:", error);
        }
    }

    function showContextMenu(event: CustomEvent) {
        if (!$clientInfo.is_owner) return;

        const { userId, event: mouseEvent } = event.detail;
        mouseEvent.preventDefault();
        contextMenuUserId = userId;
        contextMenuX = mouseEvent.clientX;
        contextMenuY = mouseEvent.clientY;
    }

    function hideContextMenu() {
        contextMenuUserId = null;
    }

    async function handleTransferOwnership(event: CustomEvent) {
        const { userId } = event.detail;

        try {
            await invoke("send_message", {
                messageType: "transfer_ownership",
                data: userId,
            });
            hideContextMenu();
        } catch (error) {
            console.error("Failed to transfer ownership:", error);
        }
    }

    async function handleTogglePermission(event: CustomEvent) {
        const { userId, permission, value } = event.detail;

        try {
            await invoke("send_message", {
                messageType: "set_permission",
                data: { client_id: userId, permission, value },
            });
        } catch (error) {
            console.error("Failed to set permission:", error);
        }
    }

    function formatTime(timestamp: string): string {
        const date = new Date(timestamp);
        return date.toLocaleTimeString();
    }

    function getUserDisplayName(user: any, index: number): string {
        return user.username || `User ${index + 1}`;
    }
    
    // Helper to get download progress for a specific user
    function getUserDownloadProgress(userId: string) {
        // Find any active downloads for this user
        for (const [key, progress] of $downloadProgress.entries()) {
            // Key format is "client_id-video_id"
            if (key.startsWith(userId + "-") && progress.progress > 0 && progress.progress < 100) {
                return {
                    progress: progress.progress,
                    speed: progress.speed_display,
                };
            }
        }
        
        return { progress: 0, speed: "" };
    }
</script>

<svelte:window onclick={hideContextMenu} />

<div class="layout">
    <div class="left-column">
        <div class="section user-settings">
            <h3>User Settings</h3>
            <div class="input-group">
                <input placeholder="Username" bind:value={username} />
                <button onclick={setUsername}>Set</button>
            </div>
        </div>

        <div class="section user-list">
            <h3>Connected Users</h3>
            <div class="user-list-content">
                {#each $users as user, index (user.id)}
                    {@const isSelf = user.id === $clientInfo.client_id}
                    {@const downloadInfo = getUserDownloadProgress(user.id)}
                    <UserItem
                        userId={user.id}
                        username={getUserDisplayName(user, index)}
                        status={user.status}
                        isOwner={user.is_owner}
                        {isSelf}
                        viewerIsOwner={$clientInfo.is_owner}
                        downloadProgress={downloadInfo.progress}
                        downloadSpeed={downloadInfo.speed}
                        on:contextmenu={showContextMenu}
                    />
                {/each}
            </div>
        </div>
        
        <HostControls isOwner={$clientInfo.is_owner} />

        <button class="disconnect-btn" onclick={disconnect}>Disconnect</button>
    </div>

    <div class="middle-column">
        <div class="status-header ok">
            <div class="status-content">
                <h2>Status: Connected</h2>
                <span class="ping">10ms</span>
            </div>
        </div>

        <div class="section settings-box">
            <h3>Settings</h3>
            <div class="input-group">
                <input
                    placeholder="Video storage location"
                    bind:value={videoStoragePath}
                />
                <button onclick={browseVideoPath}>Browse</button>
            </div>
            <div class="input-group">
                <input
                    placeholder="mpv binary location"
                    bind:value={mpvBinaryPath}
                />
                <button onclick={browseMpvBinary}>Browse</button>
            </div>
        </div>

        <div class="section log-box">
            <h3>Activity Log</h3>
            <div class="log-content">
                {#each $activityLog as log (log.timestamp)}
                    <div class="log-entry">
                        <span
                            class="log-user"
                            class:client-log={log.source === "client"}
                        >
                            {log.username || log.user_id.substring(0, 8)}
                        </span>
                        <span class="log-action">{log.action}</span>
                    </div>
                {/each}
            </div>
        </div>
    </div>

    <div class="right-column">
        <div class="section playlist">
            <h3>Playlist</h3>
            <div class="playlist-content">
                {#each $playlist as video, index (video.id)}
                    {@const activeDownload = $downloadProgress.get(video.id)}
                    <PlaylistItem
                        videoId={video.id}
                        filename={video.filename}
                        size={video.size_display}
                        isCurrent={index === $currentVideoIndex}
                        isDownloaded={$downloadedVideos.has(video.id)}
                        isDownloading={$downloadingVideos.has(video.id)}
                        downloadProgress={activeDownload?.progress || 0}
                        on:play={() =>
                            handlePlaylistClick(video.id, video.filename)}
                    />
                {/each}
            </div>
            {#if $clientInfo.is_owner}
                <button class="add-video-btn" onclick={uploadVideo}>
                    + Add Video
                </button>
            {/if}
        </div>
    </div>
</div>

{#if contextMenuUserId}
    {@const user = $users.find((u) => u.id === contextMenuUserId)}
    {@const userIndex = $users.findIndex((u) => u.id === contextMenuUserId)}
    {@const perms = $permissions.get(contextMenuUserId)}
    {#if user && perms}
        <ContextMenu
            userId={user.id}
            username={getUserDisplayName(user, userIndex)}
            x={contextMenuX}
            y={contextMenuY}
            allowPause={perms.allow_pause}
            allowSeek={perms.allow_seek}
            on:transferOwnership={handleTransferOwnership}
            on:togglePermission={handleTogglePermission}
        />
    {/if}
{/if}

<style>
    :root {
        --bg-primary: #1a1a1a;
        --bg-secondary: #242424;
        --bg-tertiary: #0f0f0f;
        --text-primary: #f6f6f6;
        --text-secondary: #a0a0a0;
        --border-color: #404040;
        --accent-blue: #396cd8;
        --accent-green: #4caf50;
        --accent-orange: #ff9800;
        --accent-red: #f44336;
        --accent-yellow: #ffc107;
        --shadow: rgba(0, 0, 0, 0.3);
    }

    .layout {
        display: flex;
        height: 100vh;
        gap: 1rem;
        padding: 1rem;
        background-color: var(--bg-primary);
        color: var(--text-primary);
        overflow: hidden;
        box-sizing: border-box;
    }

    .left-column,
    .right-column {
        width: 300px;
        display: flex;
        flex-direction: column;
        gap: 1rem;
        min-height: 0;
    }

    .middle-column {
        flex: 1;
        display: flex;
        flex-direction: column;
        gap: 1rem;
        min-width: 0;
        min-height: 0;
    }

    .section {
        background-color: var(--bg-secondary);
        border: 1px solid var(--border-color);
        border-radius: 8px;
        padding: 1rem;
        display: flex;
        flex-direction: column;
        min-height: 0;
    }

    h2 {
        margin: 0;
        font-size: 1.25rem;
        font-weight: 500;
    }

    h3 {
        margin: 0 0 1rem 0;
        font-size: 1rem;
        font-weight: 500;
        color: var(--text-secondary);
        text-transform: uppercase;
        letter-spacing: 0.5px;
        flex-shrink: 0;
    }

    .status-header {
        background-color: var(--bg-secondary);
        border: 2px solid var(--accent-orange);
        border-radius: 8px;
        padding: 1.5rem;
        flex-shrink: 0;
    }

    .status-header.ok {
        border-color: var(--accent-orange);
    }

    .status-content {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 0.25rem;
    }

    .ping {
        font-size: 0.875rem;
        color: var(--text-secondary);
    }

    .input-group {
        display: flex;
        gap: 0.5rem;
        margin-bottom: 0.75rem;
    }

    .input-group:last-child {
        margin-bottom: 0;
    }

    .input-group input {
        flex: 1;
    }

    input,
    button {
        border-radius: 6px;
        border: 1px solid var(--border-color);
        padding: 0.5rem 0.75rem;
        font-size: 0.875rem;
        font-family: inherit;
        color: var(--text-primary);
        background-color: var(--bg-tertiary);
        transition: all 0.2s;
    }

    button {
        cursor: pointer;
        font-weight: 500;
    }

    button:hover {
        border-color: var(--accent-blue);
        background-color: var(--bg-secondary);
    }

    .user-settings {
        flex-shrink: 0;
    }

    .user-list {
        flex: 1;
        min-height: 0;
    }

    .user-list-content {
        flex: 1;
        overflow-y: auto;
        min-height: 0;
    }

    .disconnect-btn {
        width: 100%;
        background-color: var(--accent-red);
        border-color: var(--accent-red);
        color: var(--text-primary);
        flex-shrink: 0;
    }

    .disconnect-btn:hover {
        background-color: #d32f2f;
        border-color: #d32f2f;
    }

    .settings-box {
        flex-shrink: 0;
    }

    .log-box {
        flex: 1;
        min-height: 0;
    }

    .log-content {
        flex: 1;
        overflow-y: auto;
        font-size: 0.875rem;
        min-height: 0;
    }

    .log-entry {
        padding: 0.5rem;
        border-bottom: 1px solid var(--border-color);
    }

    .log-user {
        color: var(--accent-blue);
        font-weight: 500;
    }

    .log-user.client-log {
        color: var(--accent-orange);
    }

    .log-action {
        color: var(--text-secondary);
    }

    .playlist {
        flex: 1;
        min-height: 0;
    }

    .playlist-content {
        flex: 1;
        overflow-y: auto;
        margin-bottom: 1rem;
        min-height: 0;
    }

    .add-video-btn {
        width: 100%;
        background-color: var(--accent-blue);
        border-color: var(--accent-blue);
        color: var(--text-primary);
        flex-shrink: 0;
    }

    .add-video-btn:hover {
        background-color: #2e5bb8;
        border-color: #2e5bb8;
    }

    @media (prefers-color-scheme: light) {
        :root {
            --bg-primary: #f6f6f6;
            --bg-secondary: #ffffff;
            --bg-tertiary: #f0f0f0;
            --text-primary: #0f0f0f;
            --text-secondary: #666666;
            --border-color: #e0e0e0;
            --shadow: rgba(0, 0, 0, 0.1);
        }

        input,
        button {
            box-shadow: 0 1px 3px var(--shadow);
        }

        .section {
            box-shadow: 0 1px 3px var(--shadow);
        }
    }
</style>