<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";

    export let isOwner: boolean = false;

    let seekAmount = 10; // seconds to seek forward/backward

    async function handlePlay() {
        try {
            // mpv_play already sends to server, no need for send_message
            await invoke("mpv_play");
        } catch (error) {
            console.error("Failed to play:", error);
        }
    }

    async function handlePause() {
        try {
            // mpv_pause already sends to server, no need for send_message
            await invoke("mpv_pause");
        } catch (error) {
            console.error("Failed to pause:", error);
        }
    }

    async function handleSeekBackward() {
        try {
            // Note: mpv_seek_relative does NOT send to server
            // For now, we only seek the host. To sync all clients, we'd need to:
            // 1. Get current position, 2. Calculate new position, 3. Use mpv_seek
            await invoke("mpv_seek_relative", { offset: -seekAmount });
        } catch (error) {
            console.error("Failed to seek backward:", error);
        }
    }

    async function handleSeekForward() {
        try {
            // Note: mpv_seek_relative does NOT send to server
            await invoke("mpv_seek_relative", { offset: seekAmount });
        } catch (error) {
            console.error("Failed to seek forward:", error);
        }
    }

    async function handleRecalibrate() {
        try {
            // Pause everyone (mpv_pause sends to server automatically)
            await invoke("mpv_pause");

            // Wait for everyone to pause
            await new Promise((resolve) => setTimeout(resolve, 200));

            // For recalibration, we need to get the host's current position
            // and then seek everyone to (position - 5)
            // Since we don't have a working mpv_get_time_pos yet,
            // we'll use a workaround: seek to a small negative offset

            // This only seeks the host, not others (since mpv_seek_relative doesn't broadcast)
            await invoke("mpv_seek_relative", { offset: -5 });

            // To sync others, we need to send an absolute seek command
            // But we don't have the current position, so this is a limitation
            // For now, tell users to manually sync or implement proper time tracking

            console.warn(
                "Recalibrate: Host seeked back 5s. Other clients not synced (requires time position tracking)",
            );
        } catch (error) {
            console.error("Failed to recalibrate:", error);
        }
    }
</script>

{#if isOwner}
    <div class="host-controls">
        <h3>Host Controls</h3>
        <div class="controls-widget">
            <button
                class="control-icon-btn"
                onclick={handleSeekBackward}
                title="Seek backward {seekAmount}s (host only)"
            >
                <svg
                    xmlns="http://www.w3.org/2000/svg"
                    width="20"
                    height="20"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                >
                    <polygon points="11 19 2 12 11 5 11 19"></polygon>
                    <polygon points="22 19 13 12 22 5 22 19"></polygon>
                </svg>
                <span class="seek-label">-{seekAmount}s</span>
            </button>

            <button
                class="control-icon-btn play-btn"
                onclick={handlePlay}
                title="Play"
            >
                <svg
                    xmlns="http://www.w3.org/2000/svg"
                    width="24"
                    height="24"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                >
                    <polygon points="5 3 19 12 5 21 5 3"></polygon>
                </svg>
            </button>

            <button
                class="control-icon-btn pause-btn"
                onclick={handlePause}
                title="Pause"
            >
                <svg
                    xmlns="http://www.w3.org/2000/svg"
                    width="24"
                    height="24"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                >
                    <rect x="6" y="4" width="4" height="16"></rect>
                    <rect x="14" y="4" width="4" height="16"></rect>
                </svg>
            </button>

            <button
                class="control-icon-btn"
                onclick={handleSeekForward}
                title="Seek forward {seekAmount}s (host only)"
            >
                <svg
                    xmlns="http://www.w3.org/2000/svg"
                    width="20"
                    height="20"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                >
                    <polygon points="13 19 22 12 13 5 13 19"></polygon>
                    <polygon points="2 19 11 12 2 5 2 19"></polygon>
                </svg>
                <span class="seek-label">+{seekAmount}s</span>
            </button>

            <button
                class="control-icon-btn recalibrate"
                onclick={handleRecalibrate}
                title="Re-calibrate (pause all, seek host back 5s)"
            >
                <svg
                    xmlns="http://www.w3.org/2000/svg"
                    width="20"
                    height="20"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                >
                    <polyline points="23 4 23 10 17 10"></polyline>
                    <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"></path>
                </svg>
            </button>
        </div>
    </div>
{/if}

<style>
    .host-controls {
        background-color: var(--bg-secondary);
        border: 1px solid var(--border-color);
        border-radius: 8px;
        padding: 0.75rem;
        flex-shrink: 0;
    }

    h3 {
        margin: 0 0 0.625rem 0;
        font-size: 0.875rem;
        font-weight: 500;
        color: var(--text-secondary);
        text-transform: uppercase;
        letter-spacing: 0.5px;
    }

    .controls-widget {
        display: grid;
        grid-template-columns: 1fr 1fr 1fr 1fr 1fr;
        gap: 0.5rem;
    }

    .control-icon-btn {
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        gap: 0.25rem;
        padding: 0.625rem;
        background-color: var(--bg-tertiary);
        border: 1px solid var(--border-color);
        border-radius: 6px;
        cursor: pointer;
        transition: all 0.2s;
        color: var(--text-primary);
        font-family: inherit;
    }

    .control-icon-btn:hover {
        background-color: var(--bg-secondary);
        border-color: var(--accent-blue);
        transform: translateY(-1px);
    }

    .control-icon-btn:active {
        transform: translateY(0);
    }

    .control-icon-btn.play-btn {
        background-color: var(--accent-green);
        border-color: var(--accent-green);
        color: white;
    }

    .control-icon-btn.play-btn:hover {
        background-color: #45a049;
        border-color: #45a049;
    }

    .control-icon-btn.pause-btn {
        background-color: var(--accent-orange);
        border-color: var(--accent-orange);
        color: white;
    }

    .control-icon-btn.pause-btn:hover {
        background-color: #e68900;
        border-color: #e68900;
    }

    .control-icon-btn.recalibrate {
        background-color: var(--accent-red);
        border-color: var(--accent-red);
        color: white;
    }

    .control-icon-btn.recalibrate:hover {
        background-color: #d32f2f;
        border-color: #d32f2f;
    }

    .seek-label {
        font-size: 0.625rem;
        font-weight: 600;
        letter-spacing: 0.5px;
    }

    .control-icon-btn svg {
        flex-shrink: 0;
    }
</style>
